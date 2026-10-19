<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import * as maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import { useKmoniStore } from "@/stores/kmoni";
import { useEewStore } from "@/stores/eew";
import { useSourcesStore } from "@/stores/sources";
import EewPanel from "@/components/EewPanel.vue";
import RemoteIslandsMap from "@/components/RemoteIslandsMap.vue";
import { buildStyle, emptyCollection, epicenterLayers, registerEpicenterIcon } from "@/lib/mapStyle";
import { JAPAN_BOUNDS, JAPAN_PADDING, ResetViewControl, fitJapan } from "@/lib/mapControls";
import { Focus } from "@lucide/vue";
import { circle, circlesBounds, radiusAt } from "@/lib/wave";
import { KMONI_LEGEND_GRADIENT, KMONI_STALE_MS, kmoniOverlay } from "@/lib/kmoniLayer";

const store = useKmoniStore();
const eew = useEewStore();
const sources = useSourcesStore();
const el = ref<HTMLDivElement>();
let map: maplibregl.Map | undefined;
let fittedEvent: string | undefined;
/** 全国が収まる標準の表示か。南西諸島のミニマップは、全国表示のときだけ出す(拡大中は範囲外の島が本図に映るため) */
const atHome = ref(true);
/** 予測円に追従して表示範囲を変えるか。地図を自分で動かすと止まり、新しい地震かボタンで再開する。 */
const follow = ref(true);
/** 追従の最小の半径(km)。発生直後の小さな円に寄りすぎないようにする */
const FOLLOW_MIN_KM = 60;

const now = ref(Date.now());
const timer = setInterval(() => (now.value = Date.now()), 1000);
const stale = computed(() => now.value - store.receivedAt > KMONI_STALE_MS);

type Fc = GeoJSON.FeatureCollection;

/** 震央・P波(外側の線)・S波(塗り)を現在時刻から計算する */
const shapes = computed(() => {
  const epi: Fc = { type: "FeatureCollection", features: [] };
  const p: Fc = { type: "FeatureCollection", features: [] };
  const s: Fc = { type: "FeatureCollection", features: [] };
  /** 追従の範囲に使う円。P波の円がまだ無ければ最小半径の円 */
  const extents: { lon: number; lat: number; rKm: number }[] = [];
  for (const e of eew.mappable) {
    const [lon, lat] = [e.longitude!, e.latitude!];
    epi.features.push({
      type: "Feature",
      properties: {},
      geometry: { type: "Point", coordinates: [lon, lat] },
    });
    const curve = eew.curveFor(e);
    if (!curve) {
      extents.push({ lon, lat, rKm: 0 });
      continue;
    }
    const t = (eew.correctedNow - new Date(e.origin_time).getTime()) / 1000;
    const rp = radiusAt(curve.p, curve.dist_km, t);
    const rs = radiusAt(curve.s, curve.dist_km, t);
    extents.push({ lon, lat, rKm: rp ?? 0 });
    if (rp != null && rp > 0)
      p.features.push({
        type: "Feature",
        properties: {},
        geometry: { type: "LineString", coordinates: circle(lon, lat, rp) },
      });
    if (rs != null && rs > 0)
      s.features.push({
        type: "Feature",
        properties: {},
        geometry: { type: "Polygon", coordinates: [circle(lon, lat, rs)] },
      });
  }
  return { epi, p, s, extents };
});

/** 自宅地点(設定済みのときだけ) */
const homePoint = computed<Fc>(() => ({
  type: "FeatureCollection",
  features: eew.home
    ? [{ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [eew.home.lon, eew.home.lat] } }]
    : [],
}));

function render() {
  if (!map?.isStyleLoaded()) return;
  (map.getSource("kmoni") as maplibregl.GeoJSONSource).setData(store.points);
}

function renderEew() {
  if (!map?.isStyleLoaded()) return;
  const { epi, p, s } = shapes.value;
  (map.getSource("eew-epicenter") as maplibregl.GeoJSONSource).setData(epi);
  (map.getSource("eew-p") as maplibregl.GeoJSONSource).setData(p);
  (map.getSource("eew-s") as maplibregl.GeoJSONSource).setData(s);
  (map.getSource("eew-home") as maplibregl.GeoJSONSource).setData(homePoint.value);
  // 新しい地震が来たら追従を再開する。複数あるときは、すべての円が収まる範囲にする
  const latest = eew.mappable[0];
  if (latest && latest.event_id !== fittedEvent) {
    fittedEvent = latest.event_id;
    follow.value = true;
  }
  if (follow.value) followView(0);
}

/** いまの表示が全国の標準の表示かを調べる */
function updateAtHome() {
  if (!map) return;
  const home = map.cameraForBounds(JAPAN_BOUNDS, { padding: JAPAN_PADDING });
  const c = map.getCenter();
  atHome.value =
    !!home?.center &&
    Math.abs(map.getZoom() - (home.zoom ?? 0)) < 0.05 &&
    Math.abs(c.lng - (home.center as maplibregl.LngLat).lng) < 0.05 &&
    Math.abs(c.lat - (home.center as maplibregl.LngLat).lat) < 0.05 &&
    map.getBearing() === 0 &&
    map.getPitch() === 0;
}

/** 追従中の表示範囲。速報がすべて終わったら全国に戻す。 */
function followView(duration: number) {
  if (!map) return;
  const bounds = circlesBounds(shapes.value.extents, FOLLOW_MIN_KM);
  if (!bounds) {
    if (fittedEvent) fitJapan(map, 400);
    fittedEvent = undefined;
    return;
  }
  // 左上の情報カードに円が隠れないよう、左を広く空ける(小さい窓では幅の3割まで)
  const { clientWidth: w, clientHeight: h } = map.getContainer();
  const padding = { top: Math.min(60, h * 0.1), bottom: Math.min(60, h * 0.1), right: 60, left: Math.min(340, w * 0.3) };
  map.fitBounds(bounds, { padding, maxZoom: 8, duration, bearing: 0, pitch: 0 });
}

