// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import ConfirmDialog from "./ConfirmDialog.svelte";
import ToggleSwitch from "./ToggleSwitch.svelte";
import { confirmDialog } from "../ui";

afterEach(() => cleanup());

describe("shared controls", () => {
  it("closes after choosing and queues a second confirmation", async () => {
    render(ConfirmDialog);
    const accepted = confirmDialog({
      title: "Apply?",
      message: "Continue?",
      confirmLabel: "Continue",
      cancelLabel: "Stay",
    });
    const queued = confirmDialog({
      title: "Delete?",
      message: "Remove row?",
      confirmLabel: "Delete",
      cancelLabel: "Keep",
    });
    await fireEvent.click(await screen.findByText("Continue"));
    await expect(accepted).resolves.toBe(true);
    expect(screen.queryByText("Apply?")).toBeNull();
    await fireEvent.click(await screen.findByText("Keep"));
    await expect(queued).resolves.toBe(false);
    expect(screen.queryByText("Delete?")).toBeNull();
  });

  it("toggles its checked value", async () => {
    render(ToggleSwitch, { label: "Enabled", checked: false });
    const checkbox = screen.getByRole("checkbox") as HTMLInputElement;
    await fireEvent.click(checkbox);
    expect(checkbox.checked).toBe(true);
  });
});
