// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { configs, language, meeting, record, recordEvents, settings, stateValue } from "../../lib/stores";
import RecordDetail from "./RecordDetail.svelte";

describe("RecordDetail attendance filters", () => {
  afterEach(() => {
    cleanup();
    record.set({ rows: [], summary: {} });
    configs.set({});
    settings.set({});
    language.set("en");
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
});
