// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import * as stores from "./stores";
import RecordDetail from "../pages/records/RecordDetail.svelte";
import LivePage from "../pages/LivePage.svelte";

const at = (h: number, m: number) => new Date(2026, 0, 1, h, m).getTime();

describe("host/co-host privilege", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    stores.viewedMeeting.set(null);
    stores.meeting.set(null);
    stores.record.set({ rows: [], summary: {} } as any);
  });

  async function openOverview(privilege: any) {
    vi.stubGlobal("fetch", vi.fn(async () => new Response("{}", { status: 200 })));
    stores.viewedMeeting.set({ instanceId: "m1", meetingNumber: "1", topic: "Demo", startedAt: at(19, 0), endedAt: at(21, 0) } as any);
    stores.viewedRecord.set({ rows: [], summary: {}, loading: false, ...(privilege ? { privilege } : {}) } as any);
    const view = render(RecordDetail);
    await fireEvent.click(screen.getByRole("tab", { name: /Overview/ }));
    return view;
  }

  it("shows co-host, the acquired time and the partial waiting-room coverage note", async () => {
    const { container } = await openOverview({ current: "cohost", acquiredAt: at(19, 42), intervals: [{ from: at(19, 42), to: at(20, 30) }], coveredWholeMeeting: false });
    expect(container.textContent).toMatch(/Host Privilege\s*Co-host/);
    expect(container.textContent).toMatch(/Privilege Acquired\s*19:42:00/);
    expect(container.textContent).toContain("Waiting-room data is only recorded while you are host or co-host: 19:42–20:30.");
  });

  it("says waiting-room data was not recorded when never privileged", async () => {
    const { container } = await openOverview({ current: "none", acquiredAt: null, intervals: [], coveredWholeMeeting: false });
    expect(container.textContent).toMatch(/Privilege Acquired\s*Not acquired/);
    expect(container.textContent).toContain("You were not host or co-host in this meeting");
  });

  it("shows a dash and no note for imported records without privilege data", async () => {
    const { container } = await openOverview(null);
    expect(container.textContent).toMatch(/Host Privilege\s*—/);
    expect(container.querySelector(".privilege-note")).toBeNull();
  });

  it("warns on the Live page only while the live record says the user is not host or co-host", async () => {
    stores.meeting.set({ instanceId: "live", meetingNumber: "1", topic: "Live", startedAt: at(19, 0) } as any);
    stores.record.set({ rows: [], summary: {} } as any);
    render(LivePage);
    expect(screen.queryByText(/You are not host or co-host/)).toBeNull();
    stores.record.set({ rows: [], summary: {}, privilege: { current: "none", intervals: [], coveredWholeMeeting: false } } as any);
    await Promise.resolve();
    expect(await screen.findByText(/You are not host or co-host/)).toBeTruthy();
    stores.record.set({ rows: [], summary: {}, privilege: { current: "cohost", intervals: [], coveredWholeMeeting: false } } as any);
    await Promise.resolve(); await new Promise((r) => setTimeout(r, 0));
    expect(screen.queryByText(/You are not host or co-host/)).toBeNull();
  });
});
