<script lang="ts">
  import { onMount } from 'svelte';
  export let open = false;
  export let title = '';
  export let message = '';
  export let confirmLabel = '';
  export let cancelLabel = '';
  export let destructive = false;
  export let onResolve: (value: boolean) => void = () => {};
  function resolve(value: boolean) { onResolve(value); }
  onMount(() => {
    const listener = (event: Event) => { const e = event as CustomEvent; open=true; title=e.detail.title; message=e.detail.message; confirmLabel=e.detail.confirmLabel; cancelLabel=e.detail.cancelLabel; destructive=!!e.detail.destructive; onResolve=e.detail.resolve; };
    window.addEventListener('attendflow:confirm', listener);
    return () => window.removeEventListener('attendflow:confirm', listener);
  });
</script>
{#if open}<div class="confirm-backdrop" role="presentation" onclick={(e)=>{if(e.target===e.currentTarget)resolve(false)}}><div class="dialog" role="dialog" aria-modal="true" aria-labelledby="confirm-title"><h2 id="confirm-title">{title}</h2><p>{message}</p><footer><button onclick={()=>resolve(false)}>{cancelLabel}</button><button class:danger={destructive} class="primary" onclick={()=>resolve(true)}>{confirmLabel}</button></footer></div></div>{/if}
<style>.confirm-backdrop{position:fixed;inset:0;z-index:100;display:grid;place-items:center;padding:20px;background:#17203366}.dialog{width:min(420px,100%);padding:20px;border:1px solid var(--line);border-radius:var(--radius-lg);background:var(--surface);box-shadow:0 12px 32px #17203333}.dialog h2{margin:0;font-size:16px}.dialog p{color:var(--muted);font-size:13px;line-height:1.5}.dialog footer{display:flex;justify-content:flex-end;gap:8px;margin-top:18px}</style>
