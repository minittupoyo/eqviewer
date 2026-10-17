import type { Eew } from "@/lib/earthquake";

export type EewSound = "forecast" | "warning";

/**
 * 速報の更新で鳴らす音を決める。新しい地震の最初の報、または予報から警報への切り替わりで鳴らす。
 * 続報・取消・訓練では鳴らさない。警報と予報が同時に来たら警報を優先する。
 */
export function soundFor(prev: Eew[], next: Eew[]): EewSound | null {
  const before = new Map(prev.map((e) => [e.event_id, e]));
  let result: EewSound | null = null;
  for (const e of next) {
    if (e.is_cancel || e.is_training) continue;
    const old = before.get(e.event_id);
    if (!old) result = e.is_warning ? "warning" : (result ?? "forecast");
    else if (e.is_warning && !old.is_warning) result = "warning";
    if (result === "warning") break;
  }
  return result;
}

let ctx: AudioContext | undefined;
function context() {
  try {
    ctx ??= new AudioContext();
    return ctx;
  } catch {
    return undefined;
  }
}

// ブラウザの自動再生制限で止まっている場合に備え、最初の操作で再開しておく
if (typeof window !== "undefined") {
  window.addEventListener("pointerdown", () => void context()?.resume(), { once: true });
}

/** (周波数Hz, 開始秒, 長さ秒) の列を、減衰する音として鳴らす */
type Note = [number, number, number];

const CHIME: Note[] = [
  [880, 0, 0.35],
  [1175, 0.18, 0.5],
];
const ALARM: Note[] = Array.from({ length: 6 }, (_, i): Note[] => [
  [1046, i * 0.5, 0.22],
  [784, i * 0.5 + 0.25, 0.22],
]).flat();

/**
 * 音を鳴らす。音源ファイルは使わず、その場で合成する(同梱ファイルの権利を避け、サイズも増やさない)。
 * `volume` は 0〜100。
 */
export function playEewSound(kind: EewSound, volume: number) {
  const c = context();
  if (!c || volume <= 0) return;
  void c.resume();
  const peak = (Math.min(100, volume) / 100) * (kind === "warning" ? 0.5 : 0.35);
  const wave: OscillatorType = kind === "warning" ? "square" : "sine";
  const t0 = c.currentTime + 0.02;
  for (const [freq, start, len] of kind === "warning" ? ALARM : CHIME) {
    const osc = c.createOscillator();
    const gain = c.createGain();
    osc.type = wave;
    osc.frequency.value = freq;
    gain.gain.setValueAtTime(0.0001, t0 + start);
    gain.gain.exponentialRampToValueAtTime(peak, t0 + start + 0.01);
    gain.gain.exponentialRampToValueAtTime(0.0001, t0 + start + len);
    osc.connect(gain).connect(c.destination);
    osc.start(t0 + start);
    osc.stop(t0 + start + len + 0.02);
  }
}
