<script lang="ts">
  import { meeting, record, recordEvents, stateValue, configs, language, mode, requestedLiveSubtab } from "../lib/stores";
  import { captureManual, captureHands, applyOverrides, saveDecision, selectProfile, profileGuess, switchMode, startNewRecord } from "../lib/api";
  import { t, dateTime, confirmDialog } from "../lib/ui";
  import ProfileOverrideForm from "../lib/ProfileOverrideForm.svelte";
  import NameMatchingPanel from "../lib/NameMatchingPanel.svelte";
  import PrimaryTabs from "../lib/common/PrimaryTabs.svelte";
  import SecondaryTabs from "../lib/common/SecondaryTabs.svelte";
  import LiveAttendanceView from "./live/LiveAttendanceView.svelte";
  import { currentUsers } from "../lib/api";
  import SortableFilterStack from "../lib/common/SortableFilterStack.svelte";
  import { statusStyle } from "../lib/status-style";
  import { filterRowsByTag } from "../lib/record-tools";
  import { busyKeys, runBusy } from "../lib/busy";
  import { toast } from "../lib/toast";
  import RecordNameEditor from "../lib/common/RecordNameEditor.svelte";
  import { recordTitle } from "../lib/record-title";
  import { captureProof } from "../lib/proof";
  import Tooltip from "../lib/common/Tooltip.svelte";

  // currentUsers() replays $recordEvents; tie it to the store so the list stays current.
  const usersFor = (_events: unknown[]) => currentUsers();
  $: liveUsers = usersFor($recordEvents);
  let primary = "attendance";
  let secondary = "status";
  let activeFilter = "";
  const primaryTabs = [
    { id: "attendance", label: t("自动考勤", "Auto Attendance") },
    { id: "manual", label: t("手动考勤", "Manual Attendance") },
    { id: "entry", label: t("进出记录", "Entry/Exit") },
    { id: "hands", label: t("举手跟踪", "Raise Hand") },
  ];
  const attendanceTabs = [
    { id: "status", label: t("状态", "Status") },
    { id: "matching", label: t("昵称匹配", "Name Matching") },
    { id: "names", label: t("改名记录", "Name Changes") },
    { id: "info", label: t("设置与信息", "Settings & Info") },
  ];
  // Only warn once the live record says so (not while it is still loading).
  $: privilege = $record.privilege;
  $: unmatchedCount = ($record.rows || []).filter((r: any) => r.matchStatus === "unmatched").length;
  $: nameChanges = $record.record?.nameChangeEvents || [];
  const statusDefs = [["seen","到过","Seen","seen"],["full_present","全程","Full Present","fullPresent"],["late","迟到","Late","late"],["returned","返回","Returned","returned"],["left_early","早退","Left Early","leftEarly"],["missing","缺席","Absent","missing"],["ignored","不计入","Ignored","ignored"],["total","总计","Total","total"]];
  $: liveStatItems = statusDefs.map((d: any) => ({ id:d[0], label:t(d[1],d[2]), color:statusStyle(d[0]).color, count:d[0] === 'total' ? ($record.summary?.total ?? ($record.rows || []).length) : ($record.summary?.[d[3]] ?? filterRowsByTag($record.rows || [],d[0]).length) })).filter((item: any) => item.id === 'total' || item.count > 0);
  $: if (activeFilter && !liveStatItems.some((item: any) => item.id === activeFilter)) activeFilter = '';
  $: if ($requestedLiveSubtab) {
    primary = "attendance";
    secondary = $requestedLiveSubtab;
    requestedLiveSubtab.set(null);
  }
  $: liveLabel = $meeting?.status === "ended" ? t("已结束", "Ended") : t("进行中", "Live");
  function pickPrimary(id: string) { primary = id; if (id === "attendance") secondary = "status"; }
  function selectStat(id: string) {
    if (id === 'ignored') { primary = 'attendance'; secondary = 'matching'; activeFilter = ''; return; }
    activeFilter = activeFilter === id ? '' : id;
    primary = 'attendance'; secondary = 'status';
  }
  async function changeMode(next: string) {
    if (next === "roster" && !($configs.roster || []).length) { toast.warning(t("当前尚未导入花名册，不能切换到花名册模式。", "Roster not loaded yet, cannot switch to roster mode.")); return; }
    await runBusy("live-mode", async () => {
      try { if (await switchMode(next)) toast.success(next === "roster" ? t("已切换到花名册模式。", "Switched to roster mode.") : t("已切换到通用模式。", "Switched to generic mode.")); }
      catch (error) { toast.error(`${t("切换模式失败：", "Failed to switch mode: ")}${error instanceof Error ? error.message : String(error)}`); }
    });
  }
  async function changeProfile(id: string) {
    await runBusy("live-profile", async () => {
      try { if (await selectProfile(id)) toast.success(t("已更新考勤配置。", "Attendance profile updated.")); else toast.error(t("保存失败：", "Save failed:")); }
      catch (error) { toast.error(`${t("保存失败：", "Save failed:")}${error instanceof Error ? error.message : String(error)}`); }
    });
  }
  async function newRecord() {
    if (!(await confirmDialog({ title: t("开始新记录？", "Start a new record?"), message: t("当前记录将在此刻结束，并为同一会议开始一条新记录。所有在场的人都会作为“会前已在”计入新记录。", "The current record ends now and a new record starts for the same meeting. Everyone present is counted as already in the meeting in the new record.") }))) return;
    await runBusy("live-new-record", async () => {
      try { await startNewRecord(); toast.success(t("已开始新记录", "New record started")); }
      catch (error) { toast.error(`${t("开始新记录失败：", "Failed to start new record: ")}${error instanceof Error ? error.message : String(error)}`); }
    });
  }
  async function captureLiveProof() {
    await runBusy("live-proof", async () => {
      try { const result = await captureProof(); toast.success(t(`已截取 ${result.pages.length} 页`, `Captured ${result.pages.length} pages`)); }
      catch (error) { toast.error(error instanceof Error ? error.message : String(error)); }
    });
  }
  const proofNote = t("实验性功能，不保证可用：会把你自己的 Zoom 切换到图库视图逐页截图，然后恢复原视图。截图只保存在本机。", "Experimental, not guaranteed to work: switches your own Zoom view to gallery, captures each page, then restores your view. Images stay on this computer.");
