<script lang="ts">
  // Pin (always on top) and full/compact switch for the native window. Hidden in the browser.
  import Tooltip from "./Tooltip.svelte";
  import { t } from "../ui";
  import { toast } from "../toast";
  import { windowState, setPinned, setWindowMode } from "../window-control";
  export let inCompact = false;
  async function togglePin() {
    const pinned = !$windowState.pinned;
    try {
      await setPinned(pinned);
      toast.success(pinned ? t("窗口已置顶", "Window kept on top") : t("已取消置顶", "Window no longer on top"));
    } catch (error) {
      toast.error(`${t("无法更改置顶：", "Could not change pinning: ")}${error instanceof Error ? error.message : String(error)}`);
    }
  }
  async function switchMode() {
    try { await setWindowMode(inCompact ? "full" : "compact"); }
    catch (error) { toast.error(`${t("无法切换窗口模式：", "Could not switch window mode: ")}${error instanceof Error ? error.message : String(error)}`); }
  }
</script>

{#if $windowState.available}
  <div class="window-controls">
    {#if $windowState.pinned && !inCompact}<span class="pinned-badge">{t("已置顶", "Pinned")}</span>{/if}
    <Tooltip text={$windowState.pinned ? t("取消置顶", "Stop keeping on top") : t("置顶窗口", "Keep window on top")}>
      <button type="button" class="icon" class:active={$windowState.pinned} aria-pressed={$windowState.pinned} aria-label={$windowState.pinned ? t("取消置顶", "Stop keeping on top") : t("置顶窗口", "Keep window on top")} onclick={togglePin}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M10.5 1.5l4 4-1.6.4-2.6 2.6.3 3.1-1.3 1.3-2.6-2.6-3.6 3.6-.7-.7 3.6-3.6L3.4 7l1.3-1.3 3.1.3 2.6-2.6z" fill="currentColor"/></svg>
      </button>
    </Tooltip>
    <Tooltip text={inCompact ? t("展开完整窗口", "Expand to full window") : t("紧凑模式", "Compact view")}>
      <button type="button" class="icon" aria-label={inCompact ? t("展开完整窗口", "Expand to full window") : t("紧凑模式", "Compact view")} onclick={switchMode}>
        {#if inCompact}
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M2 6V2h4M10 2h4v4M14 10v4h-4M6 14H2v-4" fill="none" stroke="currentColor" stroke-width="1.6"/></svg>
        {:else}
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M6 2v4H2M14 6h-4V2M10 14v-4h4M2 10h4v4" fill="none" stroke="currentColor" stroke-width="1.6"/></svg>
        {/if}
      </button>
    </Tooltip>
  </div>
  {#if $windowState.pinned}<span class="pin-edge" aria-hidden="true"></span>{/if}
{/if}

<style>
  .window-controls { display: flex; align-items: center; gap: 6px; }
  .icon { width: 30px; min-width: 30px; height: 30px; padding: 0; color: var(--muted); }
  .icon.active { background: var(--accent); border-color: var(--accent); color: #fff; }
  .icon.active:hover:not(:disabled) { background: #2a4a73; border-color: #2a4a73; }
  .pinned-badge { padding: 3px 8px; border-radius: 10px; background: var(--accent-soft); color: var(--accent); font-size: 11px; font-weight: 600; white-space: nowrap; }
  /* Window-level marker while pinned: an accent edge around the whole viewport. */
  .pin-edge { position: fixed; inset: 0; z-index: 9999; pointer-events: none; box-shadow: inset 0 0 0 3px var(--accent); }
</style>
