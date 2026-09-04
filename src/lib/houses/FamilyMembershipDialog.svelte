<script lang="ts">
import type { ModuleContext } from "../../../packages/module-api/src/index";
import { X } from "@lucide/svelte";
import { trapModalTab } from "$lib/shell/modalFocus";
import AsyncEntityPicker from "$lib/entity-lifecycle/AsyncEntityPicker.svelte";
import { toAsyncEntityPage } from "$lib/entity-lifecycle/asyncEntityQuery.ts";
import { ENTITY_ACTION_CONFIRM, MUTATION_STATUS, MUTATION_STATUS_MESSAGES } from "$lib/entity-lifecycle/vocabulary.ts";
import { confirmDialog } from "$lib/dialogs.svelte";
import { MEMBERSHIP_RELATIONSHIP, PERSON_TYPE, type HouseMemberRecord } from "./model.ts";
import { defaultMembershipDraft, membershipMetadataFields } from "./membershipFields.ts";
import {
  classifyMutationError,
  createMembership,
  createMinimalPerson,
  deleteFamilyRelationship,
  metadataFingerprint,
  serializeMetadata,
  updateFamilyMetadata,
} from "./mutations.ts";

let {
  context,
  houseId,
  houseName,
  excludeIds = [],
  editing = null,
  initialMode = "link",
  onClose,
  onSaved,
  onRemoved,
}: {
  context: ModuleContext;
  houseId: string;
  houseName: string;
  excludeIds?: string[];
  editing?: HouseMemberRecord | null;
  initialMode?: "link" | "create";
  onClose: () => void;
  onSaved: () => void;
  onRemoved?: () => void;
} = $props();

const fields = $derived(membershipMetadataFields(context));
// svelte-ignore state_referenced_locally: initial mode reflects initial props, kept in sync via effect
let mode = $state<"link" | "create">(editing ? "link" : initialMode);
// svelte-ignore state_referenced_locally
let selectedPersonId = $state<string | null>(editing?.personId ?? null);
// svelte-ignore state_referenced_locally
let selectedPersonName = $state(editing?.personName ?? "");
let selectedRevision = $state("");
let createName = $state("");
let draft = $state<Record<string, unknown>>(
  defaultMembershipDraft({
    // svelte-ignore state_referenced_locally
    role: editing?.role,
    // svelte-ignore state_referenced_locally
    customLabel: editing?.customLabel,
    // svelte-ignore state_referenced_locally
    notes: editing?.notes,
  }),
);
// svelte-ignore state_referenced_locally
let observedRevision = $state(editing?.revision ?? "");
let requestId = $state(crypto.randomUUID());
let lastFingerprint = $state("");
let busy = $state(false);
let error = $state("");
let conflict = $state(false);
let currentValues = $state<Record<string, unknown> | null>(null);
let currentRevision = $state("");
let dialogEl = $state<HTMLElement | null>(null);

const isEdit = $derived(Boolean(editing));
const title = $derived(
  isEdit ? `Edit membership · ${editing?.personName ?? "Person"}` : `Add member · ${houseName || "House"}`,
);

$effect(() => {
  if (!editing) return;
  draft = defaultMembershipDraft({
    role: editing.role,
    customLabel: editing.customLabel,
    notes: editing.notes,
  });
  observedRevision = editing.revision;
  selectedPersonId = editing.personId;
  selectedPersonName = editing.personName;
  requestId = crypto.randomUUID();
  lastFingerprint = metadataFingerprint(draft);
  error = "";
  conflict = false;
  currentValues = null;
  currentRevision = "";
});

async function searchPeople(query: {
  text: string;
  offset: number;
  limit: number;
  excludeIds?: string[];
  entityTypes?: string[];
  excludedEntityTypes?: string[];
  sortField?: string;
  sortDirection?: string;
}) {
  const page = await context.entities.query({
    text: query.text || undefined,
    types: [PERSON_TYPE],
    offset: query.offset,
    limit: query.limit,
    sortField: "name",
    sortDirection: "asc",
  });
  return toAsyncEntityPage(page, {
    excludeIds: [...(query.excludeIds ?? []), ...excludeIds],
  });
}

function setDraft(key: string, value: unknown) {
  draft = { ...draft, [key]: value };
}

function requestIdForSave() {
  const next = metadataFingerprint(draft);
  if (next !== lastFingerprint) {
    requestId = crypto.randomUUID();
    lastFingerprint = next;
  }
  return requestId;
}