/** 表示のリセット。速報があれば追従を再開し、なければ全国に戻す。 */
function resetView(duration: number) {
  if (!map) return;
  if (eew.mappable.length) {
    follow.value = true;
    followView(duration);
  } else fitJapan(map, duration);
}

/** ボタンで追従を切り替える。入れたときはすぐ合わせる。 */
function toggleFollow() {
  follow.value = !follow.value;
  if (follow.value) followView(400);
}

onMounted(() => {
  map = new maplibregl.Map({
    container: el.value!,
    attributionControl: false,
    bounds: JAPAN_BOUNDS,
    fitBoundsOptions: { padding: JAPAN_PADDING },
    style: buildStyle({
      sources: {
        ...kmoniOverlay.sources,
        "eew-epicenter": { type: "geojson", data: emptyCollection },
        "eew-p": { type: "geojson", data: emptyCollection },
        "eew-s": { type: "geojson", data: emptyCollection },
        "eew-home": { type: "geojson", data: emptyCollection },
      },
      layers: [
        ...kmoniOverlay.layers,
        { id: "eew-s-fill", type: "fill", source: "eew-s", paint: { "fill-color": "#dc2626", "fill-opacity": 0.15 } },
        { id: "eew-s-line", type: "line", source: "eew-s", paint: { "line-color": "#dc2626", "line-width": 2.5 } },
        { id: "eew-p-line", type: "line", source: "eew-p", paint: { "line-color": "#2563eb", "line-width": 2 } },
        ...epicenterLayers("eew-epicenter"),
        {
          id: "eew-home",
          type: "circle",
          source: "eew-home",
          paint: { "circle-radius": 6, "circle-color": "#16a34a", "circle-stroke-color": "#fff", "circle-stroke-width": 2 },
        },
      ],
    }),
  });
  registerEpicenterIcon(map);
  map.addControl(new maplibregl.NavigationControl({ showCompass: false }), "top-right");
  map.addControl(new ResetViewControl(() => resetView(400)), "top-right");
  map.addControl(
    new maplibregl.AttributionControl({
      compact: true,
      customAttribution: "強震モニタ(防災科研)・気象庁・Natural Earth(走時表: JMA2001)",
    }),
  );
  // ユーザー自身の操作(originalEventがある)で動かしたら、追従をやめる
  map.on("movestart", (e) => {
    if (e.originalEvent) follow.value = false;
  });
  map.on("moveend", updateAtHome);
  map.on("resize", updateAtHome);
  map.on("load", () => {
    updateAtHome();
    render();
    renderEew();
  });
});

watch(() => store.points, render);
watch([shapes, homePoint], renderEew);
onBeforeUnmount(() => {
  clearInterval(timer);
  map?.remove();
});
</script>

<template>
  <div class="relative h-full w-full">
    <div ref="el" class="h-full w-full" />
    <div class="absolute left-3 top-3 flex flex-col items-start gap-2">
      <div class="flex items-center gap-3 rounded-lg bg-slate-900/90 px-3 py-2 text-white shadow-lg backdrop-blur">
      <span
        class="size-2 rounded-full"
        :class="!store.timeLabel ? 'bg-amber-400' : stale ? 'bg-red-500' : 'animate-pulse bg-emerald-400'"
        aria-hidden="true"
      />
      <span class="text-lg font-semibold tabular-nums leading-none">{{ store.timeLabel ?? "取得中…" }}</span>
      <span v-if="store.timeLabel && stale" class="text-xs text-red-300">更新が止まっています</span>
    </div>
      <EewPanel />
      <RemoteIslandsMap v-show="atHome" :points="store.points" />
    </div>
    <button
      v-if="eew.mappable.length"
      type="button"
      class="absolute right-[10px] top-[118px] flex size-[29px] items-center justify-center rounded shadow"
      :class="follow ? 'bg-slate-900 text-white' : 'bg-white text-slate-700 hover:bg-slate-100'"
      :aria-pressed="follow"
      title="予測円に合わせて自動でズーム"
      aria-label="予測円に合わせて自動でズーム"
      @click="toggleFollow"
    >
      <Focus class="size-[18px]" aria-hidden="true" />
    </button>
    <!-- 横長の凡例。縦に長いと地図の左下を大きく覆うため -->
    <div v-if="sources.settings.show_legend" class="absolute bottom-8 left-3 w-64 rounded-lg bg-slate-900/90 px-3 py-2 text-xs text-slate-200 shadow-lg backdrop-blur">
      <ul v-if="eew.mappable.length" class="mb-2 space-y-1 border-b border-white/10 pb-2">
        <li class="flex items-center gap-2"><span class="inline-block h-0.5 w-5 bg-blue-500" />P波(初期微動)の到達予想</li>
        <li class="flex items-center gap-2"><span class="inline-block h-0.5 w-5 bg-red-500" />S波(主要動)の到達予想</li>
        <li v-if="eew.home" class="flex items-center gap-2">
          <span class="inline-block size-2.5 rounded-full border border-white bg-green-600" />自宅地点
        </li>
      </ul>
      <div class="mb-1 text-[0.7rem] text-slate-400">リアルタイム震度</div>
      <div class="h-2.5 rounded-sm" :style="{ background: KMONI_LEGEND_GRADIENT }" />
      <div class="mt-1 flex justify-between tabular-nums">
        <span>-3</span><span>-1</span><span>1</span><span>3</span><span>5</span><span>7</span>
      </div>
    </div>
  </div>
</template>
