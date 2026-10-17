import { createApp } from "vue";
import { createPinia } from "pinia";
import { setWorkerUrl } from "maplibre-gl";
// MapLibre 6 は import.meta.url からワーカーを探すが、Viteでバンドルするとそのファイルが出力されない。
// ワーカーを依存ごと別ファイルとして出力し、そのURLを明示する。
import workerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url";
import "@fontsource-variable/inter";
import "@fontsource-variable/noto-sans-jp";
import App from "./App.vue";
import "./style.css";

setWorkerUrl(workerUrl);

createApp(App).use(createPinia()).mount("#app");
