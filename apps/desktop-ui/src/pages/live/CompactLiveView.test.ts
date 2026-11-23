// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { meeting, record, stateValue, recordEvents, mode } from "../../lib/stores";
import { uiLanguage } from "../../lib/i18n-core";

const { setWindowMode } = vi.hoisted(() => ({ setWindowMode: vi.fn(() => Promise.resolve()) }));
vi.mock("../../lib/window-control", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../lib/window-control")>()), setWindowMode }));
import CompactLiveView from "./CompactLiveView.svelte";

const user = (userId: number, name: string, extra: Record<string, unknown> = {}) => ({ userId, name, videoOn: true, raisedHand: false, waitingRoom: false, ...extra });

describe("CompactLiveView", () => {
  beforeEach(() => {
    vi.clearAllMocks(); uiLanguage.set("en");
    meeting.set({ topic: "Weekly sync", instanceId: "i", meetingNumber: "1", startedAt: 0 } as any);
    stateValue.set({ mode: "generic" });
    mode.set("generic");
    recordEvents.set([{ kind: "users", action: "sync_all", seq: 1, at: 1, users: [
      user(1, "Ada", { raisedHand: true, videoOn: false }), user(2, "Bea", { raisedHand: true }), user(3, "Cy", { raisedHand: true }),
      user(4, "Dee", { raisedHand: true }), user(5, "Eli", { raisedHand: true, videoOn: false }), user(6, "Wai", { waitingRoom: true, videoOn: false }),
    ] }]);
    record.set({ rows: [], summary: {}, record: { movementEvents: [
      { type: "join", timestamp: "2026-11-23T10:00:00Z", screenName: "Ada" },
      { type: "leave", timestamp: "2026-11-23T10:05:00Z", screenName: "Bea" },
    ] } });
  });
  afterEach(cleanup);

  it("counts present people in generic mode without expected/late/missing", () => {
    render(CompactLiveView);
    expect(screen.getByText("Present")).toBeTruthy();
    expect(screen.queryByText("Late")).toBeNull();
    expect(screen.getAllByText("5").length).toBeGreaterThan(0);
  });
  it("shows present / expected, late and missing in roster mode", () => {
    mode.set("roster");
    record.set({ rows: [
      { isExpected: true, isCurrentlyPresent: true }, { isExpected: true, isCurrentlyPresent: false }, { isExpected: true, isCurrentlyPresent: true },
    ], summary: { late: 2, missing: 1 } });
    render(CompactLiveView);
    expect(screen.getByText("Present / expected")).toBeTruthy();
    expect(screen.getByText("/ 3")).toBeTruthy();
    expect(screen.getByText("Late")).toBeTruthy();
    expect(screen.getByText("Missing")).toBeTruthy();
  });
  it("lists three raised hands and the overflow count", () => {
    render(CompactLiveView);
    expect(screen.getByText(/Ada, Bea, Cy/)).toBeTruthy();
    expect(screen.getByText("+2")).toBeTruthy();
  });
  it("counts camera-off people in the meeting, not in the waiting room", () => {
    render(CompactLiveView);
    const metric = screen.getByText("Camera off").parentElement!;
    expect(metric.querySelector("b")!.textContent).toBe("2");
  });
  it("shows the latest joins and leaves, newest first", () => {
    render(CompactLiveView);
    const items = Array.from(document.querySelectorAll(".moves li")).map((li) => li.textContent);
    expect(items[0]).toContain("Left");
    expect(items[0]).toContain("Bea");
    expect(items[1]).toContain("Ada");
  });
  it("shows a not-in-meeting state", () => {
    meeting.set(null);
    render(CompactLiveView);
    expect(screen.getByText("Not in a meeting")).toBeTruthy();
  });
  it("expands to the full window from the counts", async () => {
    render(CompactLiveView);
    await fireEvent.click(document.querySelector(".counts")!);
    expect(setWindowMode).toHaveBeenCalledWith("full");
  });
});
