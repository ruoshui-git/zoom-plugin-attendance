<script lang="ts">
  export let open = false;
  export let title = "";
  export let closeLabel = "Close";
  export let onClose: () => void = () => {};
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}
  >
    <div class="panel" role="dialog" aria-modal="true" aria-label={title}>
      <header>
        <h2>{title}</h2>
        <button class="close" onclick={onClose} aria-label={closeLabel}>
          ×
        </button>
      </header>
      <slot />
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    justify-content: flex-end;
    background: #17203366;
  }
  .panel {
    width: min(760px, 96vw);
    height: 100%;
    overflow: auto;
    padding: 26px;
    background: var(--surface);
    box-shadow: -8px 0 28px #17203322;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 18px;
  }
  :global(.panel textarea) {
    width: 100%;
    height: 180px;
    margin: 10px 0;
    padding: 12px;
    border: 1px solid #dce0e7;
    border-radius: 7px;
  }
  .close {
    border: 0;
    background: transparent;
    font-size: 24px;
  }
</style>
