import { addProtocol } from "maplibre-gl";

/**
 * `tools/build-tiles.ts` が作る事前ベクタタイルのパック(`public/map/<name>.pack`)を読む。
 * 1ファイルを一度だけ取得してメモリに持ち、`pack://<name>/{z}/{x}/{y}` のタイル要求に答える。
 * 地図が複数あっても取得は共有される。
 */
interface Pack {
  toc: Record<string, [number, number]>;
  data: Promise<ArrayBuffer>;
}
const packs = new Map<string, Promise<Pack>>();

const load = (name: string) => {
  let p = packs.get(name);
  if (!p) {
    p = fetch(`/map/${name}.pack.json`)
      .then((r) => r.json())
      .then((toc) => ({ toc, data: fetch(`/map/${name}.pack`).then((r) => r.arrayBuffer()) }));
    packs.set(name, p);
  }
  return p;
};

export function registerTilePack() {
  addProtocol("pack", async (params) => {
    const [name, z, x, y] = params.url.slice("pack://".length).split("/");
    const { toc, data } = await load(name);
    const hit = toc[`${z}/${x}/${y}`];
    // 範囲外・空のタイルは空として返す
    if (!hit) return { data: new ArrayBuffer(0) };
    const gz = new Blob([(await data).slice(hit[0], hit[0] + hit[1])]);
    return { data: await new Response(gz.stream().pipeThrough(new DecompressionStream("gzip"))).arrayBuffer() };
  });
}
