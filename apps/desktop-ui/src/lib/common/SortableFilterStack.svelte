<script lang="ts">
  export let items: { id: string; label: string; count: number; shortLabel?: string; color?: string }[] = [];
  export let active = "";
  export let storageKey = "attendflow-filter-order";
  export let onSelect: (id: string) => void = () => {};
  let order: string[] = [];
  let dragging = "";
  $: ordered = [...items].sort((a, b) => {
    const ai = order.indexOf(a.id), bi = order.indexOf(b.id);
    return (ai < 0 ? items.findIndex((x) => x.id === a.id) : ai) - (bi < 0 ? items.findIndex((x) => x.id === b.id) : bi);
  });
  function loadOrder() {
    try { const raw = localStorage.getItem(storageKey); if (raw) { const parsed = JSON.parse(raw); if (Array.isArray(parsed)) order = parsed.filter((x) => typeof x === "string"); } } catch { order = []; }
  }
  function saveOrder() { try { localStorage.setItem(storageKey, JSON.stringify(order)); } catch { /* storage can be disabled */ } }
  function move(from: string, to: string) {
    const ids = ordered.map((x) => x.id); const source = ids.indexOf(from), target = ids.indexOf(to);
    if (source < 0 || target < 0 || source === target) return;
    ids.splice(target, 0, ids.splice(source, 1)[0]); order = ids; saveOrder();
  }
  loadOrder();
</script>
<div class="filter-stack" role="toolbar">
  {#each ordered as item (item.id)}
    <button class:active={active === item.id} draggable="true" aria-pressed={active === item.id} style={`--status-color:${item.color || '#596273'}`} ondragstart={(event) => { dragging = item.id; event.dataTransfer?.setData("text/plain", item.id); }} ondragover={(event) => event.preventDefault()} ondrop={(event) => { event.preventDefault(); const from = event.dataTransfer?.getData("text/plain") || dragging; move(from, item.id); dragging = ""; }} ondragend={() => dragging = ""} onclick={() => onSelect(active === item.id ? "" : item.id)}><span class="grip" aria-hidden="true">⠿</span><b>{item.count}</b> {item.shortLabel || item.label}</button>
  {/each}
</div>
<style>
 .filter-stack{display:flex;align-items:center;gap:5px;overflow:auto;padding:2px 0;scrollbar-width:thin;flex:1}.filter-stack button{flex:none;min-height:29px;padding:4px 5px;border:0;border-radius:6px;background:transparent;color:var(--status-color);font-size:12px;white-space:nowrap}.filter-stack button+button::before{content:'·';color:#929baa;margin-right:7px}.filter-stack button b{font-weight:750}.filter-stack button.active{background:color-mix(in srgb,var(--status-color) 12%,white);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--status-color) 32%,white)}.filter-stack button:hover{background:color-mix(in srgb,var(--status-color) 8%,white)}.filter-stack .grip{color:#a0a8b4;font-size:12px;cursor:grab}
</style>
