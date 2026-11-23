import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { loadWindowState, setPinned, windowState } from "./window-control";

describe("window control", () => {
  beforeEach(() => { windowState.set({ available: false, mode: "full", pinned: false }); vi.restoreAllMocks(); });
  it("treats older and browser servers as unavailable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 404 }));
    expect(await loadWindowState()).toEqual({ available: false, mode: "full", pinned: false });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true, json: async () => ({ available: false, mode: "full", pinned: false }) }));
    await loadWindowState();
    expect(get(windowState).available).toBe(false);
  });
  it("updates pinned state from the server response", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true, json: async () => ({ available: true, mode: "compact", pinned: true }) }));
    await setPinned(true);
    expect(get(windowState)).toEqual({ available: true, mode: "compact", pinned: true });
  });
  it("throws on update errors", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 409 }));
    await expect(setPinned(true)).rejects.toThrow("409");
  });
});
