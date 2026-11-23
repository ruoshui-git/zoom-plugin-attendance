// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import { meeting, recordEvents, record } from "../lib/stores";
import { tick } from "svelte";
import LivePage from "./LivePage.svelte";
import { toast } from "../lib/toast";

describe("LivePage new record action", () => {
  afterEach(() => { cleanup(); meeting.set(null); });

  it("does not offer starting a new record without an active meeting", () => {
    meeting.set(null);
    render(LivePage);
    expect(screen.queryByRole("button", { name: "Start new record" })).toBeNull();
  });
  it("captures proof and maps server errors into an error toast", async () => {
    meeting.set({ instanceId: "i", status: "live", startedAt: 0 } as any);
    vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({ error: "window_minimized" }), { status: 500 })));
    const onToast = vi.fn(); const unsubscribe = toast.subscribe(onToast);
    render(LivePage);
    await fireEvent.click(screen.getByRole("button", { name: "Capture proof" }));
    await waitFor(() => expect(onToast.mock.calls.some((call: any[]) => call[0].some((item: any) => item.message.includes("Zoom meeting window is minimized")))).toBe(true));
    unsubscribe(); vi.unstubAllGlobals();
  });
  it("keeps the live participant list current as events arrive (camera-off count)", async () => {
    meeting.set({ instanceId: "i", meetingNumber: "1", topic: "T", startedAt: 0 } as any);
    record.set({ rows: [], summary: {} });
    recordEvents.set([{ kind: "users", action: "sync_all", seq: 1, at: 1, users: [{ userId: 1, name: "A", videoOn: false }, { userId: 2, name: "B", videoOn: true }] }]);
    render(LivePage);
    await tick();
    const count = () => document.querySelector(".camera-off b")?.textContent;
    expect(count()).toBe("1");
    recordEvents.update((events) => [...events, { kind: "users", action: "update", seq: 2, at: 2, users: [{ userId: 2, name: "B", videoOn: false }] }]);
    await tick();
    expect(count()).toBe("2");
    recordEvents.set([]);
  });
});
