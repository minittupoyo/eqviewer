<script setup lang="ts">
import { computed } from "vue";
import { Activity, List } from "@lucide/vue";
import { useEewStore } from "@/stores/eew";
import { useKmoniStore } from "@/stores/kmoni";

const eew = useEewStore();
const kmoni = useKmoniStore();

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
</script>

<template>
  <nav class="flex w-20 shrink-0 flex-col border-r bg-muted/40" aria-label="表示の切り替え">
    <button
      v-for="item in items"
      :key="item.id"
      type="button"
      class="flex flex-col items-center gap-1 border-l-2 px-1 py-3 text-xs hover:bg-accent"
      :class="
        current === item.id
          ? 'border-primary bg-accent font-medium text-foreground'
          : 'border-transparent text-muted-foreground'
      "
      :aria-current="current === item.id ? 'page' : undefined"
      @click="show(item.id)"
    >
      <component :is="item.icon" class="size-5" aria-hidden="true" />
      {{ item.label }}
    </button>
  </nav>
</template>
