<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import * as maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import { useEewStore } from "@/stores/eew";
import { useKmoniStore } from "@/stores/kmoni";
import { KMONI_STALE_MS, kmoniOverlay } from "@/lib/kmoniLayer";
import RemoteIslandsMap from "@/components/RemoteIslandsMap.vue";
import { loadCities } from "@/lib/map";
import { buildStyle, emptyCollection, epicenterLayers } from "@/lib/mapStyle";
import { JAPAN_BOUNDS, JAPAN_PADDING, ResetViewControl, fitJapan } from "@/lib/mapControls";
import { circle, radiusAt } from "@/lib/wave";

const store = useEewStore();
const kmoni = useKmoniStore();
const el = ref<HTMLDivElement>();
let map: maplibregl.Map | undefined;
let fittedEvent: string | undefined;

type Fc = GeoJSON.FeatureCollection;

/** 震央・P波(外側の線)・S波(塗り)を現在時刻から計算する */
const shapes = computed(() => {
  const epi: Fc = { type: "FeatureCollection", features: [] };
  const p: Fc = { type: "FeatureCollection", features: [] };
  const s: Fc = { type: "FeatureCollection", features: [] };
  for (const e of store.mappable) {
    const [lon, lat] = [e.longitude!, e.latitude!];
    epi.features.push({
      type: "Feature",
      properties: {},
      geometry: { type: "Point", coordinates: [lon, lat] },
    });
    const curve = store.curveFor(e);
    if (!curve) continue;
    const t = (store.correctedNow - new Date(e.origin_time).getTime()) / 1000;
    const rp = radiusAt(curve.p, curve.dist_km, t);
    const rs = radiusAt(curve.s, curve.dist_km, t);
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
  return { epi, p, s };
});

/** 自宅地点(設定済みのときだけ) */
const homePoint = computed<Fc>(() => ({
  type: "FeatureCollection",
  features: store.home
    ? [{ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [store.home.lon, store.home.lat] } }]
    : [],
}));

/** 強震モニタを取得中(設定で常時取得)なら、実測の震度も重ねる。古いデータは出さない。 */
const kmoniFresh = computed(() => store.now - kmoni.receivedAt < KMONI_STALE_MS);

/** 強震モニタを重ねるとき(最新のデータがあるとき)だけ、その震度を出す */
const kmoniPoints = computed(() =>
  kmoniFresh.value ? kmoni.points : { type: "FeatureCollection" as const, features: [] },
);

function renderKmoni() {
  if (!map?.isStyleLoaded()) return;
  (map.getSource("kmoni") as maplibregl.GeoJSONSource).setData(kmoniPoints.value);
}

function render() {
  if (!map?.isStyleLoaded()) return;
  const { epi, p, s } = shapes.value;
  (map.getSource("eew-epicenter") as maplibregl.GeoJSONSource).setData(epi);
  (map.getSource("eew-p") as maplibregl.GeoJSONSource).setData(p);
  (map.getSource("eew-s") as maplibregl.GeoJSONSource).setData(s);
  (map.getSource("eew-home") as maplibregl.GeoJSONSource).setData(homePoint.value);
  // 新しい地震が来たときだけ震央へ移動する(以降はユーザー操作を尊重)
  const latest = store.mappable[0];
  if (latest && latest.event_id !== fittedEvent) {
    fittedEvent = latest.event_id;
    resetView(0);
  }
}

/** 最新の震源を中心にした表示に戻す。回転・傾きも戻す。 */
function resetView(duration: number) {
  const latest = store.mappable[0];
  if (!map) return;
  if (!latest) {
    fitJapan(map, duration);
    return;
  }
  map.easeTo({ center: [latest.longitude!, latest.latitude!], zoom: 6, bearing: 0, pitch: 0, duration });
}

onMounted(async () => {
  const cities = await loadCities();
  map = new maplibregl.Map({
    container: el.value!,
    attributionControl: false,
    bounds: JAPAN_BOUNDS,
    fitBoundsOptions: { padding: JAPAN_PADDING },
    style: buildStyle(cities, {
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
  map.addControl(new maplibregl.NavigationControl({ showCompass: false }), "top-right");
  map.addControl(new ResetViewControl(() => resetView(400)), "top-right");
  map.addControl(
    new maplibregl.AttributionControl({
      compact: true,
      customAttribution: "気象庁・Natural Earth(走時表: JMA2001)",
    }),
  );
  map.on("load", () => {
    render();
    renderKmoni();
  });
});

watch([shapes, homePoint], render);
watch([() => kmoni.points, kmoniFresh], renderKmoni);
onBeforeUnmount(() => map?.remove());
</script>

<template>
  <div class="relative h-full w-full">
    <div ref="el" class="h-full w-full" />
    <div v-if="kmoniFresh" class="absolute left-3 top-3">
      <RemoteIslandsMap :points="kmoniPoints" />
    </div>
    <ul class="absolute bottom-8 left-3 space-y-1 rounded bg-background/90 px-3 py-2 text-xs shadow">
      <li class="flex items-center gap-2">
        <span class="inline-block h-0.5 w-5 bg-blue-600" />P波(初期微動)の到達予想
      </li>
      <li class="flex items-center gap-2">
        <span class="inline-block h-0.5 w-5 bg-red-600" />S波(主要動)の到達予想
      </li>
      <li v-if="store.home" class="flex items-center gap-2">
        <span class="inline-block h-2.5 w-2.5 rounded-full border border-white bg-green-600" />自宅地点
      </li>
    </ul>
  </div>
</template>
