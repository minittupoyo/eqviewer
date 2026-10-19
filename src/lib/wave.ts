export interface Curve {
  dist_km: number[];
  p: number[];
  s: number[];
}

/**
 * 到達時間の曲線 `times`(昇順)と震央距離 `dist` から、経過秒 `t` の波面の震央距離(km)を線形補間で求める。
 * まだ地表に達していない、または表の範囲(〜2000km)を超えた場合は null。
 */
export function radiusAt(times: number[], dist: number[], t: number): number | null {
  if (!times.length || t < times[0] || t > times[times.length - 1]) return null;
  let lo = 0;
  let hi = times.length - 1;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (times[mid] <= t) lo = mid;
    else hi = mid;
  }
  const span = times[hi] - times[lo];
  const f = span > 0 ? (t - times[lo]) / span : 0;
  return dist[lo] + (dist[hi] - dist[lo]) * f;
}

const EARTH_RADIUS_KM = 6371.0088;

/** 震央から地表の大円距離 `rKm` の円(閉じたリング) */
export function circle(lon: number, lat: number, rKm: number, steps = 90): [number, number][] {
  const rad = Math.PI / 180;
  const [la, lo] = [lat * rad, lon * rad];
  const d = rKm / EARTH_RADIUS_KM;
  const ring: [number, number][] = [];
  for (let i = 0; i <= steps; i++) {
    const b = (i / steps) * 2 * Math.PI;
    const la2 = Math.asin(Math.sin(la) * Math.cos(d) + Math.cos(la) * Math.sin(d) * Math.cos(b));
    const lo2 =
      lo +
      Math.atan2(
        Math.sin(b) * Math.sin(d) * Math.cos(la),
        Math.cos(d) - Math.sin(la) * Math.sin(la2),
      );
    ring.push([lo2 / rad, la2 / rad]);
  }
  return ring;
}

/**
 * 複数の円がすべて収まる範囲 [西, 南], [東, 北]。半径は `minKm` 未満なら `minKm` に揃える
 * (発生直後に円が小さすぎて拡大しすぎるのを防ぐ)。円がなければ null。
 */
export function circlesBounds(
  circles: { lon: number; lat: number; rKm: number }[],
  minKm: number,
): [[number, number], [number, number]] | null {
  let w = Infinity, s = Infinity, e = -Infinity, n = -Infinity;
  for (const c of circles)
    for (const [lo, la] of circle(c.lon, c.lat, Math.max(c.rKm, minKm), 36)) {
      w = Math.min(w, lo); e = Math.max(e, lo); s = Math.min(s, la); n = Math.max(n, la);
    }
  return circles.length ? [[w, s], [e, n]] : null;
}

/** 2点の大円距離(km) */
export function distanceKm(lon1: number, lat1: number, lon2: number, lat2: number): number {
  const rad = Math.PI / 180;
  const [a1, a2] = [lat1 * rad, lat2 * rad];
  const h =
    Math.sin((a2 - a1) / 2) ** 2 +
    Math.cos(a1) * Math.cos(a2) * Math.sin(((lon2 - lon1) * rad) / 2) ** 2;
  return 2 * EARTH_RADIUS_KM * Math.asin(Math.min(1, Math.sqrt(h)));
}

/**
 * 震央距離 `d`(km)の地点に波が届くまでの時間(発生からの秒)。`radiusAt` の逆。
 * 表の範囲(〜2000km)を超える場合は null。
 */
export function arrivalTime(times: number[], dist: number[], d: number): number | null {
  if (!dist.length || d < dist[0] || d > dist[dist.length - 1]) return null;
  let lo = 0;
  let hi = dist.length - 1;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (dist[mid] <= d) lo = mid;
    else hi = mid;
  }
  const span = dist[hi] - dist[lo];
  const f = span > 0 ? (d - dist[lo]) / span : 0;
  return times[lo] + (times[hi] - times[lo]) * f;
}
