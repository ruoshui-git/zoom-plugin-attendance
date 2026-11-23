// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { meetings, selectedRecords } from "../lib/stores";
import RecordsPage from "./RecordsPage.svelte";

describe("RecordsPage", () => {
  afterEach(() => {
    cleanup();
    meetings.set([]);
    selectedRecords.set([]);
  });

  it("removes dropdown filters and disables export when there are no meetings", async () => {
    meetings.set([]);
    render(RecordsPage);
    expect((screen.getByRole("button", { name: "Export" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(screen.queryByRole("checkbox")).toBeNull();
    await fireEvent.click(screen.getByRole("button", { name: "Bulk Select" }));
    expect(screen.getByRole("checkbox", { name: "Select all" })).toBeTruthy();
  });
});
