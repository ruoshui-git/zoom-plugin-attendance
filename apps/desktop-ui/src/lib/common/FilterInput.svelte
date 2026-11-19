<script lang="ts">export let value=''; export let label='Filter'; export let placeholder=label; export let onInput: (value:string)=>void=()=>{};</script>
<label class="filter-input"><span>{label}</span><input bind:value {placeholder} oninput={()=>onInput(value)}/></label>
<style>.filter-input{display:flex;align-items:center;gap:6px;color:var(--muted);font-size:11px}.filter-input span{position:absolute;clip:rect(0 0 0 0);clip-path:inset(50%);width:1px;height:1px;overflow:hidden}</style>