function reloadCurrent() {
  if (!currentValues) return;
  draft = defaultMembershipDraft({
    role: typeof currentValues.role === "string" ? currentValues.role : null,
    customLabel: typeof currentValues.customLabel === "string" ? currentValues.customLabel : null,
    notes: typeof currentValues.notes === "string" ? currentValues.notes : null,
  });
  if (currentRevision) observedRevision = currentRevision;
  conflict = false;
  error = "";
  requestId = crypto.randomUUID();
  lastFingerprint = metadataFingerprint(draft);
}

async function save() {
  if (busy) return;
  const role = String(draft.role ?? "member");
  if (role === "custom" && !String(draft.customLabel ?? "").trim()) {
    error = "Custom label is required.";
    return;
  }
  busy = true;
  error = "";
  conflict = false;
  try {
    if (isEdit && editing) {
      await updateFamilyMetadata(context, editing.id, draft, observedRevision, requestIdForSave());
      onSaved();
      onClose();
      return;
    }
    let personId = selectedPersonId;
    let revision = selectedRevision;
    if (mode === "create") {
      if (!createName.trim()) {
        error = "Enter a person name.";
        busy = false;
        return;
      }
      const created = await createMinimalPerson(context, createName.trim(), crypto.randomUUID());
      personId = created.id;
      revision = created.revision;
    }
    if (!personId) {
      error = "Choose a person to add.";
      busy = false;
      return;
    }
    await createMembership(context, personId, houseId, revision, requestIdForSave(), serializeMetadata(draft));
    onSaved();
    onClose();
  } catch (cause) {
    const failure = classifyMutationError(cause);
    error = failure.message;
    if (failure.code === "revision-conflict" && editing) {
      conflict = true;
      try {
        const items = await context.relationships.list(editing.personId as never);
        const fresh = items.find((item) => item.id === editing.id);
        if (fresh) {
          currentValues = fresh.metadata ?? {};
          currentRevision = fresh.revision;
        }
      } catch {
        currentValues = null;
      }
    } else {
      requestId = crypto.randomUUID();
      lastFingerprint = "";
    }
  } finally {
    busy = false;
  }
}

async function removeMembership() {
  if (!editing || busy) return;
  const confirmed = await confirmDialog({
    title: `Remove ${editing.personName} from ${houseName || "house"}?`,
    message: ENTITY_ACTION_CONFIRM.removeMembershipMessage,
    confirmLabel: "Remove from House",
    danger: true,
  });
  if (!confirmed) return;
  busy = true;
  error = "";
  try {
    await deleteFamilyRelationship(
      context,
      editing.id,
      MEMBERSHIP_RELATIONSHIP,
      observedRevision || editing.revision,
      crypto.randomUUID(),
    );
    onRemoved?.();
    onSaved();
    onClose();
  } catch (cause) {
    const failure = classifyMutationError(cause);
    error = failure.message;
    conflict = failure.code === "revision-conflict";
  } finally {
    busy = false;
  }
}

function onKeydown(event: KeyboardEvent) {
  trapModalTab(event, dialogEl);
  if (event.key === "Escape" && !busy) {
    event.preventDefault();
    onClose();
  }
}