</script>

{#if $meeting}
  <section class="meet-head compact-head">
    <div class="meeting-title"><b title={$meeting.topic || t("未命名会议", "Untitled meeting")}>{recordTitle({...$meeting, customName: $stateValue.customName}, $meeting.topic || t("未命名会议", "Untitled meeting")).primary}</b>{#if recordTitle({...$meeting, customName: $stateValue.customName}, $meeting.topic || t("未命名会议", "Untitled meeting")).secondary}<small>{recordTitle({...$meeting, customName: $stateValue.customName}, $meeting.topic || t("未命名会议", "Untitled meeting")).secondary}</small>{/if}<RecordNameEditor customName={$stateValue.customName || ""} /><span class="meeting-number">#{ $meeting.meetingNumber || "—" }</span><span>{dateTime($meeting.startedAt)}</span></div>
    <div class="head-controls">
      <span class="live-status"><i></i>{liveLabel}</span>
      <button disabled={$meeting.status === "ended" || !!$meeting.endedAt || $busyKeys.has("live-new-record")} onclick={newRecord}>{t("开始新记录", "Start new record")}</button>
      <Tooltip text={proofNote}><button disabled={$meeting.status === "ended" || !!$meeting.endedAt || $busyKeys.has("live-proof")} onclick={captureLiveProof}>{$busyKeys.has("live-proof") ? t("截图中…", "Capturing…") : t("截图证明", "Capture proof")}</button></Tooltip><small class="experimental-tag">{t("实验性", "Experimental")}</small>
      <label class="profile"><span>{t("配置", "Profile")}</span><select aria-label={t("考勤配置", "Attendance profile")} disabled={$busyKeys.has("live-profile")} value={$stateValue.selectedProfileId || profileGuess()?.id || "default-all"} onchange={(e) => changeProfile(e.currentTarget.value)}>
        {#each ($configs.profiles || []).filter((p: any) => p.active) as p}<option value={p.id}>{p.name}</option>{/each}
      </select></label>
      <details class="overrides"><summary>{t("阈值", "Overrides")}</summary><div class="override-pop"><p>{t("迟到", "Late")}: {$record.record?.lateAfterMinutes ?? profileGuess()?.lateAfterMinutes ?? 10} {t("分钟", "min")} · {t("返回", "Return")}: {$record.record?.returnWindowMinutes ?? profileGuess()?.returnWindowMinutes ?? 8} {t("分钟", "min")}</p><ProfileOverrideForm initialLate={$stateValue.lateAfterMinutesOverride ?? null} initialReturn={$stateValue.returnWindowMinutesOverride ?? null} language={$language} onApply={applyOverrides} /></div></details>
    </div>
  </section>
  {#if privilege?.current === "none"}<p class="privilege-warning">{t("你目前不是主持人或联合主持人：等候室中的人员和等候时长不会被记录，直到你获得权限。", "You are not host or co-host: people in the waiting room and their waiting time are not recorded until you are.")}</p>{/if}
  <div class="summary-row"><SortableFilterStack items={liveStatItems} active={activeFilter} onSelect={selectStat} /><div class="mode-switch" aria-label={t("考勤模式", "Attendance mode")}><button disabled={$busyKeys.has("live-mode")} class:chosen={$mode === 'generic'} onclick={() => changeMode('generic')}>{t("普通模式", "Generic")}</button><button disabled={$busyKeys.has("live-mode")} class:chosen={$mode === 'roster'} onclick={() => changeMode('roster')}>{t("花名册", "Roster")}</button></div></div>
  <PrimaryTabs items={primaryTabs} active={primary} onSelect={pickPrimary} />
  {#if primary === "attendance"}<SecondaryTabs items={attendanceTabs.map((x) => ({ ...x, label: x.id === "matching" && unmatchedCount ? `${x.label} · ${unmatchedCount}` : x.id === "names" && nameChanges.length ? `${x.label} · ${nameChanges.length}` : x.label }))} active={secondary} onSelect={(id) => { secondary = id; }} />{/if}
  {#if primary === "attendance" && secondary === "matching"}
    <NameMatchingPanel rows={$record.rows || []} decisions={$stateValue.reconciliationDecisions || []} roster={$configs.roster || []} aliases={$configs.aliases || []} nameChanges={nameChanges} language={$language} onDecision={(d) => saveDecision(d)} onRevert={(id) => saveDecision(null, id)} />
  {:else}
    <LiveAttendanceView view={primary === "attendance" ? secondary : primary} record={$record} state={$stateValue} events={$recordEvents} users={liveUsers} language={$language} configs={$configs} mode={$mode} bind:activeFilter showSummary={false} onModeChange={changeMode} profile={profileGuess()} onCaptureManual={captureManual} onCaptureHands={captureHands} onDecision={(d: any) => saveDecision(d)} onRevert={(id: string) => saveDecision(null, id)} onOverrides={applyOverrides} onSelectProfile={selectProfile} />
  {/if}
{:else}
  <section class="empty-card"><span>◉</span><h2>{t("尚未加入会议", "No active meeting")}</h2><p>{t("连接后，会议记录会自动显示在这里。", "When you join a meeting, its attendance record will appear here.")}</p></section>
{/if}

<style>
 .privilege-warning{background:#fffbeb;border:1px solid #f3d08a;color:#92400e;border-radius:7px;padding:9px 12px;margin:7px 0;font-size:12px}.summary-row{display:flex;align-items:center;gap:10px;border-bottom:1px solid var(--line);padding:4px 0}.mode-switch{display:flex;flex:none;background:#f4f6fa;border-radius:7px;padding:2px}.mode-switch button{min-height:27px;padding:3px 9px;border:0;background:transparent;font-size:10px}.mode-switch button.chosen{background:white;color:var(--accent);box-shadow:var(--shadow-card)}.compact-head{padding:8px 12px;margin-bottom:8px;min-height:44px}.meeting-title,.head-controls{display:flex;align-items:center;gap:10px;min-width:0}.meeting-title b{font-size:13px;max-width:280px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.meeting-title span{font-size:11px;color:var(--muted);white-space:nowrap}.meeting-number{padding-left:9px;border-left:1px solid var(--line)}.head-controls{flex:none}.live-status{display:flex;align-items:center;gap:5px;color:#27835d;font-size:11px;white-space:nowrap}.live-status i{width:7px;height:7px;border-radius:50%;background:#26b47a}.profile{display:flex;align-items:center;gap:6px;color:var(--muted);font-size:10px}.profile select{min-height:30px!important;padding:5px 7px!important;max-width:150px}.overrides{position:relative}.overrides summary{cursor:pointer;list-style:none;border:1px solid var(--line);border-radius:6px;padding:6px 8px;color:var(--muted);font-size:10px}.override-pop{position:absolute;right:0;top:32px;z-index:5;width:min(420px,90vw);background:white;border:1px solid var(--line);box-shadow:0 8px 26px #18223020;border-radius:8px;padding:12px}.override-pop p{font-size:11px;color:var(--muted)}
 .experimental-tag{font-size:9px;color:var(--muted);border:1px solid var(--line);border-radius:10px;padding:2px 5px;white-space:nowrap}@media(max-width:950px){.compact-head{align-items:flex-start;flex-wrap:wrap}.head-controls{flex-wrap:wrap}.meeting-title{flex-wrap:wrap}}
</style>
