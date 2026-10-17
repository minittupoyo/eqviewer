import type { FeatureCollection, Geometry, Position } from "geojson";
import type { Point } from "@/lib/earthquake";
import { SCALE_ORDER } from "@/lib/earthquake";

export interface CityProps {
  name: string;
  regioncode: string;
  pref: string;
}
export type CityCollection = FeatureCollection<Geometry, CityProps>;

/** 震度配色(hex)。SCALE_CLASS と対応 */
export const SCALE_COLOR: Record<string, string> = {
  "1": "#e2e8f0",
  "2": "#7dd3fc",
  "3": "#34d399",
  "4": "#fde047",
  "5-": "#fb923c",
  "5-over": "#fb923c",
  "5+": "#ea580c",
  "6-": "#dc2626",
  "6+": "#991b1b",
  "7": "#86198f",
};

const PREFECTURES =
  "北海道 青森県 岩手県 宮城県 秋田県 山形県 福島県 茨城県 栃木県 群馬県 埼玉県 千葉県 東京都 神奈川県 新潟県 富山県 石川県 福井県 山梨県 長野県 岐阜県 静岡県 愛知県 三重県 滋賀県 京都府 大阪府 兵庫県 奈良県 和歌山県 鳥取県 島根県 岡山県 広島県 山口県 徳島県 香川県 愛媛県 高知県 福岡県 佐賀県 長崎県 熊本県 大分県 宮崎県 鹿児島県 沖縄県".split(" ");

let cache: Promise<CityCollection> | null = null;
export const loadCities = () =>
  (cache ??= fetch("/map/jp-city.geojson").then((r) => r.json()));

/**
 * 市町村ごとの最大震度を返す。返り値のキーは regioncode。
 * 気象庁XML由来の観測点は市町村コードで直接、P2P由来は観測点名(例: 海南市下津)の先頭一致で特定する。
 */
export function cityScales(cities: CityCollection, points: Point[]) {
  const byPref = new Map<string, CityProps[]>();
  for (const f of cities.features) {
    const list = byPref.get(f.properties.pref) ?? [];
    list.push(f.properties);
    byPref.set(f.properties.pref, list);
  }
  const known = new Set(cities.features.map((f) => f.properties.regioncode));
  const result = new Map<string, string>();
  const rank = (s: string) => SCALE_ORDER.indexOf(s);
  const put = (code: string, scale: string) => {
    const cur = result.get(code);
    if (!cur || rank(scale) > rank(cur)) result.set(code, scale);
  };
  for (const p of points) {
    if (p.is_area) continue;
    if (p.city_code && known.has(p.city_code)) {
      put(p.city_code, p.scale);
      continue;
    }
    const idx = PREFECTURES.indexOf(p.pref);
    if (idx < 0) continue;
    // 東京23区は「東京千代田区…」と表記される
    const addr = p.addr.replace(/^東京(?=.+区)/, "");
    const city = byPref
      .get(String(idx + 1).padStart(2, "0"))
      ?.filter((c) => addr.startsWith(c.name))
      .sort((a, b) => b.name.length - a.name.length)[0];
    if (!city) continue;
    put(city.regioncode, p.scale);
  }
  return result;
}

export interface AreaProps {
  code: string;
  name: string;
}
export type AreaCollection = FeatureCollection<Geometry, AreaProps>;

let areaCache: Promise<AreaCollection> | null = null;
export const loadAreas = () =>
  (areaCache ??= fetch("/map/jp-area.geojson").then((r) => r.json()));

/**
 * 震度速報の地域(細分区域)ごとの最大震度。返り値のキーは地域名(例: 茨城県北部)。
 * 気象庁XML・P2Pのどちらも地域名で来るので、名前でそのまま対応づける。
 */
export function areaScales(points: Point[]) {
  const result = new Map<string, string>();
  for (const p of points) {
    if (!p.is_area) continue;
    const cur = result.get(p.addr);
    if (!cur || SCALE_ORDER.indexOf(p.scale) > SCALE_ORDER.indexOf(cur)) result.set(p.addr, p.scale);
  }
  return result;
}

/** 指定地域の外接矩形 [west, south, east, north] */
export function boundsOfAreas(areas: AreaCollection, names: Set<string>) {
  return boundsOfFeatures(areas.features.filter((f) => names.has(f.properties.name)));
}

/** 指定コードの市町村の外接矩形 [west, south, east, north] */
export function boundsOf(cities: CityCollection, codes: Set<string>) {
  return boundsOfFeatures(cities.features.filter((f) => codes.has(f.properties.regioncode)));
}

function boundsOfFeatures(features: { geometry: Geometry }[]) {
  let b: [number, number, number, number] | null = null;
  const visit = (c: unknown) => {
    if (typeof (c as Position)[0] === "number") {
      const [x, y] = c as Position;
      b = b
        ? [Math.min(b[0], x), Math.min(b[1], y), Math.max(b[2], x), Math.max(b[3], y)]
        : [x, y, x, y];
    } else (c as unknown[]).forEach(visit);
  };
  for (const f of features) if ("coordinates" in f.geometry) visit(f.geometry.coordinates);
  return b;
}
