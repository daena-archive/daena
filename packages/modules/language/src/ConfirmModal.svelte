<script lang="ts">
import { confirmState, resolveConfirm } from "./confirm.svelte";
</script>

{#if confirmState.open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="language-modal-backdrop" role="presentation" onclick={() => resolveConfirm(false)}>
    <div class="language-modal" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <h3 id="confirm-title">{confirmState.title}</h3>
      <p>{confirmState.message}</p>
      <div class="language-modal-actions">
        <button type="button" class="language-button secondary" onclick={() => resolveConfirm(false)}>Cancel</button>
        <button type="button" class="language-button secondary language-danger" onclick={() => resolveConfirm(true)}>Confirm</button>
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
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 14px;
  background: var(--surface, #fffefa);
  box-shadow: 0 22px 70px rgba(37, 37, 31, 0.2);
}
.language-modal h3 {
  margin: 0 0 10px;
  font-size: 18px;
}
.language-modal p {
  margin: 0 0 18px;
  color: var(--ink-soft, #77766d);
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
  border: 1px solid var(--accent-dark, #365342);
  border-radius: 8px;
  background: var(--accent-dark, #365342);
  color: #fff;
  cursor: pointer;
}
.language-button:hover {
  filter: brightness(1.06);
}
.language-button.secondary {
  background: transparent;
  color: var(--accent-dark, #365342);
}
.language-button.secondary:hover {
  background: var(--surface-muted, #f4f2ec);
}
.language-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  filter: none;
}
.language-danger {
  border-color: #a14f42 !important;
  color: #a14f42 !important;
  background: transparent;
}
</style>
