import { describe, expect, it } from "vitest";
import type { Eew } from "./earthquake";
import { soundFor } from "./sound";

const eew = (id: string, over: Partial<Eew> = {}): Eew => ({
  source: "wolfx", event_id: id, serial: 1, origin_time: "", report_time: "",
  is_warning: false, is_final: false, is_cancel: false, is_training: false,
  hypocenter: null, latitude: null, longitude: null, depth_km: null, magnitude: null,
  max_scale: null, areas: [], received_at_ms: 0, ...over,
});

describe("soundFor", () => {
  it("rings once for a new event: forecast or warning", () => {
    expect(soundFor([], [eew("a")])).toBe("forecast");
    expect(soundFor([], [eew("a", { is_warning: true })])).toBe("warning");
  });
  it("stays quiet for follow-up reports of a known event", () => {
    expect(soundFor([eew("a")], [eew("a", { serial: 2 })])).toBeNull();
    expect(soundFor([eew("a", { is_warning: true })], [eew("a", { serial: 2, is_warning: true })])).toBeNull();
  });
  it("rings the warning when a forecast is upgraded", () => {
    expect(soundFor([eew("a")], [eew("a", { serial: 2, is_warning: true })])).toBe("warning");
  });
  it("ignores cancel and training, and prefers warning when mixed", () => {
    expect(soundFor([], [eew("a", { is_cancel: true }), eew("b", { is_training: true })])).toBeNull();
    expect(soundFor([], [eew("a"), eew("b", { is_warning: true })])).toBe("warning");
  });
});
