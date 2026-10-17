<script setup lang="ts">
import { computed } from "vue";
import { useEewStore } from "@/stores/eew";
import { scaleClass, scaleLabel, type Eew } from "@/lib/earthquake";
import { Badge } from "@/components/ui/badge";

const store = useEewStore();

/** 地震発生からの経過秒(PCの時計ずれを補正した時刻が基準) */
const elapsed = (iso: string) =>
  Math.max(0, Math.round((store.correctedNow - new Date(iso).getTime()) / 1000));

const arrival = (e: Eew) => store.homeArrival(e);
/** 残り秒の表示。負なら到達済み(予想)。 */
const remain = (sec: number, label: string) =>
  sec > 0 ? `${label}到達まで あと${Math.ceil(sec)}秒` : `${label}到達済み(予想)`;

const items = computed(() => store.active);
</script>

<template>
  <section v-if="items.length" class="border-b" aria-live="assertive">
    <button
      v-for="e in items"
      :key="e.event_id"
      type="button"
      class="flex w-full flex-wrap items-center gap-x-4 gap-y-1 px-4 py-3 text-left"
      @click="store.focus = true"
      :class="
        e.is_cancel
          ? 'bg-muted text-muted-foreground'
          : e.is_warning
            ? 'bg-red-600 text-white'
            : 'bg-amber-300 text-slate-900'
      "
    >
      <strong class="text-base">
        緊急地震速報({{ e.is_cancel ? "取消" : e.is_warning ? "警報" : "予報" }})
        <span v-if="e.is_training">【訓練】</span>
      </strong>
      <template v-if="!e.is_cancel">
        <span class="text-base font-semibold">{{ e.hypocenter ?? "震源推定中" }}</span>
        <Badge :class="scaleClass(e.max_scale)">最大震度 {{ scaleLabel(e.max_scale) }}</Badge>
        <span class="text-sm">
          M{{ e.magnitude != null ? e.magnitude.toFixed(1) : "不明" }} /
          深さ{{ e.depth_km != null ? `${e.depth_km}km` : "不明" }}
        </span>
        <span class="text-sm tabular-nums">発生から{{ elapsed(e.origin_time) }}秒</span>
        <span v-if="arrival(e)" class="text-sm font-semibold tabular-nums">
          自宅(震源から{{ Math.round(arrival(e)!.km) }}km):
          {{ remain(arrival(e)!.sRemain, "S波") }}
        </span>
      </template>
      <span class="ml-auto text-xs">
        第{{ e.serial }}報{{ e.is_final ? "(最終)" : "" }}
      </span>
    </button>
    <p class="bg-muted px-4 py-1 text-xs text-muted-foreground">
      非公式の参考情報です。公式の警報ではありません。
    </p>
  </section>
</template>
