<script lang="ts">export let message=''; export let tone='success';</script>
{#if message}<div class="toast" class:error={tone==='error'} role="status" aria-live="polite">{message}</div>{/if}
