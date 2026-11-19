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
  .secondary-tabs{display:flex;gap:0;overflow:auto;padding:0;background:transparent;border-bottom:1px solid var(--line);margin:0 0 8px;scrollbar-width:thin}
  button{flex:none;min-height:30px;padding:5px 11px;border:0;border-bottom:2px solid transparent;border-radius:0;background:transparent;color:var(--muted);font-size:11px;white-space:nowrap}
  button.active{border-bottom-color:var(--accent);background:transparent;color:var(--accent);font-weight:650;box-shadow:none}
</style>
