<script lang="ts">
  import FormActions from "./common/FormActions.svelte";
  import ImportWorkflowPanel from "./common/ImportWorkflowPanel.svelte";
  import { confirmDialog } from "./ui";
  import { saveConfig } from "./api";
  export let language = "en";
  export let exportLanguage = "en";
  export let roster: any[] = [];
  export let aliases: any[] = [];
  export let rosterRevision = 0;
  export let aliasesRevision = 0;
  export let onSaved: () => void = () => {};
  export let activeTab: "roster" | "aliases" = "roster";
  $: tab = activeTab;
  let bulkMode = false;
  let filters: Record<string, string> = {};
  let sortKey = "personId";
  let descending = false;
  let selected: (number | string)[] = [];
  let form: any = null;
  let editing = -1;
  let error = "";
  let bulkOpen = false,
    bulkGroup = "",
    bulkActive = "",
    bulkPerson = "",
    bulkNotes = "";
  let importText = "";
  let preview: any = null;
  let importType = "";
  let pasteOpen = false;
  let importBytes: number[] | null = null;
  let file: HTMLInputElement;
  const zh = () => language === "zh-CN";
  const t = (c: string, e: string) => (zh() ? c : e);
  function importActionLabel(action: unknown) {
    switch (String(action).toLowerCase()) {
      case "add": case "added": case "create": case "created": return t("新增", "Add");
      case "update": case "updated": return t("更新", "Update");
      case "remove": case "removed": case "delete": case "deleted": return t("删除", "Delete");
      default: return t("未知", "Unknown");
    }
  }
  const names = (r: any) =>
    `${r.personId} ${r.canonicalName || ""} ${r.pinyin || ""}`;
  const rowKey = (r: any) =>
    tab === "roster" ? r.personId : (r.id ?? (r.zoomName || r.alias));
  $: data = (tab === "roster" ? roster : aliases)
    .filter((x: any) => {
      const values =
        tab === "roster"
          ? [
              x.personId,
              x.canonicalName,
              x.pinyin,
              x.group,
              x.active === false ? "no 否 停用" : "yes 是 启用",
            ]
          : [
              x.zoomName || x.alias,
              x.personId,
              roster.find((p: any) => p.personId === x.personId)?.canonicalName,
              x.notes,
              x.lastUsedAt || "Unused",
            ];
      return values.every(
        (v: any, i: number) =>
          !filters[String(i)] ||
          String(v ?? "")
            .toLowerCase()
            .includes(filters[String(i)].toLowerCase()),
      );
    })
    .sort(
      (a: any, b: any) =>
        String(a[sortKey] ?? "").localeCompare(
          String(b[sortKey] ?? ""),
          undefined,
          { numeric: true },
        ) * (descending ? -1 : 1),
    );
  async function save(key: string, value: any, revision: number) {
    void revision;
    await saveConfig(key, value);
    onSaved();
  }
  function open(item?: any, index = -1) {
    editing = index;
    form = item
      ? { ...item }
      : {
          ...(tab === "roster"
            ? { canonicalName: "", pinyin: "", group: "", active: true }
            : { zoomName: "", personId: "", notes: "" }),
        };
    error = "";
  }
  async function submit() {
    try {
      const key = tab;
      const rows = [...(key === "roster" ? roster : aliases)];
      if (key === "roster") {
        form.canonicalName = form.canonicalName.trim();
        if (!form.canonicalName)
          throw Error(t("姓名不能为空", "Canonical name is required"));
        if (!form.pinyin.trim())
          form.pinyin = (
            await (
              await fetch("/api/names/pinyin", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ name: form.canonicalName }),
              })
            ).json()
          ).pinyin;
        form.personId =
          editing < 0
            ? Math.max(0, ...rows.map((x: any) => x.personId || 0)) + 1
            : rows[editing].personId;
        form.updatedAt = new Date().toISOString();
      } else {
        form.zoomName = form.zoomName.trim();
        if (!form.zoomName)
          throw Error(t("昵称不能为空", "Nickname is required"));
        if (
          !roster.some((x: any) => Number(x.personId) === Number(form.personId))
        )
          throw Error(t("请选择花名册人员", "Choose a roster person"));
        if (
          aliases.some(
            (x: any, i: number) =>
              i !== editing &&
              (x.zoomName || x.alias).toLowerCase() ===
                form.zoomName.toLowerCase(),
          )
        )
          throw Error(t("昵称已存在", "Nickname already exists"));
        form.personId = Number(form.personId);
      }
      if (editing < 0) rows.push(form);
      else rows[editing] = form;
      await save(
        key,
        rows,
        key === "roster" ? rosterRevision : aliasesRevision,
      );
      form = null;
    } catch (e: any) {
      error = e.message;
    }
  }
  async function remove(item: any) {
    const isRoster = tab === "roster";
    const impact = isRoster
      ? aliases.filter((a: any) => a.personId === item.personId).length
      : 0;
    if (
      !(await confirmDialog({
        title: t("确认删除", "Confirm deletion"),
        message: isRoster
          ? t(
              `删除 ${item.canonicalName} 并同时删除 ${impact} 个昵称？`,
              `Delete ${item.canonicalName} and its ${impact} aliases?`,
            )
          : t(
              `删除昵称 ${item.zoomName}？`,
              `Delete nickname ${item.zoomName}?`,
            ),
        destructive: true,
      }))
    )
      return;
    try {
      if (isRoster)
        await save(
          "roster",
          roster.filter((x: any) => x.personId !== item.personId),
          rosterRevision,
        );
      else
        await save(
          "aliases",
          aliases.filter((x: any) => x !== item),
          aliasesRevision,
        );
    } catch (e: any) {
      error = e.message;
    }
  }
  function bulkEdit() {
    bulkGroup = "";
    bulkActive = "";
    bulkPerson = "";
    bulkNotes = "";
    bulkOpen = true;
  }
  async function applyBulk() {
    let rows = (tab === "roster" ? roster : aliases).map((x: any) => {
      if (!selected.includes(rowKey(x))) return x;
      if (tab === "roster")
        return {
          ...x,
          ...(bulkGroup.trim() ? { group: bulkGroup.trim() } : {}),
          ...(bulkActive ? { active: bulkActive === "true" } : {}),
        };
      return {
        ...x,
        ...(bulkPerson ? { personId: Number(bulkPerson) } : {}),
        ...(bulkNotes.trim() ? { notes: bulkNotes.trim() } : {}),
      };
    });
    try {
      await save(
        tab,
        rows,
        tab === "roster" ? rosterRevision : aliasesRevision,
      );
      bulkOpen = false;
    } catch (e: any) {
      error = e.message;
    }
  }
  async function bulkDelete() {
    if (
      !selected.length ||
      !(await confirmDialog({
        title: t("确认删除", "Confirm deletion"),
        message: t(
          `删除所选 ${selected.length} 项？`,
          `Delete ${selected.length} selected items?`,
        ),
        destructive: true,
      }))
    )
      return;
    try {
      if (tab === "roster")
        await save(
          "roster",
          roster.filter((x: any) => !selected.includes(x.personId)),
          rosterRevision,
        );
      else
        await save(
          "aliases",
          aliases.filter((x: any) => !selected.includes(rowKey(x))),
          aliasesRevision,
        );
      selected = [];
    } catch (e: any) {
      error = e.message;
    }
  }
  function exportCsv(ids: (number | string)[] = []) {
    const lang = exportLanguage === "zh" ? "zh" : "en";
    const url = `/api/config/${tab}/export?language=${lang}${ids.length ? `&selected=${encodeURIComponent(ids.join("|"))}` : ""}`;
    const a = document.createElement("a");
    a.href = url;
    a.download = "";
    a.click();
  }
  async function previewImport() {
    try {
      const r = await fetch(`/api/config/${importType}/import/preview`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(
          importBytes ? { csvBytes: importBytes } : { csv: importText },
        ),
      });
      preview = await r.json();
      if (!r.ok) throw Error(preview.message || "Preview failed");
    } catch (e: any) {
      error = e.message;
    }
  }
  async function commitImport() {
    try {
      const r = await fetch(`/api/config/${importType}/import/commit`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          revision: preview.revision,
          value: preview.value,
          changes: preview.changes,
        }),
      });
      if (!r.ok) throw Error((await r.json()).message || "Import failed");
      preview = null;
      importText = "";
      importBytes = null;
      pasteOpen = false;
      onSaved();
    } catch (e: any) {
      error = e.message;
    }
  }
  async function readFile(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (f) {
      importBytes = Array.from(new Uint8Array(await f.arrayBuffer()));
      importText = new TextDecoder("utf-8").decode(new Uint8Array(importBytes));
      pasteOpen = true;
    }
    (e.target as HTMLInputElement).value = "";
  }
