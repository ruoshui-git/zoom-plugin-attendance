import { get as read } from "svelte/store";
import { getExampleConfigBundle } from "./example-config-data";
import { setUiLanguage } from "./i18n-core";
import * as s from "./stores";
import { t } from "./ui";
import { toast } from "./toast";

const value = <T>(store: {
  subscribe: (run: (value: T) => void) => () => void;
}) => read(store);
const set = (store: any, next: any) => store.set(next);
type DataTarget = "live" | "viewed";
let viewedRequest = 0;
const recordCache = new Map<string, any>();
let refreshRun: Promise<void> | null = null;
let queuedChanged = new Set<string>();
let refreshQueued = false;
function storesFor(target: DataTarget = "live") {
  return target === "viewed"
    ? { meeting: s.viewedMeeting, record: s.viewedRecord, state: s.viewedState, revision: s.viewedStateRevision, events: s.viewedEvents }
    : { meeting: s.meeting, record: s.record, state: s.stateValue, revision: s.stateRevision, events: s.recordEvents };
}

export async function get(path: string) {
  const response = await fetch(path);
  if (!response.ok) throw Error(`${response.status}`);
  return response.json();
}
export async function loadViewedRecord(meeting: s.Meeting) {
  const token = ++viewedRequest;
  set(s.viewedMeeting, meeting);
  set(s.viewedRecord, { rows: [], summary: {}, loading: true });
  set(s.viewedState, { mode: "generic", reconciliationDecisions: [], manualSnapshots: [], raiseHandEntries: [], raiseHandCaptures: [] });
  set(s.viewedStateRevision, 0);
  set(s.viewedEvents, []);
  const safe = async (path: string, fallback: any) => { try { return await get(path); } catch { return fallback; } };
  const [record, state, events] = await Promise.all([
    safe(`/api/meetings/${meeting.instanceId}/record`, { rows: [], summary: {} }),
    safe(`/api/meetings/${meeting.instanceId}/state`, { value: {} }),
    safe(`/api/meetings/${meeting.instanceId}/events`, []),
  ]);
  if (token !== viewedRequest || value(s.viewedMeeting)?.instanceId !== meeting.instanceId) return;
  set(s.viewedRecord, { ...record, loading: false });
  setViewedState(state);
  set(s.viewedEvents, events);
}
// Applies a GET /state response to the viewed-record stores (revision included, so a
// later saveRecordState(..., "viewed") is not rejected as a conflict).
function setViewedState(data: any) {
  const state = { mode: "generic", manualSnapshots: [], raiseHandEntries: [], raiseHandCaptures: [], ...(data?.value || {}) };
  if (!state.reconciliationDecisions) state.reconciliationDecisions = state.decisions || [];
  set(s.viewedStateRevision, data?.revision ?? 0);
  set(s.viewedState, state);
}
export function refresh(options: { changed?: string[] } = {}): Promise<void> {
  for (const id of options.changed || []) queuedChanged.add(id);
  if (refreshRun) { refreshQueued = true; return refreshRun; }
  refreshRun = (async () => {
    do {
      const changed = queuedChanged;
      queuedChanged = new Set();
      refreshQueued = false;
      await refreshOnce(changed);
    } while (refreshQueued || queuedChanged.size);
  })().finally(() => { refreshRun = null; refreshQueued = false; });
  return refreshRun;
}
async function refreshOnce(changed: Set<string>) {
  try {
    const list = await get("/api/meetings");
    set(s.meetings, list);
    const ids = new Set(list.map((item: s.Meeting) => item.instanceId));
    for (const id of recordCache.keys()) if (!ids.has(id)) recordCache.delete(id);
    const need = list.filter((item: s.Meeting) => !recordCache.has(item.instanceId) || !item.endedAt || changed.has(item.instanceId));
    let cursor = 0;
    await Promise.all(Array.from({ length: Math.min(4, need.length) }, async () => {
      while (cursor < need.length) {
        const item = need[cursor++];
        try { recordCache.set(item.instanceId, await get(`/api/meetings/${item.instanceId}/record`)); }
        catch { recordCache.set(item.instanceId, { rows: [], summary: {} }); }
      }
    }));
    const stats: [string, number, string][] = list.map((item: s.Meeting) => [item.instanceId, Number(recordCache.get(item.instanceId)?.summary?.seen ?? recordCache.get(item.instanceId)?.summary?.fullPresent) || 0, item.mode || "generic"]);
    set(
      s.recordCounts,
      Object.fromEntries(stats.map(([id, count]) => [id, count])),
    );
    set(
      s.recordModes,
      Object.fromEntries(stats.map(([id, , recordMode]) => [id, recordMode])),
    );
    let active = value(s.meeting);
    let picked = false;
    if (active?.instanceId)
      set(s.record, recordCache.get(active.instanceId) || await get(`/api/meetings/${active.instanceId}/record`));
    else {
      active = list.find((item: s.Meeting) => !item.endedAt) || null;
      set(s.meeting, active);
      picked = true;
      if (active)
        set(s.record, recordCache.get(active.instanceId) || await get(`/api/meetings/${active.instanceId}/record`));
    }
    if (picked && active) await loadState("live");
  } catch {}
}
function invalidateRecord(id?: string) { if (id) recordCache.delete(id); }
export async function loadConfig() {
  const configs: any = {};
  const revisions: any = {};
  for (const key of ["roster", "aliases", "profiles", "settings"]) {
    try {
      const data = await get(`/api/config/${key}`);
      configs[key] = data.value;
      revisions[key] = data.revision;
    } catch {
      configs[key] = [];
    }
  }
  set(s.configs, configs);
  set(s.configRevision, revisions);
  set(
    s.settings,
    configs.settings || { exportLanguage: "en", filterDefaultsClosed: false },
  );
}
export async function saveConfig(key: string, next: any) {
  const revisions = value(s.configRevision);
  const response = await fetch(`/api/config/${key}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ revision: revisions[key] || 0, value: next }),
  });
  if (response.status === 409) {
    await loadConfig();
    throw Error(
      t(
        "数据已被其他窗口修改，请重试。",
        "Data changed in another window. Please retry.",
      ),
    );
  }
  if (!response.ok) throw Error(await response.text());
  const data = await response.json();
  set(s.configs, { ...value(s.configs), [key]: data.value });
  set(s.configRevision, { ...revisions, [key]: data.revision });
}
export async function saveSettings() {
  try {
    await saveConfig("settings", value(s.settings));
  } catch (error: any) {
    toast.error(error.message);
  }
}
export function download(filename: string, data: string) {
  const anchor = document.createElement("a");
  anchor.href = URL.createObjectURL(
    new Blob([data], { type: "application/json;charset=utf-8" }),
  );
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(anchor.href);
}
export async function downloadBackup() {
  download(
    "attendflow-backup-v3.json",
    JSON.stringify(await get("/api/backup"), null, 2),
  );
}
export async function exportRecords(ids: string[] = []) {
  const data = await get("/api/records/export");
  const records = (data.records || []).filter(
    (item: any) => !ids.length || ids.includes(item.meeting.instanceId),
  );
  download(
    "attendflow-records.json",
    JSON.stringify({ version: 3, records }, null, 2),
  );
}
export async function exportOne(kind: string, target: DataTarget = "live") {
  const targetStores = storesFor(target);
  const active = value(targetStores.meeting);
  if (!active) return;
  const settings = value(s.settings);
  const response = await fetch(
    `/api/meetings/${active.instanceId}/export?kind=${kind}&language=${settings.exportLanguage === "zh" ? "zh-CN" : "en"}`,
  );
  if (!response.ok) throw Error(await response.text());
  const blob = await response.blob();
  const filename =
    response.headers
      .get("content-disposition")
      ?.match(/filename="([^"]+)"/)?.[1] || `session.${kind}`;
  const anchor = document.createElement("a");
  anchor.href = URL.createObjectURL(blob);
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(anchor.href);
  if (target === "live") { await loadState("live"); invalidateRecord(active.instanceId); await refresh({ changed: [active.instanceId] }); }
}
export async function exportAll(target: DataTarget = "live") {
  for (const kind of ["json", "summary", "manual"]) await exportOne(kind, target);
}
export async function deleteRecords(ids: string[], target: DataTarget = "live") {
  if (!ids.length) return;
  const response = await fetch("/api/records/delete", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ instanceIds: ids }),
  });
  if (!response.ok) {
    toast.error(t("无法删除正在进行的会议。", "Active meetings cannot be deleted."));
    return;
  }
  set(s.selectedRecords, []);
  if (target === "viewed") { set(s.viewedMeeting, null); set(s.viewedRecord, { rows: [], summary: {}, loading: false }); }
  else set(s.meeting, null);
  ids.forEach(invalidateRecord); await refresh({ changed: ids });
}
export async function previewRecordImport(event: Event) {
  const input = event.currentTarget as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  try {
    const data = JSON.parse(await file.text());
    const response = await fetch("/api/records/import/preview", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    const preview = await response.json();
    set(s.recordImportPreview, preview);
    if (!response.ok) throw Error(preview.message || "Preview failed");
  } catch (error: any) {
    toast.error(error.message);
  } finally {
    input.value = "";
  }
}
export async function commitRecordImport() {
  const preview = value(s.recordImportPreview);
  const response = await fetch("/api/records/import/commit", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(preview),
  });
  if (!response.ok) {
    toast.error((await response.json()).message || "Import failed");
    return;
  }
  set(s.recordImportPreview, null);
  await refresh({ changed: (preview?.records || []).map((x: any) => x.meeting?.instanceId).filter(Boolean) });
}
export async function loadExampleData() {
  const language = value(s.language);
  const configs = value(s.configs);
  const bundle = getExampleConfigBundle(language === "zh-CN" ? "zh-CN" : "en");
  const roster = configs.roster || [];
  let nextId =
    Math.max(0, ...roster.map((person: any) => Number(person.personId) || 0)) +
    1;
  const idMap = new Map<string, number>();
  const exampleRoster = bundle.rosterPeople.map((person) => {
    const personId = nextId++;
    idMap.set(person.key, personId);
    return {
      personId,
      canonicalName: person.canonicalName,
      pinyin: person.pinyin,
      group: person.group,
      active: person.active,
    };
  });
  const aliases = bundle.aliasMappings.map((alias) => ({
    zoomName: alias.zoomName,
    personId: idMap.get(alias.personKey),
    notes: alias.notes || "",
  }));
  const profiles = bundle.profiles.map(
    ({ specialParticipants, blacklist, ...profile }) => profile,
  );
  const response = await fetch("/api/config/example-data", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ roster: exampleRoster, aliases, profiles }),
  });
  const data = await response.json();
  if (!response.ok) {
    toast.error(
      t(
        "示例数据冲突，未添加任何数据：",
        "Example data conflict; nothing was added: ",
      ) + (data.message || ""),
    );
    return;
  }
  await loadConfig();
  toast.success(
    t(
      "已导入示例数据。",
      "Example roster, aliases, and four profiles loaded.",
    ),
  );
}
export async function addConfig() {
  const configTab = value(s.configTab);
  const configs = value(s.configs);
  let item: any = {};
  if (configTab === "roster")
    item = {
      personId: Date.now(),
      canonicalName: "",
      pinyin: "",
      group: "",
      active: true,
    };
  if (configTab === "aliases") item = { zoomName: "", personId: 0 };
  if (configTab === "profiles")
    item = {
      id: `profile-${Date.now()}`,
      name: "",
      description: "",
      includedSubgroups: ["*"],
      lateAfterMinutes: 10,
      returnWindowMinutes: 8,
      meetingIds: [],
      topicKeywords: [],
      groupSortOrder: [],
      active: true,
    };
  const label =
    configTab === "roster"
      ? "canonicalName"
      : configTab === "aliases"
        ? "zoomName"
        : "name";
  const name = prompt(t("请输入名称", "Enter name"));
  if (!name) return;
  item[label] = name;
  try {
    await saveConfig(configTab, [...(configs[configTab] || []), item]);
  } catch (error: any) {
    toast.error(error.message);
  }
}
export async function editConfig(index: number, item: any) {
  const configTab = value(s.configTab);
  const label =
    configTab === "roster"
      ? "canonicalName"
      : configTab === "aliases"
        ? "zoomName"
        : "name";
  const name = prompt(t("编辑名称", "Edit name"), item[label] || "");
  if (name === null) return;
  const next = [...(value(s.configs)[configTab] || [])];
  next[index] = { ...item, [label]: name };
  try {
    await saveConfig(configTab, next);
  } catch (error: any) {
    toast.error(error.message);
  }
}
export async function removeConfig(index: number) {
  if (!confirm(t("确定删除此项目？", "Delete this item?"))) return;
  const configTab = value(s.configTab);
  const next = [...(value(s.configs)[configTab] || [])];
  next.splice(index, 1);
  try {
    await saveConfig(configTab, next);
  } catch (error: any) {
    toast.error(error.message);
  }
}
export async function loadState(target: DataTarget = "live") {
  const stores = storesFor(target);
  const active = value(stores.meeting);
  if (!active) return;
  try {
    const data = await get(`/api/meetings/${active.instanceId}/state`);
    const state = data.value;
    if (!state.reconciliationDecisions)
      state.reconciliationDecisions = state.decisions || [];
    if (value(stores.meeting)?.instanceId !== active.instanceId) return;
    set(stores.revision, data.revision);
    set(stores.state, state);
    if (target === "live") set(s.mode, state.mode || "generic");
  } catch {}
}
export async function loadRecordEvents(target: DataTarget = "live") {
  const stores = storesFor(target);
  const active = value(stores.meeting);
  if (!active) return;
  try {
    const events = await get(`/api/meetings/${active.instanceId}/events`);
    if (value(stores.meeting)?.instanceId === active.instanceId) set(stores.events, events);
  } catch {
    set(stores.events, []);
  }
}
export async function refreshViewedRecord(instanceId: string) {
  if (value(s.viewedMeeting)?.instanceId !== instanceId) return;
  const [record, state, events] = await Promise.all([
    get(`/api/meetings/${instanceId}/record`).catch(() => null),
    get(`/api/meetings/${instanceId}/state`).catch(() => null),
    get(`/api/meetings/${instanceId}/events`).catch(() => null),
  ]);
  if (value(s.viewedMeeting)?.instanceId !== instanceId) return;
  if (record) set(s.viewedRecord, { ...record, loading: false });
  if (state) setViewedState(state);
  if (events) set(s.viewedEvents, events);
}
export function currentUsers() {
  const users = new Map<string, any>();
  for (const event of value(s.recordEvents)) {
    if (event.kind === "users") {
      if (event.action === "sync_all") users.clear();
      for (const user of event.users || []) {
        const id = String(user.userId);
        if (event.action === "left") users.delete(id);
        else users.set(id, { ...users.get(id), ...user });
      }
    } else if (event.kind === "hand") {
      const id = String(event.userId);
      if (users.has(id))
        users.set(id, { ...users.get(id), raisedHand: !!event.raised });
    }
  }
  return [...users.values()];
}
export async function saveRecordState(patch: any, target: DataTarget = "live") {
  const stores = storesFor(target);
  const active = value(stores.meeting);
  if (!active) return;
  const response = await fetch(`/api/meetings/${active.instanceId}/state`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      revision: value(stores.revision),
      value: { ...value(stores.state), ...patch },
    }),
  });
  if (response.status === 409) {
    await loadState(target);
    return;
  }
  if (!response.ok) throw Error(await response.text());
  const data = await response.json();
  if (value(stores.meeting)?.instanceId !== active.instanceId) return;
  set(stores.revision, data.revision);
  set(stores.state, data.value);
  // Mode/decision changes alter the record (and its count in the Records list) for either target.
  invalidateRecord(active.instanceId); await refresh({ changed: [active.instanceId] });
}
export async function captureManual() {
  const active = value(s.meeting);
  if (!active) return;
  const record = value(s.record);
  const participants = currentUsers().map((user: any) => {
    const row = (record.rows || []).find(
      (item: any) =>
        item.zoomNames?.includes(user.name) || item.zoomName === user.name,
    );
    return {
      participantUUID: String(user.userId),
      screenName: user.name,
      screenNamePinyin: row?.pinyin || "",
      role: user.host ? "host" : "participant",
      location: user.waitingRoom ? "waiting-room" : "meeting",
      sessionParticipantId: row?.sessionParticipantId,
      matchedPersonId: row?.personId,
      matchedCanonicalName: row?.canonicalName,
      matchStatus: row?.matchStatus || "not-applicable",
    };
  });
  const snapshot = {
    id: `manual-${Date.now()}`,
    timestamp: new Date().toISOString(),
    participants,
  };
  await saveRecordState({
    manualSnapshots: [...(value(s.stateValue).manualSnapshots || []), snapshot],
  });
}
export async function captureHands() {
  const raised = currentUsers().filter(
    (user: any) => user.raisedHand || user.raised_hand,
  );
  if (!raised.length) return;
  const capture = {
    id: `hands-${Date.now()}`,
    timestamp: new Date().toISOString(),
    participantUUIDs: raised.map((user: any) => String(user.userId)),
    screenNames: raised.map((user: any) => user.name || "Unknown"),
    totalCount: raised.length,
  };
  await saveRecordState({
    raiseHandCaptures: [
      ...(value(s.stateValue).raiseHandCaptures || []),
      capture,
    ],
  });
}
export async function applyOverrides(patch: any = value(s.stagedState)) {
  const active = value(s.meeting);
  if (!active) return;
  const response = await fetch(`/api/meetings/${active.instanceId}/state`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      revision: value(s.stateRevision),
      value: { ...value(s.stateValue), ...patch },
    }),
  });
  if (!response.ok) {
    await loadState();
    return;
  }
  const data = await response.json();
  set(s.stateRevision, data.revision);
  set(s.stateValue, data.value);
  set(s.stagedState, {});
  set(s.record, await get(`/api/meetings/${active.instanceId}/record`));
  invalidateRecord(active.instanceId); await refresh({ changed: [active.instanceId] });
}
export async function saveDecision(decision: any, revertId?: string) {
  const active = value(s.meeting);
  if (!active) throw Error(t("没有活动会议", "No active meeting"));
  for (let attempt = 0; attempt < 2; attempt++) {
    const state = value(s.stateValue);
    const revisions = value(s.configRevision);
    const body: any = {
      revision: value(s.stateRevision),
      aliasesRevision: revisions.aliases || 0,
      value: {
        ...state,
        reconciliationDecisions: [...(state.reconciliationDecisions || [])],
      },
    };
    if (revertId) body.revertId = revertId;
    else body.decision = decision;
    const response = await fetch(
      `/api/meetings/${active.instanceId}/reconciliation`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      },
    );
    if (response.status === 409 && attempt === 0) {
      await loadState();
      const aliases = await get("/api/config/aliases");
      set(s.configs, { ...value(s.configs), aliases: aliases.value });
      set(s.configRevision, {
        ...value(s.configRevision),
        aliases: aliases.revision,
      });
      continue;
    }
    const data = await response.json().catch(() => ({}));
    if (!response.ok)
      throw Error(
        data.message || t("保存名称匹配失败", "Could not save name matching"),
      );
    set(s.stateRevision, data.revision);
    set(s.stateValue, data.value);
    invalidateRecord(active.instanceId);
    set(s.configRevision, {
      ...value(s.configRevision),
      aliases: data.aliasesRevision,
    });
    const aliases = await get("/api/config/aliases");
    set(s.configs, { ...value(s.configs), aliases: aliases.value });
    set(s.record, await get(`/api/meetings/${active.instanceId}/record`));
    invalidateRecord(active.instanceId); await refresh({ changed: [active.instanceId] });
    toast.success(
      revertId
        ? t("已撤销决定", "Decision reverted")
        : t("名称匹配已保存", "Name matching saved"),
    );
    return;
  }
  throw Error(t("数据已被修改，请重试", "Data changed; please retry"));
}
export async function selectProfile(id: string) {
  set(s.stagedState, {
    selectedProfileId: id,
    lateAfterMinutesOverride: null,
    returnWindowMinutesOverride: null,
  });
  await applyOverrides();
}
export function profileGuess() {
  const profiles = (value(s.configs).profiles || []).filter(
    (profile: any) => profile.active,
  );
  const state = value(s.stateValue);
  const meeting = value(s.meeting);
  if (state.selectedProfileId) {
    const chosen = profiles.find(
      (profile: any) => profile.id === state.selectedProfileId,
    );
    if (chosen) return chosen;
  }
  return (
    profiles.find((profile: any) =>
      (profile.meetingIds || []).includes(meeting?.meetingNumber),
    ) ||
    profiles.find((profile: any) =>
      (profile.topicKeywords || []).some((keyword: string) =>
        (meeting?.topic || "").toLowerCase().includes(keyword.toLowerCase()),
      ),
    ) ||
    profiles.find((profile: any) => profile.id === "default-all") ||
    profiles[0]
  );
}
export function resetOverrides() {
  set(s.stagedState, {
    lateAfterMinutesOverride: null,
    returnWindowMinutesOverride: null,
  });
}
export async function switchMode(next: string) {
  const active = value(s.meeting);
  if (!active) return;
  set(s.mode, next);
  const state = { ...value(s.stateValue), mode: next };
  set(s.stateValue, state);
  try {
    const response = await fetch(`/api/meetings/${active.instanceId}/state`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ revision: value(s.stateRevision), value: state }),
    });
    if (response.status === 409) {
      await loadState();
      return;
    }
    const data = await response.json();
    set(s.stateRevision, data.revision);
    invalidateRecord(active.instanceId);
    await refresh({ changed: [active.instanceId] });
  } catch {}
}
export async function zoomSignIn() {
  const response = await fetch("/api/zoom/sign-in", { method: "POST" }).catch(
    () => null,
  );
  if (!response?.ok) {
    toast.error(t("无法开始登录", "Could not start sign-in"));
  }
}
export async function zoomSignOut() {
  if (
    !confirm(
      t(
        "退出 Zoom 登录，并从你的 Zoom 账号中移除出勤助手桌面版？考勤记录会保留在这台电脑上。",
        "Sign out and remove AttendFlow Desktop from your Zoom account? Your attendance records stay on this computer.",
      ),
    )
  )
    return false;
  const response = await fetch("/api/zoom/sign-out", { method: "POST" }).catch(
    () => null,
  );
  const data = response ? await response.json().catch(() => ({})) : {};
  (response?.ok ? toast.success : toast.error)(
    response?.ok
      ? data.revoked
        ? t("已退出 Zoom 并移除授权", "Signed out and access removed")
        : t(
            "已退出。请在 Zoom 网站的“已添加的应用”中移除 AttendFlow Desktop。",
            "Signed out. Remove AttendFlow Desktop under Added apps on the Zoom website.",
          )
      : t("退出失败", "Sign-out failed"),
  );
  return !!response?.ok;
}
export async function zoomAction(action: string) {
  if (action === "switch_account") {
    if (await zoomSignOut()) await zoomSignIn();
  } else await zoomSignIn();
}
export async function importBackup(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  try {
    const data = JSON.parse(await file.text());
    const preview = `${data.rosterPeople?.length || 0} roster · ${data.aliasMappings?.length || 0} aliases · ${data.attendanceProfiles?.length || 0} profiles`;
    if (
      !confirm(
        `${t("将导入以下配置：", "Import this backup configuration:")} ${preview}`,
      )
    )
      return;
    const response = await fetch("/api/backup", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    if (response.ok) {
      await loadConfig();
      toast.success(t("备份配置已导入", "Backup configuration imported"));
    } else toast.error(t("导入失败：", "Import failed: ") + (await response.text()));
  } catch (error: any) {
    toast.error(error.message);
  } finally {
    input.value = "";
  }
}
export function chooseLanguage(next: string) {
  localStorage.setItem("ui-language", next);
  setUiLanguage(next as any);
  window.location.reload();
}

export async function loadExampleDataQuiet() {
  const current = value(s.configs);
  const bundle = getExampleConfigBundle(value(s.language) === "zh-CN" ? "zh-CN" : "en");
  let nextId = Math.max(0, ...(current.roster || []).map((person: any) => Number(person.personId) || 0)) + 1;
  const idMap = new Map<string, number>();
  const roster = bundle.rosterPeople.map((person) => {
    const personId = nextId++;
    idMap.set(person.key, personId);
    return { personId, canonicalName: person.canonicalName, pinyin: person.pinyin, group: person.group, active: person.active };
  });
  const aliases = bundle.aliasMappings.map((alias) => ({ zoomName: alias.zoomName, personId: idMap.get(alias.personKey), notes: alias.notes || "" }));
  const profiles = bundle.profiles.map(({ specialParticipants, blacklist, ...profile }) => profile);
  const response = await fetch("/api/config/example-data", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ roster, aliases, profiles }) });
  const data = await response.json();
  if (!response.ok) throw Error(data.message || t("示例数据冲突，未添加任何数据", "Example data conflict; nothing was added"));
  await loadConfig();
  toast.success(t("已导入示例数据。", "Example roster, aliases, and four profiles loaded."));
}
