<script lang="ts">
  import {
    meeting,
    record,
    recordEvents,
    stateValue,
    configs,
    language,
    mode,
    page,
    recordCounts,
    recordModes,
    meetings,
    settings,
    zoom,
    connected,
    toastMessage,
    selectedRecords,
    configTab,
    recordImportPreview,
    stateRevision,
    configRevision,
    stagedState,
  } from "../lib/stores";
  import {
    get,
    refresh,
    loadConfig,
    saveConfig,
    saveSettings,
    download,
    downloadBackup,
    exportRecords,
    exportOne,
    exportAll,
    deleteRecords,
    previewRecordImport,
    commitRecordImport,
    loadExampleData,
    addConfig,
    editConfig,
    removeConfig,
    loadState,
    loadRecordEvents,
    currentUsers,
    saveRecordState,
    captureManual,
    captureHands,
    applyOverrides,
    saveDecision,
    selectProfile,
    profileGuess,
    resetOverrides,
    switchMode,
    zoomSignIn,
    zoomSignOut,
    zoomAction,
    importBackup,
    chooseLanguage,
  } from "../lib/api";
  import { t, dateTime, timeOnly } from "../lib/ui";
  import { getLiveStatusLabel } from "../lib/session-labels";
  import { formatMinutes } from "../lib/formatting";
  import RecordActivity from "./../lib/RecordActivity.svelte";
  import NameMatchingPanel from "./../lib/NameMatchingPanel.svelte";

  let recordImportInput: HTMLInputElement;
  let recordSearch = "";
</script>

<section class="panel records-panel">
  <div class="records-heading">
    <div>
      <h2>{t("所有会议", "All meetings")}</h2>
      <p>{$meetings.length} {t("条会议记录", "meeting records")}</p>
    </div>
  </div>
  <div class="records-toolbar">
    <input
      aria-label={t("按日期搜索", "Search by date")}
      placeholder={t("按日期或主题搜索", "Search date or topic")}
      bind:value={recordSearch}
    /><button onclick={() => recordImportInput.click()}>
      {t("导入会议记录", "Import records")}
    </button><input
      bind:this={recordImportInput}
      type="file"
      accept=".json,application/json"
      hidden
      onchange={previewRecordImport}
    /><button onclick={() => exportRecords($selectedRecords)}>
      {$selectedRecords.length
        ? t("导出所选", "Export selected")
        : t("导出全部", "Export all")}
    </button><button
      class="danger"
      disabled={!$selectedRecords.length}
      onclick={() => deleteRecords($selectedRecords)}
    >
      {t("删除所选", "Delete selected")}
    </button>
  </div>
  <div class="table-wrap records-table">
    <table>
      <thead>
        <tr>
          <th>
            <input
              type="checkbox"
              aria-label={t("全选", "Select all")}
              checked={$meetings.length > 0 &&
                $selectedRecords.length === $meetings.length}
              onchange={(e) =>
                ($selectedRecords = e.currentTarget.checked
                  ? $meetings.map((m) => m.instanceId)
                  : [])}
            />
          </th><th>{t("会议", "Meeting")}</th><th>
            {t("开始时间", "Started")}
          </th><th>{t("结束时间", "Ended")}</th><th>{t("模式", "Mode")}</th><th>
            {t("出席人数", "# Attended")}
          </th><th>{t("状态", "Status")}</th>
        </tr>
      </thead><tbody>
        {#each $meetings.filter((m) => !recordSearch || `${new Date(m.startedAt).toISOString()} ${m.topic || ""} ${m.meetingNumber || ""}`
              .toLowerCase()
              .includes(recordSearch.toLowerCase())) as m}<tr
            class="record-list-row"
          >
            <td>
              <input
                type="checkbox"
                checked={$selectedRecords.includes(m.instanceId)}
                onchange={(e) =>
                  ($selectedRecords = e.currentTarget.checked
                    ? [...$selectedRecords, m.instanceId]
                    : $selectedRecords.filter((id) => id !== m.instanceId))}
              />
            </td><td>
              <button
                class="record-open"
                title={m.topic || m.meetingNumber}
                onclick={async () => {
                  $meeting = m;
                  $record = await get(`/api/meetings/${m.instanceId}/record`);
                  await loadState();
                  await loadRecordEvents();
                  $mode = $record.mode || "generic";
                  $page = "records";
                }}
              >
                <b>{m.topic || m.meetingNumber}</b><small>
                  {m.meetingNumber}
                </small>
              </button>
            </td><td>{m.startedAt ? dateTime(m.startedAt) : "—"}</td><td>
              {m.endedAt ? dateTime(m.endedAt) : "—"}
            </td><td>
              <span class="badge">
                {($recordModes[m.instanceId] || m.mode) === "roster"
                  ? t("花名册", "Roster")
                  : t("通用", "Generic")}
              </span>
            </td><td>{$recordCounts[m.instanceId] ?? "—"}</td><td>
              <span class="badge" class:live={m.source !== "imported"}>
                {m.source === "imported"
                  ? t("导入", "Imported")
                  : t("实时", "Live")}
              </span>
              <span class="badge" class:ended={!!m.endedAt}>
                {m.endedAt ? t("已结束", "Ended") : t("进行中", "In progress")}
              </span>
              <span class="badge" class:downloaded={!!m.exportedAt}>
                {m.exportedAt
                  ? t("已下载", "Downloaded")
                  : t("未下载", "Not downloaded")}
              </span>
            </td>
          </tr>{:else}<tr>
            <td colspan="7" class="empty">
              {t("没有符合条件的会议记录", "No matching meeting records")}
            </td>
          </tr>{/each}
      </tbody>
    </table>
  </div>
