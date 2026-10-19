import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Eew } from "@/lib/earthquake";
import type { Curve } from "@/lib/wave";
import { arrivalTime, distanceKm } from "@/lib/wave";
import { playEewSound, soundFor } from "@/lib/sound";
import { useClockStore } from "@/stores/clock";
import { useKmoniStore } from "@/stores/kmoni";
import { useSourcesStore } from "@/stores/sources";

/** バックエンドの保持期間(5分)と揃える */
const RETAIN_MS = 5 * 60 * 1000;
/** 取消報はこの時間だけ表示する */
const CANCEL_MS = 30 * 1000;
/** 深さ不明のときに仮定する深さ(km) */
const DEFAULT_DEPTH_KM = 10;

export const useEewStore = defineStore("eew", () => {
  const all = ref<Eew[]>([]);
  const clock = useClockStore();
  /** PCの時計の現在時刻(ms)。受信時刻との比較など、PC時計どうしの比較に使う */
  const now = ref(Date.now());
  /** 時計ずれを補正した現在時刻(ms)。地震の発生時刻からの経過時間の計算に使う */
  const correctedNow = computed(() => now.value + (clock.offsetMs ?? 0));
  const curves = ref(new Map<number, Curve>());
  const loading = new Set<number>();

  const active = computed(() =>
    all.value.filter(
      (e) => now.value - e.received_at_ms < (e.is_cancel ? CANCEL_MS : RETAIN_MS),
    ),
  );
  /** 地図に出せる速報(取消・震源不明を除く) */
  const mappable = computed(() =>
    active.value.filter((e) => !e.is_cancel && e.latitude != null && e.longitude != null),
  );

  const depthKey = (e: Eew) => Math.round(e.depth_km ?? DEFAULT_DEPTH_KM);
  const curveFor = (e: Eew) => curves.value.get(depthKey(e));

  const sources = useSourcesStore();
  const kmoni = useKmoniStore();
  /** 自宅地点(設定済みのときだけ) */
  const home = computed(() => {
    const { home_lat: lat, home_lon: lon } = sources.settings;
    return lat != null && lon != null ? { lat, lon } : null;
  });

  /**
   * 自宅地点への波の到達までの残り秒(負なら到達済み)。震源・走時表・自宅のどれかが無い、
   * または走時表の範囲(〜2000km)を超えるときは null。震度の予測はしない。
   */
  function homeArrival(e: Eew) {
    const h = home.value;
    const curve = curveFor(e);
    if (!h || !curve || e.latitude == null || e.longitude == null || e.is_cancel) return null;
    const km = distanceKm(e.longitude, e.latitude, h.lon, h.lat);
    const p = arrivalTime(curve.p, curve.dist_km, km);
    const s = arrivalTime(curve.s, curve.dist_km, km);
    if (p == null || s == null) return null;
    const elapsed = (correctedNow.value - new Date(e.origin_time).getTime()) / 1000;
    return { km, pRemain: p - elapsed, sRemain: s - elapsed };
  }

  async function ensureCurve(key: number) {
    if (curves.value.has(key) || loading.has(key)) return;
    loading.add(key);
    try {
      curves.value.set(key, await invoke<Curve>("travel_curve", { depthKm: key }));
    } finally {
      loading.delete(key);
    }
  }
  watch(mappable, (list) => list.forEach((e) => ensureCurve(depthKey(e))), { immediate: true });

  // 円を滑らかに広げるため、速報がある間だけ高頻度で時刻を更新する
  let timer: ReturnType<typeof setInterval> | undefined;
  watch(
    () => active.value.length > 0,
    (on) => {
      clearInterval(timer);
      timer = setInterval(() => (now.value = Date.now()), on ? 200 : 1000);
    },
    { immediate: true },
  );

  async function start() {
    all.value = await invoke<Eew[]>("active_eews");
    await listen<Eew[]>("eew-updated", (ev) => {
      const sound = soundFor(all.value, ev.payload);
      all.value = ev.payload;
      // EEWは強震モニタの画面に重ねて出す。速報が来たらその画面に切り替える
      kmoni.setViewing(true);
      const { sound_enabled, sound_volume } = sources.settings;
      if (sound && sound_enabled) playEewSound(sound, sound_volume);
    });
  }

  return { active, mappable, now, correctedNow, curveFor, home, homeArrival, start };
});
