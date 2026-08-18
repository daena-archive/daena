<script lang="ts">
import { dialogState, resolveDialog } from "./dialogs.svelte";

let promptInput = $state<HTMLInputElement | null>(null);
let confirmButton = $state<HTMLButtonElement | null>(null);
let lastFocused: Element | null = null;

  $effect(() => {
    const open = dialogState.open;
    const current = dialogState.current;
    if (!open || !current) return;
    lastFocused = document.activeElement;
    const frame = window.requestAnimationFrame(() => {
      if (current.kind === "prompt") promptInput?.focus();
      else confirmButton?.focus();
    });
    return () => window.cancelAnimationFrame(frame);
  });

$effect(() => {
  if (!dialogState.open) return;
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      resolveDialog(null);
    }
  };
  window.addEventListener("keydown", onKey, true);
  return () => window.removeEventListener("keydown", onKey, true);
});

  function settle(value: boolean | string | null) {
    resolveDialog(value);
    if (dialogState.open) return;
    const focused = lastFocused;
    lastFocused = null;
    window.requestAnimationFrame(() => {
      if (focused instanceof HTMLElement && focused.isConnected) focused.focus();
    });
  }

function submitPrompt() {
  const value = promptInput?.value ?? "";
  settle(value);
}
</script>

{#if dialogState.open && dialogState.current}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="dialog-backdrop" role="presentation" onclick={() => settle(null)}>
    <div
      class="dialog-card"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="shared-dialog-title"
      aria-describedby="shared-dialog-message"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}>
      <div class="dialog-heading">
        <div>
          <span class="panel-kicker">DAENA</span><strong id="shared-dialog-title">{dialogState.current.title}</strong>
        </div>
        <button type="button" class="dialog-close" aria-label="Cancel dialog" onclick={() => settle(null)}>×</button>
      </div>
      <p id="shared-dialog-message" class="dialog-message">{dialogState.current.message}</p>
      {#if dialogState.current.kind === "prompt"}
        {@const prompt = dialogState.current}
        <input
          class="dialog-input"
          aria-label={prompt.title}
          placeholder={prompt.placeholder}
          value={prompt.value}
          bind:this={promptInput}
          oninput={(event) => (prompt.value = (event.currentTarget as HTMLInputElement).value)}
          onkeydown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              submitPrompt();
            }
          }} />
      {/if}
      <div class="dialog-actions">
        <button type="button" class="dialog-cancel" onclick={() => settle(null)}>Cancel</button>
        <button
          type="button"
          class:dialog-danger={dialogState.current.kind === "confirm" && dialogState.current.danger}
          class="dialog-confirm"
          bind:this={confirmButton}
          onclick={() => {
            if (dialogState.current?.kind === "prompt") submitPrompt();
            else settle(true);
          }}>
          {dialogState.current.confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
.dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(37, 37, 31, 0.28);
}
.dialog-card {
  width: min(440px, 100%);
  margin: 0;
  padding: 22px;
  border: 1px solid #e3d9ca;
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 22px 70px rgba(37, 37, 31, 0.2);
  outline: none;
}
.dialog-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
}
.dialog-heading strong {
  display: block;
  font-size: 20px;
  line-height: 1.25;
}
.dialog-close {
  width: 30px;
  height: 30px;
  flex: none;
  border: 0;
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
}
.dialog-close:hover {
  background: #ebe6dd;
  color: var(--ink);
}
.dialog-message {
  margin: 0;
  color: var(--ink-soft);
  font-size: 13px;
  line-height: 1.55;
}
.dialog-input {
  width: 100%;
  box-sizing: border-box;
  margin-top: 14px;
  padding: 9px 11px;
  border: 1px solid #d9cdbd;
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 13px;
}
.dialog-input:focus-visible {
  outline: 2px solid var(--accent, #365342);
  outline-offset: 1px;
}
.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 7px;
  margin-top: 20px;
}
.dialog-cancel,
.dialog-confirm {
  padding: 9px 14px;
  border: 1px solid var(--accent-dark);
  border-radius: 9px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.dialog-cancel {
  background: transparent;
  color: var(--accent-dark);
}
.dialog-cancel:hover {
  background: var(--surface-muted);
}
.dialog-confirm {
  background: var(--accent-dark);
  color: #fff;
}
.dialog-confirm:hover {
  filter: brightness(1.06);
}
.dialog-confirm.dialog-danger {
  border-color: #a14f42;
  background: #a14f42;
}
.dialog-confirm:focus-visible,
.dialog-cancel:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
</style>
