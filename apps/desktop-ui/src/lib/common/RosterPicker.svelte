<script lang="ts">
 export let roster:any[]=[]; export let value:number|undefined=undefined; export let language='en'; export let disabled=false; export let placeholder=''; export let onSelect:(id:number)=>void=()=>{};
 let query=''; let open=false; let highlighted=0; let host:HTMLElement;
 const t=(zh:string,en:string)=>language==='zh-CN'?zh:en;
 $: filtered=roster.filter(p=>p.active!==false&&(!query||`${p.canonicalName} ${p.pinyin||''} ${p.personId}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()))).slice(0,8);
 $: selected=roster.find(p=>Number(p.personId)===Number(value));
 function close(){open=false;highlighted=0;}
 function choose(p:any){onSelect(Number(p.personId));query=p.canonicalName;close();}
 function keydown(e:KeyboardEvent){if(!open){if(e.key==='ArrowDown'||e.key==='Enter'){open=true;highlighted=0;e.preventDefault();}return;}if(e.key==='ArrowDown'){highlighted=Math.min(highlighted+1,Math.max(0,filtered.length-1));e.preventDefault();}else if(e.key==='ArrowUp'){highlighted=Math.max(0,highlighted-1);e.preventDefault();}else if(e.key==='Enter'&&filtered[highlighted]){choose(filtered[highlighted]);e.preventDefault();}else if(e.key==='Escape'){close();e.preventDefault();}}
 function outside(e:MouseEvent){if(host&&!host.contains(e.target as Node))close();}
 import { onMount,onDestroy } from 'svelte'; onMount(()=>document.addEventListener('click',outside)); onDestroy(()=>document.removeEventListener('click',outside));
</script>
<div class="picker" bind:this={host}>
 <input role="combobox" aria-expanded={open} aria-controls="roster-options" aria-autocomplete="list" aria-label={placeholder||t('搜索花名册','Search roster')} placeholder={placeholder||t('姓名、拼音或编号','Name, pinyin, or ID')} disabled={disabled} value={open?query:(selected?.canonicalName||query)} onfocus={()=>{if(!disabled)open=true;}} oninput={(e)=>{query=(e.currentTarget as HTMLInputElement).value;open=true;highlighted=0;}} onkeydown={keydown}/>
 {#if open}<ul id="roster-options" role="listbox">{#if !filtered.length}<li class="empty">{t('未找到结果','No results')}</li>{:else}{#each filtered as p,i}<li role="option" aria-selected={i===highlighted} class:active={i===highlighted} onclick={()=>choose(p)} onkeydown={(e)=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();choose(p);}}} onmousedown={(e)=>e.preventDefault()}>{p.canonicalName}<small>{p.pinyin||''} · #{p.personId}</small></li>{/each}{/if}</ul>{/if}
</div>
<style>.picker{position:relative;width:100%;min-width:160px}.picker input{box-sizing:border-box;width:100%;padding:8px;border:1px solid #d8dce5;border-radius:6px;font:inherit}ul{position:absolute;z-index:5;left:0;right:0;max-height:220px;overflow:auto;margin:4px 0;padding:4px;list-style:none;background:white;border:1px solid #d8dce5;border-radius:6px;box-shadow:0 4px 12px #0002}li{display:flex;justify-content:space-between;padding:7px;cursor:pointer}li.active,li:hover{background:#edf3ff}.empty{color:#78818f;cursor:default}small{color:#78818f}</style>
