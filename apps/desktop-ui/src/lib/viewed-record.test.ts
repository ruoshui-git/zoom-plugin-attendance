import { afterEach, describe, expect, it, vi } from "vitest";
import { get as read } from "svelte/store";
import * as s from "./stores";
import { deleteRecords, loadViewedRecord, saveRecordState } from "./api";

type Reply = { body: any; delay?: number };
function mockFetch(routes: Record<string, Reply>) {
  const calls: { url: string; init?: any }[] = [];
  vi.stubGlobal("fetch", vi.fn(async (url: string, init?: any) => {
    calls.push({ url, init });
    const reply = routes[`${init?.method || "GET"} ${url}`];
    if (!reply) return new Response("not found", { status: 404 });
    if (reply.delay) await new Promise((r) => setTimeout(r, reply.delay));
    return new Response(JSON.stringify(reply.body), { status: 200, headers: { "Content-Type": "application/json" } });
  }));
  return calls;
}
const meeting = (id: string) => ({ instanceId: id, meetingNumber: id, topic: id, startedAt: 1, endedAt: 2 }) as any;
const routesFor = (id: string, delay = 0, revision = 7) => ({
  [`GET /api/meetings/${id}/record`]: { body: { rows: [{ canonicalName: `row-${id}` }], summary: { total: 1 } }, delay },
  [`GET /api/meetings/${id}/state`]: { body: { revision, value: { mode: "roster", decisions: [{ id: "d1" }] } } },
  [`GET /api/meetings/${id}/events`]: { body: [] },
});

afterEach(() => {
  vi.unstubAllGlobals();
  s.meeting.set(null);
  s.record.set({ rows: [], summary: {} });
  s.viewedMeeting.set(null);
});

describe("viewed record state", () => {
  it("opening a record leaves the live meeting stores untouched", async () => {
    mockFetch(routesFor("past"));
    const live = meeting("live");
    s.meeting.set(live);
    s.record.set({ rows: [{ canonicalName: "live row" }], summary: {} });
    await loadViewedRecord(meeting("past"));
    expect(read(s.meeting)).toBe(live);
    expect(read(s.record).rows[0].canonicalName).toBe("live row");
    expect(read(s.viewedRecord).rows[0].canonicalName).toBe("row-past");
    expect(read(s.viewedState).reconciliationDecisions).toEqual([{ id: "d1" }]);
  });

  it("ignores a slower response for a record that is no longer viewed", async () => {
    mockFetch({ ...routesFor("slow", 40), ...routesFor("fast") });
    const slow = loadViewedRecord(meeting("slow"));
    await loadViewedRecord(meeting("fast"));
    await slow;
    expect(read(s.viewedMeeting)?.instanceId).toBe("fast");
    expect(read(s.viewedRecord).rows[0].canonicalName).toBe("row-fast");
  });

  it("saves viewed-record changes with the revision that was loaded", async () => {
    const calls = mockFetch({ ...routesFor("past", 0, 7), "PUT /api/meetings/past/state": { body: { revision: 8, value: { mode: "generic" } } } });
    await loadViewedRecord(meeting("past"));
    await saveRecordState({ mode: "generic" }, "viewed");
    const put = calls.find((c) => c.init?.method === "PUT");
    expect(JSON.parse(put!.init.body).revision).toBe(7);
    expect(read(s.viewedStateRevision)).toBe(8);
  });
});

describe("deleteRecords", () => {
  it("keeps the live meeting and an unrelated viewed record when other records are deleted", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response("[]", { status: 200, headers: { "Content-Type": "application/json" } })));
    s.meeting.set(meeting("live-1"));
    s.viewedMeeting.set(meeting("kept-1"));
    await deleteRecords(["old-1", "old-2"]);
    expect(read(s.meeting)?.instanceId).toBe("live-1");
    expect(read(s.viewedMeeting)?.instanceId).toBe("kept-1");
    await deleteRecords(["kept-1"]);
    expect(read(s.viewedMeeting)).toBeNull();
    expect(read(s.meeting)?.instanceId).toBe("live-1");
  });
});
