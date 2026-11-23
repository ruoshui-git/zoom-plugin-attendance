<script lang="ts">
  import { onMount } from "svelte";
  let dialog: HTMLDivElement;
  let cancelButton: HTMLButtonElement;
  let confirmButton: HTMLButtonElement;
  let previousFocus: HTMLElement | null = null;

  type Request = {
    title: string;
    message: string;
    confirmLabel: string;
    cancelLabel: string;
    destructive: boolean;
    resolve: (value: boolean) => void;
  };

  let open = false;
  let title = "";
  let message = "";
  let confirmLabel = "";
  let cancelLabel = "";
  let destructive = false;
  const queue: Request[] = [];

  function showNext() {
    const next = queue.shift();
    if (!next) {
      open = false;
      return;
    }
    title = next.title;
    message = next.message;
    confirmLabel = next.confirmLabel;
    cancelLabel = next.cancelLabel;
    destructive = next.destructive;
    activeResolve = next.resolve;
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    open = true;
    queueMicrotask(() => (destructive ? cancelButton : confirmButton)?.focus());
  }

  let activeResolve: ((value: boolean) => void) | null = null;

  function resolve(value: boolean) {
    if (!open || !activeResolve) return;
    const finish = activeResolve;
    activeResolve = null;
    open = false;
    queueMicrotask(() => previousFocus?.focus());
    finish(value);
    queueMicrotask(showNext);
  }

  onMount(() => {
    const listener = (event: Event) => {
      const detail = (event as CustomEvent<Request>).detail;
      queue.push(detail);
      if (!open) showNext();
    };
    window.addEventListener("attendflow:confirm", listener);
    const keyboard = (event: KeyboardEvent) => {
      // An open confirm dialog is always the top layer, even though it sits earlier in
      // the DOM than the slide-overs and import panels that open it.
      if (!open || !dialog) return;
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopImmediatePropagation();
        resolve(false);
      } else if (event.key === "Enter" && document.activeElement !== cancelButton) {
        event.preventDefault();
        resolve(true);
      } else if (event.key === "Tab") {
        event.preventDefault();
        (document.activeElement === cancelButton ? confirmButton : cancelButton)?.focus();
      }
    };
    window.addEventListener("keydown", keyboard, true);
    return () => { window.removeEventListener("attendflow:confirm", listener); window.removeEventListener("keydown", keyboard, true); };
  });
</script>

{#if open}
  <div
    class="confirm-backdrop"
    role="presentation"
    onclick={(event) => {
      if (event.target === event.currentTarget) resolve(false);
    }}
  >
    <div
      class="dialog"
      role="dialog"
      bind:this={dialog}
      aria-modal="true"
      aria-labelledby="confirm-title"
    >
      <h2 id="confirm-title">{title}</h2>
      <p>{message}</p>
      <footer>
        <button bind:this={cancelButton} onclick={() => resolve(false)}>{cancelLabel}</button>
        <button
          class="primary"
          class:danger={destructive}
          bind:this={confirmButton}
          onclick={() => resolve(true)}
        >
          {confirmLabel}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 20px;
    background: #17203366;
  }
  .dialog {
    width: min(420px, 100%);
    padding: 20px;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 12px 32px #17203333;
  }
  .dialog h2 {
    margin: 0;
    font-size: 16px;
  }
  .dialog p {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.5;
  }
  .dialog footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }
</style>
