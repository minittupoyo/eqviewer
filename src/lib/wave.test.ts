import { describe, expect, it } from "vitest";
import { arrivalTime, circle, circlesBounds, distanceKm, radiusAt } from "./wave";

describe("radiusAt", () => {
  const dist = [0, 10, 20];
  const times = [2, 4, 8];

  it("interpolates linearly between mesh points", () => {
    expect(radiusAt(times, dist, 3)).toBeCloseTo(5);
    expect(radiusAt(times, dist, 6)).toBeCloseTo(15);
  });
  it("returns the mesh value on exact hits", () => {
    expect(radiusAt(times, dist, 2)).toBe(0);
    expect(radiusAt(times, dist, 8)).toBe(20);
  });
  it("is null before the wave reaches the surface and beyond the table", () => {
    expect(radiusAt(times, dist, 1)).toBeNull();
    expect(radiusAt(times, dist, 9)).toBeNull();
    expect(radiusAt([], [], 1)).toBeNull();
  });
});

describe("circle", () => {
  it("is closed and every vertex is ~r km from the center", () => {
    const ring = circle(137, 37, 100);
    expect(ring[0]).toEqual(ring[ring.length - 1]);
    const km = ([lon, lat]: [number, number]) => {
      const r = Math.PI / 180;
      const a =
        Math.sin(((lat - 37) * r) / 2) ** 2 +
        Math.cos(37 * r) * Math.cos(lat * r) * Math.sin(((lon - 137) * r) / 2) ** 2;
      return 2 * 6371.0088 * Math.asin(Math.sqrt(a));
    };
    for (const p of ring) expect(km(p)).toBeCloseTo(100, 1);
  });
});

describe("arrivalTime", () => {
  const dist = [0, 10, 20];
  const times = [2, 4, 8];

  it("is the inverse of radiusAt", () => {
    expect(arrivalTime(times, dist, 5)).toBeCloseTo(3);
    expect(arrivalTime(times, dist, 15)).toBeCloseTo(6);
    for (const t of [2.5, 5, 7.9]) {
      expect(arrivalTime(times, dist, radiusAt(times, dist, t)!)).toBeCloseTo(t);
    }
  });
  it("is null beyond the table", () => {
    expect(arrivalTime(times, dist, 21)).toBeNull();
    expect(arrivalTime([], [], 1)).toBeNull();
  });
});

describe("distanceKm", () => {
  it("matches known distances", () => {
    expect(distanceKm(139.69, 35.69, 139.69, 35.69)).toBeCloseTo(0);
    // 東京〜大阪は約400km
    expect(distanceKm(139.69, 35.69, 135.5, 34.69)).toBeGreaterThan(390);
    expect(distanceKm(139.69, 35.69, 135.5, 34.69)).toBeLessThan(410);
  });
});

describe("circlesBounds", () => {
  it("is null without circles", () => {
    expect(circlesBounds([], 50)).toBeNull();
  });

  it("raises tiny radii to the minimum", () => {
    const [[w], [e]] = circlesBounds([{ lon: 140, lat: 35, rKm: 0 }], 100)!;
    expect(e - w).toBeGreaterThan(1.5);
  });

  it("covers every circle", () => {
    const [[w, s], [e, n]] = circlesBounds(
      [
        { lon: 130, lat: 33, rKm: 100 },
        { lon: 142, lat: 43, rKm: 100 },
      ],
      50,
    )!;
    expect(w).toBeLessThan(130);
    expect(e).toBeGreaterThan(142);
    expect(s).toBeLessThan(33);
    expect(n).toBeGreaterThan(43);
  });
});
