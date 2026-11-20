// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import SortableFilterStack from "./SortableFilterStack.svelte";

const items = [
  { id: "seen", label: "Seen", count: 4 },
  { id: "late", label: "Late", count: 2 },
  { id: "absent", label: "Absent", count: 1 },
];

describe("SortableFilterStack status presets", () => {
  afterEach(cleanup);

  it("shows items in supplied order without HTML drag and drop", () => {
    const { container } = render(SortableFilterStack, { items });
    expect(screen.getAllByRole("button").map((button) => button.textContent?.trim())).toEqual(["Seen4", "Late2", "Absent1"]);
    expect(container.querySelector("[draggable]")).toBeNull();
  });

  it("sends the selected preset id", async () => {
    let selected = "";
    render(SortableFilterStack, { items, onSelect: (id) => selected = id });
    await fireEvent.click(screen.getByRole("button", { name: /Late/ }));
    expect(selected).toBe("late");
  });
});
