<script lang="ts">
  export let items: { id: string; label: string }[] = [];
  export let active = '';
  export let onSelect: (id: string) => void = () => {};
</script>

<nav class="secondary-tabs" aria-label="{items.map((item) => item.label).join(', ')}">
  {#each items as item (item.id)}
    <button class:active={active === item.id} aria-pressed={active === item.id} onclick={() => onSelect(item.id)}>{item.label}</button>
  {/each}
</nav>

<style>
  .secondary-tabs{display:flex;gap:6px;overflow:auto;padding:3px;background:var(--surface-soft);border-radius:8px;margin:0 0 12px}
  button{flex:none;min-height:28px;padding:5px 9px;border:1px solid transparent;background:transparent;color:var(--muted);font-size:11px}
  button.active{border-color:var(--line);background:var(--surface);color:var(--text);box-shadow:var(--shadow-card)}
</style>
