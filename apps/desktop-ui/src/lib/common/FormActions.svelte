<script lang="ts">export let submitLabel='Save'; export let cancelLabel='Cancel'; export let onSubmit:()=>void=()=>{}; export let onCancel:()=>void=()=>{}; export let disabled=false;</script>
<div class="actions"><button class="primary" {disabled} onclick={onSubmit}>{submitLabel}</button><button {disabled} onclick={onCancel}>{cancelLabel}</button></div>
