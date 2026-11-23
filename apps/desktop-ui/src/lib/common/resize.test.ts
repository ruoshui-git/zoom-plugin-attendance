// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import LiveAttendanceView from "../../pages/live/LiveAttendanceView.svelte";
import SlideOverPanel from "./SlideOverPanel.svelte";

const rows = [{ zoomName: "A very long Zoom display name that will not fit", currentStatus: "present", hasBeenSeen: true, isCurrentlyPresent: true, tags: ["seen", "full_present"] }];

describe("resizing and cell tooltips", () => {
  afterEach(() => { cleanup(); vi.useRealTimers(); });

  it("drags and keyboard-resizes a status column; widths reset when the mode changes columns", async () => {
    const view = render(LiveAttendanceView, { view: "status", language: "en", mode: "generic", record: { rows } });
    const handles = screen.getAllByRole("separator", { name: "Resize column" });
    const nameTh = handles[1].closest("th")!;
    nameTh.getBoundingClientRect = () => ({ width: 120 } as DOMRect);
    await fireEvent.mouseDown(handles[1], { clientX: 100 });
    await fireEvent.mouseMove(document, { clientX: 160 });
    await fireEvent.mouseUp(document, { clientX: 160 });
    expect(nameTh.style.width).toBe("180px");
    await fireEvent.keyDown(handles[1], { key: "ArrowRight" });
    expect(nameTh.style.width).toBe("196px");
    await fireEvent.keyDown(handles[1], { key: "ArrowLeft" });
    expect(nameTh.style.width).toBe("180px");
    await view.rerender({ view: "status", language: "en", mode: "roster", configs: { roster: [{ personId: 1, canonicalName: "Ada" }] }, record: { rows } });
    expect(screen.getAllByRole("separator", { name: "Resize column" })[1].closest("th")!.style.width).toBe("");
  });

  it("shows a cell's full text only when the cell is truncated", async () => {
    vi.useFakeTimers();
    render(LiveAttendanceView, { view: "status", language: "en", mode: "generic", record: { rows } });
    const cells = document.querySelectorAll<HTMLElement>(".attendance-table tbody td");
    const nameCell = cells[1], indexCell = cells[0];
    Object.defineProperty(nameCell, "scrollWidth", { value: 400 }); Object.defineProperty(nameCell, "clientWidth", { value: 100 });
    Object.defineProperty(indexCell, "scrollWidth", { value: 20 }); Object.defineProperty(indexCell, "clientWidth", { value: 20 });
    await fireEvent.mouseEnter(indexCell);
    await vi.advanceTimersByTimeAsync(600);
    expect(document.querySelector(".table-tooltip")).toBeNull();
    await fireEvent.mouseEnter(nameCell);
    await vi.advanceTimersByTimeAsync(600);
    expect(document.querySelector(".table-tooltip")?.textContent).toContain("A very long Zoom display name");
  });

  it("resizes the slide-over with the keyboard, clamped, and its close button has a tooltip", async () => {
    render(SlideOverPanel, { open: true, title: "Detail", closeLabel: "Close", width: 500, minWidth: 450, maxWidth: 600 });
    const panel = document.querySelector<HTMLElement>(".panel")!;
    const handle = screen.getByRole("separator", { name: "Resize panel" });
    const width = () => Number(panel.style.getPropertyValue("--panel-width").replace("px", ""));
    expect(width()).toBe(500);
    await fireEvent.keyDown(handle, { key: "ArrowRight" });
    expect(width()).toBe(510);
    await fireEvent.keyDown(handle, { key: "ArrowLeft", shiftKey: true });
    expect(width()).toBe(460);
    for (let i = 0; i < 20; i++) await fireEvent.keyDown(handle, { key: "ArrowRight", shiftKey: true });
    expect(width()).toBe(600);
    for (let i = 0; i < 20; i++) await fireEvent.keyDown(handle, { key: "ArrowLeft", shiftKey: true });
    expect(width()).toBe(450);
    const close = screen.getAllByRole("button", { name: "Close" }).find((b) => b.getAttribute("aria-describedby"))!;
    expect(document.getElementById(close.getAttribute("aria-describedby")!)?.textContent).toBe("Close");
  });
});
