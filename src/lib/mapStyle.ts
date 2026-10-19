import type {
  ExpressionSpecification,
  Map as MapLibreMap,
  LayerSpecification,
  SourceSpecification,
  StyleSpecification,
} from "maplibre-gl";
import { SCALE_COLOR } from "@/lib/map";

/** 市町村の `feature-state.scale` に応じた塗り色 */
const scaleColorExpr = [
  "match",
  ["feature-state", "scale"],
  ...Object.entries(SCALE_COLOR).flat(),
  "rgba(0,0,0,0)",
] as ExpressionSpecification;

/** 世界・市町村・都道府県境の共通ベースマップ。`overlay` で地図ごとのレイヤーを上に重ねる。 */
export function buildStyle(
  overlay: { sources: Record<string, SourceSpecification>; layers: LayerSpecification[] },
): StyleSpecification {
  return {
    version: 8,
    sources: {
      world: { type: "geojson", data: "/map/world.geojson", maxzoom: 8 },
      city: { type: "vector", tiles: ["pack://jp-city/{z}/{x}/{y}"], maxzoom: 10, promoteId: "regioncode" },
      pref: { type: "vector", tiles: ["pack://jp-pref/{z}/{x}/{y}"], maxzoom: 10 },
      ...overlay.sources,
    },
    layers: [
      // 震度1(淡い灰)・震度2(水色)や強震モニタの青い点と区別できる、中間の明るさの青灰色
      { id: "sea", type: "background", paint: { "background-color": "#8199b3" } },
      { id: "world", type: "fill", source: "world", paint: { "fill-color": "#f3f1ea", "fill-outline-color": "#b8b4a8" } },
      // 世界地図は簡略化で小さな島を落とすため、国内は市町村の形でも陸地を塗る
      { id: "city-land", type: "fill", source: "city", "source-layer": "city", paint: { "fill-color": "#f3f1ea" } },
      { id: "city", type: "fill", source: "city", "source-layer": "city", paint: { "fill-color": scaleColorExpr } },
      { id: "city-line", type: "line", source: "city", "source-layer": "city", minzoom: 6, paint: { "line-color": "#9a968a", "line-width": 0.4 } },
      { id: "pref-line", type: "line", source: "pref", "source-layer": "pref", paint: { "line-color": "#7a766a", "line-width": 0.8 } },
      ...overlay.layers,
    ],
  };
}

export const emptyCollection = { type: "FeatureCollection" as const, features: [] };

const EPICENTER_ICON = "epicenter-x";

/** 震央マーカー(白縁つき赤い×)のレイヤー定義。画像は `registerEpicenterIcon` で登録する。 */
export const epicenterLayers = (source: string): LayerSpecification[] => [
  {
    id: `${source}-x`,
    type: "symbol",
    source,
    layout: { "icon-image": EPICENTER_ICON, "icon-size": 1, "icon-allow-overlap": true, "icon-ignore-placement": true },
  },
];

/** 震央の×印を、地図が必要としたときに描いて登録する */
export function registerEpicenterIcon(map: MapLibreMap) {
  map.on("styleimagemissing", (e) => {
    if (e.id !== EPICENTER_ICON || map.hasImage(EPICENTER_ICON)) return;
    const ratio = 2;
    const size = 28 * ratio;
    const c = document.createElement("canvas");
    c.width = c.height = size;
    const g = c.getContext("2d")!;
    const m = 6 * ratio;
    g.lineCap = "round";
    for (const [width, color] of [[7 * ratio, "#ffffff"], [3.5 * ratio, "#dc2626"]] as const) {
      g.lineWidth = width;
      g.strokeStyle = color;
      g.beginPath();
      g.moveTo(m, m);
      g.lineTo(size - m, size - m);
      g.moveTo(size - m, m);
      g.lineTo(m, size - m);
      g.stroke();
    }
    map.addImage(EPICENTER_ICON, g.getImageData(0, 0, size, size), { pixelRatio: ratio });
  });
}

/** 震度速報の地域(細分区域)の色付け。`feature-state.scale` は地域名のキーで設定する。 */
export const areaOverlay: { sources: Record<string, SourceSpecification>; layers: LayerSpecification[] } = {
  sources: { area: { type: "vector", tiles: ["pack://jp-area/{z}/{x}/{y}"], maxzoom: 10, promoteId: "name" } },
  layers: [
    { id: "area", type: "fill", source: "area", "source-layer": "area", paint: { "fill-color": scaleColorExpr } },
    {
      id: "area-line",
      type: "line",
      source: "area",
      "source-layer": "area",
      paint: {
        "line-color": "#7a766a",
        "line-width": 0.8,
        // 震度のついた地域だけ縁を描く
        "line-opacity": ["case", ["to-boolean", ["feature-state", "scale"]], 1, 0],
      },
    },
  ],
};