</section>
{#if $meeting}<section class="panel record-detail">
    <div class="detail-heading">
      <div>
        <h2 title={$meeting.topic || $meeting.meetingNumber}>
          {$meeting.topic || $meeting.meetingNumber}
        </h2>
        <p>
          {$meeting.meetingNumber} · {$record.summary?.total ?? 0}
          {t("位参与者", "participants")}
        </p>
      </div>
      <div class="actions">
        <button onclick={() => exportOne("json")}>
          {t("导出 JSON", "Export JSON")}
        </button><button onclick={() => exportOne("summary")}>
          {t("导出考勤 CSV", "Export Attendance CSV")}
        </button><button onclick={() => exportOne("manual")}>
          {t("导出手动记录 CSV", "Export Manual Records CSV")}
        </button><button class="primary" onclick={exportAll}>
          {t("导出全部", "Export all")}
        </button>
      </div>
    </div>
    <RecordActivity
      meeting={$meeting}
      record={$record}
      state={$stateValue}
      events={$recordEvents}
      users={currentUsers()}
      language={$language}
      onCaptureManual={captureManual}
      onCaptureHands={captureHands}
    >
      <NameMatchingPanel
        slot="matching"
        rows={$record.rows || []}
        decisions={$stateValue.reconciliationDecisions || []}
        roster={$configs.roster || []}
        aliases={$configs.aliases || []}
        nameChanges={$record.record?.nameChangeEvents || []}
        language={$language}
        readOnly={true}
      />
    </RecordActivity>
  </section>{/if}
{#if $recordImportPreview}<div class="overlay">
    <section class="slide-panel import-panel">
      <h2>{t("会议记录导入预览", "Meeting record import preview")}</h2>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{t("状态", "Action")}</th><th>{t("会议", "Meeting")}</th><th>
                {t("说明", "Reason")}
              </th>
            </tr>
          </thead><tbody>
            {#each $recordImportPreview.plans || [] as p}<tr>
                <td>{p.action}</td><td>
                  {p.value?.meeting?.topic ||
                    p.value?.meeting?.meetingNumber ||
                    "—"}
                </td><td>{p.reason}</td>
              </tr>{/each}
          </tbody>
        </table>
      </div>
      <div class="actions">
        <button
          class="primary"
          disabled={($recordImportPreview.plans || []).some(
            (p: any) => p.action === "reject",
          )}
          onclick={commitRecordImport}
        >
          {t("确认导入", "Confirm import")}
        </button><button onclick={() => ($recordImportPreview = null)}>
          {t("取消", "Cancel")}
        </button>
      </div>
    </section>
  </div>{/if}
