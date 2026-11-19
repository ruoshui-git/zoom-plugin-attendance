// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, cleanup } from "@testing-library/svelte";
import ConfigWorkspace from "./ConfigWorkspace.svelte";

describe("configuration import preview component", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("sends pasted CSV to Rust and renders the returned row preview", async () => {
    const preview = {
      revision: 4,
      changes: [
        {
          row: 2,
          action: "Add",
          id: 3,
          before: null,
          after: { canonicalName: "Ada" },
          notes: "",
          aliasImpact: 0,
        },
      ],
      value: [
        {
          personId: 3,
          canonicalName: "Ada",
          pinyin: "ada",
          group: "",
          active: true,
        },
      ],
    };
    const fetchMock = vi
      .fn()
      .mockResolvedValue({ ok: true, json: async () => preview });
    vi.stubGlobal("fetch", fetchMock);
    render(ConfigWorkspace, { language: "en", roster: [], aliases: [] });
    await fireEvent.click(screen.getByText("Import"));
    await fireEvent.input(
      screen.getByPlaceholderText("Or paste CSV content here…"),
      {
        target: { value: "canonicalName\nAda" },
      },
    );
    await fireEvent.click(screen.getByText("Preview"));
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/config/roster/import/preview",
      expect.objectContaining({ method: "POST" }),
    );
    expect(await screen.findByText("Add")).toBeTruthy();
    expect(document.body.textContent).toContain("canonicalName");
    expect(document.body.textContent).toContain('"Ada"');
  });

  it("only shows selection controls after enabling bulk select", async () => {
    render(ConfigWorkspace, {
      language: "en",
      roster: [
        {
          personId: 1,
          canonicalName: "Ada",
          pinyin: "ada",
          group: "A",
          active: true,
        },
      ],
      aliases: [],
    });
    expect(screen.queryByText("Select all filtered")).toBeNull();
    await fireEvent.click(screen.getByText("Bulk Select"));
    expect(screen.getByText("Select all filtered")).toBeTruthy();
    await fireEvent.click(screen.getByText("Select all filtered"));
    await fireEvent.click(screen.getByText("Bulk edit"));
    expect(screen.getByText("Bulk edit · 1")).toBeTruthy();
    expect(screen.getByLabelText("Group (blank keeps current)")).toBeTruthy();
    expect(screen.getByLabelText("Active status")).toBeTruthy();
  });

  it("opens the editor when a roster row is clicked", async () => {
    render(ConfigWorkspace, {
      language: "en",
      roster: [
        {
          personId: 1,
          canonicalName: "Ada",
          pinyin: "ada",
          group: "A",
          active: true,
        },
      ],
      aliases: [],
    });
    await fireEvent.click(screen.getByText("Ada"));
    expect(screen.getByText("Edit roster person")).toBeTruthy();
    expect(screen.getByLabelText("Canonical name *")).toBeTruthy();
  });
});
