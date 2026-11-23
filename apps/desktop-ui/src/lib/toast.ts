import { get, writable } from "svelte/store";
import type { ToastMessage, ToastType } from "./common/toast-types";

type TimedToast = ToastMessage & { timerId: ReturnType<typeof setInterval> | null };
const store = writable<TimedToast[]>([]);
function updateToast(id: string, change: (toast: TimedToast) => void) {
  store.update((items) => { const item = items.find((toast) => toast.id === id); if (item) change(item); return [...items]; });
}
function dismiss(id: string) {
  store.update((items) => { const item = items.find((toast) => toast.id === id); if (item?.timerId) clearTimeout(item.timerId); return items.filter((toast) => toast.id !== id); });
}
function startTimer(item: TimedToast) {
  if (item.timerId) clearInterval(item.timerId);
  item.timerId = setInterval(() => {
    item.remainingTime = Math.max(0, item.remainingTime - 50);
    if (item.remainingTime <= 0) dismiss(item.id);
    else store.update((items) => [...items]);
  }, 50);
}
function show(message: string, type: ToastType = "info", duration = 5000) {
  const item: TimedToast = { id: `toast-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`, type, message, duration, remainingTime: duration, paused: false, timerId: null };
  store.update((items) => [...items, item]); if (duration > 0) startTimer(item); return item.id;
}
export const toast = {
  subscribe: (run: (items: ToastMessage[]) => void) => store.subscribe(run),
  show,
  success: (message: string, duration?: number) => show(message, "success", duration),
  error: (message: string, duration?: number) => show(message, "error", duration),
  warning: (message: string, duration?: number) => show(message, "warning", duration),
  info: (message: string, duration?: number) => show(message, "info", duration),
  pause(id: string) { updateToast(id, (item) => { if (!item.paused) { item.paused = true; if (item.timerId) clearInterval(item.timerId); item.timerId = null; } }); },
  resume(id: string) { updateToast(id, (item) => { if (item.paused) { item.paused = false; if (item.remainingTime > 0) startTimer(item); else dismiss(id); } }); },
  dismiss,
  clear() { store.update((items) => { items.forEach((item) => { if (item.timerId) clearTimeout(item.timerId); }); return []; }); },
  getProgress(id: string) { const item = get(store).find((entry) => entry.id === id); return item && item.duration > 0 ? Math.max(0, item.remainingTime / item.duration * 100) : 100; },
};
