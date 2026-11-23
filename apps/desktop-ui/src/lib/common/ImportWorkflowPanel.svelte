<script lang="ts">
  import Tooltip from "./Tooltip.svelte";
  import { onMount } from "svelte";
  export let open = false;
  export let title = "";
  export let closeLabel = "Close";
  export let onClose: () => void = () => {};
  let panel: HTMLDivElement;
  onMount(() => {
    const keydown = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || !open || !panel) return;
      const layers = Array.from(document.querySelectorAll<HTMLElement>("[role=dialog][aria-modal=true]"));
      const top = layers[layers.length - 1];
      if (top !== panel) return;
      event.preventDefault(); event.stopImmediatePropagation(); onClose();
    };
    window.addEventListener("keydown", keydown, true);
    return () => window.removeEventListener("keydown", keydown, true);
  });
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}
  >
    <div class="panel" bind:this={panel} role="dialog" aria-modal="true" aria-label={title}>
      <header>
        <h2>{title}</h2>
        <Tooltip text={closeLabel}><button class="close" onclick={onClose} aria-label={closeLabel}>
          ×
        </button></Tooltip>
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
