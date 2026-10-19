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

const HOME_ICON = "home-house";

/** 自宅地点のレイヤー定義。観測点(丸)と見間違えないよう、家の形にする。 */
export const homeLayers = (source: string): LayerSpecification[] => [
  {
    id: `${source}-house`,
    type: "symbol",
    source,
    layout: { "icon-image": HOME_ICON, "icon-allow-overlap": true, "icon-ignore-placement": true },
  },
];

const ICON_RATIO = 2;

/** 白縁つきの図形を描いた画像を、地図が必要としたときに登録する */
function registerIcon(map: MapLibreMap, id: string, size: number, draw: (g: CanvasRenderingContext2D, px: number) => void) {
  map.on("styleimagemissing", (e) => {
    if (e.id !== id || map.hasImage(id)) return;
    const px = size * ICON_RATIO;
    const c = document.createElement("canvas");
    c.width = c.height = px;
    const g = c.getContext("2d")!;
    g.lineCap = g.lineJoin = "round";
    draw(g, px);
    map.addImage(id, g.getImageData(0, 0, px, px), { pixelRatio: ICON_RATIO });
  });
}

/** 地図で使うアイコン(震央の×、自宅の家)を登録する */
export function registerMapIcons(map: MapLibreMap) {
  registerIcon(map, EPICENTER_ICON, 28, (g, px) => {
    const m = px * (6 / 28);
    for (const [width, color] of [[px / 4, "#ffffff"], [px / 8, "#dc2626"]] as const) {
      g.lineWidth = width;
      g.strokeStyle = color;
      g.beginPath();
      g.moveTo(m, m);
      g.lineTo(px - m, px - m);
      g.moveTo(px - m, m);
      g.lineTo(m, px - m);
      g.stroke();
    }
  });
  registerIcon(map, HOME_ICON, 26, (g, px) => {
    const u = px / 26;
    g.beginPath();
    g.moveTo(13 * u, 3 * u);
    g.lineTo(23 * u, 12 * u);
    g.lineTo(20 * u, 12 * u);
    g.lineTo(20 * u, 22 * u);
    g.lineTo(6 * u, 22 * u);
    g.lineTo(6 * u, 12 * u);
    g.lineTo(3 * u, 12 * u);
    g.closePath();
    g.lineWidth = 3.5 * u;
    g.strokeStyle = "#ffffff";
    g.stroke();
    g.fillStyle = "#16a34a";
    g.fill();
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
