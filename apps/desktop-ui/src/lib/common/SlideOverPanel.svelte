<script lang="ts">
  export let open = false;
  export let title = '';
  export let closeLabel = '';
  export let width = 720;
  export let onClose: () => void = () => {};
  function handleKeydown(event: KeyboardEvent) { if (event.key === 'Escape' && open && !event.defaultPrevented) onClose(); }
</script>
<svelte:window on:keydown={handleKeydown} />
{#if open}<div class="scrim"><button class="backdrop" aria-label={closeLabel} onclick={onClose}></button><div class="panel" role="dialog" aria-modal="true" tabindex="0" style={`width:min(${width}px,94vw)`} aria-label={title}><header><h2>{title}</h2><button aria-label={closeLabel} onclick={onClose}>×</button></header><div class="body"><slot /></div></div></div>{/if}
<style>
  .scrim{position:fixed;inset:0;z-index:20;background:rgba(24,32,48,.5);display:flex;justify-content:flex-end}.backdrop{position:absolute;inset:0;width:100%;height:100%;border:0;border-radius:0;background:transparent}.panel{position:relative;margin:0;height:100%;display:flex;flex-direction:column;background:var(--surface);border:0;padding:0;box-shadow:-10px 0 32px #17203322}.panel header{padding:15px 20px;margin:0;border-bottom:1px solid var(--line);display:flex;align-items:center;justify-content:space-between}.panel h2{font-size:16px;margin:0}.body{overflow:auto;padding:18px 20px;flex:1}
</style>
