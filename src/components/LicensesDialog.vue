<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";

const open = defineModel<boolean>("open", { required: true });

type Entry = { name: string; use: string; license: string; url?: string; text?: string };

const MIT_INGEN084 = `MIT License

Copyright (c) 2025 ingen084

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`;

/** データ・情報源(手書き。利用条件は各提供元の規約に従う) */
const data: Entry[] = [
  { name: "気象庁 防災情報XML・走時表(JMA2001)・震度観測点・地震情報の地域区分・市町村等(地震津波関係)", use: "地震情報、到達円、国内の地図", license: "政府標準利用規約(第2.0版)に準拠", url: "https://www.jma.go.jp/jma/kishou/info/coment.html" },
  { name: "国土数値情報 行政区域", use: "国内の地図", license: "国土数値情報ダウンロードサービス利用規約に準拠", url: "https://nlftp.mlit.go.jp/ksj/other/agreement.html" },
  { name: "Natural Earth", use: "国外の地図", license: "パブリックドメイン", url: "https://www.naturalearthdata.com/about/terms-of-use/" },
  { name: "強震モニタ(防災科学技術研究所)", use: "リアルタイム震度", license: "防災科研の利用条件に従う", url: "https://www.kmoni.bosai.go.jp/" },
  { name: "kyoshin-monitor-observation-points", use: "強震モニタの観測点の位置", license: "MIT License", url: "https://github.com/ingen084/kyoshin-monitor-observation-points", text: MIT_INGEN084 },
  { name: "P2P地震情報", use: "地震履歴、緊急地震速報(警報)", license: "P2P地震情報の利用規約に従う", url: "https://www.p2pquake.net/develop/" },
  { name: "Wolfx API", use: "緊急地震速報(非公式の参考情報)", license: "Wolfx の利用規約に従う", url: "https://wolfx.jp/" },
  { name: "AXIS", use: "緊急地震速報", license: "AXIS の利用規約に従う" },
  { name: "SQLite", use: "履歴の保存", license: "パブリックドメイン", url: "https://www.sqlite.org/copyright.html" },
];

interface Pkg {
  kind: "rust" | "npm";
  name: string;
  version: string;
  license: string;
  repo: string | null;
  texts: number[];
}
// 約800KBあるので、ダイアログを開いたときに読み込む
const generated = ref<{ packages: Pkg[]; texts: string[] } | null>(null);
watch(
  open,
  async (o) => {
    if (o && !generated.value) generated.value = (await import("@/lib/licenses.generated.json")).default as never;
  },
  { immediate: true },
);

const query = ref("");
const packages = computed(() => {
  const q = query.value.trim().toLowerCase();
  const all = generated.value?.packages ?? [];
  return q ? all.filter((p) => `${p.name} ${p.license}`.toLowerCase().includes(q)) : all;
});
const KIND_LABEL = { rust: "Rust", npm: "JavaScript" } as const;
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent>
      <DialogHeader>
        <DialogTitle>ライセンス</DialogTitle>
        <DialogDescription>使用しているデータ・情報源とソフトウェアです。</DialogDescription>
      </DialogHeader>
      <ScrollArea class="h-[65vh] pr-3">
        <div class="space-y-6 text-sm">
          <section>
            <h3 class="mb-2 font-semibold">データ・情報源</h3>
            <ul class="space-y-2">
              <li v-for="e in data" :key="e.name">
                <details v-if="e.text">
                  <summary class="cursor-pointer font-medium">{{ e.name }}</summary>
                  <pre class="mt-1 whitespace-pre-wrap rounded-md bg-muted p-3 text-xs">{{ e.text }}</pre>
                </details>
                <div v-else class="font-medium">{{ e.name }}</div>
                <div class="text-xs text-muted-foreground">
                  {{ e.use }} / {{ e.license }}
                  <template v-if="e.url"> / <span class="break-all">{{ e.url }}</span></template>
                </div>
              </li>
            </ul>
          </section>

          <section>
            <h3 class="mb-1 font-semibold">ソフトウェア(アプリに含まれるライブラリ)</h3>
            <p class="mb-2 text-xs text-muted-foreground">
              項目を開くと、各ライブラリが同梱しているライセンス全文を表示します。
            </p>
            <Input v-model="query" placeholder="名前・ライセンスで絞り込む" class="mb-2" />
            <p v-if="!generated" class="text-xs text-muted-foreground">読み込み中…</p>
            <p v-else class="mb-2 text-xs text-muted-foreground">{{ packages.length }}件</p>
            <ul class="divide-y">
              <li v-for="p in packages" :key="`${p.kind}-${p.name}-${p.version}`" class="py-1.5">
                <details>
                  <summary class="cursor-pointer">
                    <span class="font-medium">{{ p.name }}</span>
                    <span class="ml-1.5 text-xs text-muted-foreground">{{ p.version }} · {{ p.license }} · {{ KIND_LABEL[p.kind] }}</span>
                  </summary>
                  <p v-if="p.repo" class="mt-1 break-all text-xs text-muted-foreground">{{ p.repo }}</p>
                  <pre
                    v-for="t in p.texts"
                    :key="t"
                    class="mt-1 whitespace-pre-wrap rounded-md bg-muted p-3 text-xs"
                  >{{ generated!.texts[t] }}</pre>
                  <p v-if="!p.texts.length" class="mt-1 text-xs text-muted-foreground">
                    ライセンス全文のファイルは同梱されていません(宣言されたライセンス: {{ p.license }})。
                  </p>
                </details>
              </li>
            </ul>
          </section>
        </div>
      </ScrollArea>
    </DialogContent>
  </Dialog>
</template>
