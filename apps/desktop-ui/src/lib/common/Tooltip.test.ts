// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import AttendanceFilterPanel from "./AttendanceFilterPanel.svelte";

describe("Tooltip", () => {
  afterEach(() => { cleanup(); vi.useRealTimers(); });

  it("shows on hover and focus after the delay, hides on leave, blur and Escape, and is linked by aria-describedby", async () => {
    vi.useFakeTimers();
    render(AttendanceFilterPanel, { rows: [], language: "en", initiallyOpen: true });
    const reset = screen.getByRole("button", { name: "Reset All" });
    const tip = document.getElementById(reset.getAttribute("aria-describedby")!)!;
    expect(tip.getAttribute("role")).toBe("tooltip");
    expect(tip.textContent).toBe("Reset All");
    expect(tip.classList.contains("visible")).toBe(false);
    await fireEvent.mouseEnter(reset.parentElement!);
    await vi.advanceTimersByTimeAsync(400);
    expect(tip.classList.contains("visible")).toBe(true);
    await fireEvent.mouseLeave(reset.parentElement!);
    expect(tip.classList.contains("visible")).toBe(false);
    await fireEvent.focusIn(reset);
    await vi.advanceTimersByTimeAsync(400);
    expect(tip.classList.contains("visible")).toBe(true);
    await fireEvent.keyDown(reset, { key: "Escape" });
    expect(tip.classList.contains("visible")).toBe(false);
    await fireEvent.focusIn(reset);
    await vi.advanceTimersByTimeAsync(400);
    await fireEvent.focusOut(reset);
    expect(tip.classList.contains("visible")).toBe(false);
  });
});
