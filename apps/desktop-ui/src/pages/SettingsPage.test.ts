// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { uiScale } from "../lib/stores";
import SettingsPage from "./SettingsPage.svelte";

const { success } = vi.hoisted(() => ({ success: vi.fn() }));
vi.mock("../lib/toast", () => ({ toast: { success, error: vi.fn() } }));

describe("SettingsPage UI size slider", () => {
  beforeEach(() => {
    localStorage.clear();
    uiScale.set("compact");
    success.mockClear();
  });
  afterEach(cleanup);

  it("updates its label on input and applies the scale with one toast on change", async () => {
    render(SettingsPage);
    const slider = screen.getByRole("slider", { name: "Settings panel UI size" });

    await fireEvent.input(slider, { target: { value: "2" } });
    expect(screen.getByText("Large")).toBeTruthy();
    expect(get(uiScale)).toBe("compact");
    expect(success).not.toHaveBeenCalled();

    await fireEvent.change(slider, { target: { value: "2" } });
    expect(get(uiScale)).toBe("large");
    expect(success).toHaveBeenCalledTimes(1);
    expect(success).toHaveBeenCalledWith("Settings panel UI size set to Large.");
  });
});
