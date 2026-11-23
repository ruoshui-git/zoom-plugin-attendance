// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
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

  it("deduplicates manual capture and reports the participant count", async () => {
    const { toast } = await import("../../lib/toast");
    toast.clear();
    let release!: (value: any) => void;
    const capture = vi.fn(() => new Promise((resolve) => { release = resolve; }));
    render(LiveAttendanceView, { view: "manual", state: {}, onCaptureManual: capture });
    let messages: any[] = [];
    const unsubscribe = toast.subscribe((value) => { messages = value; });
    const button = screen.getByRole("button", { name: "Capture Attendance" });
    await fireEvent.click(button);
    await fireEvent.click(button);
    expect(capture).toHaveBeenCalledOnce();
    expect(button.hasAttribute("disabled")).toBe(true);
    release({ timestamp: "2026-01-01T12:34:56.000Z", count: 3 });
    await waitFor(() => expect(messages.some((item) => item.message.includes("3 participants.") && item.message.startsWith("Manual snapshot recorded:"))).toBe(true));
    unsubscribe();
    toast.clear();
  });

  it("disables raise hand capture when no participant has a raised hand", () => {
    render(LiveAttendanceView, { view: "hands", users: [{ name: "Ada", raisedHand: false }] });
    expect(screen.getByRole("button", { name: "Capture Raise Hand Records" }).hasAttribute("disabled")).toBe(true);
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

  it("shows manual snapshot participant id and match status columns", () => {
    const { container } = render(LiveAttendanceView, { view: "manual", state: { manualSnapshots: [{ timestamp: "2026-01-01T12:00:00Z", participants: [
      { participantUUID: "participant-1", screenName: "Ada", matchStatus: "matched" },
    ] }] } });
    expect(container.textContent).toContain("Participant ID");
    expect(container.textContent).toContain("Match Status");
    expect(container.textContent).toContain("participant-1");
  });

  it("counts entry chips, filters on toggle, and Clear Filters restores all", async () => {
    const { container } = render(LiveAttendanceView, { view: "entry", record: { movementEvents: [
      { type: "join", timestamp: "2026-01-01T10:00:00Z", participant: { screenName: "Ada" } },
      { type: "leave", timestamp: "2026-01-01T11:00:00Z", participant: { screenName: "Ada" } },
    ] } });
    const chips = Array.from(container.querySelectorAll<HTMLButtonElement>(".filter-chips button"));
    expect(chips).toHaveLength(7);
    expect(chips.map((chip) => chip.textContent)).toContain("Joined · 1");
    expect(chips.map((chip) => chip.textContent)).toContain("Left · 1");
    await fireEvent.click(chips.find((chip) => chip.textContent?.startsWith("Left"))!);
    expect(container.querySelectorAll("tbody tr")).toHaveLength(1);
    expect(container.textContent).toContain("Joined");
    expect(container.querySelector("tbody")?.textContent).not.toContain("Left");
    await fireEvent.click(screen.getByRole("button", { name: "Clear Filters" }));
    expect(container.querySelectorAll("tbody tr")).toHaveLength(2);
  });

  it("filters name changes by old and new names in the three-column table", async () => {
    const { container } = render(LiveAttendanceView, { view: "names", record: { record: { nameChangeEvents: [
      { timestamp: "2026-01-01T10:00:00Z", previousScreenName: "Old Ada", nextScreenName: "Ada New" },
      { timestamp: "2026-01-01T11:00:00Z", previousScreenName: "Other", nextScreenName: "Someone Else" },
    ] } } });
    expect(Array.from(container.querySelectorAll("thead th")).map((th) => th.textContent)).toEqual(["Time", "Previous Name", "New Name"]);
    await fireEvent.input(screen.getByPlaceholderText("Old/new name or pinyin"), { target: { value: "old ada" } });
    expect(container.querySelectorAll("tbody tr")).toHaveLength(1);
    expect(container.textContent).toContain("Ada New");
    expect(container.textContent).not.toContain("Someone Else");
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
    expect(screen.getByText("Alex · abcd")).toBeTruthy();
    expect(screen.getByText("Alex · efgh")).toBeTruthy();
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

  it("renders late rules using the computed meeting time for positive and negative offsets", () => {
    const props = { view: "info", record: { instanceId: "instance-1", meetingNumber: "123", startedAt: "2026-01-01T10:00:00Z", record: { lateAfterMinutes: 10, returnWindowMinutes: 8, selectedProfileId: "default-all" } }, state: { selectedProfileId: "default-all" }, configs: { profiles: [{ id: "default-all", name: "All", lateAfterMinutes: 10, returnWindowMinutes: 8, active: true }] } };
    const first = render(LiveAttendanceView, props);
    expect(first.container.textContent).toContain("Arriving after 06:10 will count as late");
    first.unmount();
    render(LiveAttendanceView, { ...props, record: { ...props.record, record: { ...props.record.record, lateAfterMinutes: -10 } }, state: { selectedProfileId: "default-all", lateAfterMinutesOverride: -10 } });
    expect(screen.getByText(/10 minutes before the meeting starts/)).toBeTruthy();
    expect(screen.getByText(/05:50/)).toBeTruthy();
  });

  it("applies entered values, rejects invalid values, and restores the default profile", async () => {
    const onOverrides = vi.fn().mockResolvedValue(undefined);
    const onSelectProfile = vi.fn().mockResolvedValue(undefined);
    const profiles = [{ id: "default-all", name: "All", lateAfterMinutes: 0, returnWindowMinutes: 8, active: true }];
    render(LiveAttendanceView, { view: "info", language: "en", record: { startedAt: "2026-01-01T10:00:00Z", record: { lateAfterMinutes: 0, returnWindowMinutes: 8, selectedProfileId: "default-all" } }, state: { selectedProfileId: "default-all" }, configs: { profiles }, onOverrides, onSelectProfile });
    const numbers = screen.getAllByRole("spinbutton");
    await fireEvent.input(numbers[0], { target: { value: "12" } });
    await fireEvent.input(numbers[1], { target: { value: "9" } });
    await fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(onOverrides).toHaveBeenCalledWith({ lateAfterMinutesOverride: 12, returnWindowMinutesOverride: 9 });
    await fireEvent.input(numbers[0], { target: { value: "121" } });
    await fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(screen.getByText("Late override must be between −60 and 120 minutes")).toBeTruthy();
    expect(onOverrides).toHaveBeenCalledOnce();
    await fireEvent.click(screen.getByRole("button", { name: "Restore Default" }));
    expect(onSelectProfile).toHaveBeenCalledWith("default-all");
  });
  it("saves profile-equal values as no override and reports a failed save", async () => {
    const { toast } = await import("../../lib/toast");
    toast.clear();
    const onOverrides = vi.fn().mockResolvedValueOnce(true).mockResolvedValueOnce(false);
    const profiles = [{ id: "default-all", name: "All", lateAfterMinutes: 10, returnWindowMinutes: 8, active: true }];
    render(LiveAttendanceView, { view: "info", language: "en", record: { record: { startedAt: "2026-01-01T10:00:00Z", lateAfterMinutes: 10, returnWindowMinutes: 8, selectedProfileId: "default-all" } }, state: { selectedProfileId: "default-all" }, configs: { profiles }, onOverrides });
    await fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(onOverrides).toHaveBeenLastCalledWith({ lateAfterMinutesOverride: null, returnWindowMinutesOverride: null });
    await fireEvent.input(screen.getAllByRole("spinbutton")[0], { target: { value: "15" } });
    await fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await new Promise((r) => setTimeout(r, 0));
    let items: any[] = []; toast.subscribe((v) => (items = v))();
    expect(items.map((i) => i.type)).toEqual(["success", "error"]);
    toast.clear();
  });
  it("Settings & Info picks up profile values that load after the first render", async () => {
    const view = render(LiveAttendanceView, { view: "info", language: "en", record: { record: { startedAt: "2026-01-01T10:00:00Z" } }, state: {}, configs: { profiles: [] } });
    await view.rerender({ view: "info", language: "en", record: { record: { startedAt: "2026-01-01T10:00:00Z" } }, state: {}, configs: { profiles: [{ id: "default-all", name: "All", lateAfterMinutes: 10, returnWindowMinutes: 8, active: true }] } });
    expect(screen.getByText(/10 minutes after the meeting starts/)).toBeTruthy();
    expect((screen.getAllByRole("spinbutton")[0] as HTMLInputElement).value).toBe("10");
  });
});

