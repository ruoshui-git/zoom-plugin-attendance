<script lang="ts">
  export let items: { id: string; label: string }[] = [];
  export let active = '';
  export let onSelect: (id: string) => void = () => {};
</script>

<nav class="primary-tabs" aria-label="{items.map((item) => item.label).join(', ')}">
  {#each items as item (item.id)}
    <button class:active={active === item.id} aria-current={active === item.id ? 'page' : undefined} onclick={() => onSelect(item.id)}>{item.label}</button>
  {/each}
</nav>

<style>
  .primary-tabs{display:flex;gap:0;overflow:auto;border-bottom:1px solid var(--line);margin:0 0 6px;scrollbar-width:thin}
  button{flex:none;min-height:33px;border:0;border-bottom:2px solid transparent;border-radius:0;background:transparent;padding:5px 13px;color:var(--muted);font-size:12px;white-space:nowrap}
  button.active{border-bottom-color:var(--accent);color:var(--accent);font-weight:650}
</style>
