// @vitest-environment happy-dom
import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import SidebarNav from "./SidebarNav.svelte";
import { mode, page } from "./stores";

describe("sidebar navigation", () => {
  afterEach(() => cleanup());

  it("has no top-level matching page and shows the roster badge on Live", () => {
    mode.set("roster");
    render(SidebarNav, { matchingCount: 3 });
    expect(screen.queryByRole("button", { name: /Name Matching/ })).toBeNull();
    expect(screen.getByRole("button", { name: /Live.*3/ })).toBeTruthy();
    expect(screen.getAllByRole("button")).toHaveLength(4);
    mode.set("generic");
    page.set("live");
  });
});
