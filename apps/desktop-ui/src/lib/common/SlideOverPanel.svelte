<script lang="ts">
  import { t } from "../ui";
  import Tooltip from "./Tooltip.svelte";
  export let open = false;
  export let title = '';
  export let closeLabel = '';
  export let width = 720;
  export let minWidth = 450;
  export let maxWidth = 1200;
  export let onClose: () => void = () => {};
  let panelWidth = width, resizing = false, justResized = false, startX = 0, startWidth = 0;
  function clamp(candidate: number) { const upper = Math.min(maxWidth, Math.max(320, window.innerWidth - 16)); return Math.max(Math.min(minWidth, upper), Math.min(upper, candidate)); }
  function beginResize(event: MouseEvent) { event.preventDefault(); event.stopPropagation(); resizing = true; startX = event.clientX; startWidth = panelWidth; document.addEventListener('mousemove', resize); document.addEventListener('mouseup', endResize); }
  function resize(event: MouseEvent) { if (resizing) panelWidth = clamp(startWidth + startX - event.clientX); }
  function endResize() { resizing = false; justResized = true; document.removeEventListener('mousemove', resize); document.removeEventListener('mouseup', endResize); setTimeout(() => justResized = false, 120); }
  function resizeKey(event: KeyboardEvent) { if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); const step = event.shiftKey ? 50 : 10; panelWidth = clamp(panelWidth + (event.key === 'ArrowLeft' ? -step : step)); /* AttendFlow: 10px, Shift 50px */ } }
  function closeFromBackdrop() { if (!resizing && !justResized) onClose(); }
  function handleKeydown(event: KeyboardEvent) { if (event.key === 'Escape' && open && !event.defaultPrevented) onClose(); }
</script>
<svelte:window on:keydown={handleKeydown} />
{#if open}<div class="scrim"><button class="backdrop" aria-label={closeLabel} onclick={closeFromBackdrop}></button><!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions --><div class="panel" role="dialog" aria-modal="true" tabindex="0" style={`--panel-width:${clamp(panelWidth)}px;width:min(var(--panel-width),94vw)`} aria-label={title}><!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions --><div class="resize-handle" role="separator" aria-label={t('调整面板宽度','Resize panel')} aria-orientation="vertical" tabindex="0" onmousedown={beginResize} onkeydown={resizeKey}></div><header><h2>{title}</h2><Tooltip text={closeLabel}><button aria-label={closeLabel} onclick={onClose}>×</button></Tooltip></header><div class="body"><slot /></div></div></div>{/if}
<style>
  .scrim{position:fixed;inset:0;z-index:20;background:rgba(24,32,48,.5);display:flex;justify-content:flex-end}.backdrop{position:absolute;inset:0;width:100%;height:100%;border:0;border-radius:0;background:transparent}.panel{position:relative;margin:0;height:100%;display:flex;flex-direction:column;background:var(--surface);border:0;padding:0;box-shadow:-10px 0 32px #17203322}.resize-handle{position:absolute;z-index:2;left:0;top:0;bottom:0;width:10px;cursor:col-resize;touch-action:none}.resize-handle:hover,.resize-handle:focus-visible{background:#356fd122;outline:0}.panel header{padding:15px 20px;margin:0;border-bottom:1px solid var(--line);display:flex;align-items:center;justify-content:space-between}.panel h2{font-size:16px;margin:0}.body{overflow:auto;padding:18px 20px;flex:1}
</style>
