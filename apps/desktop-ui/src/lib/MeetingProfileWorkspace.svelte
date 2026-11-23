<script lang="ts">
  import FormActions from "./common/FormActions.svelte";
  import { confirmDialog } from "./ui";
  import { saveConfig } from "./api";
  import { tick } from "svelte";
  import ImportPreviewSummary from "./common/ImportPreviewSummary.svelte";
  import { toast } from "./toast";
  export let profiles: any[] = [];
  export let revision = 0;
  export let language = "en";
  export let onSaved: () => void = () => {};
  const t = (zh: string, en: string) => (language === "zh-CN" ? zh : en);
  function importActionLabel(action: unknown) {
    switch (String(action).toLowerCase()) {
      case "add": case "added": case "create": case "created": return t("新增", "Add");
      case "update": case "updated": return t("更新", "Update");
      case "remove": case "removed": case "delete": case "deleted": return t("删除", "Delete");
      case "reject": case "rejected": return t("拒绝", "Reject");
      default: return t("未知", "Unknown");
    }
  }
  let query = "",
    active = "all",
    sortKey = "name",
    asc = true,
    selected: string[] = [],
    bulkMode = false,
    open = false,
    editing: any = null,
    draft: any = {},
    error = "",
    importFile: HTMLInputElement,
    preview: any = null,
    bulkActive = "",
    bulkLate = "",
    bulkReturn = "";
  let previewHeading: HTMLHeadingElement;
  $: changes = preview?.changes || [];
  $: applicableCount = changes.filter((c: any) => !["reject", "rejected"].includes(String(c.action || c.status).toLowerCase()) && c.rejected !== true).length;
  function fields(value: any, before?: any) { if (!value || typeof value !== "object") return []; return Object.entries(value).filter(([key, val]) => key !== "id" && (!before || JSON.stringify(before[key]) !== JSON.stringify(val))); }
  const display = (v: any) => v == null || v === "" ? "—" : Array.isArray(v) ? v.join(", ") : typeof v === "boolean" ? (v ? t("是", "Yes") : t("否", "No")) : String(v);
  async function focusPreview() { await tick(); previewHeading?.scrollIntoView({ behavior: "smooth", block: "start" }); previewHeading?.focus(); }
  const split = (s: string, comma = false) =>
    s
      .split(comma ? /[,\n]/ : /[\n]/)
      .map((x) => x.trim())
      .filter(Boolean);
  function start(p: any = null) {
    editing = p;
    draft = p
      ? {
          ...p,
          includedSubgroups: [...(p.includedSubgroups || [])].join("\n"),
          meetingIds: (p.meetingIds || []).join("\n"),
          topicKeywords: (p.topicKeywords || []).join(", "),
          groupSortOrder: (p.groupSortOrder || []).join("\n"),
        }
      : {
          id: `profile-${Date.now()}`,
          name: "",
          description: "",
          includedSubgroups: "*",
          lateAfterMinutes: 10,
          returnWindowMinutes: 8,
          meetingIds: "",
          topicKeywords: "",
          groupSortOrder: "",
          active: true,
        };
    error = "";
    open = true;
  }
  async function put(rows: any[]) {
    await saveConfig("profiles", rows);
    onSaved();
  }
  async function save() {
    const p = {
      ...draft,
      includedSubgroups: split(draft.includedSubgroups, true),
      meetingIds: split(draft.meetingIds, true),
      topicKeywords: split(draft.topicKeywords, true),
      groupSortOrder: split(draft.groupSortOrder, true),
    };
    delete p.specialParticipants;
    delete p.blacklist;
    if (!p.name.trim()) {
      error = t("名称为必填项", "Name is required");
      return;
    }
    if (
      !Number.isInteger(+p.lateAfterMinutes) ||
      +p.lateAfterMinutes <= 0 ||
      !Number.isInteger(+p.returnWindowMinutes) ||
      +p.returnWindowMinutes <= 0
    ) {
      error = t(
        "迟到和返回阈值必须为正整数",
        "Late and return thresholds must be positive integers",
      );
      return;
    }
    try {
      await put(
        editing
          ? profiles.map((x) => (x.id === p.id ? p : x))
          : [...profiles, p],
      );
      open = false;
    } catch (e: any) {
      error = e.message;
    }
  }
  async function remove() {
    const ids = selected.filter((id) => id !== "default-all");
    if (!ids.length) return;
    if (
      !(await confirmDialog({
        title: t("确认删除", "Confirm deletion"),
        message: t(
          "删除所选配置？默认配置会保留。",
          "Delete selected profiles? The default profile is kept.",
        ),
        destructive: true,
      }))
    )
      return;
    try {
      await put(profiles.filter((p) => !ids.includes(p.id)));
      selected = [];
    } catch (e: any) {
      error = e.message;
    }
  }
  async function removeEditing() {
    if (!editing || editing.id === "default-all") return;
    if (
      !(await confirmDialog({
        title: t("删除考勤配置", "Delete Attendance Profile"),
        message: t(
          `确定要删除“${editing.name}”吗？`,
          `Delete “${editing.name}”?`,
        ),
        destructive: true,
      }))
    )
      return;
    try {
      await put(profiles.filter((p) => p.id !== editing.id));
      open = false;
      editing = null;
    } catch (e: any) {
      error = e.message;
    }
  }
  async function bulkEdit() {
    if (!selected.length) return;
    const rows = profiles.map((p) => {
      if (!selected.includes(p.id) || p.id === "default-all") return p;
      const n = { ...p };
      if (bulkActive !== "") n.active = bulkActive === "true";
      if (
        bulkLate.trim() &&
        Number.isInteger(Number(bulkLate)) &&
        Number(bulkLate) > 0
      )
        n.lateAfterMinutes = Number(bulkLate);
      if (
        bulkReturn.trim() &&
        Number.isInteger(Number(bulkReturn)) &&
        Number(bulkReturn) > 0
      )
        n.returnWindowMinutes = Number(bulkReturn);
      return n;
    });
    try {
      await put(rows);
      bulkActive = "";
      bulkLate = "";
      bulkReturn = "";
    } catch (e: any) {
      error = e.message;
    }
  }
  async function reset() {
    const p = profiles.find((x) => x.id === "default-all");
    if (
      !p ||
      !(await confirmDialog({
        title: t("重置为出厂默认", "Reset factory defaults"),
        message: t(
          "将默认配置恢复为工厂设置（迟到 10 分钟、返回 8 分钟、启用、无分组）？",
          "Reset default profile to factory settings (late 10, return 8, active, no subgroups)?",
        ),
        destructive: true,
      }))
    )
      return;
    try {
      await put(
        profiles.map((x) =>
          x.id === "default-all"
            ? {
                ...x,
                description: "Factory default profile",
                includedSubgroups: [],
                lateAfterMinutes: 10,
                returnWindowMinutes: 8,
                meetingIds: [],
                topicKeywords: [],
                groupSortOrder: [],
                active: true,
              }
            : x,
        ),
      );
    } catch (e: any) {
      error = e.message;
    }
  }
  function exportJson(ids: string[] = []) {
    const arr = profiles
      .filter((p) => !ids.length || ids.includes(p.id))
      .map(({ specialParticipants, blacklist, ...p }) => p);
    const stamp = new Date()
      .toISOString()
      .replace(/[-:]/g, "")
      .replace(/\.\d+Z$/, "Z");
    const name = ids.length
      ? `attendance-profiles-selected-${stamp}.json`
      : `attendance-profiles-${stamp}.json`;
    const a = document.createElement("a");
    a.href = URL.createObjectURL(
      new Blob([JSON.stringify(arr, null, 2)], { type: "application/json" }),
    );
    a.download = name;
    a.click();
    URL.revokeObjectURL(a.href);
  }
  async function readImport(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f) return;
    try {
      const data = JSON.parse(await f.text());
      if (!Array.isArray(data))
        throw Error(t("JSON 必须是数组", "JSON must be an array"));
      const r = await fetch("/api/profiles/import/preview", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ profiles: data }),
      });
      preview = await r.json();
      if (!r.ok) throw Error(preview.message || "Preview failed");
      await focusPreview();
      await focusPreview();
    } catch (e: any) {
      error = e.message; toast.error(e.message);
    } finally {
      (e.target as HTMLInputElement).value = "";
    }
  }
  async function commit() {
    try {
      const r = await fetch("/api/profiles/import/commit", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(preview),
      });
      const d = await r.json();
      if (!r.ok) throw Error(d.message || "Import failed");
      preview = null;
      revision = d.revision;
      onSaved();
      toast.success(t("考勤配置导入成功。", "Attendance profiles imported successfully."));
    } catch (e: any) {
      error = e.message; toast.error(e.message);
    }
  }
  $: visible = profiles
    .filter(
      (p) =>
        (!query ||
          `${p.name} ${p.description}`
            .toLowerCase()
            .includes(query.toLowerCase())) &&
        (active === "all" || String(p.active) === active),
    )
    .sort(
      (a, b) =>
        String(a[sortKey] ?? "").localeCompare(String(b[sortKey] ?? "")) *
        (asc ? 1 : -1),
    );
