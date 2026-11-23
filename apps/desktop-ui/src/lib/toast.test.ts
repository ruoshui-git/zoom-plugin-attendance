import { afterEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { toast } from "./toast";

describe("toast store", () => {
  afterEach(() => { toast.clear(); vi.useRealTimers(); });
  it("auto dismisses after its duration", () => {
    vi.useFakeTimers(); toast.success("Saved", 100); const id = get(toast)[0].id;
    vi.advanceTimersByTime(110); expect(get(toast)).toHaveLength(0); expect(id).toBeTruthy();
  });
  it("pauses and resumes from the remaining time", () => {
    vi.useFakeTimers(); const id = toast.info("Working", 200); vi.advanceTimersByTime(60); toast.pause(id); const left = get(toast)[0].remainingTime;
    vi.advanceTimersByTime(300); expect(get(toast)[0].remainingTime).toBe(left); toast.resume(id); vi.advanceTimersByTime(left + 10); expect(get(toast)).toHaveLength(0);
  });
  it("dismisses one toast and stacks several", () => {
    const first = toast.info("First", 0); toast.warning("Second", 0); toast.error("Third", 0);
    expect(get(toast)).toHaveLength(3); toast.dismiss(first); expect(get(toast).map((item) => item.message)).toEqual(["Second", "Third"]);
  });
  it("reports progress decreasing toward zero", () => {
    vi.useFakeTimers(); const id = toast.info("Progress", 300); vi.advanceTimersByTime(100);
    expect(toast.getProgress(id)).toBeLessThan(100); vi.advanceTimersByTime(150); expect(toast.getProgress(id)).toBeGreaterThan(0);
  });
});
