// jp-city / jp-pref / jp-area の GeoJSON を事前にベクタタイル化し、1ファイルにまとめる。
// 実行: bun tools/build-tiles.ts
// 出力: public/map/<name>.pack(gzip済みタイルの連結)と <name>.pack.json({"z/x/y":[offset,length]})
import geojsonvt from "geojson-vt";
import { fromGeojsonVt } from "vt-pbf";
import earcut, { deviation } from "earcut";
import { gzipSync } from "node:zlib";
import { readFileSync, writeFileSync } from "node:fs";

const MAX_ZOOM = 10;
// 簡略化の強さの候補。タイルごとに、塗りの三角形分割が破綻しないものを順に試す。
// (簡略化・クリップで輪が自己交差すると MapLibre の earcut が失敗し、その地域の塗りが消える)
// 強くしすぎると隣り合う市町村の境界が別々に簡略化されて隙間が出る。タイルごとに強さが違うと
// タイルの境目にも隙間が出るので、強い値(12, 24)は小さい値がすべて破綻したタイルだけで使う。
const TOLERANCES = [6, 0, 2, 4, 8, 12, 24];
const SMALL = 5; // TOLERANCES のうち小さい値の数
// 描画と震度の対応づけに使う属性だけ残す
const KEEP: Record<string, string[]> = { "jp-city": ["regioncode"], "jp-pref": [], "jp-area": ["name"] };

for (const [name, keep] of Object.entries(KEEP)) {
  const src = JSON.parse(readFileSync(`public/map/${name}.geojson`, "utf8"));
  for (const f of src.features) f.properties = Object.fromEntries(keep.map((k) => [k, f.properties[k]]));
  const layer = name.slice(3);
  const indexes = TOLERANCES.map(
    (tolerance) => new (geojsonvt as any)(src, { maxZoom: MAX_ZOOM, indexMaxZoom: MAX_ZOOM, indexMaxPoints: 0, tolerance, buffer: 128, extent: 8192 }),
  );
  const chunks: Buffer[] = [];
  const toc: Record<string, [number, number]> = {};
  let offset = 0;
  const [w, south, e, n] = bboxOf(src);
  const tx = (lon: number, z: number) => Math.floor(((lon + 180) / 360) * 2 ** z);
  const ty = (lat: number, z: number) => {
    const r = (lat * Math.PI) / 180;
    return Math.floor(((1 - Math.log(Math.tan(r) + 1 / Math.cos(r)) / Math.PI) / 2) * 2 ** z);
  };
  const coords: [number, number, number][] = [];
  for (let z = 0; z <= MAX_ZOOM; z++)
    for (let x = tx(w, z); x <= tx(e, z); x++) for (let y = ty(n, z); y <= ty(south, z); y++) coords.push([z, x, y]);
  for (const [z, x, y] of coords) {
    const tile = pickTile(indexes, z, x, y);
    if (!tile || !tile.features.length) continue;
    const buf = gzipSync(Buffer.from(fromGeojsonVt({ [layer]: tile }, { version: 2, extent: 8192 })));
    toc[`${z}/${x}/${y}`] = [offset, buf.length];
    chunks.push(buf);
    offset += buf.length;
  }
  writeFileSync(`public/map/${name}.pack`, Buffer.concat(chunks));
  writeFileSync(`public/map/${name}.pack.json`, JSON.stringify(toc));
  console.log(name, Object.keys(toc).length, "tiles", (offset / 1e6).toFixed(1), "MB");
}

function bboxOf(fc: any): [number, number, number, number] {
  const b: [number, number, number, number] = [180, 90, -180, -90];
  const visit = (c: any) => {
    if (typeof c[0] === "number") {
      b[0] = Math.min(b[0], c[0]); b[1] = Math.min(b[1], c[1]); b[2] = Math.max(b[2], c[0]); b[3] = Math.max(b[3], c[1]);
    } else c.forEach(visit);
  };
  for (const f of fc.features) visit(f.geometry.coordinates);
  return b;
}

function signedArea(r: number[][]) {
  let a = 0;
  for (let i = 0, j = r.length - 1; i < r.length; j = i++) a += (r[j][0] - r[i][0]) * (r[i][1] + r[j][1]);
  return a;
}

/** 塗りの三角形分割のずれ(最悪値)。0に近いほど正しく塗れる。 */
function fillError(tile: any) {
  let worst = 0;
  if (!tile) return worst;
  for (const f of tile.features) {
    if (f.type !== 3) continue;
    const polys: number[][][][] = [];
    let outer = 0;
    for (const ring of f.geometry) {
      const a = signedArea(ring);
      if (!a) continue;
      if (!polys.length || Math.sign(a) === outer) {
        outer = Math.sign(a);
        polys.push([ring]);
      } else polys[polys.length - 1].push(ring);
    }
    for (const p of polys) {
      const flat: number[] = [];
      const holes: number[] = [];
      p.forEach((r, i) => {
        if (i) holes.push(flat.length / 2);
        for (const [px, py] of r) flat.push(px, py);
      });
      worst = Math.max(worst, deviation(flat, holes, 2, earcut(flat, holes)));
    }
  }
  return worst;
}

function pickTile(indexes: any[], z: number, x: number, y: number) {
  let best: any = null;
  let bestErr = Infinity;
  // 低ズームでは強い値の隙間が目立つうえ、破綻するのは元データの不正な輪(名古屋港区など)なので使わない
  for (const index of z < 7 ? indexes.slice(0, SMALL) : indexes) {
    const tile = index.getTile(z, x, y);
    const err = fillError(tile);
    if (err < 1e-2) return tile;
    if (err < bestErr) [best, bestErr] = [tile, err];
  }
  return best;
}
