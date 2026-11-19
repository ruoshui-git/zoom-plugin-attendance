<script lang="ts">
  export let items: { id: string; label: string; count: number }[] = [];
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
    <button class:active={active === item.id} draggable="true" aria-pressed={active === item.id} ondragstart={(event) => { dragging = item.id; event.dataTransfer?.setData("text/plain", item.id); }} ondragover={(event) => event.preventDefault()} ondrop={(event) => { event.preventDefault(); const from = event.dataTransfer?.getData("text/plain") || dragging; move(from, item.id); dragging = ""; }} ondragend={() => dragging = ""} onclick={() => onSelect(active === item.id ? "" : item.id)}><span class="grip" aria-hidden="true">⠿</span>{item.label}<small>{item.count}</small></button>
  {/each}
</div>
<style>
 .filter-stack{display:flex;gap:5px;overflow:auto;padding:2px 0 6px;scrollbar-width:thin}.filter-stack button{flex:none;min-height:29px;padding:4px 8px;border:1px solid transparent;border-radius:6px;background:transparent;color:#596273;font-size:11px;white-space:nowrap}.filter-stack button.active{color:#204d84;background:#edf4ff;border-color:#d9e6fb}.filter-stack button:hover{background:#f3f6fa}.filter-stack .grip{color:#a0a8b4;font-size:12px;cursor:grab}.filter-stack small{font-size:10px;color:#788393;font-weight:600}
</style>
