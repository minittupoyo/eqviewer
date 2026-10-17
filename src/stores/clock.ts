import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Offset {
  offset_ms: number | null;
}

/** PCの時計のずれ。`offsetMs` は「正しい時刻 − PCの時刻」で、未測定なら null。 */
export const useClockStore = defineStore("clock", () => {
  const offsetMs = ref<number | null>(null);

  async function start() {
    offsetMs.value = (await invoke<Offset>("clock_offset")).offset_ms;
    await listen<Offset>("clock-offset", (e) => (offsetMs.value = e.payload.offset_ms));
  }

  return { offsetMs, start };
});
