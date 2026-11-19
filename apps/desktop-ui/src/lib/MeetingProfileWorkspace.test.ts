// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import MeetingProfileWorkspace from "./MeetingProfileWorkspace.svelte";
import ConfirmDialog from "./common/ConfirmDialog.svelte";

afterEach(() => cleanup());

describe("attendance profile workspace", () => {
  it("shows required name validation and locks the default profile name", async () => {
    render(ConfirmDialog);
    render(MeetingProfileWorkspace, {
      profiles: [
        {
          id: "default-all",
          name: "All participants",
          includedSubgroups: [],
          lateAfterMinutes: 10,
          returnWindowMinutes: 8,
          active: true,
        },
      ],
      language: "en",
    });
    await fireEvent.click(screen.getByText("＋ Add Profile"));
    const name = screen.getByLabelText("Name (required)") as HTMLInputElement;
    expect(name.disabled).toBe(false);
    await fireEvent.click(screen.getByText("Save"));
    expect(screen.getByText("Name is required")).toBeTruthy();
    await fireEvent.click(screen.getByText("×"));
    await fireEvent.click(screen.getByText("All participants"));
    expect(
      (screen.getByLabelText("Name (required)") as HTMLInputElement).disabled,
    ).toBe(true);
  });

  it("uses the shared confirmation dialog before deleting selected profiles", async () => {
    render(ConfirmDialog);
    render(MeetingProfileWorkspace, {
      profiles: [
        {
          id: "custom",
          name: "Custom",
          includedSubgroups: [],
          lateAfterMinutes: 10,
          returnWindowMinutes: 8,
          active: true,
        },
      ],
      language: "en",
    });
    await fireEvent.click(screen.getByText("Bulk Select"));
    const checkbox = screen.getAllByRole("checkbox")[0];
    await fireEvent.click(checkbox);
    await fireEvent.click(screen.getByText("Delete selected"));
    expect(
      await screen.findByText(
        "Delete selected profiles? The default profile is kept.",
      ),
    ).toBeTruthy();
    await fireEvent.click(screen.getByText("Cancel"));
    expect(screen.getByText("Custom")).toBeTruthy();
  });
});
