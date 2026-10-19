<script setup lang="ts">
import { useHistoryStore } from "@/stores/history";
import { formatTime, scaleClass, scaleLabel } from "@/lib/earthquake";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";

const store = useHistoryStore();

function select(id: string) {
  store.selectedId = id;
}
</script>

<template>
  <ScrollArea class="h-full">
    <ul>
      <li v-for="eq in store.items" :key="eq.id">
        <button
          class="flex w-full items-center gap-3 border-b px-3 py-2 text-left hover:bg-accent"
          :class="{ 'bg-accent': eq.id === store.selected?.id }"
          @click="select(eq.id)"
        >
          <Badge class="w-14 shrink-0 justify-center" :class="scaleClass(eq.max_scale)">
            {{ scaleLabel(eq.max_scale) }}
          </Badge>
          <span class="min-w-0 flex-1">
            <span class="block truncate text-sm font-medium">{{ eq.name ?? "震源調査中" }}</span>
            <span class="block text-xs text-muted-foreground">{{ formatTime(eq.origin_time) }}</span>
          </span>
          <span class="shrink-0 text-sm tabular-nums">
            {{ eq.magnitude != null ? `M${eq.magnitude.toFixed(1)}` : "M不明" }}
          </span>
        </button>
      </li>
    </ul>
    <button
      v-if="store.hasMore && store.items.length"
      class="w-full py-3 text-sm text-muted-foreground hover:text-foreground"
      @click="store.loadMore()"
    >
      さらに読み込む
    </button>
  </ScrollArea>
</template>
