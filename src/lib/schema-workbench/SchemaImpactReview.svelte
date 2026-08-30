<script lang="ts">
import type { SchemaOverlayPreviewResult } from "$lib/project/client";
import { AlertTriangle, Check, X } from "@lucide/svelte";

let {
  preview,
  busy = false,
  onCancel,
  onConfirm,
}: {
  preview: SchemaOverlayPreviewResult;
  busy?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
} = $props();

const changeKindLabel =
  preview.changeKind === "requires-reassignment"
    ? "Requires reassignment"
    : preview.changeKind === "hiding-only"
      ? "Hides existing schema"
      : "Additive";

const canConfirm = preview.ok && preview.unresolvedTypeRemovals.length === 0;
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="impact-backdrop"
  role="presentation"
  tabindex="-1"
  onclick={onCancel}
  onkeydown={(e) => e.key === "Escape" && onCancel()}>
  <!-- svelte-ignore a11y_autofocus -->
  <div
    class="impact-dialog"
    role="alertdialog"
    aria-modal="true"
    tabindex="-1"
    aria-labelledby="schema-impact-title"
    autofocus
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.key === "Escape" && onCancel()}>
    <div class="dialog-icon" class:warn={!canConfirm} class:ok={canConfirm}>
      {#if canConfirm}
        <Check size={18} strokeWidth={1.8} aria-hidden="true" />
      {:else}
        <AlertTriangle size={18} strokeWidth={1.8} aria-hidden="true" />
      {/if}
    </div>
    <strong id="schema-impact-title">Review schema impact</strong>
    <p>
      This save is <span class="kind">{changeKindLabel}</span>. Confirm the live-data impact before writing the project
      overlay.
    </p>

    {#if preview.errors.length > 0}
      <div class="impact-group">
        <span>Blocking issues</span>
        <ul class="issue-list">
          {#each preview.errors as issue}
            <li>
              <strong>{issue.kind}:{issue.id}</strong>
              {#if issue.property}<code>{issue.property}</code>{/if}
              — {issue.message}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if preview.warnings.length > 0}
      <div class="impact-group">
        <span>Warnings</span>
        <ul class="issue-list">
          {#each preview.warnings as issue}
            <li>
              <strong>{issue.kind}:{issue.id}</strong> — {issue.message}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if preview.affectedTypes.length > 0}
      <div class="impact-group">
        <span>Types</span>
        <ul>
          {#each preview.affectedTypes as item}
            <li>
              <code>{item.entityType}</code>
              <em>{item.change}</em>
              · {item.entityCount}
              {item.entityCount === 1 ? "entity" : "entities"}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if preview.affectedFields.length > 0}
      <div class="impact-group">
        <span>Fields</span>
        <ul>
          {#each preview.affectedFields as item}
            <li>
              <code>{item.fieldKey}</code>
              <em>{item.change}</em>
              · {item.valueCount}
              {item.valueCount === 1 ? "stored value" : "stored values"}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if preview.affectedTemplates.length > 0}
      <div class="impact-group">
        <span>Templates</span>
        <ul>
          {#each preview.affectedTemplates as id}
            <li><code>{id}</code></li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if preview.compatibilityNotes.length > 0}
      <div class="impact-group">
        <span>Notes</span>
        <ul>
          {#each preview.compatibilityNotes as note}
            <li>{note}</li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="impact-actions">
      <button type="button" class="quiet" disabled={busy} onclick={onCancel}
        ><X size={14} strokeWidth={1.8} aria-hidden="true" /> Cancel</button>
      <button type="button" class="primary" disabled={busy || !canConfirm} onclick={onConfirm}>
        {#if busy}Saving…{:else}Confirm save{/if}
      </button>
    </div>
  </div>
</div>

<style>
.impact-backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: grid;
  place-items: center;
  padding: 1.5rem;
  background: color-mix(in oklab, var(--color-bg, #0f1115) 55%, transparent);
}

.impact-dialog {
  width: min(32rem, 100%);
  max-height: min(85vh, 40rem);
  overflow: auto;
  display: grid;
  gap: 0.75rem;
  padding: 1.1rem 1.15rem 1rem;
  border-radius: 12px;
  border: 1px solid color-mix(in oklab, var(--color-border, #3a3f4b) 80%, transparent);
  background: var(--color-panel, #171a21);
  color: var(--color-fg, #e8eaef);
  box-shadow: 0 18px 48px color-mix(in oklab, #000 35%, transparent);
}

.dialog-icon {
  width: 2rem;
  height: 2rem;
  display: grid;
  place-items: center;
  border-radius: 999px;
}

.dialog-icon.warn {
  color: #c47b2d;
  background: color-mix(in oklab, #c47b2d 18%, transparent);
}

.dialog-icon.ok {
  color: #3f8f6b;
  background: color-mix(in oklab, #3f8f6b 18%, transparent);
}

.impact-dialog > strong {
  font-size: 1.05rem;
}

.impact-dialog > p {
  margin: 0;
  color: color-mix(in oklab, var(--color-fg, #e8eaef) 75%, transparent);
  line-height: 1.45;
}

.kind {
  font-weight: 600;
  color: var(--color-fg, #e8eaef);
}

.impact-group {
  display: grid;
  gap: 0.35rem;
}

.impact-group > span {
  font-size: 0.72rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: color-mix(in oklab, var(--color-fg, #e8eaef) 55%, transparent);
}

.impact-group ul,
.issue-list {
  margin: 0;
  padding-left: 1.1rem;
  display: grid;
  gap: 0.25rem;
  font-size: 0.86rem;
  line-height: 1.4;
}

.impact-group code,
.issue-list code {
  font-size: 0.8em;
}

.impact-group em {
  font-style: normal;
  opacity: 0.75;
}

.impact-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.25rem;
}
</style>
