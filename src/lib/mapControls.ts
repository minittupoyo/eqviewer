import type { IControl, Map as MapLibreMap } from "maplibre-gl";

/**
 * 強震モニタの観測点(南西諸島の枠を除く)がすべて収まる範囲 [西, 南], [東, 北]。
 * 実際の観測点の範囲(東経128.65〜145.80度、北緯30.26〜45.47度)に少し余裕を足している。
 * 南西諸島は別のミニマップで見せるので含めない。Rust側のテストでも同じ範囲を検証している。
 */
export const JAPAN_BOUNDS: [[number, number], [number, number]] = [
  [128.6, 30.2],
  [145.85, 45.52],
];

/** 範囲の外側に残す余白(px)。下は出典表示の分を多めに取る。 */
export const JAPAN_PADDING = { top: 24, bottom: 36, left: 24, right: 24 };

/** 観測点がすべて収まる表示にする。画面の大きさに合わせて倍率が決まる。回転・傾きも戻す。 */
export function fitJapan(map: MapLibreMap, duration = 0) {
  map.fitBounds(JAPAN_BOUNDS, { padding: JAPAN_PADDING, duration, bearing: 0, pitch: 0 });
}

const ICON =
  '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" ' +
  'stroke-linecap="round" aria-hidden="true"><circle cx="12" cy="12" r="6"/>' +
  '<path d="M12 2v4M12 18v4M2 12h4M18 12h4"/></svg>';

/**
 * ズーム・移動・回転をした地図を、その地図の標準の表示に戻すボタン。
 * ズームボタンの下に並べる(MapLibreの標準コントロールと同じ見た目)。
 */
export class ResetViewControl implements IControl {
  private container?: HTMLDivElement;

  constructor(private readonly onReset: () => void) {}

  onAdd(): HTMLElement {
    const button = document.createElement("button");
    button.type = "button";
    button.title = "表示をリセット";
    button.setAttribute("aria-label", "表示をリセット");
    button.innerHTML = ICON;
    button.style.display = "flex";
    button.style.alignItems = "center";
    button.style.justifyContent = "center";
    button.addEventListener("click", () => this.onReset());

    this.container = document.createElement("div");
    this.container.className = "maplibregl-ctrl maplibregl-ctrl-group";
    this.container.append(button);
    return this.container;
  }

  onRemove(): void {
    this.container?.remove();
    this.container = undefined;
  }
}