</script>

<section class="panel config-workspace">
  <div class="panel-title">
    <h2>
      {tab === "roster"
        ? t("花名册", "Roster")
        : t("名称匹配", "Name Matching")}
    </h2>
    <div class="actions">
      <button class="primary" onclick={() => open()}>
        ＋ {tab === "roster"
          ? t("添加人员", "Add Person")
          : t("添加匹配", "Add Match")}
      </button><button
        onclick={() => {
          importType = tab;
          importText = "";
          importBytes = null;
          preview = null;
          pasteOpen = true;
        }}
      >
        {t("导入", "Import")}
      </button><button onclick={() => exportCsv()}>
        {t("导出", "Export")}
      </button><span class="count-pill">
        {tab === "roster" ? roster.length : aliases.length}
        {t("条记录", "records")}
      </span><button
        class="primary"
        onclick={() => {
          bulkMode = true;
        }}
      >
        {t("批量管理", "Bulk Select")}
      </button><input
        bind:this={file}
        type="file"
        accept=".csv,text/csv"
        hidden
        onchange={readFile}
      />
    </div>
  </div>
  {#if bulkMode}<div class="toolbar">
      <button onclick={() => (selected = data.map((x: any) => rowKey(x)))}>
        {t("全选筛选结果", "Select all filtered")}
      </button><button onclick={bulkEdit} disabled={!selected.length}>
        {t("批量编辑", "Bulk edit")}
      </button><button
        class="danger"
        onclick={bulkDelete}
        disabled={!selected.length}
      >
        {t("批量删除", "Bulk delete")}
      </button><button
        onclick={() => exportCsv(selected)}
        disabled={!selected.length}
      >
        {t("导出所选", "Export selected")}
      </button><button
        onclick={() => {
          selected = [];
          bulkMode = false;
        }}
      >
        {t("退出批量管理", "Exit Bulk Select")}
      </button><span>
        {t(`已选择 ${selected.length} 项`, `${selected.length} selected`)}
      </span>
    </div>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          {#if bulkMode}<th
            ></th>{/if}{#each tab === "roster" ? ["ID", "Canonical Name", "Pinyin", "Group", "Active"] : ["Nickname", "ID", "Canonical Name", "Notes", "Last Used"] as h, i}<th>
              <button
                class="sort"
                onclick={() => {
                  sortKey =
                    tab === "roster"
                      ? [
                          "personId",
                          "canonicalName",
                          "pinyin",
                          "group",
                          "active",
                        ][i]
                      : [
                          "zoomName",
                          "personId",
                          "personId",
                          "notes",
                          "lastUsedAt",
                        ][i];
                  descending = !descending;
                }}
              >
                {t(
                  {
                    ID: "编号",
                    "Canonical Name": "姓名",
                    Pinyin: "拼音",
                    Group: "分组",
                    Active: "启用",
                    Nickname: "昵称",
                    Notes: "备注",
                    "Last Used": "最后使用",
                  }[h] || h,
                  h,
                ).toUpperCase()}
              </button><input
                class="col-filter"
                placeholder={t(
                  (tab === "roster"
                    ? ["筛选 ID", "筛选姓名", "筛选拼音", "筛选分组", "是/否"]
                    : ["筛选昵称", "筛选 ID", "筛选姓名", "筛选备注", "筛选最后使用"])[i],
                  (tab === "roster"
                    ? ["Filter ID", "Filter name", "Filter pinyin", "Filter group", "yes/no"]
                    : ["Filter nickname", "Filter ID", "Filter name", "Filter notes", "Filter last used"])[i],
                )}
                bind:value={filters[String(i)]}
              />
            </th>{/each}
        </tr>
      </thead><tbody>
        {#each data as item}<tr
            class="data-row"
            onclick={() =>
              open(item, (tab === "roster" ? roster : aliases).indexOf(item))}
            onkeydown={(event) =>
              event.key === "Enter" &&
              open(item, (tab === "roster" ? roster : aliases).indexOf(item))}
            tabindex="0"
          >
            {#if bulkMode}<td onclick={(event) => event.stopPropagation()}>
                <input
                  type="checkbox"
                  checked={selected.includes(rowKey(item))}
                  onchange={(e) =>
                    (selected = e.currentTarget.checked
                      ? [...selected, rowKey(item)]
                      : selected.filter((x) => x !== rowKey(item)))}
                />
              </td>{/if}{#if tab === "roster"}<td>{item.personId}</td><td
                title={item.canonicalName}
              >
                <b>{item.canonicalName}</b>
              </td><td>{item.pinyin || "—"}</td><td>
                {item.group || t("未分组", "Ungrouped")}
              </td><td>
                <i class:inactive={item.active === false} class="status-dot"
                ></i>
              </td>{:else}<td title={item.zoomName || item.alias}>
                {item.zoomName || item.alias}
              </td><td>{item.personId}</td><td>
                <b>
                  {roster.find((p: any) => p.personId === item.personId)
                    ?.canonicalName || "—"}
                </b>
              </td><td>{item.notes || "—"}</td><td>
                {item.lastUsedAt || t("未使用", "Unused")}
              </td>{/if}
          </tr>{:else}<tr>
            <td colspan={bulkMode ? 6 : 5} class="empty">
              {t("暂无数据", "No data")}
            </td>
          </tr>{/each}
      </tbody>
    </table>
  </div>
</section>
{#if form}<div class="overlay">
    <section class="slide-panel">
      <button class="close" onclick={() => (form = null)}>×</button>
      <h2>
        {editing < 0 ? t("新增", "Add") : t("编辑", "Edit")}
        {tab === "roster"
          ? t("花名册人员", "roster person")
          : t("名称匹配", "nickname mapping")}
      </h2>
      {#if tab === "roster"}{#if editing >= 0}<label>
            ID<input value={form.personId} readonly />
          </label>{/if}<label>
          {t("姓名", "Canonical name")} *<input
            bind:value={form.canonicalName}
          />
        </label><label>
          {t("拼音", "Pinyin")}<input
            bind:value={form.pinyin}
            placeholder={t("留空自动生成", "Auto-generated when blank")}
          />
        </label><label>
          {t("分组", "Group")}<input bind:value={form.group} />
        </label><label class="check">
          <input type="checkbox" bind:checked={form.active} />
          {t("启用", "Active")}
        </label>{:else}<label>
          {t("昵称", "Nickname")} *<input bind:value={form.zoomName} />
        </label><label>
          {t("对应人员", "Roster person")} *<select
            bind:value={form.personId}
          >
            <option value="">
              {t("选择人员", "Choose person")}
            </option>{#each roster as p}<option value={p.personId}>
                {p.personId} · {p.canonicalName} · {p.pinyin}
              </option>{/each}
          </select>
        </label><label>
          {t("备注", "Notes")}<textarea bind:value={form.notes}></textarea>
        </label>{/if}{#if error}<p class="error">{error}</p>{/if}<FormActions
        submitLabel={t("保存", "Save")}
        cancelLabel={t("取消", "Cancel")}
        onSubmit={submit}
        onCancel={() => (form = null)}
      />{#if editing >= 0}<button class="danger" onclick={() => remove(form)}>
          {t("删除", "Delete")}
        </button>{/if}
    </section>
  </div>{/if}
{#if bulkOpen}<div class="overlay">
    <section class="slide-panel">
      <h2>{t("批量编辑", "Bulk edit")} · {selected.length}</h2>
      {#if tab === "roster"}<label>
          {t("分组（留空保持不变）", "Group (blank keeps current)")}<input
            bind:value={bulkGroup}
          />
        </label><label>
          {t("启用状态", "Active status")}<select bind:value={bulkActive}>
            <option value="">{t("保持不变", "Keep unchanged")}</option><option
              value="true"
            >
              {t("启用", "Active")}
            </option><option value="false">{t("停用", "Inactive")}</option>
          </select>
        </label>{:else}<label>
          {t(
            "花名册人员 ID（留空保持不变）",
            "Roster person ID (blank keeps current)",
          )}<select bind:value={bulkPerson}>
            <option value="">
              {t("保持不变", "Keep unchanged")}
            </option>{#each roster as p}<option value={p.personId}>
                {p.personId} · {p.canonicalName}
              </option>{/each}
          </select>
        </label><label>
          {t("备注（留空保持不变）", "Notes (blank keeps current)")}<input
            bind:value={bulkNotes}
          />
        </label>{/if}<FormActions
        submitLabel={t("应用", "Apply")}
        cancelLabel={t("取消", "Cancel")}
        onSubmit={applyBulk}
        onCancel={() => (bulkOpen = false)}
      />
    </section>
  </div>{/if}
<ImportWorkflowPanel
  open={!preview && pasteOpen}
  title={`${t("导入", "Import")} · ${tab === "roster" ? t("花名册", "Roster") : t("名称匹配", "Name Matching")}`}
  closeLabel={t("关闭", "Close")}
  onClose={() => (pasteOpen = false)}
>
  <p>
    {t(
      "选择 CSV 文件，或粘贴 CSV 内容。请先预览变更再确认导入。",
      "Choose a CSV file or paste CSV content. Preview changes before confirming the import.",
    )}
  </p>
  <button onclick={() => file?.click()}>
    {t("选择 CSV 文件", "Choose CSV file")}
  </button><textarea
    bind:value={importText}
    oninput={() => (importBytes = null)}
    placeholder={t("或在此粘贴 CSV 内容…", "Or paste CSV content here…")}
  ></textarea>
  <div class="actions">
    <button class="primary" onclick={previewImport}>
      {t("生成预览", "Preview")}
    </button><button
      onclick={() => {
        importText = "";
        importBytes = null;
      }}
    >
      {t("清除", "Clear")}
    </button><button
      onclick={() => {
        importText = "";
        importBytes = null;
        pasteOpen = false;
      }}
    >
      {t("取消", "Cancel")}
    </button>
  </div>
</ImportWorkflowPanel>
{#if preview}<div class="overlay">
    <section class="slide-panel import-panel">
      <h2>{t("导入预览", "Import preview")}</h2>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              {#each [["row", "行", "Row"], ["action", "操作", "Action"], ["id", "编号", "ID"], ["before", "导入前", "Before"], ["after", "导入后", "After"], ["notes", "说明", "Notes"], ["aliasImpact", "关联昵称", "Alias impact"]] as [key, zh, en]}<th>
                  {t(zh, en)}
                </th>{/each}
            </tr>
          </thead><tbody>
            {#each preview.changes || [] as c}<tr>
                <td>{c.row}</td><td>{importActionLabel(c.action)}</td><td>{c.id}</td><td>
                  {JSON.stringify(c.before)}
                </td><td>{JSON.stringify(c.after)}</td><td>{c.notes}</td><td>
                  {c.aliasImpact || 0}
                </td>
              </tr>{/each}
          </tbody>
        </table>
      </div>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        <button class="primary" onclick={commitImport}>
          {t("确认导入", "Commit import")}
        </button><button onclick={() => (preview = null)}>
          {t("返回", "Back")}
        </button>
      </div>
    </section>
  </div>{/if}

<style>
  .config-workspace {
    padding: 18px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--surface);
  }
  .actions {
    display: flex;
    gap: 8px;
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
  .toolbar {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 10px 0;
  }
  .sort {
    border: 0;
    background: transparent;
    color: inherit;
    padding: 0;
    cursor: pointer;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }
  .col-filter {
    display: block;
    width: 100%;
    min-width: 60px;
    margin-top: 8px;
    border: 1px solid #e2e5ea;
    border-radius: 6px;
    padding: 7px;
    font-size: 11px;
  }
  .table-wrap {
    max-height: calc(100vh - 330px);
    overflow: auto;
    border: 1px solid var(--line);
    border-radius: 10px;
  }
  .table-wrap table {
    width: 100%;
    min-width: 760px;
    border-collapse: collapse;
    font-size: 13px;
  }
  .table-wrap th {
    position: sticky;
    top: 0;
    z-index: 2;
    padding: 11px 14px;
    background: #f3f4f7;
    color: #42516a;
    text-align: left;
  }
  .table-wrap th input {
    font-weight: 400;
  }
  .table-wrap td {
    height: 52px;
    padding: 10px 14px;
    border-top: 1px solid #e1e5ed;
  }
  .data-row {
    cursor: pointer;
  }
  .data-row:hover {
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
  .overlay {
    position: fixed;
    inset: 0;
    background: #17203366;
    display: flex;
    justify-content: flex-end;
    z-index: 5;
  }
  .slide-panel {
    height: 100%;
    width: min(440px, 95vw);
    overflow: auto;
    background: white;
    padding: 28px;
    box-shadow: -8px 0 28px #17203322;
  }
  .slide-panel h2 {
    font-size: 18px;
    margin: 12px 0 22px;
  }
  .slide-panel label {
    display: grid;
    gap: 6px;
    margin: 15px 0;
    font-size: 12px;
    color: #555e6b;
  }
  .slide-panel input:not([type="checkbox"]),
  .slide-panel textarea {
    width: 100%;
    padding: 10px;
    border: 1px solid #dce0e7;
    border-radius: 7px;
  }
  .slide-panel label.check {
    display: flex;
    align-items: center;
  }
  .close {
    float: right;
    border: 0;
    background: transparent;
    font-size: 24px;
  }
  .danger {
    color: #b83333;
  }
  .error {
    color: #b42318;
  }
</style>
