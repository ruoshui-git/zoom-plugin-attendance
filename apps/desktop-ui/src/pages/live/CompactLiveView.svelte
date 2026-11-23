<script lang="ts">
  // Compact live window (R-C1): a narrow summary like Zoom's minimized meeting window.
  import { meeting, record, recordEvents, mode } from "../../lib/stores";
  import { currentUsers } from "../../lib/api";
  import { t, timeOnly } from "../../lib/ui";
  import { setWindowMode } from "../../lib/window-control";
  import { cameraOffCount } from "../../lib/media-labels";
  import { recordTitle } from "../../lib/record-title";
  import WindowControls from "../../lib/common/WindowControls.svelte";
  import { stateValue } from "../../lib/stores";

  // currentUsers() replays the live event log; re-run whenever it changes.
  const presentUsers = (_events: unknown[]) => currentUsers().filter((u: any) => !u.waitingRoom);
  $: users = presentUsers($recordEvents);
  $: rows = $record.rows || [];
  $: roster = $mode === "roster";
  $: present = roster ? rows.filter((r: any) => r.isCurrentlyPresent && r.isExpected !== false).length : users.length;
  $: expected = rows.filter((r: any) => r.isExpected).length;
  $: late = Number($record.summary?.late ?? 0);
  $: missing = Number($record.summary?.missing ?? 0);
  $: hands = users.filter((u: any) => u.raisedHand).map((u: any) => u.name || "—");
  $: cameraOff = cameraOffCount(users);
  $: movement = ($record.record?.movementEvents || $record.movementEvents || [])
    .filter((e: any) => ["join", "leave"].includes(e.type || e.eventType))
    .slice()
    .sort((a: any, b: any) => Date.parse(b.timestamp) - Date.parse(a.timestamp))
    .slice(0, 3);
  $: title = $meeting ? recordTitle({ ...$meeting, customName: $stateValue.customName }, $meeting.topic || t("未命名会议", "Untitled meeting")).primary : "";
  const expand = () => setWindowMode("full").catch(() => {});
</script>

<main class="compact">
  <div class="bar">
    <b class="title" title={title}>{title || t("实时出勤", "Live attendance")}</b>
    <WindowControls inCompact />
  </div>
  {#if !$meeting}
    <p class="empty">{t("未在会议中", "Not in a meeting")}</p>
  {:else}
    <button type="button" class="counts" onclick={expand} aria-label={t("展开完整窗口", "Expand to full window")}>
      <span class="metric"><b>{present}{#if roster}<small> / {expected}</small>{/if}</b><span>{roster ? t("出席 / 应到", "Present / expected") : t("在会", "Present")}</span></span>
      {#if roster}
        <span class="metric"><b class="late">{late}</b><span>{t("迟到", "Late")}</span></span>
        <span class="metric"><b class="missing">{missing}</b><span>{t("缺席", "Missing")}</span></span>
      {/if}
      <span class="metric"><b>{cameraOff}</b><span>{t("摄像头关闭", "Camera off")}</span></span>
    </button>
    <div class="line" title={hands.join(", ")}>
      <span class="label">✋ {t("举手", "Hands")} {hands.length}</span>
      <span class="names">{hands.slice(0, 3).join(t("、", ", ")) || "—"}{#if hands.length > 3}<em> +{hands.length - 3}</em>{/if}</span>
    </div>
    <ul class="moves">
      {#each movement as e}
        <li><time>{timeOnly(e.timestamp)}</time><span class:left={(e.type || e.eventType) === "leave"}>{(e.type || e.eventType) === "join" ? t("加入", "Joined") : t("离开", "Left")}</span><b title={e.screenName}>{e.screenName || "—"}</b></li>
      {:else}
        <li class="none">{t("暂无进出记录", "No joins or leaves yet")}</li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  .compact { box-sizing: border-box; height: 100vh; padding: 8px 10px 10px; background: var(--canvas); color: var(--text); font-size: 12px; display: flex; flex-direction: column; gap: 8px; overflow: hidden; }
  .bar { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-width: 0; }
  .title { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .counts { display: grid; grid-template-columns: repeat(auto-fit, minmax(64px, 1fr)); gap: 6px; width: 100%; min-height: 0; padding: 8px; background: var(--surface); border: 1px solid var(--line); border-radius: var(--radius-md); text-align: left; }
  .metric { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .metric b { font-size: 18px; line-height: 1.1; color: var(--text); }
  .metric b small { font-size: 12px; color: var(--muted); font-weight: 600; }
  .metric > span { font-size: 10px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .metric b.late { color: var(--warning); }
  .metric b.missing { color: var(--danger); }
  .line { display: flex; gap: 8px; align-items: baseline; min-width: 0; font-size: 11px; }
  .label { flex: none; color: var(--muted); }
  .names { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .names em { font-style: normal; color: var(--muted); }
  .moves { list-style: none; margin: 0; padding: 0; display: grid; gap: 3px; font-size: 11px; min-height: 0; overflow: hidden; }
  .moves li { display: grid; grid-template-columns: auto auto 1fr; gap: 8px; align-items: baseline; min-width: 0; }
  .moves time { color: var(--muted); font-variant-numeric: tabular-nums; }
  .moves span { color: var(--success); }
  .moves span.left { color: var(--muted); }
  .moves b { font-weight: 600; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .moves .none, .empty { color: var(--muted); }
  .empty { margin: 16px 0; text-align: center; }
</style>
