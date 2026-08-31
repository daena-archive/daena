<script lang="ts">
import type { ModuleContext } from "../../../packages/module-api/src/index";
import AsyncEntityPicker from "$lib/ui-ux/AsyncEntityPicker.svelte";
import { toAsyncEntityPage } from "$lib/ui-ux/asyncEntityQuery.ts";
import { ENTITY_ACTION_CONFIRM, MUTATION_STATUS, MUTATION_STATUS_MESSAGES } from "$lib/ui-ux/vocabulary.ts";
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
  if (event.key === "Escape") {
    event.preventDefault();
    onClose();
  }
}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="backdrop" role="presentation" onclick={onClose} onkeydown={onKeydown}>
  <div
    class="card"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={onKeydown}>
    <header class="head">
      <div>
        <span class="kicker">MEMBERSHIP</span>
        <strong>{title}</strong>
      </div>
      <button type="button" class="quiet-button ghost" onclick={onClose}>Close</button>
    </header>

    {#if !isEdit}
      <div class="mode-toggle" role="tablist" aria-label="Add member mode">
        <button
          type="button"
          class:active={mode === "link"}
          role="tab"
          aria-selected={mode === "link"}
          onclick={() => (mode = "link")}>Link existing</button>
        <button
          type="button"
          class:active={mode === "create"}
          role="tab"
          aria-selected={mode === "create"}
          onclick={() => (mode = "create")}>Create person</button>
      </div>
      {#if mode === "link"}
        <AsyncEntityPicker
          search={searchPeople}
          entityTypes={[PERSON_TYPE]}
          {excludeIds}
          selectedIds={selectedPersonId ? [selectedPersonId] : []}
          placeholder="Search people"
          ariaLabel="Search people to add"
          emptyMessage="No matching people."
          onSelect={(option) => {
            selectedPersonId = option.id;
            selectedPersonName = option.name;
            selectedRevision = typeof option.revision === "string" ? option.revision : "";
          }} />
        {#if selectedPersonId}
          <p class="selected">Selected: <strong>{selectedPersonName}</strong></p>
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
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: grid;
  place-items: center;
  padding: 24px;
  background: color-mix(in srgb, var(--ink) 35%, transparent);
}
.card {
  display: grid;
  gap: 14px;
  width: min(440px, 100%);
  max-height: min(88vh, 720px);
  overflow: auto;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: 0 18px 48px color-mix(in srgb, var(--ink) 18%, transparent);
}
.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.mode-toggle {
  display: flex;
  gap: 6px;
}
.mode-toggle button {
  flex: 1;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted, var(--surface));
  color: var(--ink-muted);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.mode-toggle button.active {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  color: var(--ink);
}
.form-grid {
  display: grid;
  gap: 10px;
}
.field {
  display: grid;
  gap: 4px;
  color: var(--ink-muted);
  font-size: 11px;
  font-weight: 700;
}
.field input,
.field select,
.field textarea {
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: 13px/1.35 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.hint,
.selected {
  margin: 0;
  color: var(--ink-muted);
  font-size: 12px;
}
.error {
  margin: 0;
  color: var(--theme-danger-text, #b42318);
  font-size: 12px;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}
.ghost {
  border-color: transparent;
  background: transparent;
}
</style>
