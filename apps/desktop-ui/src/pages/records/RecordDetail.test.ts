// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { configs, language, viewedMeeting as meeting, viewedRecord as record, viewedEvents as recordEvents, settings, viewedState as stateValue } from "../../lib/stores";
import RecordDetail from "./RecordDetail.svelte";
import { uiLanguage } from "../../lib/i18n-core";

describe("RecordDetail attendance filters", () => {
  afterEach(() => {
    cleanup();
    record.set({ rows: [], summary: {}, loading: false });
    configs.set({});
    settings.set({});
    language.set("en");
    uiLanguage.set("en");
    meeting.set(null);
    stateValue.set({ manualSnapshots: [], reconciliationDecisions: [], raiseHandCaptures: [] });
    recordEvents.set([]);
  });

  it("uses the reusable panel for group filtering and toggled status presets", async () => {
    record.set({ mode: "roster", rows: [
      { canonicalName: "Ada", group: "A", isExpected: true, currentStatus: "present", hasBeenSeen: true, tags: ["seen"] },
      { canonicalName: "Grace", group: "B", isExpected: true, currentStatus: "late", wasLate: true, hasBeenSeen: true, tags: ["late", "seen"] },
    ], summary: { total: 2 } });
    configs.set({ roster: [{ canonicalName: "Ada", group: "A" }, { canonicalName: "Grace", group: "B" }], profiles: [{ id: "selected", groupSortOrder: ["B", "A"] }] });
    record.update((value) => ({ ...value, record: { selectedProfileId: "selected" } }));
    settings.set({ filterDefaultsClosed: false });

    const { container } = render(RecordDetail);
    expect(container.querySelector(".filter-panel")).toBeTruthy();
    expect(screen.getByText("Ada")).toBeTruthy();
    expect(screen.getByText("Grace")).toBeTruthy();

    await fireEvent.click(Array.from(container.querySelectorAll(".filter-chip")).find((chip) => chip.textContent === "A")!);
    expect(screen.queryByText("Ada")).toBeNull();
    expect(screen.getByText("Grace")).toBeTruthy();
    await fireEvent.click(Array.from(container.querySelectorAll(".filter-chip")).find((chip) => chip.textContent === "A")!);

    const late = Array.from(container.querySelectorAll(".filter-stack button")).find((button) => button.textContent?.includes("Late"))!;
    await fireEvent.click(late);
    expect(screen.getByText("Grace")).toBeTruthy();
    expect(screen.queryByText("Ada")).toBeNull();
    await fireEvent.click(late);
    expect(screen.getByText("Ada")).toBeTruthy();
  });

  it('renders reaction feedback, Yes check-in, counts, and searchable log', async () => {
    language.set('zh-CN');
    uiLanguage.set('zh-CN');
    meeting.set({ startedAt: '2026-01-01T10:00:00' } as any);
    recordEvents.set([
      { kind: 'users', at: 1, users: [{ userId: 7, name: 'Ada' }] },
      { kind: 'feedback', seq: 2, at: new Date(2026, 0, 1, 10, 1).getTime(), userId: 7, type: 2 },
      { kind: 'reaction', seq: 3, at: new Date(2026, 0, 1, 10, 2).getTime(), userId: 7, type: 2 },
    ] as any);
    const { container } = render(RecordDetail);
    await fireEvent.click(screen.getByRole('tab', { name: /反应与反馈/ }));
    expect(container.textContent).toContain('应用连接前设置的反馈不可见');
    expect(container.textContent).toContain('Ada');
    expect(container.textContent).toContain('点赞');
    expect(container.textContent).toContain('1');
    expect(screen.getByLabelText('开始时间')).toBeTruthy();
  });

  it("localizes manual attendance values and hides unknown raw values in Chinese", async () => {
    language.set("zh-CN");
    uiLanguage.set("zh-CN");
    stateValue.set({ manualSnapshots: [{ timestamp: "2026-01-01T12:00:00.000Z", participants: [
      { screenName: "测试参与者", matchStatus: "not-applicable", location: "unknown-x" },
    ] }] });
    const { container } = render(RecordDetail);
    await fireEvent.click(screen.getByRole("tab", { name: /手动考勤记录/ }));
    expect(container.textContent).toContain("不适用");
    expect(container.textContent).toContain("未知");
    expect(container.textContent).not.toContain("not-applicable");
    expect(container.textContent).not.toContain("unknown-x");
  });

  it("shows overview metadata and operator action counts with profile names", async () => {
    meeting.set({ meetingNumber: "123456", startedAt: "2026-01-01T10:00:00Z", endedAt: "2026-01-01T11:30:00Z", source: "imported" } as any);
    configs.set({ profiles: [{ id: "p1", name: "Staff Profile" }] });
    record.set({ rows: [
      { matchStatus: "unmatched" }, { matchStatus: "guest" }, { matchStatus: "matched" },
    ], durationMinutes: 89.7, record: { selectedProfileId: "p1", revision: 4 } });
    stateValue.set({ manualSnapshots: [{}, {}], reconciliationDecisions: [{}, {}, {}] });
    recordEvents.set([{ type: "join" }, { type: "leave" }] as any);
    const { container } = render(RecordDetail);
    await fireEvent.click(screen.getByRole("tab", { name: /Overview/ }));
    expect(container.textContent).toContain("Staff Profile");
    expect(container.textContent).toContain("01:29");
    expect(container.textContent).not.toContain("1:30");
    expect(container.textContent).toContain("Pending Export");
    expect(container.textContent).toContain("Manual Attendance Records");
    expect(container.textContent).toContain("Matching Decisions");
    expect(container.textContent).toContain("Entry/Exit");
    expect(container.textContent).toContain("Unmatched");
    expect(container.textContent).toContain("Guests");
    expect(container.textContent).toMatch(/Manual Attendance Records\s*2/);
    expect(container.textContent).toMatch(/Matching Decisions\s*3/);
    expect(container.textContent).toMatch(/Unmatched\s*1/);
    expect(container.textContent).toMatch(/Guests\s*1/);
    await fireEvent.click(screen.getByRole("tab", { name: /Auto Attendance/ }));
    await fireEvent.click(screen.getByRole("tab", { name: /Attendance Profile/ }));
    expect(container.textContent).toContain("Staff Profile");
  });

  it("shows Proof captures and opens a participant viewer that closes with Escape", async () => {
    meeting.set({ instanceId: "proof-meeting", startedAt: "2026-01-01T10:00:00Z" } as any);
    stateValue.set({ proofCaptures: [{ id: "c1", startedAt: "2026-01-01T10:00:00Z", pages: [{ file: "page-1.png", at: "2026-01-01T10:01:00Z", cameraOnCount: 1, darkTiles: 2, participants: [{ name: "Ada", videoOn: true }] }] }] });
    const { container } = render(RecordDetail);
    await fireEvent.click(screen.getByRole("tab", { name: /Proof/ }));
    expect(container.textContent).toContain("Experimental");
    await waitFor(() => expect(container.querySelector(".card")).toBeTruthy());
    await fireEvent.click(container.querySelector(".card")!);
    expect(screen.getByRole("dialog").textContent).toContain("Ada");
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("shows exported status when the record has an export timestamp", async () => {
    record.set({ rows: [], record: { exportedAt: "2026-01-02T12:00:00Z" } });
    const { container } = render(RecordDetail);
    await fireEvent.click(screen.getByRole("tab", { name: /Overview/ }));
    expect(container.textContent).toContain("Exported");
  });

  it("sorts attendance by minutes and restores filter-panel order on the third click", async () => {
    record.set({ mode: "generic", rows: [
      { canonicalName: "Long", totalPresenceMinutes: 30, currentStatus: "present", matchStatus: "matched" },
      { canonicalName: "Short", totalPresenceMinutes: 10, currentStatus: "present", matchStatus: "matched" },
      { canonicalName: "Medium", totalPresenceMinutes: 20, currentStatus: "present", matchStatus: "matched" },
    ], summary: { total: 3 } });
    const { container } = render(RecordDetail);
    const order = () => Array.from(container.querySelectorAll(".table-wrap tbody tr td:nth-child(2) b"), (node) => node.textContent);
    const originalOrder = order();
    const minutesHeader = screen.getByRole("button", { name: /Minutes/ });
    await fireEvent.click(minutesHeader);
    expect(order()).toEqual(["Short", "Medium", "Long"]);
    await fireEvent.click(minutesHeader);
    expect(order()).toEqual(["Long", "Medium", "Short"]);
    await fireEvent.click(minutesHeader);
    expect(order()).toEqual(originalOrder);
  });

  it("shows loading text instead of stale record rows", () => {
    language.set("en");
    record.set({ rows: [{ canonicalName: "Old row" }], summary: { total: 1 }, loading: true });
    const { container } = render(RecordDetail);
    expect(container.textContent).toContain("Loading…");
    expect(container.textContent).not.toContain("Old row");
  });

  it("localizes the ungrouped attendance value", () => {
    language.set("zh-CN");
    uiLanguage.set("zh-CN");
    record.set({ mode: "generic", rows: [{ canonicalName: "Ada", group: "__UNGROUPED__", currentStatus: "present", matchStatus: "matched" }], summary: { total: 1 }, loading: false });
    const { container } = render(RecordDetail);
    expect(container.querySelector(".group-cell")?.textContent).toBe("未分组");
  });
});
