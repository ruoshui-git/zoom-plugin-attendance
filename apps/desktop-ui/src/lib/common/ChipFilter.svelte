<script lang="ts">
  export let label = ""; export let selected = false; export let onClick: () => void = () => {};
</script>
<button class:active={selected} aria-pressed={selected} onclick={onClick}>{label}</button>
<style>button{min-height:26px;padding:4px 8px;border:1px solid var(--line);border-radius:16px;background:white;color:#596273;font-size:10px}button.active{border-color:#aac4f5;background:#edf4ff;color:#245caa}</style>
