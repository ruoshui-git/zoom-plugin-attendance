// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import PrimaryTabs from "./PrimaryTabs.svelte";
import SecondaryTabs from "./SecondaryTabs.svelte";

describe("meeting tab primitives", () => {
  afterEach(cleanup);
  it("selects a primary meeting view", async () => {
    const select = vi.fn();
    render(PrimaryTabs, { items: [{ id: "attendance", label: "Auto Attendance" }, { id: "hands", label: "Raise Hand" }], active: "attendance", onSelect: select });
    await fireEvent.click(screen.getByRole("button", { name: "Raise Hand" }));
    expect(select).toHaveBeenCalledWith("hands");
    expect(screen.getByRole("button", { name: "Auto Attendance" }).getAttribute("aria-current")).toBe("page");
  });
  it("selects an attendance secondary view", async () => {
    const select = vi.fn();
    render(SecondaryTabs, { items: [{ id: "status", label: "Status" }, { id: "matching", label: "Name Matching" }], active: "status", onSelect: select });
    await fireEvent.click(screen.getByRole("button", { name: "Name Matching" }));
    expect(select).toHaveBeenCalledWith("matching");
    expect(screen.getByRole("button", { name: "Status" }).getAttribute("aria-pressed")).toBe("true");
  });
});
