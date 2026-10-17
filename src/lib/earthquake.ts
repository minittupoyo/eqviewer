export interface Point {
  pref: string;
  addr: string;
  scale: string;
  is_area: boolean;
  /** 市町村コード(気象庁XML由来。地図の regioncode と同じ) */
  city_code?: string | null;
}

export interface Earthquake {
  /** 統合キー(検知時刻)。続きの取得や選択の識別に使う */
  id: string;
  /** 表示用の発生時刻 */
  origin_time: string;
  name: string | null;
  latitude: number | null;
  longitude: number | null;
  depth_km: number | null;
  magnitude: number | null;
  max_scale: string | null;
  domestic_tsunami: string | null;
  foreign_tsunami: string | null;
  points: Point[];
}

const SCALE_LABEL: Record<string, string> = {
  "5-": "5弱",
  "5+": "5強",
  "6-": "6弱",
  "6+": "6強",
  "5-over": "5弱以上",
};

export const scaleLabel = (s: string | null) =>
  s ? (SCALE_LABEL[s] ?? s) : "不明";

/** 震度ごとの配色(気象庁の震度配色に準拠) */
export const SCALE_CLASS: Record<string, string> = {
  "1": "bg-slate-200 text-slate-900",
  "2": "bg-sky-300 text-slate-900",
  "3": "bg-emerald-400 text-slate-900",
  "4": "bg-yellow-300 text-slate-900",
  "5-": "bg-orange-400 text-slate-900",
  "5-over": "bg-orange-400 text-slate-900",
  "5+": "bg-orange-600 text-white",
  "6-": "bg-red-600 text-white",
  "6+": "bg-red-800 text-white",
  "7": "bg-fuchsia-800 text-white",
};

export const scaleClass = (s: string | null) =>
  (s && SCALE_CLASS[s]) || "bg-muted text-muted-foreground";

/** 震度の大きい順に並べるための序数 */
export const SCALE_ORDER = ["1", "2", "3", "4", "5-", "5-over", "5+", "6-", "6+", "7"];

export const formatTime = (iso: string) => {
  const d = new Date(iso);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}/${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
};

export const TSUNAMI_LABEL: Record<string, string> = {
  None: "津波の心配なし",
  Unknown: "不明",
  Checking: "津波の有無を調査中",
  NonEffective: "若干の海面変動(被害の心配なし)",
  Watch: "津波注意報",
  Warning: "津波予報あり",
  WarningPacific: "太平洋で津波の可能性",
  WarningPacificWide: "太平洋広域で津波の可能性",
  WarningIndian: "インド洋で津波の可能性",
  WarningIndianWide: "インド洋広域で津波の可能性",
};

export interface ForecastArea {
  name: string;
  scale_from: string | null;
  scale_to: string | null;
  is_warning: boolean;
  arrived: boolean;
}

export interface Eew {
  source: string;
  event_id: string;
  serial: number;
  origin_time: string;
  report_time: string;
  is_warning: boolean;
  is_final: boolean;
  is_cancel: boolean;
  is_training: boolean;
  hypocenter: string | null;
  latitude: number | null;
  longitude: number | null;
  depth_km: number | null;
  magnitude: number | null;
  max_scale: string | null;
  areas: ForecastArea[];
  received_at_ms: number;
}
