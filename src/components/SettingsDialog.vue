<script setup lang="ts">
import { ref, watch } from "vue";
import { useSourcesStore } from "@/stores/sources";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { playEewSound } from "@/lib/sound";
import LicensesDialog from "@/components/LicensesDialog.vue";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

const store = useSourcesStore();
const open = ref(false);
const licensesOpen = ref(false);
const enabled = ref(false);
const token = ref("");
const kmoniAlways = ref(false);
const closeToTray = ref(true);
const focusOnEew = ref(true);
const soundEnabled = ref(true);
const soundVolume = ref(70);
const homeLat = ref("");
const homeLon = ref("");
const error = ref("");

// 開くたびに保存済みの値から編集を始める
watch(open, (o) => {
  if (!o) return;
  enabled.value = store.settings.axis_enabled;
  token.value = store.settings.axis_token;
  kmoniAlways.value = store.settings.kmoni_always;
  closeToTray.value = store.settings.close_to_tray;
  focusOnEew.value = store.settings.focus_on_eew;
  soundEnabled.value = store.settings.sound_enabled;
  soundVolume.value = store.settings.sound_volume;
  homeLat.value = store.settings.home_lat?.toString() ?? "";
  homeLon.value = store.settings.home_lon?.toString() ?? "";
  error.value = "";
});

/** 空欄なら未設定(null)、数値でない・範囲外なら例外 */
function parseCoord(text: string, label: string, min: number, max: number): number | null {
  const t = text.trim();
  if (!t) return null;
  const v = Number(t);
  if (!Number.isFinite(v) || v < min || v > max) throw new Error(`${label}は${min}〜${max}の数値で入力してください`);
  return v;
}

async function save() {
  try {
    const lat = parseCoord(homeLat.value, "緯度", -90, 90);
    const lon = parseCoord(homeLon.value, "経度", -180, 180);
    if ((lat == null) !== (lon == null)) throw new Error("自宅地点は緯度と経度の両方を入力してください");
    await store.save({
      axis_enabled: enabled.value,
      axis_token: token.value.trim(),
      kmoni_always: kmoniAlways.value,
      close_to_tray: closeToTray.value,
      focus_on_eew: focusOnEew.value,
      sound_enabled: soundEnabled.value,
      sound_volume: soundVolume.value,
      home_lat: lat,
      home_lon: lon,
    });
    open.value = false;
  } catch (e) {
    error.value = String(e);
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogTrigger as-child>
      <Button variant="ghost" size="sm" class="h-6 px-2 text-xs">設定</Button>
    </DialogTrigger>
    <DialogContent>
      <DialogHeader>
        <DialogTitle>設定</DialogTitle>
        <DialogDescription>受信元と取得の設定です。</DialogDescription>
      </DialogHeader>

      <div class="space-y-4">
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col gap-1">
            <Label for="axis-enabled">AXISを使う</Label>
            <p class="text-xs text-muted-foreground">有効にするとwolfxは停止します(P2P地震情報の警報は常に受信)</p>
          </div>
          <Switch id="axis-enabled" v-model="enabled" />
        </div>
        <div class="space-y-2">
          <Label for="axis-token">AXISアクセストークン</Label>
          <Input id="axis-token" v-model="token" type="password" autocomplete="off" />
          <p class="text-xs text-muted-foreground">
            axis.prioris.jp で取得します。実験的な配信のため、再配信や一般公開はできません。
          </p>
          <p v-if="enabled && !token.trim()" class="text-xs text-destructive">
            トークンが空のため、AXISは使われずwolfxを使用します。
          </p>
        </div>
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col gap-1">
            <Label for="kmoni-always">強震モニタを常時取得する</Label>
            <p class="text-xs text-muted-foreground">緊急地震速報の地図に実測の震度を重ねます。画面を開いていない間も毎秒取得するため、通常はオフにしておけます</p>
          </div>
          <Switch id="kmoni-always" v-model="kmoniAlways" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col gap-1">
            <Label for="close-to-tray">閉じてもトレイに常駐する</Label>
            <p class="text-xs text-muted-foreground">オフにすると、ウィンドウを閉じたときにアプリを終了します。トレイのメニューからも終了できます</p>
          </div>
          <Switch id="close-to-tray" v-model="closeToTray" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col gap-1">
            <Label for="focus-on-eew">警報を受信したらウィンドウを前面に出す</Label>
            <p class="text-xs text-muted-foreground">予報のみのときは通知だけで、ウィンドウは動きません</p>
          </div>
          <Switch id="focus-on-eew" v-model="focusOnEew" />
        </div>
        <div class="space-y-2">
          <div class="flex items-center justify-between gap-4">
            <div class="flex flex-col gap-1">
              <Label for="sound-enabled">受信音を鳴らす</Label>
              <p class="text-xs text-muted-foreground">
                新しい緊急地震速報(予報は短いチャイム、警報は警報音)と、予報から警報への切り替わりで鳴らします
              </p>
            </div>
            <Switch id="sound-enabled" v-model="soundEnabled" />
          </div>
          <div v-if="soundEnabled" class="flex items-center gap-3">
            <Label for="sound-volume" class="shrink-0 text-xs font-normal">音量 {{ soundVolume }}</Label>
            <input id="sound-volume" v-model.number="soundVolume" type="range" min="0" max="100" step="5" class="w-full accent-primary" />
            <Button variant="outline" size="sm" @click="playEewSound('forecast', soundVolume)">予報</Button>
            <Button variant="outline" size="sm" @click="playEewSound('warning', soundVolume)">警報</Button>
          </div>
        </div>
        <div class="space-y-2">
          <Label for="home-lat">自宅地点(緯度・経度)</Label>
          <div class="flex gap-2">
            <Input id="home-lat" v-model="homeLat" inputmode="decimal" placeholder="緯度 例: 35.6812" />
            <Input id="home-lon" v-model="homeLon" inputmode="decimal" placeholder="経度 例: 139.7671" />
          </div>
          <p class="text-xs text-muted-foreground">
            設定すると、緊急地震速報にS波が届くまでの残り時間を出します。震度の予測は行いません。空欄で無効です。
          </p>
        </div>
        <p v-if="error" class="text-sm text-destructive">{{ error }}</p>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="licensesOpen = true">ライセンス</Button>
        <Button @click="save">保存</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
  <LicensesDialog v-model:open="licensesOpen" />
</template>
