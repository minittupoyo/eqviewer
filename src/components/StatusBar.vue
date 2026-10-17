<script setup lang="ts">
import { SOURCE_LABEL, useSourcesStore, type SourceState } from "@/stores/sources";
import { computed } from "vue";
import { useClockStore } from "@/stores/clock";
import SettingsDialog from "@/components/SettingsDialog.vue";

const store = useSourcesStore();
const clock = useClockStore();

/** ずれが1秒以上あるときだけ知らせる(到達円の精度に効くため)。未測定なら補正できていない旨を示す。 */
const clockNote = computed(() => {
  const ms = clock.offsetMs;
  if (ms == null) return { text: "時刻未補正", title: "NTPサーバーに接続できていません。PCの時計をそのまま使います" };
  if (Math.abs(ms) < 1000) return null;
  const sec = (Math.abs(ms) / 1000).toFixed(1);
  return { text: `PCの時計が${sec}秒${ms > 0 ? "遅れて" : "進んで"}います(補正済み)`, title: "NICTのNTPサーバーとの差です" };
});

const DOT: Record<SourceState["state"], string> = {
  connected: "bg-emerald-500",
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
  <footer class="flex items-center gap-4 border-t px-3 py-1 text-xs text-muted-foreground">
    <span>EEW受信:</span>
    <span
      v-for="(s, name) in store.status"
      :key="name"
      class="flex items-center gap-1.5"
      :title="s.state === 'disconnected' ? s.detail : undefined"
    >
      <span class="inline-block size-2 rounded-full" :class="DOT[s.state]" />
      {{ SOURCE_LABEL[name] ?? name }} {{ LABEL[s.state] }}
    </span>
    <span v-if="clockNote" class="ml-auto" :title="clockNote.title">{{ clockNote.text }}</span>
    <span :class="clockNote ? '' : 'ml-auto'"><SettingsDialog /></span>
  </footer>
</template>
