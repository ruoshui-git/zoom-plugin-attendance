<script lang="ts">
  import { meeting, record, recordEvents, stateValue, configs, language, mode, page } from "../lib/stores";
  import { captureManual, captureHands, applyOverrides, saveDecision, selectProfile, profileGuess, switchMode } from "../lib/api";
  import { t, dateTime } from "../lib/ui";
  import ProfileOverrideForm from "../lib/ProfileOverrideForm.svelte";
  import NameMatchingPanel from "../lib/NameMatchingPanel.svelte";
  import PrimaryTabs from "../lib/common/PrimaryTabs.svelte";
  import SecondaryTabs from "../lib/common/SecondaryTabs.svelte";
  import LiveAttendanceView from "./live/LiveAttendanceView.svelte";
  import { currentUsers } from "../lib/api";

  let primary = "attendance";
  let secondary = "status";
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
  $: if ($page === "matching") { primary = "attendance"; secondary = "matching"; }
  $: liveLabel = $meeting?.status === "ended" ? t("已结束", "Ended") : t("进行中", "Live");
  function pickPrimary(id: string) { primary = id; if (id === "attendance") secondary = "status"; }
</script>

{#if $meeting}
  <section class="meet-head compact-head">
    <div class="meeting-title"><b title={$meeting.topic || t("未命名会议", "Untitled meeting")}>{$meeting.topic || t("未命名会议", "Untitled meeting")}</b><span class="meeting-number">#{ $meeting.meetingNumber || "—" }</span><span>{dateTime($meeting.startedAt)}</span></div>
    <div class="head-controls">
      <span class="live-status"><i></i>{liveLabel}</span>
      <label class="profile"><span>{t("配置", "Profile")}</span><select aria-label={t("考勤配置", "Attendance profile")} value={$stateValue.selectedProfileId || profileGuess()?.id || "default-all"} onchange={(e) => selectProfile(e.currentTarget.value)}>
        {#each ($configs.profiles || []).filter((p: any) => p.active) as p}<option value={p.id}>{p.name}</option>{/each}
      </select></label>
      <details class="overrides"><summary>{t("阈值", "Overrides")}</summary><div class="override-pop"><p>{t("迟到", "Late")}: {$record.record?.lateAfterMinutes ?? profileGuess()?.lateAfterMinutes ?? 10} {t("分钟", "min")} · {t("返回", "Return")}: {$record.record?.returnWindowMinutes ?? profileGuess()?.returnWindowMinutes ?? 8} {t("分钟", "min")}</p><ProfileOverrideForm initialLate={$stateValue.lateAfterMinutesOverride ?? null} initialReturn={$stateValue.returnWindowMinutesOverride ?? null} language={$language} onApply={applyOverrides} /></div></details>
    </div>
  </section>
  <PrimaryTabs items={primaryTabs} active={primary} onSelect={pickPrimary} />
  {#if primary === "attendance"}<SecondaryTabs items={attendanceTabs.map((x) => ({ ...x, label: x.id === "matching" && unmatchedCount ? `${x.label} · ${unmatchedCount}` : x.id === "names" && nameChanges.length ? `${x.label} · ${nameChanges.length}` : x.label }))} active={secondary} onSelect={(id) => { secondary = id; if (id !== "matching") $page = "live"; }} />{/if}
  {#if primary === "attendance" && secondary === "matching"}
    <NameMatchingPanel rows={$record.rows || []} decisions={$stateValue.reconciliationDecisions || []} roster={$configs.roster || []} aliases={$configs.aliases || []} nameChanges={nameChanges} language={$language} onDecision={(d) => saveDecision(d)} onRevert={(id) => saveDecision(null, id)} />
  {:else}
    <LiveAttendanceView view={primary === "attendance" ? secondary : primary} record={$record} state={$stateValue} events={$recordEvents} users={currentUsers()} language={$language} configs={$configs} mode={$mode} onModeChange={switchMode} profile={profileGuess()} onCaptureManual={captureManual} onCaptureHands={captureHands} onDecision={(d: any) => saveDecision(d)} onRevert={(id: string) => saveDecision(null, id)} onOverrides={applyOverrides} onSelectProfile={selectProfile} />
  {/if}
{:else}
  <section class="empty-card"><span>◉</span><h2>{t("尚未加入会议", "No active meeting")}</h2><p>{t("连接后，会议记录会自动显示在这里。", "When you join a meeting, its attendance record will appear here.")}</p></section>
{/if}

<style>
 .compact-head{padding:8px 12px;margin-bottom:8px;min-height:44px}.meeting-title,.head-controls{display:flex;align-items:center;gap:10px;min-width:0}.meeting-title b{font-size:13px;max-width:280px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.meeting-title span{font-size:11px;color:var(--muted);white-space:nowrap}.meeting-number{padding-left:9px;border-left:1px solid var(--line)}.head-controls{flex:none}.live-status{display:flex;align-items:center;gap:5px;color:#27835d;font-size:11px;white-space:nowrap}.live-status i{width:7px;height:7px;border-radius:50%;background:#26b47a}.profile{display:flex;align-items:center;gap:6px;color:var(--muted);font-size:10px}.profile select{min-height:30px!important;padding:5px 7px!important;max-width:150px}.overrides{position:relative}.overrides summary{cursor:pointer;list-style:none;border:1px solid var(--line);border-radius:6px;padding:6px 8px;color:var(--muted);font-size:10px}.override-pop{position:absolute;right:0;top:32px;z-index:5;width:min(420px,90vw);background:white;border:1px solid var(--line);box-shadow:0 8px 26px #18223020;border-radius:8px;padding:12px}.override-pop p{font-size:11px;color:var(--muted)}
 @media(max-width:950px){.compact-head{align-items:flex-start;flex-wrap:wrap}.head-controls{flex-wrap:wrap}.meeting-title{flex-wrap:wrap}}
</style>
