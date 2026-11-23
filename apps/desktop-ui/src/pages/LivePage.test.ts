// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import { meeting } from "../lib/stores";
import LivePage from "./LivePage.svelte";

describe("LivePage new record action", () => {
  afterEach(() => { cleanup(); meeting.set(null); });

  it("does not offer starting a new record without an active meeting", () => {
    meeting.set(null);
    render(LivePage);
    expect(screen.queryByRole("button", { name: "Start new record" })).toBeNull();
  });
});
