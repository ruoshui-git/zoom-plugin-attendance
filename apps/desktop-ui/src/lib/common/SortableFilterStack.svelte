<script lang="ts">
  export let items: { id: string; label: string; count: number; shortLabel?: string; color?: string }[] = [];
  export let active = "";
  export let onSelect: (id: string) => void = () => {};
</script>

<div class="filter-stack" role="toolbar" aria-label="Attendance status presets">
  {#each items as item (item.id)}
    <button class:active={active === item.id} aria-pressed={active === item.id} style={`--status-color:${item.color || '#596273'}`} onclick={() => onSelect(item.id)}>
      <small>{item.label}</small><b>{item.count}</b>
    </button>
  {/each}
</div>

<style>
  .filter-stack{display:flex;align-items:center;gap:6px;overflow:auto;padding:3px 0;scrollbar-width:thin;flex:1}
  .filter-stack button{display:flex;align-items:center;gap:5px;flex:none;padding:5px 8px;border:1px solid transparent;border-radius:5px;background:transparent;text-align:left;color:var(--muted);font-size:11px;white-space:nowrap;cursor:pointer}
  .filter-stack button small{font-size:11px;color:inherit}
  .filter-stack button b{font-size:12px;color:var(--status-color)}
  .filter-stack button:hover{background:color-mix(in srgb,var(--status-color) 8%,white)}
  .filter-stack button.active{background:color-mix(in srgb,var(--status-color) 12%,white);border-color:color-mix(in srgb,var(--status-color) 45%,white);color:var(--status-color)}
</style>
