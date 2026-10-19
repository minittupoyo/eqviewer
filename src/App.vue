<script setup lang="ts">
import { onMounted } from "vue";
import { useHistoryStore } from "@/stores/history";
import { useEewStore } from "@/stores/eew";
import EewPanel from "@/components/EewPanel.vue";
import EarthquakeList from "@/components/EarthquakeList.vue";
import { useSourcesStore } from "@/stores/sources";
import AppHeader from "@/components/AppHeader.vue";
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
    <AppHeader />
    <div class="flex min-h-0 flex-1">
      <!-- 強震モニタは地図だけ。履歴の一覧は地震情報の項目でのみ出す -->
      <main
        class="relative grid min-h-0 flex-1"
        :class="kmoni.viewing ? 'grid-cols-1' : 'grid-cols-[22rem_1fr]'"
      >
        <aside v-if="!kmoni.viewing" class="min-h-0 border-r"><EarthquakeList /></aside>
        <!-- 地図は切り替えのたびに作り直さず保持する(日本の形の読み込みが重いため) -->
        <KeepAlive>
          <EewMap v-if="eew.showMap" />
          <KmoniMap v-else-if="kmoni.viewing" />
          <EarthquakeDetail v-else-if="store.selected" :eq="store.selected" />
        </KeepAlive>
        <!-- 地図に出せない速報(取消・震源不明)や、履歴の画面でも見落とさないよう、地図のない表示ではここに出す -->
        <EewPanel v-if="!eew.showMap && !kmoni.viewing" class="absolute left-3 top-3 z-10" />
        <p v-if="!eew.showMap && !kmoni.viewing && !store.selected" class="p-4 text-sm text-muted-foreground">履歴を取得しています…</p>
      </main>
    </div>
  </div>
</template>
