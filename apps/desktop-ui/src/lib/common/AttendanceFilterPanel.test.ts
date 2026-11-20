// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import AttendanceFilterPanel from "./AttendanceFilterPanel.svelte";

describe("AttendanceFilterPanel", () => {
  afterEach(cleanup);

  it("renders roster sections, toggles group chips, and emits sorted filtered rows", async () => {
    const onRows = vi.fn();
    const rows = [
      { canonicalName: "Zoe", group: "B", isExpected: true, currentStatus: "present", pinyin: "zoe" },
      { canonicalName: "Ada", group: "A", isExpected: true, currentStatus: "present", pinyin: "ada" },
      { canonicalName: "Visitor", group: "A", isExpected: false, currentStatus: "present", pinyin: "visitor" },
    ];
    const { container } = render(AttendanceFilterPanel, {
      rows,
      roster: [{ group: "A" }, { group: "B" }],
      mode: "roster",
      initiallyOpen: true,
      onRows,
    });
    expect(container.querySelector('[data-section="expectation"]')).toBeTruthy();
    expect(container.querySelector('[data-section="group"]')).toBeTruthy();
    expect(onRows.mock.lastCall?.[0].map((row: any) => row.canonicalName)).toEqual(["Ada", "Zoe", "Visitor"]);
    await fireEvent.click(Array.from(container.querySelectorAll(".filter-chip")).find((chip) => chip.textContent === "B")!);
    expect(onRows.mock.lastCall?.[0].map((row: any) => row.canonicalName)).toEqual(["Ada", "Visitor"]);
  });
});
