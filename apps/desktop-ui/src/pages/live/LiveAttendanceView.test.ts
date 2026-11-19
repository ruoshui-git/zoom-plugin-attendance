// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import LiveAttendanceView from "./LiveAttendanceView.svelte";

describe("LiveAttendanceView", () => {
  afterEach(cleanup);

  it("filters status rows with location chips and search", async () => {
    render(LiveAttendanceView, { view: "status", record: { rows: [
      { canonicalName: "Ada Lovelace", currentLocation: "meeting", currentStatus: "fullPresent", hasBeenSeen: true },
      { canonicalName: "Grace Hopper", currentLocation: "waiting-room", currentStatus: "late", wasLate: true },
    ] } });
    expect(screen.getByText("Ada Lovelace")).toBeTruthy();
    expect(screen.getByText("Grace Hopper")).toBeTruthy();
    await fireEvent.click(screen.getByRole("button", { name: "Waiting room" }));
    expect(screen.getByText("Grace Hopper")).toBeTruthy();
    expect(screen.queryByText("Ada Lovelace")).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: "Waiting room" }));
    await fireEvent.input(screen.getByRole("textbox", { name: "Search names" }), { target: { value: "Ada" } });
    expect(screen.getByText("Ada Lovelace")).toBeTruthy();
    expect(screen.queryByText("Grace Hopper")).toBeNull();
  });

  it("keeps one compact status summary and filters from its counts", async () => {
    const { container } = render(LiveAttendanceView, { view: "status", mode: "roster", record: { rows: [
      { canonicalName: "Ada Lovelace", currentStatus: "present", hasBeenSeen: true, isCurrentlyPresent: true, wasLate: false },
      { canonicalName: "Grace Hopper", currentStatus: "late", hasBeenSeen: true, isCurrentlyPresent: true, wasLate: true },
    ] } });
    expect(container.querySelectorAll(".summary-row")).toHaveLength(1);
    expect(container.querySelectorAll(".summary-line")).toHaveLength(0);
    await fireEvent.click(screen.getByRole("button", { name: /1 FP/ }));
    expect(screen.getByText("Ada Lovelace")).toBeTruthy();
    expect(screen.queryByText("Grace Hopper")).toBeNull();
  });

  it("wires manual attendance capture to the existing action", async () => {
    const capture = vi.fn();
    render(LiveAttendanceView, { view: "manual", state: { manualSnapshots: [] }, onCaptureManual: capture });
    await fireEvent.click(screen.getByRole("button", { name: "Capture Attendance" }));
    expect(capture).toHaveBeenCalledOnce();
  });
});
