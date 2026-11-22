// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import { meetings, selectedRecords } from "../lib/stores";
import RecordsPage from "./RecordsPage.svelte";

describe("RecordsPage", () => {
  afterEach(() => {
    cleanup();
    meetings.set([]);
    selectedRecords.set([]);
  });

  it("disables export all when there are no meetings", () => {
    meetings.set([]);
    render(RecordsPage);
    expect((screen.getByRole("button", { name: "Export All" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
