<script setup lang="ts">
import { onMounted } from "vue";
import { useHistoryStore } from "@/stores/history";
import { useEewStore } from "@/stores/eew";
import EewBanner from "@/components/EewBanner.vue";
import EarthquakeList from "@/components/EarthquakeList.vue";
import StatusBar from "@/components/StatusBar.vue";
import { useSourcesStore } from "@/stores/sources";
import SideNav from "@/components/SideNav.vue";
import KmoniMap from "@/components/KmoniMap.vue";
import { useKmoniStore } from "@/stores/kmoni";
import { useClockStore } from "@/stores/clock";
import EewMap from "@/components/EewMap.vue";
import EarthquakeDetail from "@/components/EarthquakeDetail.vue";

const store = useHistoryStore();
const eew = useEewStore();
const sources = useSourcesStore();
const kmoni = useKmoniStore();
const clock = useClockStore();
onMounted(() => {
  store.start();
  eew.start();
  sources.start();
  kmoni.start();
  clock.start();
});
</script>

<template>
  <div class="flex h-screen flex-col bg-background text-foreground">
    <EewBanner />
    <div class="flex min-h-0 flex-1">
      <SideNav />
      <!-- 強震モニタは地図だけ。履歴の一覧は地震情報の項目でのみ出す -->
      <main
        class="grid min-h-0 flex-1"
        :class="kmoni.viewing ? 'grid-cols-1' : 'grid-cols-[22rem_1fr]'"
      >
        <aside v-if="!kmoni.viewing" class="min-h-0 border-r"><EarthquakeList /></aside>
        <EewMap v-if="eew.showMap" />
        <KmoniMap v-else-if="kmoni.viewing" />
        <EarthquakeDetail v-else-if="store.selected" :eq="store.selected" />
        <p v-else class="p-4 text-sm text-muted-foreground">履歴を取得しています…</p>
      </main>
    </div>
    <StatusBar />
  </div>
</template>
