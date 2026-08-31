<script lang="ts">
import type { MetadataFieldDefinition, ModuleContext } from "../../../packages/module-api/src/index";
import DateEditor from "$lib/date/DateEditor.svelte";
import { confirmDialog } from "$lib/dialogs.svelte";
import {
  PARENT_KINDS,
  PARTNER_KINDS,
  PARTNER_STATUSES,
  formatRelationshipTitle,
  formatRelationshipTypeLabel,
  type FamilyPerson,
  type FamilyRelationship,
} from "./model.ts";
import { parseFamilyRelationship } from "./projection.ts";
import {
  classifyMutationError,
  deleteFamilyRelationship,
  metadataFingerprint,
  updateFamilyMetadata,
} from "./mutations.ts";

let {
  context,
  relationship,
  people,
  onUpdated,
  onDeleted,
  onClose,
  docked = false,
}: {
  context: ModuleContext;
  relationship: FamilyRelationship;
  people: Map<string, FamilyPerson>;
  onUpdated: (relationship: FamilyRelationship) => void;
  onDeleted: (id: string) => void;
  onClose: () => void;
  docked?: boolean;
} = $props();

let draft = $state<Record<string, unknown>>({});
let observedRevision = $state("");
let requestId = $state(crypto.randomUUID());
let lastFingerprint = $state("");
let saving = $state(false);
let error = $state("");
let conflict = $state(false);
let currentValues = $state<Record<string, unknown> | null>(null);
let currentRevision = $state("");

const sourceName = $derived(people.get(relationship.sourceId)?.name ?? relationship.sourceId);
const targetName = $derived(people.get(relationship.targetId)?.name ?? relationship.targetId);
const relationshipTitle = $derived(formatRelationshipTitle(relationship.kind, sourceName, targetName));
const relationshipTypeLabel = $derived(formatRelationshipTypeLabel(relationship));
const fields = $derived(metadataFieldsFor(context, relationship.type, relationship.kind));

function metadataFieldsFor(
  moduleContext: ModuleContext,
  type: string,
  kind: FamilyRelationship["kind"],
): MetadataFieldDefinition[] {
  for (const schema of moduleContext.module.schemas ?? []) {
    for (const field of schema.fields ?? []) {
      if (field.relationshipType === type && field.metadataFields?.length) return field.metadataFields;
    }
  }
  if (kind === "partner") {
    return [
      { key: "kind", label: "Partnership type", type: "enum", required: true, options: [...PARTNER_KINDS] },
      { key: "customLabel", label: "Custom label", type: "text" },
      { key: "status", label: "Status", type: "enum", options: [...PARTNER_STATUSES] },
      { key: "start", label: "Starts", type: "date" },
      { key: "end", label: "Ends", type: "date" },
      { key: "notes", label: "Notes", type: "text" },
    ];
  }
  return [
    { key: "kind", label: "Parent type", type: "enum", required: true, options: [...PARENT_KINDS] },
    { key: "customLabel", label: "Custom label", type: "text" },
    { key: "start", label: "Starts", type: "date" },
    { key: "end", label: "Ends", type: "date" },
    { key: "notes", label: "Notes", type: "text" },
  ];
}

function relationshipDraft(value: FamilyRelationship): Record<string, unknown> {
  return {
    kind: value.kind === "parent" ? value.parentKind : value.partnerKind,
    customLabel: value.customLabel ?? "",
    status: value.status,
    start: value.start,
    end: value.end,
    notes: value.notes ?? "",
  };
}

function resetFrom(value: FamilyRelationship) {
  draft = relationshipDraft(value);
  observedRevision = value.revision;
  requestId = crypto.randomUUID();
  lastFingerprint = metadataFingerprint(draft);
  error = "";
  conflict = false;
  currentValues = null;
  currentRevision = "";
}

function requestIdForSave() {
  const next = metadataFingerprint(draft);
  if (next !== lastFingerprint) {
    requestId = crypto.randomUUID();
    lastFingerprint = next;
  }
  return requestId;
}

let appliedRelationship = $state("");
$effect(() => {
  const key = `${relationship.id}:${relationship.revision}`;
  if (key === appliedRelationship) return;
  appliedRelationship = key;
  resetFrom(relationship);
});

