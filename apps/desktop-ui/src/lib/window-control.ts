import { writable } from "svelte/store";

export type WindowMode = "full" | "compact";
export type WindowState = { available: boolean; mode: WindowMode; pinned: boolean };

const initial: WindowState = { available: false, mode: "full", pinned: false };
export const windowState = writable<WindowState>(initial);

export async function loadWindowState(): Promise<WindowState> {
  try {
    const response = await fetch("/api/window");
    if (!response.ok) {
      if (response.status === 404 || response.status === 409) {
        windowState.set(initial);
        return initial;
      }
      throw new Error(`${response.status}`);
    }
    const state = (await response.json()) as WindowState;
    const next = state?.available ? state : initial;
    windowState.set(next);
    return next;
  } catch {
    windowState.set(initial);
  }
  return initial;
}

async function updateWindow(body: Partial<Pick<WindowState, "mode" | "pinned">>): Promise<WindowState> {
  const response = await fetch("/api/window", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error(`${response.status}`);
  const state = (await response.json()) as WindowState;
  windowState.set(state);
  return state;
}

export const setWindowMode = (mode: WindowMode) => updateWindow({ mode });
export const setPinned = (pinned: boolean) => updateWindow({ pinned });
