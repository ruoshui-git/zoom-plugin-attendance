<script lang="ts">
  export let items: { id: string; label: string; count: number; shortLabel?: string; color?: string }[] = [];
  export let active = "";
  export let onSelect: (id: string) => void = () => {};
</script>

<div class="filter-stack" role="toolbar" aria-label="Attendance status presets">
  {#each items as item (item.id)}
    <button class:active={active === item.id} aria-pressed={active === item.id} style={`--status-color:${item.color || '#596273'}`} onclick={() => onSelect(item.id)}>
      <small>{item.shortLabel || item.label}</small><b>{item.count}</b>
    </button>
  {/each}
</div>

<style>
  .filter-stack{display:flex;align-items:center;gap:4px;overflow:auto;padding:2px 0;scrollbar-width:thin;flex:1}
  .filter-stack button{display:grid;flex:none;min-width:52px;padding:4px 7px;border:1px solid var(--line);border-radius:6px;background:white;text-align:left;color:var(--status-color);font-size:10px;white-space:nowrap}
  .filter-stack button small{color:var(--muted)}
  .filter-stack button b{font-size:13px}
  .filter-stack button.active{background:color-mix(in srgb,var(--status-color) 12%,white);border-color:var(--status-color)}
</style>