</script>

<section class="panel profile-workspace">
  <div class="panel-title">
    <h2>{t("考勤配置", "Attendance Profiles")}</h2>
    <div class="actions">
      <button class="primary" onclick={() => start()}>
        ＋ {t("添加配置", "Add Profile")}
      </button><button onclick={() => importFile.click()}>
        {t("导入", "Import")}
      </button><button onclick={() => exportJson()}>
        {t("导出", "Export")}
      </button><span class="count-pill">
        {profiles.length}
        {t("条记录", "records")}
      </span><button class="primary" onclick={() => (bulkMode = true)}>
        {t("批量管理", "Bulk Select")}
      </button><input
        bind:this={importFile}
        type="file"
        accept=".json,application/json"
        hidden
        onchange={readImport}
      />
    </div>
  </div>
  <div class="toolbar">
    <input
      placeholder={t("按名称筛选", "Filter by name")}
      bind:value={query}
    /><select bind:value={active}>
      <option value="all">{t("全部状态", "All statuses")}</option><option
        value="true"
      >
        {t("启用", "Active")}
      </option><option value="false">{t("停用", "Inactive")}</option>
    </select><button
      onclick={() => {
        sortKey = "name";
        asc = !asc;
      }}
    >
      {t("名称排序", "Sort name")}
      {asc ? "↑" : "↓"}
    </button><button onclick={reset}>
      {t("重置为出厂默认", "Reset to factory defaults")}
    </button>
  </div>
  {#if bulkMode}<div class="toolbar">
      <label>
        {t("批量启用", "Bulk active")}
        <select bind:value={bulkActive}>
          <option value="">—</option><option value="true">
            {t("启用", "Active")}
          </option><option value="false">{t("停用", "Inactive")}</option>
        </select>
      </label><label>
        {t("迟到分钟", "Late min")}
        <input
          type="number"
          min="1"
          bind:value={bulkLate}
          placeholder={t("保持不变", "Keep")}
        />
      </label><label>
        {t("返回分钟", "Return min")}
        <input
          type="number"
          min="1"
          bind:value={bulkReturn}
          placeholder={t("保持不变", "Keep")}
        />
      </label><button disabled={!selected.length} onclick={bulkEdit}>
        {t("应用批量编辑", "Apply bulk edit")}
      </button><button
        onclick={() => {
          selected = [];
          bulkMode = false;
        }}
      >
        {t("退出批量管理", "Exit Bulk Select")}
      </button><button disabled={!selected.length} onclick={remove}>
        {t("删除所选", "Delete selected")}
      </button>
    </div>{/if}
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          {#if bulkMode}<th
            ></th>{/if}{#each [["name", "Name", "名称"], ["description", "Description", "描述"], ["includedSubgroups", "Subgroups", "分组"], ["lateAfterMinutes", "Late (min)", "迟到(分)"], ["returnWindowMinutes", "Return (min)", "返回(分)"], ["active", "Active", "启用"]] as [key, en, zh]}<th>
              <button
                class="sort"
                onclick={() => {
                  sortKey = key;
                  asc = !asc;
                }}
              >
                {t(zh, en).toUpperCase()}
                {sortKey === key ? (asc ? "↑" : "↓") : ""}
              </button>
            </th>{/each}
        </tr>
      </thead><tbody>
        {#each visible as p}<tr
            class="data-row"
            onclick={() => start(p)}
            onkeydown={(event) => event.key === "Enter" && start(p)}
            tabindex="0"
          >
            {#if bulkMode}<td onclick={(event) => event.stopPropagation()}>
                <input
                  type="checkbox"
                  checked={selected.includes(p.id)}
                  onchange={(e) =>
                    (selected = e.currentTarget.checked
                      ? [...selected, p.id]
                      : selected.filter((x) => x !== p.id))}
                />
              </td>{/if}<td title={p.name}>
              <b>{p.name}</b>{#if p.id === "default-all"}
                <span class="chip">{t("默认", "Default")}</span>{/if}
            </td><td>{p.description || "—"}</td><td>
              {(p.includedSubgroups || []).join(", ") || "—"}
            </td><td>{p.lateAfterMinutes}</td><td>
              {p.returnWindowMinutes}
            </td><td><i class:inactive={!p.active} class="status-dot"></i></td>
          </tr>{:else}<tr>
            <td colspan={bulkMode ? 7 : 6} class="empty">
              {t("没有配置", "No profiles")}
            </td>
          </tr>{/each}
      </tbody>
    </table>
  </div>
  <details class="wildcard-help">
    <summary>{t("通配符帮助", "Wildcard help")}</summary>
    <p>
      {t(
        "仅支持 * 匹配任意字符。问号、方括号、大括号和 ** 不支持。留空分组表示不限制。",
        "Only * is supported and matches any characters. ?, square brackets, braces, and ** are invalid. Empty subgroup list means no restriction.",
      )}
    </p>
  </details>
</section>
{#if open}<div class="overlay">
    <section class="slide-panel">
      <button class="close" onclick={() => (open = false)}>×</button>
      <h2>
        {editing
          ? t("编辑考勤配置", "Edit profile")
          : t("新增考勤配置", "Add profile")}
      </h2>
      <label>
        {t("名称（必填）", "Name (required)")}<input
          bind:value={draft.name}
          disabled={editing?.id === "default-all"}
        />
      </label><label>
        {t("描述", "Description")}<textarea bind:value={draft.description}
        ></textarea>
      </label><label>
        {t(
          "纳入分组（逗号或换行；* 通配符）",
          "Included subgroups (comma or newline; * wildcard)",
        )}<textarea bind:value={draft.includedSubgroups}></textarea>
      </label><label>
        {t(
          "迟到阈值（分钟，正整数）",
          "Late threshold (minutes, positive integer)",
        )}<input type="number" min="1" bind:value={draft.lateAfterMinutes} />
      </label><label>
        {t(
          "返回窗口（分钟，正整数）",
          "Return window (minutes, positive integer)",
        )}<input type="number" min="1" bind:value={draft.returnWindowMinutes} />
      </label><label>
        {t(
          "会议 ID（精确匹配，逗号或换行）",
          "Meeting IDs (exact, comma or newline)",
        )}<textarea bind:value={draft.meetingIds}></textarea>
      </label><label>
        {t(
          "主题关键词（逗号分隔）",
          "Topic keywords (comma separated)",
        )}<textarea bind:value={draft.topicKeywords}></textarea>
      </label><label>
        {t(
          "分组排序（每行一个，可用 *）",
          "Group sort order (one per line, * allowed)",
        )}<textarea bind:value={draft.groupSortOrder}></textarea>
      </label><label class="check">
        <input type="checkbox" bind:checked={draft.active} />{t(
          "启用",
          "Active",
        )}
      </label>{#if error}<p class="error">{error}</p>{/if}<FormActions
        submitLabel={t("保存", "Save")}
        cancelLabel={t("取消", "Cancel")}
        onSubmit={save}
        onCancel={() => (open = false)}
      />{#if editing && editing.id !== "default-all"}<button
          class="danger"
          onclick={removeEditing}
        >
          {t("删除", "Delete")}
        </button>{/if}
    </section>
  </div>{/if}
{#if preview}<div class="overlay">
    <section class="slide-panel import-panel">
      <h2 bind:this={previewHeading} tabindex="-1">{t("配置导入预览", "Profile import preview")}</h2>
      <ImportPreviewSummary {changes} {t} />
      <div class="toolbar">
        <label>
          {t("批量启用", "Bulk active")}
          <select bind:value={bulkActive}>
            <option value="">—</option><option value="true">
              {t("启用", "Active")}
            </option><option value="false">{t("停用", "Inactive")}</option>
          </select>
        </label><label>
          {t("迟到分钟", "Late min")}
          <input
            type="number"
            min="1"
            bind:value={bulkLate}
            placeholder={t("保持不变", "Keep")}
          />
        </label><label>
          {t("返回分钟", "Return min")}
          <input
            type="number"
            min="1"
            bind:value={bulkReturn}
            placeholder={t("保持不变", "Keep")}
          />
        </label><button disabled={!selected.length} onclick={bulkEdit}>
          {t("应用批量编辑", "Apply bulk edit")}
        </button>
      </div>
      <p class="import-help">{t("文件必须是 JSON 数组，每个对象至少包含 id 和 name。按 id 新增或更新；未出现在文件中的现有配置会保留。", "The file must be a JSON array. Each object needs id and name. Profiles are added or updated by id; existing profiles not in the file are kept.")}</p>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{t("行", "Row")}</th><th>{t("配置", "Profile")}</th><th>{t("当前数据", "Current")}</th><th>{t("导入后", "After Import")}</th><th>
                {t("操作", "Action")}
              </th><th>{t("原因", "Reason")}</th>
            </tr>
          </thead><tbody>
            {#each changes as c}<tr class:rejected={["reject", "rejected"].includes(String(c.action || c.status).toLowerCase()) || c.rejected === true}>
                <td>{c.row}</td><td>{c.id}</td><td>{#each fields(c.before) as [key, value]}<div><b>{key}:</b> {display(value)}</div>{/each}</td><td>{#each fields(c.after, c.action === "update" || c.action === "updated" ? c.before : null) as [key, value]}<div><b>{key}:</b> {display(value)}</div>{/each}</td><td>{importActionLabel(c.action || c.status)}</td><td>
                  {#each fields(c.after, c.action === "update" || c.action === "updated" ? c.before : null) as [key, value]}<div><b>{key}:</b> {display(value)}</div>{/each}{c.reason || c.notes || ""}
                </td>
              </tr>{/each}
          </tbody>
        </table>
      </div>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        <button class="primary" onclick={commit} disabled={!applicableCount}>
          {t(`确认导入 ${applicableCount} 条`, `Commit ${applicableCount} changes`)}
        </button><button onclick={() => (preview = null)}>
          {t("取消", "Cancel")}
        </button>
      </div>
    </section>
  </div>{/if}

<style>
  .profile-workspace {
    padding: 18px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--surface);
  }
  .rejected { background: #fff0f0; color: #8a1c1c; }
  .import-help { color: #596273; font-size: 12px; line-height: 1.5; }
  .profile-workspace .panel-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .profile-workspace .panel-title h2 {
    margin: 0;
  }
  .actions {
    display: flex;
    gap: 7px;
    align-items: center;
    flex-wrap: wrap;
  }
  .count-pill {
    padding: 7px 13px;
    border-radius: 20px;
    background: #f1f3f7;
    color: #516078;
    font-size: 12px;
    font-weight: 650;
    white-space: nowrap;
  }
  .wildcard-help {
    margin-top: 14px;
    color: #596273;
    font-size: 12px;
  }
  .profile-workspace input:not([type="checkbox"]),
  .profile-workspace select {
    padding: 8px;
    border: 1px solid #dce0e7;
    border-radius: 6px;
  }
  .profile-workspace .toolbar {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .profile-workspace .table-wrap {
    max-height: calc(100vh - 330px);
    overflow: auto;
    border: 1px solid var(--line);
    border-radius: 10px;
  }
  .profile-workspace table {
    width: 100%;
    min-width: 760px;
    table-layout: fixed;
    border-collapse: collapse;
    font-size: 13px;
  }
  .profile-workspace th {
    position: sticky;
    top: 0;
    z-index: 2;
    padding: 11px 14px;
    background: #f3f4f7;
    color: #42516a;
    text-align: left;
  }
  .profile-workspace td {
    height: 52px;
    padding: 10px 14px;
    border-top: 1px solid #e1e5ed;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .profile-workspace th:nth-child(1), .profile-workspace td:nth-child(1) { width: 20%; }
  .profile-workspace th:nth-child(2), .profile-workspace td:nth-child(2) { width: 31%; }
  .profile-workspace th:nth-child(3), .profile-workspace td:nth-child(3) { width: 20%; }
  .profile-workspace th:nth-child(4), .profile-workspace td:nth-child(4) { width: 10%; }
  .profile-workspace th:nth-child(5), .profile-workspace td:nth-child(5) { width: 12%; }
  .profile-workspace th:nth-child(6), .profile-workspace td:nth-child(6) { width: 7%; }
  .profile-workspace .data-row {
    cursor: pointer;
  }
  .profile-workspace .data-row:hover {
    background: #f7f9fc;
  }
  .status-dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #138447;
  }
  .status-dot.inactive {
    background: #aeb5c0;
  }
  .slide-panel textarea {
    min-height: 56px;
  }
</style>
