import { writable } from "svelte/store";

export const UI_SCALE_STEPS = ["compact", "medium", "large", "xlarge", "xxlarge"] as const;
export type UiScale = (typeof UI_SCALE_STEPS)[number];
export const UI_SCALE_ZOOM: Record<UiScale, number> = {
  compact: 1,
  medium: 1.2,
  large: 1.45,
  xlarge: 1.7,
  xxlarge: 1.95,
};

function initialUiScale(): UiScale {
  try {
    const saved = localStorage.getItem("attendflow-desktop.ui-scale");
    if (UI_SCALE_STEPS.includes(saved as UiScale)) return saved as UiScale;
    // Migrate the old UI size setting once, keeping its selected value.
    const legacy = localStorage.getItem("ui-size");
    const scale = UI_SCALE_STEPS.includes(legacy as UiScale) ? legacy as UiScale : "compact";
    localStorage.setItem("attendflow-desktop.ui-scale", scale);
    return scale;
  } catch {
    return "compact";
  }
}

export const uiScale = writable<UiScale>(initialUiScale());
uiScale.subscribe((scale) => {
  try {
    document.documentElement.style.zoom = String(UI_SCALE_ZOOM[scale]);
    localStorage.setItem("attendflow-desktop.ui-scale", scale);
  } catch {
    // Storage and document may be unavailable in tests or during early startup.
  }
});

export function setUiScale(scale: UiScale): void {
  uiScale.set(scale);
}

export type Row = Record<string, any>;
export type Meeting = Record<string, any>;

export const page = writable("live");
export const requestedLiveSubtab = writable<string | null>(null);
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
