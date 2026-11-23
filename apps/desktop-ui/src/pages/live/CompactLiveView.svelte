<script lang="ts">
  import { meeting, record, stateValue, recordEvents } from "../../lib/stores";
  import { t } from "../../lib/ui";
  import { windowState, setWindowMode, setPinned } from "../../lib/window-control";
  import { toast } from "../../lib/toast";
  import { uiLanguage } from "../../lib/i18n-core";
  $: lang = $uiLanguage === "en";
  $: rows = $record.rows || [];
  $: present = rows.filter((r: any) => r.isCurrentlyPresent).length;
  $: expected = Number($record.summary?.expected || 0);
  $: roster = $stateValue.mode === "roster";
  $: hands = ($stateValue.raiseHandEntries || []).filter((h: any) => h.isRaised || h.raised || h.action === "raise").map((h: any) => h.name || h.screenName).filter(Boolean);
  $: cameraOff = rows.filter((r: any) => r.isCurrentlyPresent && (r.videoOn === false || r.video_on === false)).length;
  $: changes = ($recordEvents || []).filter((e: any) => ["join", "leave"].includes(e.type)).slice(-3).reverse();
  async function expand() { try { await setWindowMode("full"); } catch (e) { toast.error(e instanceof Error ? e.message : t("操作失败", "Could not update window")); } }
  async function pin() { try { await setPinned(!$windowState.pinned); } catch (e) { toast.error(e instanceof Error ? e.message : t("操作失败", "Could not update window")); } }
</script>
<main class="compact" lang={lang ? "en" : "zh-CN"}>
  <header><b>{$meeting?.topic || t("会议出勤", "Meeting attendance")}</b><button onclick={expand}>{t("展开", "Expand")}</button></header>
  {#if !$meeting}<p class="empty">{t("未在会议中", "Not in a meeting")}</p>{:else}
    <button class="counts" onclick={expand}><b>{present}{roster ? ` / ${expected}` : ""}</b><span>{t("出席", "Present")}{roster ? ` · ${t("迟到", "Late")} ${$record.summary?.late || 0} · ${t("缺席", "Missing")} ${$record.summary?.missing || 0}` : ""}</span></button>
    <div class="row"><span>{t("举手", "Hands raised")}: {hands.slice(0,3).join(", ") || "—"}{hands.length > 3 ? ` +${hands.length - 3}` : ""}</span><span>{t("摄像头关闭", "Camera off")}: {cameraOff}</span></div>
    <section>{#each changes as e}<div>{e.timestamp?.slice(11,19) || ""} · {e.type === "join" ? t("加入", "Joined") : t("离开", "Left")} · {e.name || e.screenName || "—"}</div>{/each}</section>
    <footer><button aria-pressed={$windowState.pinned} onclick={pin}>{$windowState.pinned ? "📌" : "♧"} {t("置顶", "Pin")}</button><button onclick={expand}>{t("展开", "Expand")}</button></footer>
  {/if}
</main>
<style>
  .compact{box-sizing:border-box;min-height:100vh;padding:8px 10px;background:var(--page);color:var(--ink);font-size:12px;display:grid;grid-template-rows:auto auto auto 1fr auto;gap:7px}header,footer,.row{display:flex;align-items:center;justify-content:space-between;gap:8px}button{font:inherit;font-size:11px;padding:4px 8px}header b{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.counts{display:grid;text-align:left;gap:2px;background:var(--surface);border:1px solid var(--line);border-radius:7px;color:inherit;padding:7px}.counts b{font-size:18px}.row,section{font-size:11px}section{overflow:hidden}section div{padding:3px 0}.empty{align-self:center;text-align:center}
</style>
