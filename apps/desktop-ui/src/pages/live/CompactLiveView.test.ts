// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { meeting, record, stateValue, recordEvents } from "../../lib/stores";
import { uiLanguage } from "../../lib/i18n-core";

const { setWindowMode } = vi.hoisted(() => ({ setWindowMode: vi.fn() }));
vi.mock("../../lib/window-control", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../lib/window-control")>()), setWindowMode }));
import CompactLiveView from "./CompactLiveView.svelte";

describe("CompactLiveView", () => {
  beforeEach(() => {
    vi.clearAllMocks(); uiLanguage.set("en");
    meeting.set({ topic: "Weekly sync" });
    record.set({ rows: [
      { isCurrentlyPresent: true, videoOn: false },
      { isCurrentlyPresent: true, videoOn: true },
      { isCurrentlyPresent: false, currentLocation: "waiting-room", videoOn: false },
    ], summary: { expected: 8, late: 2, missing: 1 } });
    stateValue.set({ mode: "roster", raiseHandEntries: ["Ada", "Bea", "Cy", "Dee", "Eli"].map((name) => ({ name, isRaised: true })) });
    recordEvents.set([]);
  });
  afterEach(cleanup);

  it("renders present and expected counts in roster mode", () => {
    render(CompactLiveView);
    expect(screen.getByText("2 / 8")).toBeTruthy();
    expect(screen.getByText(/Late 2 · Missing 1/)).toBeTruthy();
  });
  it("omits the expected count outside roster mode", () => {
    stateValue.set({ mode: "generic", raiseHandEntries: [] });
    render(CompactLiveView);
    expect(screen.getByText("2")).toBeTruthy();
    expect(screen.queryByText(/Late/)).toBeNull();
  });
  it("shows three hands and the overflow count for five raised users", () => {
    render(CompactLiveView);
    expect(screen.getByText(/Ada, Bea, Cy \+2/)).toBeTruthy();
  });
  it("counts camera-off present users and excludes waiting-room users", () => {
    render(CompactLiveView);
    expect(screen.getByText(/Camera off: 1/)).toBeTruthy();
  });
  it("renders the no-meeting state", () => {
    meeting.set(null);
    render(CompactLiveView);
    expect(screen.getByText("Not in a meeting")).toBeTruthy();
  });
  it("expands to full mode", async () => {
    render(CompactLiveView);
    await fireEvent.click(screen.getAllByRole("button", { name: "Expand" })[0]);
    expect(setWindowMode).toHaveBeenCalledWith("full");
  });
});
