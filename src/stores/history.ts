import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Earthquake } from "@/lib/earthquake";

const PAGE = 100;

export const useHistoryStore = defineStore("history", () => {
  const items = ref<Earthquake[]>([]);
  const selectedId = ref<string | null>(null);
  const hasMore = ref(true);

  const selected = computed(
    () => items.value.find((e) => e.id === selectedId.value) ?? items.value[0] ?? null,
  );

  /** 先頭から読み直す(起動時・バックグラウンド同期後) */
  async function reload() {
    const n = Math.max(items.value.length, PAGE);
    items.value = await invoke<Earthquake[]>("list_earthquakes", { limit: n });
    hasMore.value = items.value.length >= n;
  }

  async function loadMore() {
    const last = items.value.at(-1);
    if (!last || !hasMore.value) return;
    const more = await invoke<Earthquake[]>("list_earthquakes", {
      limit: PAGE,
      before: last.id,
    });
    items.value.push(...more);
    hasMore.value = more.length === PAGE;
  }

  async function start() {
    await reload();
    await listen("history-updated", reload);
  }

  return { items, selected, selectedId, hasMore, start, loadMore };
});
