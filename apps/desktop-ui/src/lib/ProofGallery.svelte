<script lang="ts">
  import { onMount } from 'svelte';
  import { t, dateTime } from './ui';
  import { proofImageUrl } from './proof';
  export let captures: any[] = [];
  export let instanceId = '';
  let selected: any = null;
  function keydown(event: KeyboardEvent) { if (event.key === 'Escape') selected = null; }
  onMount(() => { window.addEventListener('keydown', keydown); return () => window.removeEventListener('keydown', keydown); });
</script>

<p class="note">{t("实验性功能，不保证可用：会把你自己的 Zoom 切换到图库视图逐页截图，然后恢复原视图。截图只保存在本机。", "Experimental, not guaranteed to work: switches your own Zoom view to gallery, captures each page, then restores your view. Images stay on this computer.")}</p>
{#each captures.slice().reverse() as capture}
  <section><h3>{dateTime(capture.startedAt)} · {capture.pages?.length || 0} {t('页','pages')}</h3>
    {#if capture.error}<p>{capture.error}</p>{/if}
    <div class="grid">{#each capture.pages || [] as page, i}
      <button class="card" onclick={() => selected = { capture, page, i }}><img loading="lazy" src={proofImageUrl(instanceId, page.file)} alt={t(`第 ${i+1}/${capture.pages.length} 页截图`, `Page ${i+1}/${capture.pages.length} screenshot`)}/><b>{dateTime(page.at || capture.startedAt)}</b><small>{t(`第 ${i+1}/${capture.pages.length} 页`, `Page ${i+1}/${capture.pages.length}`)}</small><small>{t(`${page.cameraOnCount || 0} 人开摄像头 · 暗色方格估计 ${page.darkTiles ?? 0}`, `${page.cameraOnCount || 0} cameras on · ${page.darkTiles ?? 0} estimated dark tiles`)}</small></button>
    {/each}</div>
  </section>
{/each}
{#if selected}<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions --><div class="scrim" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) selected = null; }}><div class="dialog" role="dialog" aria-modal="true" aria-label={t('截图详情','Proof image details')}><button class="close" aria-label={t('关闭','Close')} onclick={() => selected = null}>×</button><img class="full" src={proofImageUrl(instanceId, selected.page.file)} alt={t('完整截图','Full screenshot')}/><aside><h3>{dateTime(selected.page.at || selected.capture.startedAt)} · {t(`第 ${selected.i+1}/${selected.capture.pages.length} 页`, `Page ${selected.i+1}/${selected.capture.pages.length}`)}</h3>{#each selected.page.participants || [] as participant}<p>{participant.name || participant.screenName || '—'} · {participant.videoOn ? t('摄像头开启','Camera on') : t('摄像头关闭','Camera off')}</p>{:else}<p>{t('此页没有参与者明细。','No participant details for this page.')}</p>{/each}</aside></div></div>{/if}

<style>
  .note{font-size:11px;color:var(--muted)}.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:10px}.card{display:grid;text-align:left;gap:4px;padding:7px;background:white;border:1px solid var(--line);border-radius:8px}.card img{width:100%;aspect-ratio:16/9;object-fit:cover}.card small{font-size:10px;color:var(--muted)}.scrim{position:fixed;inset:0;z-index:1000;background:#111b;display:grid;place-items:center;padding:20px}.dialog{position:relative;display:grid;grid-template-columns:minmax(0,1fr) 280px;gap:14px;width:min(1100px,96vw);max-height:92vh;overflow:auto;background:white;border-radius:10px;padding:16px}.full{max-width:100%;max-height:80vh;object-fit:contain;margin:auto}.close{position:absolute;right:8px;top:8px}@media(max-width:700px){.dialog{grid-template-columns:1fr}}
</style>
