<script lang="ts">
  import { meeting, record, recordEvents, stateValue, configs, language, mode, requestedLiveSubtab } from "../lib/stores";
  import { captureManual, captureHands, applyOverrides, saveDecision, selectProfile, profileGuess, switchMode } from "../lib/api";
  import { t, dateTime } from "../lib/ui";
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
</script>

{#if $meeting}
  <section class="meet-head compact-head">
    <div class="meeting-title"><b title={$meeting.topic || t("未命名会议", "Untitled meeting")}>{$meeting.topic || t("未命名会议", "Untitled meeting")}</b><span class="meeting-number">#{ $meeting.meetingNumber || "—" }</span><span>{dateTime($meeting.startedAt)}</span></div>
    <div class="head-controls">
      <span class="live-status"><i></i>{liveLabel}</span>
      <label class="profile"><span>{t("配置", "Profile")}</span><select aria-label={t("考勤配置", "Attendance profile")} disabled={$busyKeys.has("live-profile")} value={$stateValue.selectedProfileId || profileGuess()?.id || "default-all"} onchange={(e) => changeProfile(e.currentTarget.value)}>
        {#each ($configs.profiles || []).filter((p: any) => p.active) as p}<option value={p.id}>{p.name}</option>{/each}
      </select></label>
      <details class="overrides"><summary>{t("阈值", "Overrides")}</summary><div class="override-pop"><p>{t("迟到", "Late")}: {$record.record?.lateAfterMinutes ?? profileGuess()?.lateAfterMinutes ?? 10} {t("分钟", "min")} · {t("返回", "Return")}: {$record.record?.returnWindowMinutes ?? profileGuess()?.returnWindowMinutes ?? 8} {t("分钟", "min")}</p><ProfileOverrideForm initialLate={$stateValue.lateAfterMinutesOverride ?? null} initialReturn={$stateValue.returnWindowMinutesOverride ?? null} language={$language} onApply={applyOverrides} /></div></details>
    </div>
  </section>
  <div class="summary-row"><SortableFilterStack items={liveStatItems} active={activeFilter} onSelect={selectStat} /><div class="mode-switch" aria-label={t("考勤模式", "Attendance mode")}><button disabled={$busyKeys.has("live-mode")} class:chosen={$mode === 'generic'} onclick={() => changeMode('generic')}>{t("普通模式", "Generic")}</button><button disabled={$busyKeys.has("live-mode")} class:chosen={$mode === 'roster'} onclick={() => changeMode('roster')}>{t("花名册", "Roster")}</button></div></div>
  <PrimaryTabs items={primaryTabs} active={primary} onSelect={pickPrimary} />
  {#if primary === "attendance"}<SecondaryTabs items={attendanceTabs.map((x) => ({ ...x, label: x.id === "matching" && unmatchedCount ? `${x.label} · ${unmatchedCount}` : x.id === "names" && nameChanges.length ? `${x.label} · ${nameChanges.length}` : x.label }))} active={secondary} onSelect={(id) => { secondary = id; }} />{/if}
  {#if primary === "attendance" && secondary === "matching"}
    <NameMatchingPanel rows={$record.rows || []} decisions={$stateValue.reconciliationDecisions || []} roster={$configs.roster || []} aliases={$configs.aliases || []} nameChanges={nameChanges} language={$language} onDecision={(d) => saveDecision(d)} onRevert={(id) => saveDecision(null, id)} />
  {:else}
    <LiveAttendanceView view={primary === "attendance" ? secondary : primary} record={$record} state={$stateValue} events={$recordEvents} users={currentUsers()} language={$language} configs={$configs} mode={$mode} bind:activeFilter showSummary={false} onModeChange={changeMode} profile={profileGuess()} onCaptureManual={captureManual} onCaptureHands={captureHands} onDecision={(d: any) => saveDecision(d)} onRevert={(id: string) => saveDecision(null, id)} onOverrides={applyOverrides} onSelectProfile={selectProfile} />
  {/if}
{:else}
  <section class="empty-card"><span>◉</span><h2>{t("尚未加入会议", "No active meeting")}</h2><p>{t("连接后，会议记录会自动显示在这里。", "When you join a meeting, its attendance record will appear here.")}</p></section>
{/if}

<style>
 .summary-row{display:flex;align-items:center;gap:10px;border-bottom:1px solid var(--line);padding:4px 0}.mode-switch{display:flex;flex:none;background:#f4f6fa;border-radius:7px;padding:2px}.mode-switch button{min-height:27px;padding:3px 9px;border:0;background:transparent;font-size:10px}.mode-switch button.chosen{background:white;color:var(--accent);box-shadow:var(--shadow-card)}.compact-head{padding:8px 12px;margin-bottom:8px;min-height:44px}.meeting-title,.head-controls{display:flex;align-items:center;gap:10px;min-width:0}.meeting-title b{font-size:13px;max-width:280px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.meeting-title span{font-size:11px;color:var(--muted);white-space:nowrap}.meeting-number{padding-left:9px;border-left:1px solid var(--line)}.head-controls{flex:none}.live-status{display:flex;align-items:center;gap:5px;color:#27835d;font-size:11px;white-space:nowrap}.live-status i{width:7px;height:7px;border-radius:50%;background:#26b47a}.profile{display:flex;align-items:center;gap:6px;color:var(--muted);font-size:10px}.profile select{min-height:30px!important;padding:5px 7px!important;max-width:150px}.overrides{position:relative}.overrides summary{cursor:pointer;list-style:none;border:1px solid var(--line);border-radius:6px;padding:6px 8px;color:var(--muted);font-size:10px}.override-pop{position:absolute;right:0;top:32px;z-index:5;width:min(420px,90vw);background:white;border:1px solid var(--line);box-shadow:0 8px 26px #18223020;border-radius:8px;padding:12px}.override-pop p{font-size:11px;color:var(--muted)}
 @media(max-width:950px){.compact-head{align-items:flex-start;flex-wrap:wrap}.head-controls{flex-wrap:wrap}.meeting-title{flex-wrap:wrap}}
</style>
