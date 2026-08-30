<script lang="ts">
import type { Snippet } from "svelte";
import { X } from "@lucide/svelte";
import { ENTITY_ACTIONS, MUTATION_STATUS } from "./vocabulary.ts";
import { trapModalTab } from "$lib/shell/modalFocus";

export type IdentityTypeGroup = { heading: string; types: string[] };

let {
  entityName,
  name = $bindable(""),
  entityType = $bindable<string | null>(null),
  originalType = null,
  typeGroups,
  typeLabel,
  workspaceLabel,
  warning = null,
  busy = false,
  allowUncategorized = false,
  mutation,
  onSave,
  onClose,
}: {
  entityName: string;
  name?: string;
  entityType?: string | null;
  originalType?: string | null;
  typeGroups: IdentityTypeGroup[];
  typeLabel: (type: string | null) => string;
  workspaceLabel: (type: string | null) => string;
  warning?: string | null;
  busy?: boolean;
  allowUncategorized?: boolean;
  mutation?: Snippet;
  onSave: () => void;
  onClose: () => void;
} = $props();

let dialogEl = $state<HTMLElement | null>(null);
const unchanged = $derived(name.trim() === entityName && (entityType ?? null) === (originalType ?? null));

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && !busy) {
    event.preventDefault();
    onClose();
    return;
  }
  if (event.key === "Enter" && !busy && name.trim() && !unchanged) {
    const target = event.target as HTMLElement | null;
    if (target?.tagName === "INPUT") {
      event.preventDefault();
      onSave();
    }
  }
  trapModalTab(event, dialogEl);
}
</script>

<div
  class="modal-backdrop"
  role="presentation"
  onclick={() => {
    if (!busy) onClose();
  }}
  onkeydown={(event) => {
    if (event.key === "Escape" && !busy) onClose();
  }}
  tabindex="-1">
  <div
    class="dialog entity-identity-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="entity-identity-title"
    tabindex="-1"
    bind:this={dialogEl}
    onclick={(event) => event.stopPropagation()}
    onkeydown={onKeydown}>
    <div class="new-form-heading">
      <div>
        <span class="panel-kicker">{ENTITY_ACTIONS.editIdentity.toUpperCase()}</span>
        <strong id="entity-identity-title">{ENTITY_ACTIONS.editIdentity}: {entityName}</strong>
      </div>
      <button type="button" class="new-form-close" aria-label="Close edit dialog" onclick={onClose} disabled={busy}
        ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
    </div>
    <p class="dialog-body-copy">Change the name and type. The stable ID stays the same, so links and assets follow.</p>
    <label class="create-input-field" for="entity-edit-name"
      ><span>Name</span><input
        id="entity-edit-name"
        type="text"
        bind:value={name}
        placeholder="Entity name"
        disabled={busy} /></label>
    <label class="create-input-field" for="entity-edit-type"
      ><span>Type</span><select
        id="entity-edit-type"
        class="entity-edit-select"
        bind:value={entityType}
        disabled={busy}
        aria-label="Entity type">
        {#if allowUncategorized || originalType == null}<option value={null}>Uncategorized — no template</option>{/if}
        {#each typeGroups as group}
          <optgroup label={group.heading.toUpperCase()}>
            {#each group.types as type}<option value={type}>{typeLabel(type)}</option>{/each}
          </optgroup>
        {/each}
      </select>
      <small class="field-hint">Workspace: {workspaceLabel(entityType)}</small></label>
    {#if warning}<p class="plugin-warning entity-edit-warning" role="note">{warning}</p>{/if}
    {#if mutation}
      <div class="identity-mutation">
        {@render mutation()}
      </div>
    {/if}
    <div class="new-form-actions">
      <button type="button" class="quiet-button" onclick={onClose} disabled={busy}>Cancel</button>
      <button type="button" class="primary-button" onclick={onSave} disabled={busy || !name.trim() || unchanged}
        >{busy ? MUTATION_STATUS.saving : "Save"}</button>
    </div>
  </div>
</div>

<style>
.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(28, 26, 22, 0.42);
}
.dialog {
  width: min(440px, 100%);
  max-height: min(90vh, 720px);
  overflow: auto;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.new-form-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.16em;
}
.new-form-heading strong {
  display: block;
  margin-top: 4px;
  color: var(--ink);
  font: 500 20px/1.2 var(--font-display);
}
.new-form-close {
  display: grid;
  width: 32px;
  height: 32px;
  place-items: center;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
}
.dialog-body-copy {
  margin: 0 0 14px;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.5;
}
.create-input-field {
  display: grid;
  gap: 6px;
  margin-bottom: 12px;
  color: var(--ink-soft);
  font-size: 11px;
}
.create-input-field input,
.entity-edit-select {
  min-height: 36px;
  padding: 0 10px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
}
.field-hint {
  color: var(--ink-faint);
}
.entity-edit-warning {
  margin: 0 0 12px;
}
.identity-mutation {
  margin-bottom: 12px;
}
.new-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 8px;
}
.quiet-button,
.primary-button {
  min-height: 34px;
  padding: 0 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
}
.quiet-button {
  border: 1px solid var(--line-strong);
  background: var(--surface);
  color: var(--ink-soft);
}
.primary-button {
  border: 1px solid var(--accent-dark);
  background: var(--accent-dark);
  color: var(--on-accent);
}
.quiet-button:disabled,
.primary-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
