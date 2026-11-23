import { get, writable } from "svelte/store";

/** Keys of actions currently running. Buttons use `$busyKeys.has(key)` to disable themselves. */
export const busyKeys = writable<Set<string>>(new Set());

/**
 * Runs `fn` unless an action with the same key is already running (no double submits).
 * Resolves to fn's result, or undefined when skipped. The key is cleared even if fn throws.
 */
export async function runBusy<T>(key: string, fn: () => Promise<T> | T): Promise<T | undefined> {
  if (get(busyKeys).has(key)) return undefined;
  busyKeys.update((keys) => new Set(keys).add(key));
  try {
    return await fn();
  } finally {
    busyKeys.update((keys) => {
      const next = new Set(keys);
      next.delete(key);
      return next;
    });
  }
}
