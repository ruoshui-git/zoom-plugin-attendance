<script lang="ts">
  import { fade } from "svelte/transition";
  import { toast } from "../toast";
  import { t } from "../ui";
  import type { ToastMessage, ToastType } from "./toast-types";
  let { toasts = [], onDismiss }: { toasts: ToastMessage[]; onDismiss: (id: string) => void } = $props();
  let progress = $state(new Map<string, number>());
  $effect(() => {
    if (!toasts.length) { progress = new Map(); return; }
    const timer = setInterval(() => { progress = new Map(toasts.map((item) => [item.id, toast.getProgress(item.id)])); }, 50);
    return () => clearInterval(timer);
  });
  const icons: Record<ToastType, string> = { success: "✓", error: "✕", warning: "⚠", info: "ℹ" };
</script>

<div class="toast-container" aria-live="polite">
  {#each toasts as item (item.id)}
    <div class="toast toast-{item.type}" role="status" out:fade={{ duration: 200 }} onmouseenter={() => toast.pause(item.id)} onmouseleave={() => toast.resume(item.id)}>
      <span class="toast-icon" aria-hidden="true">{icons[item.type]}</span>
      <span class="toast-message">{item.message}</span>
      <button type="button" class="toast-dismiss" onclick={() => onDismiss(item.id)} aria-label={t("关闭", "Close")}>✕</button>
      {#if item.duration > 0}<div class="toast-progress"><div class="toast-progress-bar" style="width: {progress.get(item.id) ?? toast.getProgress(item.id)}%"></div></div>{/if}
    </div>
  {/each}
</div>

<style>
  .toast-container{position:fixed;top:1rem;right:1rem;z-index:9999;display:flex;flex-direction:column;gap:.5rem;width:min(24rem,calc(100vw - 2rem));max-height:calc(100vh - 2rem);overflow:auto;pointer-events:none}
  .toast{position:relative;display:flex;align-items:flex-start;gap:.75rem;padding:.875rem 1rem 1rem;border:1px solid var(--line);border-left:4px solid var(--accent);border-radius:var(--radius-md);background:var(--surface);color:var(--text);box-shadow:0 4px 12px #0002;pointer-events:auto;animation:toast-in .2s ease-out;overflow-wrap:anywhere}
  .toast-success{border-left-color:var(--success)}.toast-error{border-left-color:var(--danger)}.toast-warning{border-left-color:var(--warning)}.toast-info{border-left-color:var(--accent)}
  .toast-icon{flex:none;width:1.25rem;font-size:1rem;font-weight:700;text-align:center;color:var(--accent)}.toast-success .toast-icon{color:var(--success)}.toast-error .toast-icon{color:var(--danger)}.toast-warning .toast-icon{color:var(--warning)}
  .toast-message{flex:1;min-width:0;font-size:max(12px,var(--font-body));line-height:1.45;white-space:pre-wrap;color:var(--text)}
  .toast-dismiss{flex:none;width:1.75rem;min-height:1.75rem;margin:-.25rem -.25rem 0 0;padding:0;border:0;background:transparent;color:var(--muted);font-size:1rem}.toast-dismiss:hover{background:var(--surface-soft);color:var(--text)}
  .toast-progress{position:absolute;bottom:0;left:0;right:0;height:3px;background:var(--line);overflow:hidden}.toast-progress-bar{height:100%;background:currentColor;transition:width .05s linear}.toast-success .toast-progress-bar{color:var(--success)}.toast-error .toast-progress-bar{color:var(--danger)}.toast-warning .toast-progress-bar{color:var(--warning)}.toast-info .toast-progress-bar{color:var(--accent)}
  @keyframes toast-in{from{transform:translateX(30%);opacity:0}to{transform:translateX(0);opacity:1}}
  @media(prefers-reduced-motion:reduce){.toast{animation:none}.toast-progress-bar{transition:none}}
</style>
