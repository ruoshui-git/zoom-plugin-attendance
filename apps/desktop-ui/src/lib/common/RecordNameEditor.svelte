<script lang="ts">
  import { saveRecordState } from '../api';
  import { runBusy } from '../busy';
  import { toast } from '../toast';
  import { t } from '../ui';
  import Tooltip from './Tooltip.svelte';
  export let target: 'live' | 'viewed' = 'live';
  export let customName = '';
  let editing = false;
  let draft = '';
  let input: HTMLInputElement;
  function begin() { draft = customName || ''; editing = true; setTimeout(() => input?.focus(), 0); }
  function cancel() { editing = false; draft = customName || ''; }
  async function save() {
    if (!editing) return;
    editing = false;
    const name = draft.trim().slice(0, 100);
    if (name === (customName || '')) return;
    await runBusy(`record-name-${target}`, async () => {
      try { await saveRecordState(name ? { customName: name } : { customName: undefined }, target); toast.success(t('已保存记录名称','Record name saved')); }
      catch (error: any) { toast.error(`${t('保存失败：','Save failed: ')}${error?.message || error}`); }
    });
  }
  function keydown(event: KeyboardEvent) { if (event.key === 'Escape') { event.preventDefault(); cancel(); } else if (event.key === 'Enter') { event.preventDefault(); void save(); } }
</script>
{#if editing}
  <input bind:this={input} bind:value={draft} maxlength="100" aria-label={t('记录名称','Record name')} onkeydown={keydown} onblur={() => void save()} />
{:else}
  <Tooltip text={t('重命名记录','Rename record')}><button type="button" class="rename" aria-label={t('重命名记录','Rename record')} onclick={begin}>✎</button></Tooltip>
{/if}
<style>.rename{border:0;background:transparent;color:var(--muted);padding:2px 5px;min-height:24px;cursor:pointer}.rename:hover{color:var(--accent)}</style>
