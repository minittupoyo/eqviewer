import type {
  ExpressionSpecification,
  LayerSpecification,
  SourceSpecification,
  StyleSpecification,
} from "maplibre-gl";
import { SCALE_COLOR, type CityCollection } from "@/lib/map";

/** 市町村の `feature-state.scale` に応じた塗り色 */
const scaleColorExpr = [
  "match",
  ["feature-state", "scale"],
  ...Object.entries(SCALE_COLOR).flat(),
  "rgba(0,0,0,0)",
] as ExpressionSpecification;

/** 世界・市町村・都道府県境の共通ベースマップ。`overlay` で地図ごとのレイヤーを上に重ねる。 */
export function buildStyle(
  cities: CityCollection,
  overlay: { sources: Record<string, SourceSpecification>; layers: LayerSpecification[] },
): StyleSpecification {
  return {
    version: 8,
    sources: {
      world: { type: "geojson", data: "/map/world.geojson" },
      city: { type: "geojson", data: cities, promoteId: "regioncode" },
      pref: { type: "geojson", data: "/map/jp-pref.geojson" },
      ...overlay.sources,
    },
    layers: [
      // 震度1(淡い灰)・震度2(水色)や強震モニタの青い点と区別できる、中間の明るさの青灰色
      { id: "sea", type: "background", paint: { "background-color": "#8199b3" } },
      { id: "world", type: "fill", source: "world", paint: { "fill-color": "#f3f1ea", "fill-outline-color": "#b8b4a8" } },
      // 世界地図は簡略化で小さな島を落とすため、国内は市町村の形でも陸地を塗る
      { id: "city-land", type: "fill", source: "city", paint: { "fill-color": "#f3f1ea" } },
      { id: "city", type: "fill", source: "city", paint: { "fill-color": scaleColorExpr } },
      { id: "city-line", type: "line", source: "city", minzoom: 6, paint: { "line-color": "#9a968a", "line-width": 0.4 } },
      { id: "pref-line", type: "line", source: "pref", paint: { "line-color": "#7a766a", "line-width": 0.8 } },
      ...overlay.layers,
    ],
  };
}

export const emptyCollection = { type: "FeatureCollection" as const, features: [] };

/** 震央マーカー(白縁つき赤丸)のレイヤー定義 */
export const epicenterLayers = (source: string): LayerSpecification[] => [
  { id: `${source}-halo`, type: "circle", source, paint: { "circle-radius": 9, "circle-color": "#ffffff" } },
  { id: `${source}-dot`, type: "circle", source, paint: { "circle-radius": 6, "circle-color": "#dc2626" } },
];

/** 震度速報の地域(細分区域)の色付け。`feature-state.scale` は地域名のキーで設定する。 */
export const areaOverlay: { sources: Record<string, SourceSpecification>; layers: LayerSpecification[] } = {
  sources: { area: { type: "geojson", data: "/map/jp-area.geojson", promoteId: "name" } },
  layers: [
    { id: "area", type: "fill", source: "area", paint: { "fill-color": scaleColorExpr } },
    {
      id: "area-line",
      type: "line",
      source: "area",
      paint: {
        "line-color": "#7a766a",
        "line-width": 0.8,
        // 震度のついた地域だけ縁を描く
        "line-opacity": ["case", ["to-boolean", ["feature-state", "scale"]], 1, 0],
      },
    },
  ],
};
