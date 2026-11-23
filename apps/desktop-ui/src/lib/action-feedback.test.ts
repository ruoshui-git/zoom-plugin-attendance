// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { get } from "svelte/store";
import ConfigWorkspace from "./ConfigWorkspace.svelte";
import RecordsPage from "../pages/RecordsPage.svelte";
import RecordDetail from "../pages/records/RecordDetail.svelte";
import * as stores from "./stores";
import { loadConfig, saveSettings, switchMode } from "./api";
import { toast } from "./toast";

const flush = () => new Promise((r) => setTimeout(r, 0));
const toastTexts = () => get(toast).map((item: any) => `${item.type}:${item.message}`);

describe("action feedback (busy states and toasts)", () => {
  afterEach(() => { cleanup(); vi.unstubAllGlobals(); toast.clear(); });

  async function openAddRoster() {
    render(ConfigWorkspace, { language: "en", roster: [], aliases: [] });
    await fireEvent.click(screen.getByRole("button", { name: /Add Person/ }));
    const name = screen.getByLabelText(/Canonical name/) as HTMLInputElement;
    await fireEvent.input(name, { target: { value: "Ada Lovelace" } });
  }

  it("a double click on Save sends one request and shows the success toast", async () => {
    let release!: (v: Response) => void;
    const fetchMock = vi.fn((url: string, init?: any) => init?.method === "PUT"
      ? new Promise<Response>((r) => { release = r; })
      : Promise.resolve(new Response("{}", { status: 200 })));
    vi.stubGlobal("fetch", fetchMock);
    await openAddRoster();
    const save = screen.getByRole("button", { name: "Save" });
    await fireEvent.click(save);
    await fireEvent.click(screen.getByRole("button", { name: /Saving…|Save/ }));
    expect(fetchMock.mock.calls.filter(([, init]) => init?.method === "PUT")).toHaveLength(1);
    expect((screen.getByRole("button", { name: "Saving…" }) as HTMLButtonElement).disabled).toBe(true);
    release(new Response(JSON.stringify({ revision: 1, value: [{ personId: 1, canonicalName: "Ada Lovelace" }] }), { status: 200 }));
    await flush(); await flush();
    expect(toastTexts()).toContain("success:Roster row added.");
  });

  it("a failed save shows the error toast and re-enables Save", async () => {
    vi.stubGlobal("fetch", vi.fn(async (url: string, init?: any) => init?.method === "PUT"
      ? new Response("disk full", { status: 500 })
      : new Response("{}", { status: 200 })));
    await openAddRoster();
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await flush(); await flush();
    expect(toastTexts()).toContain("error:Save failed: disk full");
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(false);
  });
  it("a failed record import preview says Preview failed, not Export failed", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({ message: "bad bundle" }), { status: 422 })));
    render(RecordsPage);
    await fireEvent.click(screen.getByRole("button", { name: "Paste JSON" }));
    await fireEvent.input(document.querySelector("textarea")!, { target: { value: "[{}]" } });
    await fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await flush(); await flush();
    expect(toastTexts()).toContain("error:Preview failed: bad bundle");
    expect(toastTexts().some((x) => x.includes("Export failed"))).toBe(false);
  });

  it("a failed delete in the record panel says Delete failed and re-enables Delete", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response("locked", { status: 500 })));
    stores.viewedMeeting.set({ instanceId: "m1", meetingNumber: "1", topic: "Demo", startedAt: 1, endedAt: 2 } as any);
    stores.viewedRecord.set({ rows: [], summary: {}, loading: false } as any);
    render(RecordDetail);
    window.addEventListener("attendflow:confirm", (e: any) => e.detail.resolve(true), { once: true });
    await fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await flush(); await flush(); await flush();
    expect(toastTexts()).toContain("error:Delete failed: locked");
    expect((screen.getByRole("button", { name: "Delete" }) as HTMLButtonElement).disabled).toBe(false);
    stores.viewedMeeting.set(null);
  });
  it("switching to roster with no roster makes no request; a failed switch restores the previous mode", async () => {
    const fetchMock = vi.fn(async () => new Response("boom", { status: 500 }));
    vi.stubGlobal("fetch", fetchMock);
    stores.meeting.set({ instanceId: "live-1" } as any);
    stores.mode.set("generic");
    stores.configs.set({ roster: [] } as any);
    expect(await switchMode("roster")).toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
    stores.configs.set({ roster: [{ personId: 1, canonicalName: "Ada" }] } as any);
    await expect(switchMode("roster")).rejects.toThrow("boom");
    expect(get(stores.mode)).toBe("generic");
    expect(get(stores.stateValue).mode).toBe("generic");
    stores.meeting.set(null);
  });
  it("changing only the export language toasts only that change", async () => {
    vi.stubGlobal("fetch", vi.fn(async (url: string, init?: any) => {
      if (init?.method === "PUT") return new Response(JSON.stringify({ revision: 2, value: JSON.parse(init.body).value }), { status: 200 });
      return new Response(JSON.stringify({ revision: 1, value: String(url).endsWith("/settings") ? {} : [] }), { status: 200 });
    }));
    await loadConfig();
    toast.clear();
    stores.settings.set({ exportLanguage: "zh", filterDefaultsClosed: false } as any);
    await saveSettings();
    expect(toastTexts()).toEqual(["success:Export language set to Chinese."]);
  });
});
