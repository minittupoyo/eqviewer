import { defineStore } from "pinia";
import { computed, ref, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Station {
  lon: number;
  lat: number;
}
interface Frame {
  time: string;
  values: (number | null)[];
}

const SECTION_KEY = "eqviewer.section";

/**
 * 前回開いていた項目が強震モニタか。記憶が無い(初回)・読めない場合は強震モニタから始める。
 * localStorage は使えないことがあるので、失敗しても動くようにする。
 */
export function loadViewingKmoni(): boolean {
  try {
    return localStorage.getItem(SECTION_KEY) !== "quake";
  } catch {
    return true;
  }
}

function saveViewingKmoni(on: boolean) {
  try {
    localStorage.setItem(SECTION_KEY, on ? "kmoni" : "quake");
  } catch {
    // 記憶できなくても動作には影響しない
  }
}

export const useKmoniStore = defineStore("kmoni", () => {
  // 観測点は約1,400件あり毎秒更新されるので、深いリアクティブにはしない
  const stations = shallowRef<Station[]>([]);
  const values = shallowRef<(number | null)[]>([]);
  /** 最新フレームの時刻 "YYYYMMDDHHMMSS"(JST) */
  const time = ref<string | null>(null);
  const receivedAt = ref(0);
  /** 強震モニタの項目を開いているか。開いている間だけバックエンドが画像を取得する。前回の項目を引き継ぐ。 */
  const viewing = ref(loadViewingKmoni());

  function setViewing(on: boolean) {
    viewing.value = on;
    saveViewingKmoni(on);
    invoke("kmoni_set_view", { on });
  }

  /** 震度が読めた観測点のGeoJSON(震度 `i` つき) */
  const points = computed(() => ({
    type: "FeatureCollection" as const,
    features: stations.value.flatMap((s, k) => {
      const i = values.value[k];
      return i == null
        ? []
        : [
            {
              type: "Feature" as const,
              properties: { i },
              geometry: { type: "Point" as const, coordinates: [s.lon, s.lat] },
            },
          ];
    }),
  }));

  const timeLabel = computed(() => {
    const t = time.value;
    return t ? `${t.slice(0, 4)}/${t.slice(4, 6)}/${t.slice(6, 8)} ${t.slice(8, 10)}:${t.slice(10, 12)}:${t.slice(12, 14)}` : null;
  });

  async function start() {
    // 起動時に開いている項目をバックエンドに伝える(強震モニタなら取得を始める)
    await invoke("kmoni_set_view", { on: viewing.value });
    stations.value = await invoke<Station[]>("kmoni_stations");
    await listen<Station[]>("kmoni-stations", (e) => (stations.value = e.payload));
    await listen<Frame>("kmoni-frame", (e) => {
      values.value = e.payload.values;
      time.value = e.payload.time;
      receivedAt.value = Date.now();
    });
  }

  return { points, time, timeLabel, receivedAt, viewing, setViewing, start };
});
