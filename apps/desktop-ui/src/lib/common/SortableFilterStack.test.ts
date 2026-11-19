// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import SortableFilterStack from "./SortableFilterStack.svelte";

const items = [
  { id: "seen", label: "Seen", count: 4 },
  { id: "late", label: "Late", count: 2 },
  { id: "absent", label: "Absent", count: 1 },
];

describe("SortableFilterStack", () => {
  afterEach(() => { cleanup(); localStorage.clear(); });

  it("reorders tabs by dragging and persists their order", async () => {
    render(SortableFilterStack, { items, storageKey: "test-order" });
    const buttons = screen.getAllByRole("button");
    await fireEvent.dragStart(buttons[0]);
    await fireEvent.drop(buttons[2]);
    expect(screen.getAllByRole("button").map((button) => button.textContent?.replace(/⠿/g, "").trim())).toEqual(["2 Late", "1 Absent", "4 Seen"]);
    expect(JSON.parse(localStorage.getItem("test-order") || "[]")).toEqual(["late", "absent", "seen"]);
  });

  it("restores persisted ordering and sends selected filter id", async () => {
    localStorage.setItem("test-order", JSON.stringify(["absent", "seen", "late"]));
    let selected = "";
    render(SortableFilterStack, { items, storageKey: "test-order", onSelect: (id) => selected = id });
    expect(screen.getAllByRole("button")[0].textContent).toContain("Absent");
    await fireEvent.click(screen.getByRole("button", { name: /Late/ }));
    expect(selected).toBe("late");
  });
});
