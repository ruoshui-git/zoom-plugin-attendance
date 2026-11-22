import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get as read } from "svelte/store";

// api.ts keeps a module-level record cache, so each test loads fresh modules.
let api: typeof import("./api");
let s: typeof import("./stores");
let urls: string[] = [];
let gate: Promise<void> | null = null;

const meetings = [
  { instanceId: "e1", endedAt: 2, mode: "roster" },
  { instanceId: "e2", endedAt: 2, mode: null },
  { instanceId: "e3", endedAt: 2 },
  { instanceId: "live", endedAt: null, mode: "roster" },
];
function respond(body: any) {
  return new Response(JSON.stringify(body), { status: 200, headers: { "Content-Type": "application/json" } });
}

beforeEach(async () => {
  vi.resetModules();
  urls = [];
  gate = null;
  vi.stubGlobal("fetch", vi.fn(async (url: string) => {
    urls.push(url);
    if (url === "/api/meetings") { if (gate) await gate; return respond(meetings); }
    const record = url.match(/^\/api\/meetings\/([^/]+)\/record$/);
    if (record) return respond({ rows: [], summary: { seen: record[1].length } });
    if (url.endsWith("/state")) return respond({ revision: 1, value: { mode: "roster" } });
    return new Response("", { status: 404 });
  }));
  s = await import("./stores");
  api = await import("./api");
});
afterEach(() => vi.unstubAllGlobals());

const recordFetches = () => urls.filter((u) => u.endsWith("/record"));

describe("refresh", () => {
  it("fetches each record once, then only the live one", async () => {
    await api.refresh();
    expect(recordFetches().sort()).toEqual(["/api/meetings/e1/record", "/api/meetings/e2/record", "/api/meetings/e3/record", "/api/meetings/live/record"]);
    expect(read(s.recordCounts)).toEqual({ e1: 2, e2: 2, e3: 2, live: 4 });
    expect(read(s.meeting)?.instanceId).toBe("live");
    urls = [];
    await api.refresh();
    expect(urls.filter((u) => !u.endsWith("/state"))).toEqual(["/api/meetings", "/api/meetings/live/record"]);
  });

  it("refetches a meeting named as changed", async () => {
    await api.refresh();
    urls = [];
    await api.refresh({ changed: ["e2"] });
    expect(recordFetches().sort()).toEqual(["/api/meetings/e2/record", "/api/meetings/live/record"]);
  });

  it("takes attendance modes from the meetings list without per-meeting state requests", async () => {
    await api.refresh();
    expect(read(s.recordModes)).toEqual({ e1: "roster", e2: "generic", e3: "generic", live: "roster" });
    // Only the live meeting's state is loaded (for the Live page's mode switch).
    expect(urls.filter((u) => u.endsWith("/state"))).toEqual(["/api/meetings/live/state"]);
  });

  it("coalesces overlapping calls into at most one follow-up run", async () => {
    let release!: () => void;
    gate = new Promise((r) => (release = r));
    const runs = [api.refresh(), api.refresh({ changed: ["e1"] }), api.refresh({ changed: ["e3"] }), api.refresh()];
    release();
    gate = null;
    await Promise.all(runs);
    expect(urls.filter((u) => u === "/api/meetings")).toHaveLength(2);
  });
});
