import { writable } from "svelte/store";

export type Row = Record<string, any>;
export type Meeting = Record<string, any>;

export const page = writable("live");
export const connected = writable(false);
export const meeting = writable<Meeting | null>(null);
export const meetings = writable<Meeting[]>([]);
export const record = writable<any>({ rows: [], summary: {} });
export const recordCounts = writable<Record<string, number>>({});
export const recordModes = writable<Record<string, string>>({});
export const configs = writable<any>({});
export const configRevision = writable<any>({});
export const settings = writable<any>({});
export const stateValue = writable<any>({
  mode: "generic",
  reconciliationDecisions: [],
  manualSnapshots: [],
  raiseHandEntries: [],
  raiseHandCaptures: [],
});
export const stateRevision = writable(0);
export const mode = writable("generic");
export const language = writable("en");
export const recordEvents = writable<any[]>([]);
export const recordSearch = writable("");
export const selectedRecords = writable<string[]>([]);
export const configTab = writable("roster");
export const stagedState = writable<any>({});
export const recordImportPreview = writable<any>(null);
export const zoom = writable<any>(null);
export const toastMessage = writable("");
export const filter = writable("");
export const activeFilter = writable("");
export const fileInput = writable<HTMLInputElement | null>(null);
export const recordImportInput = writable<HTMLInputElement | null>(null);
