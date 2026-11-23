// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { language, settings } from "../../lib/stores";
import { uiLanguage } from "../../lib/i18n-core";
import LiveAttendanceView from "./LiveAttendanceView.svelte";

describe("LiveAttendanceView", () => {
  afterEach(() => { cleanup(); language.set("en"); uiLanguage.set("en"); });

  it("uses status summary items as click presets and searches names", async () => {
    const { container } = render(LiveAttendanceView, { view: "status", record: { rows: [
      { canonicalName: "Ada Lovelace", currentLocation: "meeting", currentStatus: "fullPresent", hasBeenSeen: true },
      { canonicalName: "Grace Hopper", currentLocation: "waiting-room", currentStatus: "late", wasLate: true },
    ] } });
    expect(screen.getByText("Ada Lovelace")).toBeTruthy();
    expect(screen.getByText("Grace Hopper")).toBeTruthy();
    await fireEvent.click(container.querySelectorAll(".filter-stack button")[1]);
    expect(screen.getByText("Grace Hopper")).toBeTruthy();
    expect(screen.queryByText("Ada Lovelace")).toBeNull();
    await fireEvent.click(container.querySelectorAll(".filter-stack button").item(container.querySelectorAll(".filter-stack button").length - 1));
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
    await fireEvent.click(container.querySelectorAll(".filter-stack button")[1]);
    expect(screen.getByText("Ada Lovelace")).toBeTruthy();
    expect(screen.queryByText("Grace Hopper")).toBeNull();
  });

  it("starts with the filters collapsed when the setting is enabled", () => {
    settings.set({ filterDefaultsClosed: true });
    const { container } = render(LiveAttendanceView, { view: "status", record: { rows: [] } });
    expect(container.querySelector(".filter-controls")).toBeNull();
    settings.set({ filterDefaultsClosed: false });
  });

  it("wires manual attendance capture to the existing action", async () => {
    const capture = vi.fn();
    render(LiveAttendanceView, { view: "manual", state: { manualSnapshots: [] }, onCaptureManual: capture });
    await fireEvent.click(screen.getByRole("button", { name: "Capture Attendance" }));
    expect(capture).toHaveBeenCalledOnce();
  });

  it("localizes manual attendance values and hides unknown raw values in Chinese", () => {
    language.set("zh-CN");
    uiLanguage.set("zh-CN");
    const { container } = render(LiveAttendanceView, { view: "manual", language: "zh-CN", state: { manualSnapshots: [{ timestamp: "2026-01-01T12:00:00.000Z", participants: [
      { screenName: "测试参与者", matchStatus: "not-applicable", location: "unknown-x" },
    ] }] } });
    expect(container.textContent).toContain("不适用");
    expect(container.textContent).toContain("未知");
    expect(container.textContent).not.toContain("not-applicable");
    expect(container.textContent).not.toContain("unknown-x");
  });

  it("shows full status names and toggles a preset off on a second click", async () => {
    const { container } = render(LiveAttendanceView, { view: "status", language: "en", record: { rows: [
      { canonicalName: "Late Member", currentStatus: "late", wasLate: true, tags: ["late"] },
      { canonicalName: "On Time Member", currentStatus: "present", tags: ["seen"] },
    ] } });
    const strip = container.querySelector(".filter-stack")!;
    expect(strip.textContent).toContain("Late");
    expect(strip.textContent).not.toMatch(/\bLt\b/);
    const late = Array.from(strip.querySelectorAll<HTMLButtonElement>("button")).find((b) => b.textContent?.includes("Late"))!;
    await fireEvent.click(late);
    expect(screen.getByText("Late Member")).toBeTruthy();
    expect(screen.queryByText("On Time Member")).toBeNull();
    await fireEvent.click(late);
    expect(screen.getByText("On Time Member")).toBeTruthy();
  });

  it("hides group in generic mode and localizes it in roster mode", async () => {
    const { container, rerender } = render(LiveAttendanceView, { view: "status", mode: "generic", language: "en", record: { rows: [
      { canonicalName: "A very long participant name that must be allowed to wrap", group: "__UNGROUPED__", currentStatus: "present" },
    ] } });
    expect(container.querySelector(".group-cell")).toBeNull();
    await rerender({ view: "status", mode: "roster", configs: { roster: [{}] }, language: "en", record: { rows: [{ canonicalName: "A very long participant name that must be allowed to wrap", group: "__UNGROUPED__", currentStatus: "present" }] } });
    const group = container.querySelector(".group-cell")!;
    expect(group.textContent).toBe("Ungrouped");
    expect(group.getAttribute("title")).toBe("Ungrouped");
    expect(container.querySelector(".participant-name")!.className).toContain("participant-name");
  });

  it("shows live duplicate warnings and disambiguated generic names with status table columns", () => {
    const { container } = render(LiveAttendanceView, { view: "status", language: "en", record: { rows: [
      { zoomName: "Alex", participantUUID: "participant-1234-abcd", isCurrentlyPresent: true, currentStatus: "late", firstJoinAt: "2026-01-01T12:34:56", lastSeenAt: "2026-01-01T13:02:03", totalPresenceMinutes: 12.7 },
      { zoomName: "Alex", participantUUID: "participant-5678-efgh", isCurrentlyPresent: true, currentStatus: "present" },
    ] } });
    expect(screen.getByText(/Duplicate names:/)).toBeTruthy();
    expect(container.querySelector("thead")?.textContent).toContain("Zoom Name");
    expect(screen.getByText("Alex #ABCD")).toBeTruthy();
    expect(screen.getByText("Alex #EFGH")).toBeTruthy();
    expect(container.querySelector(".group-cell")).toBeNull();
    expect(container.querySelector("tbody tr.status-late")?.className).toContain("status-late");
    expect(container.querySelector("tbody tr.status-late")?.textContent).toContain("12:34:56");
    expect(container.querySelector("tbody tr.status-late")?.textContent).toContain("13:02:03");
  });

  it("marks unexpected roster participants and shows roster group column", () => {
    const { container } = render(LiveAttendanceView, { view: "status", mode: "roster", configs: { roster: [{}] }, language: "en", record: { rows: [
      { canonicalName: "Visitor", zoomName: "V", isExpected: false, group: "__UNGROUPED__", currentStatus: "leftEarly" },
    ] } });
    expect(screen.getByLabelText("Unexpected roster participant")).toBeTruthy();
    expect(container.querySelector("thead")?.textContent).toContain("Group");
    expect(container.querySelector("tbody tr")?.className).toContain("status-leftEarly");
  });

  it("shows Full Present in generic mode, hides zero counts and uses engine tags for badges", () => {
    const { container } = render(LiveAttendanceView, { view: "status", mode: "generic", record: {
      summary: { seen: 2, fullPresent: 1, late: 1, returned: 0, leftEarly: 1, missing: 0, ignored: 0, total: 2 },
      rows: [
        { canonicalName: "Ada Lovelace", currentStatus: "present", hasBeenSeen: true, isCurrentlyPresent: true, tags: ["seen", "full_present"] },
        { canonicalName: "Grace Hopper", currentStatus: "leftEarly", hasBeenSeen: true, wasLate: true, tags: ["seen", "late", "left_early"] },
      ] } });
    const stats = Array.from(container.querySelectorAll(".filter-stack button small")).map((n) => n.textContent);
    expect(stats).toEqual(["Seen", "Full Present", "Late", "Left Early", "Total"]);
    const badgeRows = Array.from(container.querySelectorAll("tbody tr")).map((tr) => Array.from(tr.querySelectorAll(".badges > *")).map((b) => b.textContent?.trim()));
    expect(badgeRows[0]).toEqual(["Full Present"]);
    expect(badgeRows[1]).toEqual(["Left Early", "Late", "Seen"]);
    expect(container.querySelector("thead button")).toBeNull();
  });
});
