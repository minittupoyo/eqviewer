import type { ExpressionSpecification, LayerSpecification, SourceSpecification } from "maplibre-gl";
import { emptyCollection } from "@/lib/mapStyle";
import { KMONI_COLOR_STOPS } from "@/lib/kmoniColors";

/** 最新フレームがこの時間より古ければ「更新停止」とみなす */
export const KMONI_STALE_MS = 10_000;

const colorExpr = [
  "interpolate",
  ["linear"],
  ["get", "i"],
  ...KMONI_COLOR_STOPS.flat(),
] as ExpressionSpecification;

/** 強震モニタの観測点(リアルタイム震度)を重ねるソースとレイヤー */
export const kmoniOverlay: {
  sources: Record<string, SourceSpecification>;
  layers: LayerSpecification[];
} = {
  sources: { kmoni: { type: "geojson", data: emptyCollection } },
  layers: [
    {
      id: "kmoni-points",
      type: "circle",
      source: "kmoni",
      layout: { "circle-sort-key": ["get", "i"] },
      paint: {
        "circle-color": colorExpr,
        "circle-radius": ["interpolate", ["linear"], ["zoom"], 4, 3.2, 8, 7.5, 11, 11],
        "circle-stroke-color": "#ffffff",
        "circle-stroke-width": 0.7,
      },
    },
  ],
};

/** 凡例用のCSSグラデーション(下が-3、上が7) */
export const KMONI_LEGEND_GRADIENT = `linear-gradient(to top, ${KMONI_COLOR_STOPS.map(
  ([v, c]) => `${c} ${((v + 3) / 10) * 100}%`,
).join(", ")})`;
