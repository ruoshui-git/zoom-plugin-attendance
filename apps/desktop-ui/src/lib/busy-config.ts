import { writable } from "svelte/store";

export const busyKeys = writable<Set<string>>(new Set());

export async function runBusy<T>(key: string, fn: () => Promise<T>): Promise<T | undefined> {
  let running = false;
  const unsubscribe = busyKeys.subscribe((keys) => { running = keys.has(key); });
  unsubscribe();
  if (running) return undefined;
  busyKeys.update((keys) => new Set(keys).add(key));
  try { return await fn(); }
  finally { busyKeys.update((keys) => { const next = new Set(keys); next.delete(key); return next; }); }
}
