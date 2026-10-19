<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import * as maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import { buildStyle } from "@/lib/mapStyle";
import { kmoniOverlay } from "@/lib/kmoniLayer";

/** 全国の地図では範囲外になる南西諸島(奄美〜八重山)の実測震度を、小さな別地図で見せる */
const props = defineProps<{ points: GeoJSON.FeatureCollection }>();
const el = ref<HTMLDivElement>();
let map: maplibregl.Map | undefined;

const BOUNDS: [[number, number], [number, number]] = [
  [122.3, 23.6],
  [131.7, 29.6],
];

function render() {
  if (!map?.isStyleLoaded()) return;
  (map.getSource("kmoni") as maplibregl.GeoJSONSource).setData(props.points);
}

onMounted(() => {
  map = new maplibregl.Map({
    container: el.value!,
    attributionControl: false,
    interactive: false,
    bounds: BOUNDS,
    fitBoundsOptions: { padding: 0 },
    style: buildStyle(kmoniOverlay),
  });
  map.on("load", render);
});

watch(() => props.points, render);
// 枠の大きさが変わったら描画と表示範囲を合わせる
let observer: ResizeObserver | undefined;
onMounted(() => {
  observer = new ResizeObserver(() => {
    map?.resize();
    map?.fitBounds(BOUNDS, { padding: 0, duration: 0 });
  });
  observer.observe(el.value!);
});
onBeforeUnmount(() => {
  observer?.disconnect();
  map?.remove();
});
</script>

<template>
  <!-- 幅はウィンドウの幅・高さに応じて変わる(小さい画面で地図を覆わず、大きい画面では見やすくする) -->
  <div class="relative aspect-[8/5] w-[clamp(12rem,min(24vw,45vh),32rem)] overflow-hidden rounded-lg border-2 border-slate-900/90 bg-background shadow-lg">
    <div ref="el" class="h-full w-full" />
    <span class="absolute left-1.5 top-1.5 rounded bg-slate-900/85 px-1.5 py-0.5 text-xs text-white">南西諸島</span>
  </div>
</template>
