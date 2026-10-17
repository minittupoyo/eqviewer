<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import * as maplibregl from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import type { Earthquake } from "@/lib/earthquake";
import {
  areaScales,
  boundsOf,
  boundsOfAreas,
  cityScales,
  loadAreas,
  loadCities,
  type AreaCollection,
  type CityCollection,
} from "@/lib/map";
import { areaOverlay, buildStyle, epicenterLayers } from "@/lib/mapStyle";
import { ResetViewControl } from "@/lib/mapControls";

const props = defineProps<{ eq: Earthquake }>();
const el = ref<HTMLDivElement>();
let map: maplibregl.Map | undefined;
let cities: CityCollection | undefined;
let areas: AreaCollection | undefined;
let painted: string[] = [];
let paintedAreas: string[] = [];

function epicenter(eq: Earthquake) {
  return {
    type: "FeatureCollection" as const,
    features:
      eq.latitude != null && eq.longitude != null
        ? [
            {
              type: "Feature" as const,
              properties: {},
              geometry: { type: "Point" as const, coordinates: [eq.longitude, eq.latitude] },
            },
          ]
        : [],
  };
}

function update() {
  if (!map || !cities) return;
  const eq = props.eq;
  for (const id of painted) map.removeFeatureState({ source: "city", id });
  const scales = cityScales(cities, eq.points);
  painted = [...scales.keys()];
  for (const [id, scale] of scales) map.setFeatureState({ source: "city", id }, { scale });
  for (const id of paintedAreas) map.removeFeatureState({ source: "area", id });
  const aScales = areaScales(eq.points);
  paintedAreas = [...aScales.keys()];
  for (const [id, scale] of aScales) map.setFeatureState({ source: "area", id }, { scale });
  (map.getSource("epicenter") as maplibregl.GeoJSONSource).setData(epicenter(eq));

  fit(0);
}

/** 色付けした市町村と震央が収まる表示にする。回転・傾きも戻す。 */
function fit(duration: number) {
  if (!map || !cities) return;
  const eq = props.eq;
  const pts: [number, number][] = [];
  for (const b of [
    boundsOf(cities, new Set(painted)),
    areas && boundsOfAreas(areas, new Set(paintedAreas)),
  ])
    if (b) pts.push([b[0], b[1]], [b[2], b[3]]);
  if (eq.longitude != null && eq.latitude != null) pts.push([eq.longitude, eq.latitude]);
  if (!pts.length) return;
  const bounds = pts.reduce((acc, p) => acc.extend(p), new maplibregl.LngLatBounds(pts[0], pts[0]));
  map.fitBounds(bounds, { padding: 40, maxZoom: 9, duration, bearing: 0, pitch: 0 });
}

onMounted(async () => {
  const loaded = (cities = await loadCities());
  areas = await loadAreas();
  map = new maplibregl.Map({
    container: el.value!,
    attributionControl: false,
    center: [137, 38],
    zoom: 4,
    style: buildStyle(loaded, {
      sources: { ...areaOverlay.sources, epicenter: { type: "geojson", data: epicenter(props.eq) } },
      layers: [...areaOverlay.layers, ...epicenterLayers("epicenter")],
    }),
  });
  map.addControl(new maplibregl.NavigationControl({ showCompass: false }), "top-right");
  map.addControl(new ResetViewControl(() => fit(400)), "top-right");
  map.addControl(
    new maplibregl.AttributionControl({
      compact: true,
      customAttribution: "気象庁・Natural Earth",
    }),
  );
  map.on("load", update);
});

watch(() => props.eq, update);
// 地図の枠の大きさが変わったら(境目の移動・最大化)描画を合わせる
let observer: ResizeObserver | undefined;
onMounted(() => {
  observer = new ResizeObserver(() => map?.resize());
  observer.observe(el.value!);
});
onBeforeUnmount(() => {
  observer?.disconnect();
  map?.remove();
});
</script>

<template>
  <div ref="el" class="h-full w-full" />
</template>
