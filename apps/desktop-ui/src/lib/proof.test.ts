import { describe, it, expect, vi, afterEach } from "vitest";
import { setUiLanguage } from "./i18n-core";
import { captureProof, flattenProofPages } from "./proof";

afterEach(() => vi.unstubAllGlobals());

describe("proof helpers", () => {
  it("localizes known server errors", async () => {
    setUiLanguage("en");
    vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({ error: "window_minimized" }), { status: 500 })));
    await expect(captureProof()).rejects.toThrow("The Zoom meeting window is minimized");
    setUiLanguage("zh-CN");
    await expect(captureProof()).rejects.toThrow("Zoom 会议窗口已最小化");
  });

  it("returns the page count from a successful capture", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({ captureId: "proof-1", pages: 3 }), { status: 200 })));
    expect((await captureProof()).pages).toBe(3);
  });

  it("flattens newest capture first", () => {
    const pages = flattenProofPages({ proofCaptures: [{ id: "old", pages: [{ file: "a" }] }, { id: "new", pages: [{ file: "b" }] }] });
    expect(pages.map((page: any) => page.captureId)).toEqual(["new", "old"]);
    expect(pages[0].pageNumber).toBe(1);
  });
});
