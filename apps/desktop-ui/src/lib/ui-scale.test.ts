// @vitest-environment happy-dom
import { beforeEach, describe, expect, it, vi } from "vitest";

describe("ui scale persistence and document zoom", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.style.zoom = "";
    vi.resetModules();
  });

  it("persists and applies every scale step", async () => {
    const { UI_SCALE_STEPS, UI_SCALE_ZOOM, uiScale } = await import("./stores");
    for (const scale of UI_SCALE_STEPS) {
      uiScale.set(scale);
      expect(localStorage.getItem("attendflow-desktop.ui-scale")).toBe(scale);
      expect(document.documentElement.style.zoom).toBe(String(UI_SCALE_ZOOM[scale]));
    }
  });

  it("migrates an existing ui-size setting", async () => {
    localStorage.setItem(["ui", "size"].join("-"), "large");
    const { get } = await import("svelte/store");
    const { uiScale } = await import("./stores");
    expect(get(uiScale)).toBe("large");
    expect(localStorage.getItem("attendflow-desktop.ui-scale")).toBe("large");
    expect(document.documentElement.style.zoom).toBe("1.45");
  });
});