$effect(() => {
  dialogEl?.focus();
});
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="overlay"
  bind:this={dialogEl}
  tabindex="-1"
  role="dialog"
  aria-modal="true"
  aria-label={title}
  onkeydown={onKeydown}
  onclick={(event) => {
    if (event.target === event.currentTarget && !busy) onClose();
  }}>
  <div class="card" role="document" onclick={(event) => event.stopPropagation()}>
    <header class="dialog-heading">
      <div>
        <span class="panel-kicker">MEMBERSHIP</span>
        <strong>{title}</strong>
      </div>
      <button type="button" class="dialog-close" aria-label="Close dialog" disabled={busy} onclick={onClose}
        ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
    </header>

    {#if !isEdit}
      <div class="tabs">
        <button type="button" class="quiet-button" aria-pressed={mode === "link"} onclick={() => (mode = "link")}
          >Link existing</button>
        <button type="button" class="quiet-button" aria-pressed={mode === "create"} onclick={() => (mode = "create")}
          >Create person</button>
      </div>
      {#if mode === "link"}
        <AsyncEntityPicker
          search={searchPeople}
          entityTypes={[PERSON_TYPE]}
          {excludeIds}
          selectedIds={selectedPersonId ? [selectedPersonId] : []}
          dropdown={false}
          disabled={busy}
          placeholder="Search people"
          ariaLabel="Search people to add"
          emptyMessage="No matching people."
          onSelect={(option) => {
            selectedPersonId = option.id;
            selectedPersonName = option.name;
            selectedRevision = typeof option.revision === "string" ? option.revision : "";
          }} />
        {#if selectedPersonId}
          <p class="hint">Selected: <strong>{selectedPersonName}</strong></p>
        {/if}
      {:else}
        <label class="field"
          >Name
          <input bind:value={createName} placeholder="Person name" aria-label="New person name" />
        </label>
        <p class="hint">Creates a Lore person and adds them to this house.</p>
      {/if}
    {/if}

    <div class="form-grid">
      {#each fields as field (field.key)}
        {#if field.type === "enum"}
          <label class="field">
            <span>{field.label}</span>
            <select
              value={String(draft[field.key] ?? "")}
              onchange={(event) => setDraft(field.key, (event.currentTarget as HTMLSelectElement).value)}>
              {#each field.options ?? [] as option}
                <option value={option}>{option}</option>
              {/each}
            </select>
          </label>
        {:else if field.key === "notes"}
          <label class="field"
            ><span>{field.label}</span>
            <textarea
              rows="3"
              value={String(draft[field.key] ?? "")}
              oninput={(event) => setDraft(field.key, (event.currentTarget as HTMLTextAreaElement).value)}></textarea
            ></label>
        {:else}
          <label class="field">
            <span>{field.label}</span>
            <input
              value={String(draft[field.key] ?? "")}
              oninput={(event) => setDraft(field.key, (event.currentTarget as HTMLInputElement).value)} />
          </label>
        {/if}
      {/each}
    </div>

    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if conflict}
      <div class="actions">
        <p class="hint">{MUTATION_STATUS_MESSAGES.conflictTitle}</p>
        <button type="button" class="quiet-button" onclick={reloadCurrent}
          >{MUTATION_STATUS_MESSAGES.conflictReload}</button>
        <button type="button" class="quiet-button ghost" onclick={() => (conflict = false)}
          >{MUTATION_STATUS_MESSAGES.conflictReviewDraft}</button>
      </div>
    {/if}
    <div class="actions">
      {#if isEdit}
        <button type="button" class="danger-button" disabled={busy} onclick={() => void removeMembership()}
          >Remove from House</button>
      {/if}
      <button type="button" class="quiet-button" disabled={busy} onclick={onClose}>Cancel</button>
      <button type="button" class="primary-button" disabled={busy || conflict} onclick={() => void save()}>
        {busy ? MUTATION_STATUS.working : isEdit ? "Save" : "Add to House"}
      </button>
    </div>
  </div>
</div>

<style>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(37, 37, 31, 0.28);
}
.card {
  width: min(440px, 100%);
  max-height: min(84vh, 720px);
  overflow: auto;
  display: grid;
  gap: 12px;
  padding: 22px;
  border: 1px solid var(--theme-warning-border, #e3d9ca);
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
}
.dialog-heading strong {
  display: block;
  font-size: 16px;
  line-height: 1.3;
  color: var(--ink);
}
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.dialog-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  flex: none;
  border: 0;
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  cursor: pointer;
}
.dialog-close:hover {
  background: var(--theme-warning-bg, #ebe6dd);
  color: var(--ink);
}
.dialog-close:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.tabs,
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.tabs {
  justify-content: space-between;
}
.actions {
  justify-content: flex-end;
}
.form-grid {
  display: grid;
  gap: 10px;
}
.field {
  display: grid;
  gap: 4px;
  color: var(--ink);
  font:
    12px/1.4 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.field > span {
  color: var(--ink-muted);
  font-size: 11px;
  font-weight: 700;
}
.field input,
.field select,
.field textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 7px 9px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 13px;
}
.field input:not([type="checkbox"]):not([type="hidden"]),
.field select {
  height: 34px;
  min-height: 34px;
  padding: 0 9px;
}
.field input:focus-visible,
.field select:focus-visible,
.field textarea:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
  border-color: var(--accent);
}
.hint {
  margin: 0;
  color: var(--ink-muted);
  font-size: 12px;
  line-height: 1.4;
}
.error {
  margin: 0;
  color: var(--theme-danger-text, #8a2b2b);
  background: var(--danger-bg, #fff2ee);
  border: 1px solid var(--danger-line, #edcec5);
  border-radius: 8px;
  padding: 8px 10px;
  font-size: 12px;
}
.quiet-button,
.primary-button,
.danger-button {
  min-height: 34px;
  padding: 0 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.quiet-button {
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
}
.quiet-button[aria-pressed="true"] {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  color: var(--ink);
}
.quiet-button.ghost {
  border-color: transparent;
  background: transparent;
}
.primary-button {
  background: var(--accent-dark, var(--accent));
  border-color: transparent;
  color: #fff;
}
.danger-button {
  border-color: var(--theme-danger-border, #e2c4bb);
  background: var(--surface);
  color: var(--theme-danger-text, #8a3b2d);
}
.quiet-button:disabled,
.primary-button:disabled,
.danger-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
