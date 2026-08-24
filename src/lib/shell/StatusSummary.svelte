<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  visible: boolean;
  loading: boolean;
  loadError: string;
  saving: boolean;
  saveError: string;
  dirty: boolean;
  savedAt: string;
  actions?: Snippet;
  onRetryLoad: () => void;
  onRetrySave: () => void;
}

let { visible, loading, loadError, saving, saveError, dirty, savedAt, actions, onRetryLoad, onRetrySave }: Props =
  $props();
</script>

{#if visible}
  <div class="editor-status" role="status" aria-live="polite">
    {#if loading}
      <span class="status-dot status-saving"></span> Loading…
    {:else if loadError}
      <span class="status-dot status-warning"></span><span title={loadError}>Could not load entry</span>
      <button class="quiet-button" type="button" onclick={onRetryLoad}>Retry</button>
    {:else if saving}
      <span class="status-dot status-saving"></span> Saving…
    {:else if saveError}
      <span class="status-dot status-warning"></span><span title={saveError}>Save paused — retrying</span>
      <button class="quiet-button" type="button" onclick={onRetrySave}>Retry now</button>
    {:else if dirty}
      <span class="status-dot status-warning"></span> Unsaved changes
    {:else if savedAt}
      <span class="status-saved">✓</span> Saved {savedAt}
    {/if}
    {#if actions}{@render actions()}{/if}
  </div>
{/if}

<style>
.editor-status {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 4px;
  color: var(--ink-faint);
  font-size: 11px;
}
.status-dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  margin-right: 1px;
  border-radius: 50%;
}
.status-saving,
.status-warning {
  background: #d6a35f;
}
.status-saved {
  margin-right: 1px;
  color: var(--theme-success-text, #6fa276);
}
.quiet-button {
  padding: 10px 12px;
  border: 1px solid var(--theme-warning-border, #ded8cd);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  color: var(--ink-soft);
  cursor: pointer;
  font-size: 12px;
}
.quiet-button:hover {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--surface-muted);
  color: var(--ink);
}
.quiet-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
@media (max-width: 760px) {
  .editor-status {
    flex-wrap: wrap;
  }
}
</style>
