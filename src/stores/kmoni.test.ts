import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const invoke = vi.fn(async () => []);
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...(a as [])) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

function stubStorage(initial: Record<string, string> = {}) {
  const data = { ...initial };
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => data[k] ?? null,
    setItem: (k: string, v: string) => void (data[k] = v),
  });
  return data;
}

async function freshStore() {
  vi.resetModules();
  setActivePinia(createPinia());
  const { useKmoniStore } = await import("./kmoni");
  return useKmoniStore();
}

beforeEach(() => {
  invoke.mockClear();
  vi.unstubAllGlobals();
});

describe("前回開いていた項目の記憶", () => {
  it("初回(記憶なし)は強震モニタから始める", async () => {
    stubStorage();
    expect((await freshStore()).viewing).toBe(true);
  });

  it("前回が地震情報なら地震情報から始める", async () => {
    stubStorage({ "eqviewer.section": "quake" });
    expect((await freshStore()).viewing).toBe(false);
  });

  it("前回が強震モニタなら強震モニタから始める", async () => {
    stubStorage({ "eqviewer.section": "kmoni" });
    expect((await freshStore()).viewing).toBe(true);
  });

  it("切り替えると記憶される", async () => {
    const data = stubStorage();
    const store = await freshStore();
    store.setViewing(false);
    expect(data["eqviewer.section"]).toBe("quake");
    store.setViewing(true);
    expect(data["eqviewer.section"]).toBe("kmoni");
  });

  it("localStorage が使えなくても強震モニタから始まり、切り替えも例外にならない", async () => {
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    });
    const store = await freshStore();
    expect(store.viewing).toBe(true);
    expect(() => store.setViewing(false)).not.toThrow();
    expect(store.viewing).toBe(false);
  });
});

describe("バックエンドとの同期", () => {
  it("起動時に開いている項目をバックエンドに伝える(強震モニタなら取得を始める)", async () => {
    stubStorage();
    const store = await freshStore();
    await store.start();
    expect(invoke).toHaveBeenCalledWith("kmoni_set_view", { on: true });
  });

  it("地震情報から始めたときは取得を始めない", async () => {
    stubStorage({ "eqviewer.section": "quake" });
    const store = await freshStore();
    await store.start();
    expect(invoke).toHaveBeenCalledWith("kmoni_set_view", { on: false });
  });

  it("切り替えのたびにバックエンドに伝える", async () => {
    stubStorage();
    const store = await freshStore();
    store.setViewing(false);
    expect(invoke).toHaveBeenLastCalledWith("kmoni_set_view", { on: false });
  });
});