async function save() {
  if (saving) return;
  const kind = String(draft.kind ?? "");
  if (kind === "custom" && !String(draft.customLabel ?? "").trim()) {
    error = "Custom label is required.";
    return;
  }
  saving = true;
  error = "";
  try {
    const updated = await updateFamilyMetadata(context, relationship.id, draft, observedRevision, requestIdForSave());
    const parsed = parseFamilyRelationship(updated);
    onUpdated(parsed.parsed);
    resetFrom(parsed.parsed);
  } catch (cause) {
    const failure = classifyMutationError(cause);
    error = failure.message;
    if (failure.code === "revision-conflict") {
      conflict = true;
      try {
        const items = await context.relationships.list(relationship.sourceId as never);
        const fresh = items.find((item) => item.id === relationship.id);
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
    saving = false;
  }
}

function reloadCurrent() {
  if (!currentValues) return;
  draft = {
    kind: currentValues.kind ?? draft.kind,
    customLabel: currentValues.customLabel ?? "",
    status: currentValues.status ?? draft.status,
    start: currentValues.start ?? draft.start,
    end: currentValues.end ?? draft.end,
    notes: currentValues.notes ?? "",
  };
  if (currentRevision) observedRevision = currentRevision;
  conflict = false;
  error = "Loaded current values. Save will use a new request after you confirm the draft.";
  lastFingerprint = "";
  requestId = crypto.randomUUID();
}

function setDraft(key: string, value: unknown) {
  draft = { ...draft, [key]: value };
}

async function remove() {
  if (saving) return;
  const kind = relationship.label || relationship.kind;
  const ok = await confirmDialog({
    title: `Delete ${kind}?`,
    message: `Remove the ${kind} relationship between ${sourceName} and ${targetName}? Both people stay in Lore.`,
    confirmLabel: "Delete relationship",
    danger: true,
  });
  if (!ok) return;
  saving = true;
  error = "";
  const deleteId = crypto.randomUUID();
  try {
    await deleteFamilyRelationship(context, relationship.id, relationship.type, observedRevision, deleteId);
    onDeleted(relationship.id);
  } catch (cause) {
    const failure = classifyMutationError(cause);
    error = failure.message;
    conflict = failure.code === "revision-conflict";
  } finally {
    saving = false;
  }
}
</script>

<aside class="panel" class:docked aria-label="Relationship">
  <header class="panel-head">
    <div>
      <span class="kicker">RELATIONSHIP</span>
      <strong class="title">{relationshipTitle}</strong>
      <span class="subtitle">{relationshipTypeLabel}</span>
    </div>
    <button type="button" class="quiet-button ghost" onclick={onClose}>Close</button>
  </header>

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
      {:else if field.type === "date"}
        <DateEditor
          label={field.label}
          value={draft[field.key]}
          calendars={[]}
          onChange={(next) => setDraft(field.key, next)}
          onClear={() => setDraft(field.key, null)} />
      {:else if field.type === "boolean"}
        <label class="field check">
          <input
            type="checkbox"
            checked={Boolean(draft[field.key])}
            onchange={(event) => setDraft(field.key, (event.currentTarget as HTMLInputElement).checked)} />
          <span>{field.label}</span>
        </label>
      {:else if field.type === "number"}
        <label class="field">
          <span>{field.label}</span>
          <input
            type="number"
            value={draft[field.key] ?? ""}
            onchange={(event) => setDraft(field.key, Number((event.currentTarget as HTMLInputElement).value))} />
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
      <button type="button" class="quiet-button" onclick={reloadCurrent}>Reload current values</button>
      <button type="button" class="quiet-button ghost" onclick={() => (conflict = false)}>Review draft</button>
    </div>
  {/if}
  <div class="actions sticky-actions">
    <button type="button" class="primary-button" disabled={saving || conflict} onclick={() => void save()}>Save</button>
    <button type="button" class="danger-button" disabled={saving} onclick={() => void remove()}>Delete</button>
  </div>
</aside>

<style>
.panel {
  display: grid;
  align-content: start;
  gap: 12px;
  width: min(420px, 100%);
  max-height: min(80vh, 720px);
  overflow: auto;
  padding: 16px;
  background: var(--surface);
}
.panel.docked {
  width: 100%;
  height: 100%;
  max-height: none;
  padding: 16px;
  border: none;
  border-radius: 0;
}
.panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--line-soft, var(--line));
}
.panel-head div {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.kicker {
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.title {
  color: var(--ink);
  font: 600 13px/1.3 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.subtitle {
  color: var(--ink-muted);
  font: 11px/1.35 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.form-grid {
  display: grid;
  gap: 10px;
}
.field {
  display: grid;
  gap: 4px;
  color: var(--ink);
  font: 12px/1.4 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.field > span {
  color: var(--ink);
  font-weight: 600;
  font-size: 11px;
}
.field.check {
  display: flex;
  gap: 8px;
  align-items: center;
}
.field.check span {
  font-weight: 500;
}
.error {
  color: var(--theme-danger-text, #8a2b2b);
  font: 11px/1.4 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  background: var(--danger-bg, #fff2ee);
  border: 1px solid var(--danger-line, #edcec5);
  border-radius: 8px;
  padding: 8px 10px;
}
input,
textarea,
select {
  width: 100%;
  box-sizing: border-box;
  padding: 7px 9px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 13px;
}
input:not([type="checkbox"]):not([type="hidden"]),
select {
  height: 34px;
  min-height: 34px;
  padding: 0 9px;
}
input:focus-visible,
textarea:focus-visible,
select:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
  border-color: var(--accent);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding-top: 4px;
}
.sticky-actions {
  position: sticky;
  bottom: 0;
  padding-top: 10px;
  background: linear-gradient(transparent, var(--surface) 30%);
}
.quiet-button,
.primary-button,
.danger-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 34px;
  padding: 0 12px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
}
.quiet-button {
  border: 1px solid var(--theme-warning-border, #ded8cd);
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
}
.quiet-button:hover {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--surface-muted);
  color: var(--ink);
}
.quiet-button.ghost {
  border-color: transparent;
  background: transparent;
  box-shadow: none;
}
.quiet-button.ghost:hover {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--surface-muted);
}
.primary-button {
  border: 1px solid transparent;
  background: var(--accent-dark, var(--accent));
  color: #fff;
}
.primary-button:hover {
  background: #2b4535;
}
.danger-button {
  border: 1px solid var(--theme-danger-border, #e2c4bb);
  background: var(--surface);
  color: var(--theme-danger-text, #8a3b2d);
}
.danger-button:hover {
  background: var(--theme-danger-bg, #f8ece8);
}
.quiet-button:focus-visible,
.primary-button:focus-visible,
.danger-button:focus-visible {
  outline: 3px solid var(--focus-ring, rgba(180, 119, 63, 0.24));
  outline-offset: 2px;
}
.primary-button:disabled,
.danger-button:disabled,
.quiet-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  box-shadow: none;
}
@media (prefers-reduced-motion: reduce) {
  .panel {
    scroll-behavior: auto;
  }
}
</style>
