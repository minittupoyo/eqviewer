import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type SourceState =
  | { state: "connecting" }
  | { state: "connected" }
  | { state: "disconnected"; detail: string };

export interface Settings {
  axis_enabled: boolean;
  axis_token: string;
  kmoni_always: boolean;
  close_to_tray: boolean;
  focus_on_eew: boolean;
  show_legend: boolean;
  sound_enabled: boolean;
  sound_volume: number;
  home_lat: number | null;
  home_lon: number | null;
}

export const SOURCE_LABEL: Record<string, string> = {
  axis: "AXIS",
  wolfx: "wolfx",
  p2pquake: "P2P地震情報(警報のみ)",
};

export const useSourcesStore = defineStore("sources", () => {
  const status = ref<Record<string, SourceState>>({});
  const settings = ref<Settings>({ axis_enabled: false, axis_token: "", kmoni_always: false, close_to_tray: true, focus_on_eew: true, show_legend: true, sound_enabled: true, sound_volume: 70, home_lat: null, home_lon: null });

  async function start() {
    status.value = await invoke("source_status");
    settings.value = await invoke("get_settings");
    await listen<Record<string, SourceState>>("source-status", (e) => (status.value = e.payload));
  }

  async function save(next: Settings) {
    await invoke("save_settings", { new: next });
    settings.value = next;
  }

  return { status, settings, start, save };
});
