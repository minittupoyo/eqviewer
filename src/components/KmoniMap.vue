<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import * as maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import { useKmoniStore } from "@/stores/kmoni";
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
    <div class="absolute left-3 top-3 rounded bg-background/90 px-3 py-1.5 text-sm shadow">
      <span class="font-medium">リアルタイム震度</span>
      <span class="ml-2 tabular-nums text-muted-foreground">{{ store.timeLabel ?? "取得中…" }}</span>
      <span v-if="store.timeLabel && stale" class="ml-2 text-destructive">更新が止まっています</span>
    </div>
    <div class="absolute left-3 top-14"><RemoteIslandsMap :points="store.points" /></div>
    <div class="absolute bottom-8 left-3 flex items-stretch gap-2 rounded bg-background/90 px-3 py-2 text-xs shadow">
      <div class="h-32 w-3 rounded-sm" :style="{ background: KMONI_LEGEND_GRADIENT }" />
      <div class="flex flex-col justify-between tabular-nums">
        <span>7</span><span>5</span><span>3</span><span>1</span><span>-1</span><span>-3</span>
      </div>
    </div>
  </div>
</template>
