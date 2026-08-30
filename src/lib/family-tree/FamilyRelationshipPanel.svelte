<script lang="ts">
import type { MetadataFieldDefinition, ModuleContext } from "../../../packages/module-api/src/index";
import DateEditor from "$lib/date/DateEditor.svelte";
import { confirmDialog } from "$lib/dialogs.svelte";
import { PARENT_KINDS, PARTNER_KINDS, PARTNER_STATUSES, type FamilyPerson, type FamilyRelationship } from "./model.ts";
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
  <header>
    <div>
      <strong>{sourceName} → {targetName}</strong>
      <span>{relationship.label}</span>
    </div>
    <button type="button" class="quiet-button" onclick={onClose}>Close</button>
  </header>
  {#each fields as field (field.key)}
    {#if field.type === "enum"}
      <label>
        {field.label}
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
      <label class="check">
        <input
          type="checkbox"
          checked={Boolean(draft[field.key])}
          onchange={(event) => setDraft(field.key, (event.currentTarget as HTMLInputElement).checked)} />
        {field.label}
      </label>
    {:else if field.type === "number"}
      <label>
        {field.label}
        <input
          type="number"
          value={draft[field.key] ?? ""}
          onchange={(event) => setDraft(field.key, Number((event.currentTarget as HTMLInputElement).value))} />
      </label>
    {:else if field.key === "notes"}
      <label
        >{field.label}
        <textarea
          rows="3"
          value={String(draft[field.key] ?? "")}
          oninput={(event) => setDraft(field.key, (event.currentTarget as HTMLTextAreaElement).value)}></textarea
        ></label>
    {:else}
      <label>
        {field.label}
        <input
          value={String(draft[field.key] ?? "")}
          oninput={(event) => setDraft(field.key, (event.currentTarget as HTMLInputElement).value)} />
      </label>
    {/if}
  {/each}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if conflict}
    <div class="actions">
      <button type="button" class="quiet-button" onclick={reloadCurrent}>Reload current values</button>
      <button type="button" class="quiet-button" onclick={() => (conflict = false)}>Review draft</button>
    </div>
  {/if}
  <div class="actions">
    <button type="button" class="primary-button" disabled={saving || conflict} onclick={() => void save()}>Save</button>
    <button type="button" class="danger-button" disabled={saving} onclick={() => void remove()}>Delete</button>
  </div>
</aside>

<style>
.panel {
  display: grid;
  align-content: start;
  gap: 8px;
  width: min(420px, 100%);
  max-height: min(80vh, 720px);
  overflow: auto;
  padding: 16px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--surface);
}
.panel.docked {
  width: 100%;
  height: 100%;
  max-height: none;
  padding: 14px;
  border: none;
  border-radius: 0;
}
header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
header div {
  display: grid;
  gap: 2px;
}
header span,
label,
.error {
  color: var(--ink-muted);
  font:
    12px/1.4 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
label {
  display: grid;
  gap: 4px;
  color: var(--ink);
}
.check {
  display: flex;
  gap: 8px;
  align-items: center;
}
.error {
  color: var(--theme-danger-text, #8a2b2b);
}
input,
textarea,
select {
  width: 100%;
  box-sizing: border-box;
  padding: 6px 8px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.quiet-button,
.primary-button,
.danger-button {
  padding: 8px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  font-size: 12px;
  cursor: pointer;
}
.quiet-button {
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
}
.primary-button {
  background: var(--accent-dark, var(--accent));
  border-color: transparent;
  color: #fff;
}
.danger-button {
  background: var(--theme-danger-bg, #8a2b2b);
  border-color: transparent;
  color: #fff;
}
</style>
