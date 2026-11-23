<script lang="ts">
  import { onDestroy } from "svelte";
  export let targetTable: HTMLTableElement | null = null;
  let content = "", x = 0, y = 0, timer: ReturnType<typeof setTimeout> | undefined;
  function hide() { if (timer) clearTimeout(timer); timer = undefined; content = ""; }
  function enter(event: MouseEvent) {
    const cell = (event.target as HTMLElement | null)?.closest("td, th") as HTMLElement | null;
    hide();
    if (!cell || cell.scrollWidth <= cell.clientWidth) return;
    const text = cell.innerText || cell.textContent || "";
    if (!text.trim()) return;
    timer = setTimeout(() => { content = text; x = Math.min(event.clientX + 12, window.innerWidth - 340); y = Math.min(event.clientY + 14, window.innerHeight - 80); timer = undefined; }, 500);
  }
  function move(event: MouseEvent) { if (content) { x = Math.min(event.clientX + 12, window.innerWidth - 340); y = Math.min(event.clientY + 14, window.innerHeight - 80); } }
  function leave(event: MouseEvent) { if (!(event.relatedTarget as HTMLElement | null)?.closest("td, th")) hide(); }
  // Re-attach when the table element changes; always detach on destroy.
  let attached: HTMLTableElement | null = null;
  function detach() { attached?.removeEventListener("mouseenter", enter, true); attached?.removeEventListener("mousemove", move, true); attached?.removeEventListener("mouseleave", leave, true); attached = null; }
  $: if (targetTable !== attached) { detach(); hide(); if (targetTable) { targetTable.addEventListener("mouseenter", enter, true); targetTable.addEventListener("mousemove", move, true); targetTable.addEventListener("mouseleave", leave, true); attached = targetTable; } }
  onDestroy(() => { detach(); hide(); });
</script>
{#if content}<div class="table-tooltip" style={`left:${Math.max(8,x)}px;top:${Math.max(8,y)}px`}>{content}</div>{/if}
<style>.table-tooltip{position:fixed;z-index:9999;max-width:min(520px,calc(100vw - 16px));padding:6px 10px;border-radius:6px;background:#202938;color:white;font-size:12px;white-space:pre-wrap;overflow-wrap:anywhere;pointer-events:none;box-shadow:0 4px 16px #0003}</style>
