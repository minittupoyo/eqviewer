<script setup lang="ts">
import { computed, ref } from "vue";
import {
  SCALE_ORDER,
  TSUNAMI_LABEL,
  formatTime,
  scaleClass,
  scaleLabel,
  type Earthquake,
} from "@/lib/earthquake";
import { Badge } from "@/components/ui/badge";
import EarthquakeMap from "@/components/EarthquakeMap.vue";
import { ScrollArea } from "@/components/ui/scroll-area";

const props = defineProps<{ eq: Earthquake }>();

/** 震度の大きい順にまとめた観測点 */
const groups = computed(() => {
  const by = new Map<string, string[]>();
  for (const p of props.eq.points) {
    const label = p.is_area ? p.addr : `${p.pref} ${p.addr}`;
    by.set(p.scale, [...(by.get(p.scale) ?? []), label]);
  }
  return [...by.entries()].sort(
    (a, b) => SCALE_ORDER.indexOf(b[0]) - SCALE_ORDER.indexOf(a[0]),
  );
});

// 地図と詳細の境目をドラッグ(またはキーボード)で動かし、地図の高さの割合を変えられる。次回も引き継ぐ。
const RATIO_KEY = "eqviewer.mapRatio";
const [MIN_RATIO, MAX_RATIO, DEFAULT_RATIO] = [0.2, 0.8, 0.5];
const clamp = (v: number) => Math.min(MAX_RATIO, Math.max(MIN_RATIO, v));
function loadRatio() {
  try {
    const v = Number(localStorage.getItem(RATIO_KEY));
    return v ? clamp(v) : DEFAULT_RATIO;
  } catch {
    return DEFAULT_RATIO;
  }
}
const ratio = ref(loadRatio());
const root = ref<HTMLDivElement>();
function setRatio(v: number) {
  ratio.value = clamp(v);
  try {
    localStorage.setItem(RATIO_KEY, String(ratio.value));
  } catch {
    /* 保存できない環境では次回は既定値に戻る */
  }
}
function drag(e: PointerEvent) {
  const handle = e.currentTarget as HTMLElement;
  handle.setPointerCapture(e.pointerId);
  const move = (ev: PointerEvent) => {
    const r = root.value!.getBoundingClientRect();
    ratio.value = clamp((ev.clientY - r.top) / r.height);
  };
  const up = () => {
    handle.removeEventListener("pointermove", move);
    handle.removeEventListener("pointerup", up);
    setRatio(ratio.value);
  };
  handle.addEventListener("pointermove", move);
  handle.addEventListener("pointerup", up);
}
function key(e: KeyboardEvent) {
  const d = e.key === "ArrowUp" ? -0.05 : e.key === "ArrowDown" ? 0.05 : 0;
  if (d) {
    e.preventDefault();
    setRatio(ratio.value + d);
  }
}

const tsunami = (v: string | null) => (v && TSUNAMI_LABEL[v]) || v;
</script>

<template>
  <div ref="root" class="flex h-full min-h-0 flex-col">
    <div class="min-h-0 shrink-0" :style="{ height: `${ratio * 100}%` }">
      <EarthquakeMap :eq="eq" />
    </div>
    <div
      role="separator"
      aria-orientation="horizontal"
      aria-label="地図と詳細の境目(ドラッグまたは上下キーで高さを変更、ダブルクリックで戻す)"
      :aria-valuenow="Math.round(ratio * 100)"
      aria-valuemin="20"
      aria-valuemax="80"
      tabindex="0"
      class="h-1.5 shrink-0 cursor-row-resize touch-none border-y bg-muted transition-colors hover:bg-primary/30 focus-visible:bg-primary/40 focus-visible:outline-none"
      @pointerdown="drag"
      @keydown="key"
      @dblclick="setRatio(DEFAULT_RATIO)"
    />
    <ScrollArea class="min-h-0 flex-1">
    <div class="space-y-4 p-4">
    <header class="flex items-center gap-3">
      <Badge class="px-3 py-1 text-base" :class="scaleClass(eq.max_scale)">
        最大震度 {{ scaleLabel(eq.max_scale) }}
      </Badge>
      <h2 class="text-lg font-semibold">{{ eq.name ?? "震源調査中" }}</h2>
    </header>

    <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
      <dt class="text-muted-foreground">発生時刻</dt>
      <dd>{{ formatTime(eq.origin_time) }}</dd>
      <dt class="text-muted-foreground">マグニチュード</dt>
      <dd>{{ eq.magnitude != null ? eq.magnitude.toFixed(1) : "不明" }}</dd>
      <dt class="text-muted-foreground">深さ</dt>
      <dd>
        {{ eq.depth_km == null ? "不明" : eq.depth_km === 0 ? "ごく浅い" : `${eq.depth_km}km` }}
      </dd>
      <template v-if="tsunami(eq.domestic_tsunami)">
        <dt class="text-muted-foreground">国内津波</dt>
        <dd>{{ tsunami(eq.domestic_tsunami) }}</dd>
      </template>
      <template v-if="eq.foreign_tsunami && eq.foreign_tsunami !== 'Unknown'">
        <dt class="text-muted-foreground">海外津波</dt>
        <dd>{{ tsunami(eq.foreign_tsunami) }}</dd>
      </template>
    </dl>

    <section v-for="[scale, names] in groups" :key="scale" class="space-y-1">
      <Badge :class="scaleClass(scale)">震度{{ scaleLabel(scale) }}</Badge>
      <p class="text-sm leading-relaxed">{{ names.join("、") }}</p>
    </section>
    </div>
    </ScrollArea>
  </div>
</template>
