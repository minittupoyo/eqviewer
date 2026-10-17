// jp-city / jp-pref / jp-area の GeoJSON を事前にベクタタイル化し、1ファイルにまとめる。
// 実行: bun tools/build-tiles.ts
// 出力: public/map/<name>.pack(gzip済みタイルの連結)と <name>.pack.json({"z/x/y":[offset,length]})
import geojsonvt from "geojson-vt";
import { fromGeojsonVt } from "vt-pbf";
import { gzipSync } from "node:zlib";
import { readFileSync, writeFileSync } from "node:fs";

const MAX_ZOOM = 10;
// 描画と震度の対応づけに使う属性だけ残す
const KEEP: Record<string, string[]> = { "jp-city": ["regioncode"], "jp-pref": [], "jp-area": ["name"] };

for (const [name, keep] of Object.entries(KEEP)) {
  const src = JSON.parse(readFileSync(`public/map/${name}.geojson`, "utf8"));
  for (const f of src.features) f.properties = Object.fromEntries(keep.map((k) => [k, f.properties[k]]));
  const layer = name.slice(3);
  const index = new (geojsonvt as any)(src, { maxZoom: MAX_ZOOM, indexMaxZoom: MAX_ZOOM, indexMaxPoints: 0, tolerance: 2, buffer: 64 });
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
    const tile = index.getTile(z, x, y);
    if (!tile || !tile.features.length) continue;
    const buf = gzipSync(Buffer.from(fromGeojsonVt({ [layer]: tile }, { version: 2 })));
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
