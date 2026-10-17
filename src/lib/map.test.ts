import { describe, expect, it } from "vitest";
import { areaScales, boundsOfAreas, cityScales, type AreaCollection, type CityCollection } from "./map";

const city = (name: string, regioncode: string): CityCollection["features"][number] => ({
  type: "Feature",
  properties: { name, regioncode, pref: regioncode.slice(0, 2) },
  geometry: { type: "Polygon", coordinates: [] },
});
const cities: CityCollection = {
  type: "FeatureCollection",
  features: [city("海南市", "3020200"), city("湯浅町", "3036100"), city("千代田区", "1310100")],
};
const pt = (pref: string, addr: string, scale: string, city_code?: string) => ({
  pref,
  addr,
  scale,
  is_area: false,
  city_code,
});

describe("cityScales", () => {
  it("uses the city code directly when it is known", () => {
    const m = cityScales(cities, [pt("和歌山県", "表記ゆれのある名前", "3", "3020200")]);
    expect(m.get("3020200")).toBe("3");
  });
  it("falls back to name-prefix matching without a code", () => {
    const m = cityScales(cities, [pt("和歌山県", "湯浅町青木", "2")]);
    expect(m.get("3036100")).toBe("2");
  });
  it("falls back to name matching when the code is not on the map", () => {
    const m = cityScales(cities, [pt("和歌山県", "海南市下津", "1", "9999999")]);
    expect(m.get("3020200")).toBe("1");
  });
  it("keeps the strongest intensity per city", () => {
    const m = cityScales(cities, [
      pt("和歌山県", "海南市下津", "1", "3020200"),
      pt("和歌山県", "海南市日方", "4", "3020200"),
      pt("和歌山県", "海南市大野", "2", "3020200"),
    ]);
    expect(m.get("3020200")).toBe("4");
  });
  it("handles Tokyo wards written with a leading 東京", () => {
    const m = cityScales(cities, [pt("東京都", "東京千代田区大手町", "2")]);
    expect(m.get("1310100")).toBe("2");
  });
  it("ignores area-level entries", () => {
    const m = cityScales(cities, [{ ...pt("和歌山県", "和歌山県北部", "3"), is_area: true }]);
    expect(m.size).toBe(0);
  });
});

describe("areaScales", () => {
  const pt = (addr: string, scale: string, is_area = true) => ({ pref: "宮城県", addr, scale, is_area, city_code: null });
  it("keys by area name, keeps the maximum, ignores station points", () => {
    const m = areaScales([pt("宮城県北部", "5-"), pt("宮城県北部", "6+"), pt("宮城県南部", "4"), pt("海南市下津", "3", false)]);
    expect([...m]).toEqual([["宮城県北部", "6+"], ["宮城県南部", "4"]]);
  });
});

describe("boundsOfAreas", () => {
  const sq = (x: number, y: number) => ({ type: "Polygon" as const, coordinates: [[[x, y], [x + 1, y], [x + 1, y + 1], [x, y + 1], [x, y]]] });
  const areas: AreaCollection = {
    type: "FeatureCollection",
    features: [
      { type: "Feature", properties: { code: "1", name: "A" }, geometry: sq(130, 30) },
      { type: "Feature", properties: { code: "2", name: "B" }, geometry: sq(140, 40) },
    ],
  };
  it("covers only the named areas", () => {
    expect(boundsOfAreas(areas, new Set(["A"]))).toEqual([130, 30, 131, 31]);
    expect(boundsOfAreas(areas, new Set(["A", "B"]))).toEqual([130, 30, 141, 41]);
    expect(boundsOfAreas(areas, new Set())).toBeNull();
  });
});
