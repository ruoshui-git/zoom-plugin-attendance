<script lang="ts">
  import { onMount, tick } from 'svelte';
  export let labels: { all: string; json: string; attendance: string; manual: string };
  export let onExport: (kind: string) => void = () => {};
  export let disabled = false;
  let open = false;
  let menu: HTMLDivElement;
  let toggle: HTMLButtonElement;
  let list: HTMLDivElement;
  const menuId = `export-menu-${Math.random().toString(36).slice(2, 9)}`;
  $: options = [['all', labels.all], ['json', labels.json], ['summary', labels.attendance], ['manual', labels.manual]] as const;
  const items = () => Array.from(list?.querySelectorAll<HTMLButtonElement>('[role=menuitem]') || []);
  async function setOpen(value: boolean, focusFirst = false) {
    open = value;
    if (value && focusFirst) { await tick(); items()[0]?.focus(); }
  }
  function choose(kind: string) { open = false; toggle?.focus(); onExport(kind); }
  function menuKeydown(event: KeyboardEvent) {
    const all = items(); const index = all.indexOf(document.activeElement as HTMLButtonElement);
    const move = (to: number) => { event.preventDefault(); all[(to + all.length) % all.length]?.focus(); };
    if (event.key === 'ArrowDown') move(index + 1);
    else if (event.key === 'ArrowUp') move(index - 1);
    else if (event.key === 'Home') move(0);
    else if (event.key === 'End') move(all.length - 1);
    else if (event.key === 'Tab') open = false;
  }
  function toggleKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'Enter' || event.key === ' ') { event.preventDefault(); void setOpen(true, true); }
  }
  onMount(() => {
    const outside = (event: MouseEvent) => { if (open && !menu?.contains(event.target as Node)) open = false; };
    const keydown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || !open) return;
      event.preventDefault(); event.stopImmediatePropagation(); open = false; toggle?.focus();
    };
    document.addEventListener('mousedown', outside);
    window.addEventListener('keydown', keydown, true);
    return () => { document.removeEventListener('mousedown', outside); window.removeEventListener('keydown', keydown, true); };
  });
</script>
<div class="menu" bind:this={menu}>
  <button bind:this={toggle} type="button" {disabled} aria-haspopup="menu" aria-expanded={open} aria-controls={menuId} onclick={() => setOpen(!open)} onkeydown={toggleKeydown}>{labels.all}<span class="caret" aria-hidden="true">▾</span></button>
  {#if open}
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div class="items" id={menuId} role="menu" tabindex="-1" bind:this={list} onkeydown={menuKeydown}>
      {#each options as [kind, label]}<button type="button" role="menuitem" tabindex="-1" onclick={() => choose(kind)}>{label}</button>{/each}
    </div>
  {/if}
</div>
<style>
  .menu{position:relative}
  .caret{font-size:10px;color:var(--muted);margin-left:2px}
  /* A vertical list regardless of the container (e.g. a white-space:nowrap toolbar). */
  .items{position:absolute;z-index:20;right:0;top:calc(100% + 4px);display:flex;flex-direction:column;align-items:stretch;gap:2px;min-width:200px;padding:4px;white-space:normal;background:var(--surface);border:1px solid var(--line);border-radius:8px;box-shadow:0 6px 18px #1822301c}
  .items button{display:flex;width:100%;justify-content:flex-start;text-align:left;border:0;border-radius:6px;background:transparent;font-size:12px;white-space:nowrap}
  .items button:hover,.items button:focus-visible{background:var(--accent-soft);color:var(--accent);outline:none}
</style>
