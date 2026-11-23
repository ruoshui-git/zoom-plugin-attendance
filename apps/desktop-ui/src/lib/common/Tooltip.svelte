<script lang="ts">
  import { onMount, tick } from "svelte";
  import type { Snippet } from "svelte";

  /**
   * Hover/focus tooltip for icon-only controls (port of AttendFlow's Tooltip).
   * The bubble is position:fixed from the trigger's rect, so scrolling/overflow containers
   * never clip it; it flips below when there is no room above and is clamped to the viewport.
   */
  let { text = "", delay = 350, children }: { text?: string; delay?: number; children: Snippet } = $props();
  let visible = $state(false);
  let top = $state(0);
  let left = $state(0);
  let below = $state(false);
  let host: HTMLSpanElement;
  let bubble: HTMLSpanElement | undefined = $state();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const id = `tooltip-${Math.random().toString(36).slice(2, 10)}`;
  const GAP = 6, EDGE = 8;

  async function place() {
    await tick();
    const trigger = (host.firstElementChild as HTMLElement | null) ?? host;
    const r = trigger.getBoundingClientRect();
    const bw = bubble?.offsetWidth ?? 0, bh = bubble?.offsetHeight ?? 0;
    below = r.top - GAP - bh < EDGE;
    top = below ? r.bottom + GAP : r.top - GAP - bh;
    left = Math.min(Math.max(r.left + r.width / 2 - bw / 2, EDGE), window.innerWidth - bw - EDGE);
  }
  function show() {
    if (!text) return;
    clearTimeout(timer);
    timer = setTimeout(() => { visible = true; place(); }, delay);
  }
  function hide() { clearTimeout(timer); visible = false; }
  function keydown(event: KeyboardEvent) { if (event.key === "Escape" && visible) { event.preventDefault(); hide(); } } // Esc closes only the tooltip, not the panel behind it

  onMount(() => {
    const target = host.querySelector<HTMLElement>("button, a, input, select, [tabindex]");
    target?.setAttribute("aria-describedby", id);
    window.addEventListener("scroll", hide, true);
    return () => { clearTimeout(timer); window.removeEventListener("scroll", hide, true); };
  });
</script>

<span class="tip" role="presentation" bind:this={host} onmouseenter={show} onmouseleave={hide} onfocusin={show} onfocusout={hide} onkeydown={keydown}>
  {@render children()}
  <span bind:this={bubble} class="bubble" class:below class:visible id={id} role="tooltip" style={`top:${top}px;left:${left}px`} aria-hidden={!visible}>{text}</span>
</span>

<style>
  .tip { position: relative; display: inline-flex; }
  .bubble { position: fixed; z-index: 10000; max-width: min(260px, calc(100vw - 16px)); width: max-content; padding: 6px 8px; border-radius: 5px; background: #202a39; color: #fff; font-size: 11px; line-height: 1.35; white-space: normal; pointer-events: none; visibility: hidden; opacity: 0; transition: opacity 0.12s; }
  .bubble.visible { visibility: visible; opacity: 1; }
  @media (prefers-reduced-motion: reduce) { .bubble { transition: none; } }
</style>
