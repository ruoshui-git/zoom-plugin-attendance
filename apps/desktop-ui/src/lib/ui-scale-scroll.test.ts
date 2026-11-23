import { describe, expect, it } from "vitest";
import { getScrollDelta } from "./ui-scale-scroll";

describe("UI scale scroll restoration", () => {
  it("returns the scroll adjustment needed to restore the slider's viewport position", () => {
    expect(getScrollDelta(240, 315)).toBe(75);
    expect(getScrollDelta(315, 240)).toBe(-75);
  });
});
