<script lang="ts">
  export let labels: { all: string; json: string; attendance: string; manual: string };
  export let onExport: (kind: string) => void = () => {};
  let open = false;
</script>
<div class="menu"><button aria-expanded={open} onclick={() => open = !open}>{labels.all}⌄</button>{#if open}<div class="items"><button onclick={() => { open = false; onExport('all'); }}>{labels.all}</button><button onclick={() => { open = false; onExport('json'); }}>{labels.json}</button><button onclick={() => { open = false; onExport('summary'); }}>{labels.attendance}</button><button onclick={() => { open = false; onExport('manual'); }}>{labels.manual}</button></div>{/if}</div>
<style>.menu{position:relative}.items{position:absolute;z-index:4;right:0;top:calc(100% + 4px);min-width:190px;padding:4px;background:var(--surface);border:1px solid var(--line);border-radius:8px;box-shadow:0 6px 18px #1822301c}.items button{width:100%;justify-content:flex-start;border:0;font-size:11px;white-space:nowrap}</style>
