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
  import ProfileOverrideForm from "./../lib/ProfileOverrideForm.svelte";
  import RecordActivity from "./../lib/RecordActivity.svelte";
  import NameMatchingPanel from "./../lib/NameMatchingPanel.svelte";

  let rows: any[] = [];
  let matchingCount = 0;
  let filter = "";
  let activeFilter = "";
  $: rows = ($record.rows || []).filter((r: any) => {
    if (
      filter &&
      !`${r.canonicalName || ""} ${r.zoomName || ""}`
        .toLowerCase()
        .includes(filter.toLowerCase())
    )
      return false;
    if (!activeFilter) return true;
    const tags = r.tags || [];
    return (
      tags.includes(activeFilter) ||
      (
        {
          seen: r.hasBeenSeen,
          full_present: r.isCurrentlyPresent && !r.wasLate,
          late: r.wasLate,
          returned: r.currentStatus === "returnedAfterLongAbsence",
          left_early: r.currentStatus === "leftEarly",
          missing: r.currentStatus === "missing",
          ignored: r.matchStatus === "guest" || r.matchStatus === "unmatched",
        } as any
      )[activeFilter]
    );
  });
  $: matchingCount =
    ($record.rows || []).filter((r: any) => r.matchStatus === "unmatched")
      .length +
    ($record.record?.nameChangeEvents || []).filter(
      (e: any) =>
        !($stateValue.reconciliationDecisions || []).some(
          (d: any) =>
            d.sessionParticipantId === e.sessionParticipantId &&
            Date.parse(d.createdAt || "") >= Date.parse(e.timestamp || ""),
        ),
    ).length;
  const stats = [
    ["seen", "Seen", "到过"],
    ["full_present", "Full Present", "全程到场"],
    ["late", "Late", "迟到"],
    ["returned", "Returned", "返回"],
    ["left_early", "Left Early", "早退"],
    ["missing", "Absent", "缺席"],
    ["ignored", "Ignored", "不计入"],
    ["total", "Total", "总计"],
  ];
  function summaryRows(key: string) {
    activeFilter = key === "total" || activeFilter === key ? "" : key;
  }
</script>

{#if $meeting}<section class="meet-head">
    <div>
      <b>{$meeting.topic || t("未命名会议", "Untitled meeting")}</b><small>
        {$meeting.meetingNumber} · {dateTime($meeting.startedAt)}
      </small>
    </div>
    <div class="switch">
      <button
        class:chosen={$mode === "generic"}
        onclick={() => switchMode("generic")}
      >
        {t("通用", "Generic")}
      </button><button
        class:chosen={$mode === "roster"}
        onclick={() => switchMode("roster")}
      >
        {t("花名册", "Roster")}
      </button>
    </div>
  </section>
  <section class="panel live-profile">
    <label>
      {t("考勤配置", "Attendance profile")}<select
        value={$stateValue.selectedProfileId ||
          profileGuess()?.id ||
          "default-all"}
        onchange={(e) => selectProfile(e.currentTarget.value)}
      >
        {#each ($configs.profiles || []).filter((p: any) => p.active) as p}<option
            value={p.id}
          >
            {p.name}
          </option>{/each}
      </select>
    </label>
    <div class="muted">
      {t("迟到", "Late")}: {$record.record?.lateAfterMinutes ??
        profileGuess()?.lateAfterMinutes ??
        10}
      {t("分钟", "min")} · {t("返回", "Return")}: {$record.record
        ?.returnWindowMinutes ??
        profileGuess()?.returnWindowMinutes ??
        8}
      {t("分钟", "min")} · {t("预期小组", "Expected groups")}: {(
        profileGuess()?.includedSubgroups || []
      ).join(", ") || t("全部", "All")}
    </div>
    {#key $stateValue.selectedProfileId}<ProfileOverrideForm
        initialLate={$stateValue.lateAfterMinutesOverride ?? null}
        initialReturn={$stateValue.returnWindowMinutesOverride ?? null}
        language={$language}
        onApply={applyOverrides}
      />{/key}
  </section>
  <section class="stats">
    {#each stats as [key, en, zh]}<button
        class:chosen={activeFilter === key}
        onclick={() => {
          if (key === "ignored" && $mode === "roster") {
            $page = "matching";
            return;
          }
          summaryRows(key);
        }}
      >
        <small>{t(zh, en)}</small><strong>
          {key === "total"
            ? ($record.summary?.total ?? 0)
            : ($record.summary?.[
                key === "full_present"
                  ? "fullPresent"
                  : key === "left_early"
                    ? "leftEarly"
                    : key
              ] ?? 0)}
        </strong>
      </button>{/each}
  </section>
  <section class="toolbar">
    <input
      placeholder={t("搜索姓名…", "Search names…")}
      bind:value={filter}
    /><span>{rows.length} {t("位参与者", "participants")}</span>
  </section>
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>{t("姓名", "Name")}</th><th>{t("Zoom 名称", "Zoom name")}</th><th>
            {t("状态", "Status")}
          </th><th>{t("位置", "Location")}</th><th>{t("小组", "Group")}</th><th>
            {t("首次加入", "First join")}
          </th><th>{t("最近进出", "Last seen")}</th><th>
            {t("分钟", "Minutes")}
          </th>{#if $meeting.isHost}<th>{t("等候室分钟", "Waiting")}</th>{/if}
        </tr>
      </thead><tbody>
        {#each rows as r}<tr>
            <td><b>{r.canonicalName || r.zoomName}</b></td><td>
              {r.zoomName}
            </td><td>
              <span class="chip">
                {getLiveStatusLabel(r.currentStatus, $language as any)}
              </span>
            </td><td>
              {r.currentLocation === "meeting"
                ? t("会议中", "In meeting")
                : r.currentLocation === "waiting-room"
                  ? t("等候室", "Waiting room")
                  : "—"}
            </td><td>{r.group || "—"}</td><td>
              {r.firstJoinAt ? timeOnly(r.firstJoinAt) : "—"}
            </td><td>{r.lastSeenAt ? timeOnly(r.lastSeenAt) : "—"}</td><td>
              {formatMinutes(r.totalPresenceMinutes)}
            </td>{#if $meeting.isHost}<td>
                {Math.round(r.waitingRoomMinutes || 0)}
              </td>{/if}
          </tr>{:else}<tr>
            <td colspan="9" class="empty">
              {t("等待参与者事件…", "Waiting for participant events…")}
            </td>
          </tr>{/each}
      </tbody>
    </table>
  </div>
  <RecordActivity
    meeting={$meeting}
    record={$record}
    state={$stateValue}
    events={$recordEvents}
    users={currentUsers()}
    language={$language}
    live={true}
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
      onDecision={(d) => saveDecision(d)}
      onRevert={(id) => saveDecision(null, id)}
    />
  </RecordActivity>{:else}<section class="empty-card">
    <span>◉</span>
    <h2>{t("尚未加入会议", "No active meeting")}</h2>
    <p>
      {t(
        "连接后，会议记录会自动显示在这里。",
        "When you join a meeting, its attendance record will appear here.",
      )}
    </p>
  </section>{/if}
