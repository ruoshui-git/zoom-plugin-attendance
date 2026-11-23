// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import Toast from "./Toast.svelte";

describe("Toast", () => {
  afterEach(cleanup);
  it("renders type, message and close button and dismisses on click", async () => {
    let result: ReturnType<typeof render>;
    const onDismiss = vi.fn(async (id: string) => { await result.rerender({ toasts: [], onDismiss }); });
    result = render(Toast, { props: { toasts: [{ id: "a", type: "error", message: "Could not save", duration: 5000, remainingTime: 5000, paused: false }], onDismiss } });
    expect(screen.getByText("Could not save")).toBeTruthy();
    expect(screen.getByText("Could not save").closest(".toast")?.classList.contains("toast-error")).toBe(true);
    await fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onDismiss).toHaveBeenCalledWith("a");
    await new Promise((resolve) => setTimeout(resolve, 250));
    expect(screen.queryByText("Could not save")).toBeNull();
  });
});
