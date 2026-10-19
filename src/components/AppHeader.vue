<script setup lang="ts">
import { computed } from "vue";
import { Activity, List } from "@lucide/vue";
import { useEewStore } from "@/stores/eew";
import { useKmoniStore } from "@/stores/kmoni";
import { SOURCE_LABEL, useSourcesStore, type SourceState } from "@/stores/sources";
import { useClockStore } from "@/stores/clock";
import SettingsDialog from "@/components/SettingsDialog.vue";

const eew = useEewStore();
const kmoni = useKmoniStore();
const sources = useSourcesStore();
const clock = useClockStore();

/** EEWの地図を出している間は、どちらの項目も選択中にしない(右側はEEWの地図だから) */
const current = computed(() => (eew.showMap ? null : kmoni.viewing ? "kmoni" : "quake"));

const items = [
  { id: "kmoni", label: "強震モニタ", icon: Activity },
  { id: "quake", label: "地震情報", icon: List },
] as const;

function show(id: (typeof items)[number]["id"]) {
  kmoni.setViewing(id === "kmoni");
  eew.focus = false;
}

/** ずれが1秒以上あるときだけ知らせる(到達円の精度に効くため)。未測定なら補正できていない旨を示す。 */
const clockNote = computed(() => {
  const ms = clock.offsetMs;
  if (ms == null) return { text: "時刻未補正", title: "NTPサーバーに接続できていません。PCの時計をそのまま使います" };
  if (Math.abs(ms) < 1000) return null;
  const sec = (Math.abs(ms) / 1000).toFixed(1);
  return { text: `PCの時計が${sec}秒${ms > 0 ? "遅れて" : "進んで"}います(補正済み)`, title: "NICTのNTPサーバーとの差です" };
});

const DOT: Record<SourceState["state"], string> = {
  connected: "bg-emerald-400",
  connecting: "bg-amber-400",
  disconnected: "bg-red-500",
};
const LABEL: Record<SourceState["state"], string> = {
  connected: "接続中",
  connecting: "接続待ち",
  disconnected: "切断",
};
</script>

<template>
  <header class="flex h-12 shrink-0 items-center gap-4 bg-slate-900 px-3 text-slate-200">
    <span class="text-sm font-semibold tracking-wide text-white">EQViewer</span>

    <nav class="flex gap-1 rounded-lg bg-white/5 p-1" aria-label="表示の切り替え">
      <button
        v-for="item in items"
        :key="item.id"
        type="button"
        class="flex items-center gap-1.5 rounded-md px-3 py-1 text-sm transition-colors"
        :class="
          current === item.id
            ? 'bg-white font-medium text-slate-900'
            : 'text-slate-300 hover:bg-white/10 hover:text-white'
        "
        :aria-current="current === item.id ? 'page' : undefined"
        @click="show(item.id)"
      >
        <component :is="item.icon" class="size-4" aria-hidden="true" />
        {{ item.label }}
      </button>
    </nav>

    <!-- 受信元ごとの接続状態。EEWを受け取れているかは常に見えている必要がある -->
    <ul class="ml-auto flex items-center gap-3 text-xs" aria-label="EEW受信元の接続状態">
      <li
        v-for="(s, name) in sources.status"
        :key="name"
        class="flex items-center gap-1.5"
        :title="s.state === 'disconnected' ? s.detail : LABEL[s.state]"
      >
        <span class="size-2 rounded-full" :class="DOT[s.state]" />
        <span>{{ SOURCE_LABEL[name] ?? name }}</span>
        <span v-if="s.state !== 'connected'" class="text-amber-300">{{ LABEL[s.state] }}</span>
      </li>
      <li v-if="clockNote" :title="clockNote.title" class="text-amber-300">{{ clockNote.text }}</li>
    </ul>
    <SettingsDialog />
  </header>
</template>
