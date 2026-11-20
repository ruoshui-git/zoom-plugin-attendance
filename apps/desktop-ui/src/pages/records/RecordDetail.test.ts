// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { configs, language, meeting, record, recordEvents, settings, stateValue } from "../../lib/stores";
import RecordDetail from "./RecordDetail.svelte";
import { uiLanguage } from "../../lib/i18n-core";

describe("RecordDetail attendance filters", () => {
  afterEach(() => {
    cleanup();
    record.set({ rows: [], summary: {} });
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
});
