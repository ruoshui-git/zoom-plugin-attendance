import { describe, expect, it } from "vitest";
import { get } from "svelte/store";
import { busyKeys, runBusy } from "./busy";

describe("runBusy", () => {
  it("skips a second run while the first is pending and clears the key afterwards", async () => {
    let release!: () => void;
    let calls = 0;
    const first = runBusy("save", () => { calls++; return new Promise<string>((r) => { release = () => r("done"); }); });
    expect(get(busyKeys).has("save")).toBe(true);
    expect(await runBusy("save", () => { calls++; return "second"; })).toBeUndefined();
    release();
    expect(await first).toBe("done");
    expect(calls).toBe(1);
    expect(get(busyKeys).has("save")).toBe(false);
  });
  it("clears the key when the action throws", async () => {
    await expect(runBusy("boom", () => { throw new Error("x"); })).rejects.toThrow("x");
    expect(get(busyKeys).has("boom")).toBe(false);
  });
});
