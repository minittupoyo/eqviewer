<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import * as maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import { useKmoniStore } from "@/stores/kmoni";
import EewPanel from "@/components/EewPanel.vue";
import RemoteIslandsMap from "@/components/RemoteIslandsMap.vue";
import { buildStyle } from "@/lib/mapStyle";
import { JAPAN_BOUNDS, JAPAN_PADDING, ResetViewControl, fitJapan } from "@/lib/mapControls";
import { KMONI_LEGEND_GRADIENT, KMONI_STALE_MS, kmoniOverlay } from "@/lib/kmoniLayer";

const store = useKmoniStore();
const el = ref<HTMLDivElement>();
let map: maplibregl.Map | undefined;

const now = ref(Date.now());
const timer = setInterval(() => (now.value = Date.now()), 1000);
const stale = computed(() => now.value - store.receivedAt > KMONI_STALE_MS);

function render() {
  if (!map?.isStyleLoaded()) return;
  (map.getSource("kmoni") as maplibregl.GeoJSONSource).setData(store.points);
}

onMounted(() => {
  map = new maplibregl.Map({
    container: el.value!,
    attributionControl: false,
    bounds: JAPAN_BOUNDS,
    fitBoundsOptions: { padding: JAPAN_PADDING },
    style: buildStyle(kmoniOverlay),
  });
  map.addControl(new maplibregl.NavigationControl({ showCompass: false }), "top-right");
  map.addControl(new ResetViewControl(() => map && fitJapan(map, 400)), "top-right");
  map.addControl(
    new maplibregl.AttributionControl({
      compact: true,
      customAttribution: "強震モニタ(防災科研)・気象庁・Natural Earth",
    }),
  );
  map.on("load", render);
});

watch(() => store.points, render);
onBeforeUnmount(() => {
  clearInterval(timer);
  map?.remove();
});
</script>

<template>
  <div class="relative h-full w-full">
    <div ref="el" class="h-full w-full" />
    <div class="absolute left-3 top-3 flex flex-col items-start gap-2">
      <EewPanel />
      <div class="flex items-center gap-3 rounded-lg bg-slate-900/90 px-3 py-2 text-white shadow-lg backdrop-blur">
      <span
        class="size-2 rounded-full"
        :class="!store.timeLabel ? 'bg-amber-400' : stale ? 'bg-red-500' : 'animate-pulse bg-emerald-400'"
        aria-hidden="true"
      />
      <span class="text-lg font-semibold tabular-nums leading-none">{{ store.timeLabel ?? "取得中…" }}</span>
      <span v-if="store.timeLabel && stale" class="text-xs text-red-300">更新が止まっています</span>
    </div>
      <RemoteIslandsMap :points="store.points" />
    </div>
    <!-- 横長の凡例。縦に長いと地図の左下を大きく覆うため -->
    <div class="absolute bottom-8 left-3 w-64 rounded-lg bg-slate-900/90 px-3 py-2 text-xs text-slate-200 shadow-lg backdrop-blur">
      <div class="mb-1 text-[0.7rem] text-slate-400">リアルタイム震度</div>
      <div class="h-2.5 rounded-sm" :style="{ background: KMONI_LEGEND_GRADIENT }" />
      <div class="mt-1 flex justify-between tabular-nums">
        <span>-3</span><span>-1</span><span>1</span><span>3</span><span>5</span><span>7</span>
      </div>
    </div>
  </div>
</template>
