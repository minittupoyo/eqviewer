<script setup lang="ts">
import { useEewStore } from "@/stores/eew";
import { scaleClass, scaleLabel, type Eew } from "@/lib/earthquake";

const store = useEewStore();

/** 地震発生からの経過秒(PCの時計ずれを補正した時刻が基準) */
const elapsed = (iso: string) =>
  Math.max(0, Math.round((store.correctedNow - new Date(iso).getTime()) / 1000));

const arrival = (e: Eew) => store.homeArrival(e);
/** 残り秒の表示。負なら到達済み(予想)。 */
const remain = (sec: number) => (sec > 0 ? `あと${Math.ceil(sec)}秒` : "到達済み(予想)");

const kind = (e: Eew) => (e.is_cancel ? "取消" : e.is_warning ? "警報" : "予報");
const tone = (e: Eew) =>
  e.is_cancel
    ? "bg-slate-600 text-white"
    : e.is_warning
      ? "bg-red-600 text-white"
      : "bg-amber-300 text-slate-900";
</script>

<template>
  <section v-if="store.active.length" class="w-72 space-y-2" aria-live="assertive">
    <div
      v-for="e in store.active"
      :key="e.event_id"
      class="block w-full overflow-hidden rounded-lg bg-slate-900/95 text-left text-white shadow-lg backdrop-blur"
    >
      <div class="flex items-center justify-between px-3 py-1 text-xs font-semibold" :class="tone(e)">
        <span>緊急地震速報({{ kind(e) }}){{ e.is_training ? "【訓練】" : "" }}</span>
        <span>第{{ e.serial }}報{{ e.is_final ? "(最終)" : "" }}</span>
      </div>
      <div v-if="!e.is_cancel" class="space-y-1 px-3 py-2">
        <div class="flex items-center gap-3">
          <span class="rounded px-2 py-0.5 text-center leading-tight" :class="scaleClass(e.max_scale)">
            <span class="block text-[0.6rem]">最大震度</span>
            <span class="text-xl font-bold">{{ scaleLabel(e.max_scale) }}</span>
          </span>
          <span class="min-w-0">
            <span class="block truncate text-base font-semibold">{{ e.hypocenter ?? "震源推定中" }}</span>
            <span class="block text-xs text-slate-300">
              M{{ e.magnitude != null ? e.magnitude.toFixed(1) : "不明" }} / 深さ{{
                e.depth_km != null ? `${e.depth_km}km` : "不明"
              }}
            </span>
          </span>
        </div>
        <div class="flex justify-between text-xs tabular-nums text-slate-300">
          <span>発生から{{ elapsed(e.origin_time) }}秒</span>
          <span v-if="arrival(e)" class="font-semibold text-white">
            自宅({{ Math.round(arrival(e)!.km) }}km) S波 {{ remain(arrival(e)!.sRemain) }}
          </span>
        </div>
      </div>
      <p v-else class="px-3 py-2 text-xs text-slate-300">この速報は取り消されました。</p>
    </div>
  </section>
</template>
