<script lang="ts">
import { confirmState, resolveConfirm } from "./confirm.svelte";

let confirmButton: HTMLButtonElement | undefined = $state();
let lastFocused: Element | null = null;

$effect(() => {
  if (!confirmState.open) return;
  lastFocused = document.activeElement;
  const frame = window.requestAnimationFrame(() => confirmButton?.focus());
  return () => window.cancelAnimationFrame(frame);
});

$effect(() => {
  if (!confirmState.open) return;
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      resolveConfirm(false);
    }
  };
  window.addEventListener("keydown", onKey, true);
  return () => window.removeEventListener("keydown", onKey, true);
});

function settle(value: boolean) {
  if (!confirmState.open) return;
  resolveConfirm(value);
  const focused = lastFocused;
  lastFocused = null;
  window.requestAnimationFrame(() => {
    if (focused instanceof HTMLElement && focused.isConnected) focused.focus();
  });
}
</script>

{#if confirmState.open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="language-modal-backdrop" role="presentation" onclick={() => settle(false)}>
    <div
      class="language-modal"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}>
      <h3 id="confirm-title">{confirmState.title}</h3>
      <p>{confirmState.message}</p>
      <div class="language-modal-actions">
        <button type="button" class="language-button secondary" onclick={() => settle(false)}>Cancel</button>
        <button
          type="button"
          class="language-button secondary language-danger"
          bind:this={confirmButton}
          onclick={() => settle(true)}>Confirm</button>
      </div>
    </div>
  </div>
{/if}

<style>
.language-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(37, 37, 31, 0.28);
}
.language-modal {
  width: min(400px, 100%);
  padding: 22px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 22px 70px rgba(37, 37, 31, 0.2);
}
.language-modal h3 {
  margin: 0 0 10px;
  font-size: 18px;
}
.language-modal p {
  margin: 0 0 18px;
  color: var(--ink-soft);
  font-size: 13px;
  line-height: 1.55;
}
.language-modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.language-button {
  padding: 8px 12px;
  border: 1px solid var(--accent-dark);
  border-radius: 8px;
  background: var(--accent-dark);
  color: #fff;
  cursor: pointer;
}
.language-button:hover {
  filter: brightness(1.06);
}
.language-button.secondary {
  background: transparent;
  color: var(--accent-dark);
}
.language-button.secondary:hover {
  background: var(--surface-muted);
}
.language-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  filter: none;
}
.language-danger {
  border-color: var(--danger) !important;
  color: var(--danger) !important;
  background: transparent;
}
</style>
