<script lang="ts">
import type { EntityTemplate, FieldDefinition, ModuleSchemaOverlay } from "$lib/project/client";
import { onMount } from "svelte";
import { setSchemaEditorDirtyCheck } from "$lib/schemaEditorGuard";

type PackageManifest = {
  schemas: Array<{
    namespace: string;
    entityTypes: string[];
    fields: FieldDefinition[];
  }>;
  templates: EntityTemplate[];
};

type FieldType = FieldDefinition["type"];

const FIELD_TYPES: FieldType[] = ["text", "number", "boolean", "date"];

let {
  projectOpen,
  packageManifest,
  overlay,
  busy = false,
  message = "",
  onSave,
  onDirtyChange,
}: {
  projectOpen: boolean;
  packageManifest: PackageManifest;
  overlay: ModuleSchemaOverlay;
  busy?: boolean;
  message?: string;
  onSave: (overlay: ModuleSchemaOverlay) => Promise<void>;
  onDirtyChange?: (dirty: boolean) => void;
} = $props();

const packageSchema = $derived(packageManifest.schemas[0]);
const packageTypes = $derived([...(packageSchema?.entityTypes ?? [])].sort());
const packageFields = $derived([...(packageSchema?.fields ?? [])]);
const packageTemplates = $derived([...(packageManifest.templates ?? [])]);

function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/** Show `currentOwner` / `word_count` / `my-type` as readable labels. */
function humanizeId(value: string): string {
  const spaced = value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[_-]+/g, " ")
    .trim()
    .replace(/\s+/g, " ");
  if (!spaced) return value;
  return spaced.replace(/\b\w/g, (char) => char.toUpperCase());
}

function fieldTypeLabel(type: FieldType): string {
  return type.charAt(0).toUpperCase() + type.slice(1);
}

/** Entity/template ids: lowercase kebab (`Test` → `test`, `My Type` → `my-type`). */
function slugifyTypeId(value: string): string {
  return value
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** IDs must start with a-z per host validation. */
function ensureTypeId(value: string, fallback = "custom"): string {
  let id = slugifyTypeId(value);
  if (!id) id = fallback;
  if (!/^[a-z]/.test(id)) id = `${fallback}-${id}`;
  return id;
}

/** Field keys: lowercase snake (`WordCount` → `word_count`). */
function slugifyFieldKey(value: string): string {
  return value
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function ensureFieldKey(value: string, fallback = "field"): string {
  let key = slugifyFieldKey(value);
  if (!key) key = fallback;
  if (!/^[a-z]/.test(key)) key = `${fallback}_${key}`;
  return key;
}

function normalizeOverlay(value: ModuleSchemaOverlay): ModuleSchemaOverlay {
  const customEntityTypes = [
    ...new Set((value.customEntityTypes ?? []).map((name) => ensureTypeId(name, "type")).filter(Boolean)),
  ].sort();
  const customFields = cloneJson(value.customFields ?? []).map((field) => ({
    ...field,
    key: ensureFieldKey(field.key),
    entityTypes: field.entityTypes?.map((name) => ensureTypeId(name, "type")).filter(Boolean),
  }));
  const customTemplates = cloneJson(value.customTemplates ?? []).map((template) => ({
    ...template,
    id: ensureTypeId(template.id || template.name, "template"),
    entityType: ensureTypeId(template.entityType, "type"),
    description: template.description?.trim() ? template.description.trim() : null,
  }));
  return {
    version: value.version || 1,
    disabledEntityTypes: [...(value.disabledEntityTypes ?? [])].sort(),
    disabledFields: [...(value.disabledFields ?? [])].sort(),
    disabledTemplates: [...(value.disabledTemplates ?? [])].sort(),
    customEntityTypes,
    customFields,
    customTemplates,
  };
}

function fingerprint(value: ModuleSchemaOverlay): string {
  return JSON.stringify(normalizeOverlay(value));
}

// Remounted by parent `{#key overlayRevision}`; initial overlay is intentional.
// svelte-ignore state_referenced_locally
const baseline = fingerprint(overlay);
// Plain snapshot — leave checks must not depend on reading $state from external closures.
// svelte-ignore state_referenced_locally
let draftPlain = normalizeOverlay(overlay);
// svelte-ignore state_referenced_locally
let draft = $state<ModuleSchemaOverlay>(draftPlain);
/** Synced on every setDraft; read by the leave guard (plain bool, not $state). */
let dirtyFlag = false;
let dirty = $state(false);

let newType = $state("");
let editingTypeId = $state<string | null>(null);
let editTypeValue = $state("");

let newFieldLabel = $state("");
let newFieldType = $state<FieldType>("text");
let newFieldEntityTypes = $state<string[]>([]);
let editingFieldKey = $state<string | null>(null);
let editFieldLabel = $state("");
let editFieldType = $state<FieldType>("text");
let editFieldEntityTypes = $state<string[]>([]);

let newTemplateName = $state("");
let newTemplateEntityType = $state("");
let newTemplateDescription = $state("");
let editingTemplateId = $state<string | null>(null);
let editTemplateName = $state("");
let editTemplateEntityType = $state("");
let editTemplateDescription = $state("");

function reportDirty(next: boolean) {
  dirtyFlag = next;
  dirty = next;
  onDirtyChange?.(next);
}

function setDraft(next: ModuleSchemaOverlay) {
  draftPlain = normalizeOverlay(next);
  draft = draftPlain;
  reportDirty(fingerprint(draftPlain) !== baseline);
}

onMount(() => {
  setSchemaEditorDirtyCheck(() => dirtyFlag);
  return () => {
    setSchemaEditorDirtyCheck(null);
    dirtyFlag = false;
    onDirtyChange?.(false);
  };
});

function isDisabled(list: string[] | undefined, id: string) {
  return (list ?? []).includes(id);
}

function toggleDisabled(listKey: "disabledEntityTypes" | "disabledFields" | "disabledTemplates", id: string) {
  const current = new Set(draft[listKey] ?? []);
  if (current.has(id)) current.delete(id);
  else current.add(id);
  setDraft({ ...draft, [listKey]: [...current].sort() });
}

function effectiveTypes() {
  return [
    ...packageTypes.filter((type) => !isDisabled(draft.disabledEntityTypes, type)),
    ...(draft.customEntityTypes ?? []),
  ];
}

function scopeLabel(entityTypes: string[] | undefined): string {
  if (!entityTypes || entityTypes.length === 0) return "No types";
  return entityTypes.map(humanizeId).join(", ");
}

function toggleInList(list: string[], id: string): string[] {
  const next = new Set(list);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return [...next].sort();
}

function defaultTemplateFields(entityType: string): Record<string, unknown> {
  const fields: Record<string, unknown> = {};
  for (const field of packageFields) {
    const scoped = field.entityTypes;
    if (scoped && scoped.length > 0 && !scoped.includes(entityType)) continue;
    if (isDisabled(draft.disabledFields, field.key)) continue;
    fields[field.key] =
      field.type === "number" ? "" : field.type === "boolean" ? false : field.type === "relationship" ? [] : "";
  }
  for (const field of draft.customFields ?? []) {
    const scoped = field.entityTypes;
    if (scoped && scoped.length > 0 && !scoped.includes(entityType)) continue;
    fields[field.key] =
      field.type === "number" ? "" : field.type === "boolean" ? false : field.type === "relationship" ? [] : "";
  }
  return fields;
}

async function save() {
  await onSave(normalizeOverlay(draft));
}

function cancelTypeEdit() {
  editingTypeId = null;
  editTypeValue = "";
}

function startTypeEdit(type: string) {
  editingFieldKey = null;
  editingTemplateId = null;
  editingTypeId = type;
  editTypeValue = humanizeId(type);
}

function addCustomType() {
  const name = ensureTypeId(newType, "type");
  if (!newType.trim()) return;
  if (packageTypes.includes(name) || (draft.customEntityTypes ?? []).includes(name)) return;
  setDraft({
    ...draft,
    customEntityTypes: [...(draft.customEntityTypes ?? []), name].sort(),
  });
  newType = "";
}

function commitTypeEdit() {
  if (!editingTypeId) return;
  const from = editingTypeId;
  const to = ensureTypeId(editTypeValue, "type");
  if (!editTypeValue.trim() || to === from) {
    cancelTypeEdit();
    return;
  }
  if (packageTypes.includes(to) || (draft.customEntityTypes ?? []).includes(to)) return;
  setDraft({
    ...draft,
    customEntityTypes: (draft.customEntityTypes ?? []).map((item) => (item === from ? to : item)).sort(),
    customFields: (draft.customFields ?? []).map((field) => ({
      ...field,
      entityTypes: field.entityTypes?.map((type) => (type === from ? to : type)),
    })),
    customTemplates: (draft.customTemplates ?? []).map((template) => ({
      ...template,
      entityType: template.entityType === from ? to : template.entityType,
    })),
  });
  cancelTypeEdit();
}

function removeCustomType(name: string) {
  if (editingTypeId === name) cancelTypeEdit();
  setDraft({
    ...draft,
    customEntityTypes: (draft.customEntityTypes ?? []).filter((item) => item !== name),
    customFields: (draft.customFields ?? []).map((field) => ({
      ...field,
      entityTypes: field.entityTypes?.filter((type) => type !== name),
    })),
    customTemplates: (draft.customTemplates ?? []).filter((template) => template.entityType !== name),
  });
}

function dependentsForType(name: string): {
  fields: FieldDefinition[];
  exclusiveFields: FieldDefinition[];
  sharedFields: FieldDefinition[];
  templates: EntityTemplate[];
} {
  const fields = (draft.customFields ?? []).filter((field) => field.entityTypes?.includes(name));
  return {
    fields,
    exclusiveFields: fields.filter((field) => (field.entityTypes ?? []).length === 1),
    sharedFields: fields.filter((field) => (field.entityTypes ?? []).length > 1),
    templates: (draft.customTemplates ?? []).filter((template) => template.entityType === name),
  };
}

type TypeRemovalPrompt = {
  typeId: string;
  exclusiveFields: FieldDefinition[];
  sharedFields: FieldDefinition[];
  templates: EntityTemplate[];
  removeSharedFields: boolean;
};

let typeRemovalPrompt = $state<TypeRemovalPrompt | null>(null);

function requestRemoveCustomType(name: string) {
  const { exclusiveFields, sharedFields, templates } = dependentsForType(name);
  if (exclusiveFields.length === 0 && sharedFields.length === 0 && templates.length === 0) {
    removeCustomType(name);
    return;
  }
  typeRemovalPrompt = {
    typeId: name,
    exclusiveFields,
    sharedFields,
    templates,
    removeSharedFields: false,
  };
}

function cancelTypeRemoval() {
  typeRemovalPrompt = null;
}

function confirmTypeRemoval() {
  const prompt = typeRemovalPrompt;
  if (!prompt) return;
  const { typeId, removeSharedFields } = prompt;
  if (editingTypeId === typeId) cancelTypeEdit();
  const fieldsToRemove = new Set(
    (draft.customFields ?? [])
      .filter((field) => {
        const scoped = field.entityTypes ?? [];
        if (!scoped.includes(typeId)) return false;
        if (scoped.length === 1) return true;
        return removeSharedFields;
      })
      .map((field) => field.key),
  );
  setDraft({
    ...draft,
    customEntityTypes: (draft.customEntityTypes ?? []).filter((item) => item !== typeId),
    customFields: (draft.customFields ?? [])
      .filter((field) => !fieldsToRemove.has(field.key))
      .map((field) => ({
        ...field,
        entityTypes: field.entityTypes?.filter((type) => type !== typeId),
      })),
    customTemplates: (draft.customTemplates ?? [])
      .filter((template) => template.entityType !== typeId)
      .map((template) => {
        if (fieldsToRemove.size === 0) return template;
        const fields = { ...(template.fields as Record<string, unknown>) };
        for (const key of fieldsToRemove) delete fields[key];
        return {
          ...template,
          fields,
          requiredFields: template.requiredFields?.filter((item) => !fieldsToRemove.has(item)) ?? null,
        };
      }),
  });
  typeRemovalPrompt = null;
}

function cancelFieldEdit() {
  editingFieldKey = null;
  editFieldLabel = "";
  editFieldType = "text";
  editFieldEntityTypes = [];
}

function startFieldEdit(field: FieldDefinition) {
  editingTypeId = null;
  editingTemplateId = null;
  editingFieldKey = field.key;
  editFieldLabel = field.label;
  editFieldType = FIELD_TYPES.includes(field.type) ? field.type : "text";
  editFieldEntityTypes = [...(field.entityTypes ?? [])].sort();
}

function addCustomField() {
  const label = newFieldLabel.trim();
  if (!label || newFieldEntityTypes.length === 0) return;
  const key = ensureFieldKey(label);
  if (packageFields.some((field) => field.key === key)) return;
  if ((draft.customFields ?? []).some((field) => field.key === key)) return;
  const field: FieldDefinition = {
    key,
    label,
    type: newFieldType,
    entityTypes: [...newFieldEntityTypes].sort(),
  };
  setDraft({ ...draft, customFields: [...(draft.customFields ?? []), field] });
  newFieldLabel = "";
  newFieldEntityTypes = [];
  newFieldType = "text";
}

function commitFieldEdit() {
  if (!editingFieldKey) return;
  const key = editingFieldKey;
  const label = editFieldLabel.trim();
  if (!label || editFieldEntityTypes.length === 0) return;
  setDraft({
    ...draft,
    customFields: (draft.customFields ?? []).map((field) =>
      field.key === key
        ? {
            ...field,
            label,
            type: editFieldType,
            entityTypes: [...editFieldEntityTypes].sort(),
          }
        : field,
    ),
  });
  cancelFieldEdit();
}

function removeCustomField(key: string) {
  if (editingFieldKey === key) cancelFieldEdit();
  setDraft({
    ...draft,
    customFields: (draft.customFields ?? []).filter((field) => field.key !== key),
    customTemplates: (draft.customTemplates ?? []).map((template) => {
      const fields = { ...(template.fields as Record<string, unknown>) };
      delete fields[key];
      return {
        ...template,
        fields,
        requiredFields: template.requiredFields?.filter((item) => item !== key) ?? null,
      };
    }),
  });
}

function cancelTemplateEdit() {
  editingTemplateId = null;
  editTemplateName = "";
  editTemplateEntityType = "";
  editTemplateDescription = "";
}

function startTemplateEdit(template: EntityTemplate) {
  editingTypeId = null;
  editingFieldKey = null;
  editingTemplateId = template.id;
  editTemplateName = template.name;
  editTemplateEntityType = template.entityType;
  editTemplateDescription = template.description ?? "";
}

function addCustomTemplate() {
  const name = newTemplateName.trim();
  const entityType = newTemplateEntityType.trim();
  if (!name || !entityType) return;
  if (!effectiveTypes().includes(entityType)) return;
  let id = ensureTypeId(name, "template");
  const taken = (candidate: string) =>
    packageTemplates.some((template) => template.id === candidate) ||
    (draft.customTemplates ?? []).some((template) => template.id === candidate);
  if (taken(id)) {
    id = ensureTypeId(`${name}-${entityType}`, "template");
  }
  if (taken(id)) {
    let suffix = 2;
    while (taken(`${id}-${suffix}`)) suffix += 1;
    id = `${id}-${suffix}`;
  }
  const description = newTemplateDescription.trim();
  const template: EntityTemplate = {
    id,
    name,
    entityType,
    description: description || null,
    icon: name.slice(0, 1).toUpperCase(),
    fields: defaultTemplateFields(entityType),
    requiredFields: [],
  };
  setDraft({ ...draft, customTemplates: [...(draft.customTemplates ?? []), template] });
  newTemplateName = "";
  newTemplateEntityType = "";
  newTemplateDescription = "";
}

function commitTemplateEdit() {
  if (!editingTemplateId) return;
  const id = editingTemplateId;
  const name = editTemplateName.trim();
  const entityType = editTemplateEntityType.trim();
  if (!name || !entityType || !effectiveTypes().includes(entityType)) return;
  const description = editTemplateDescription.trim();
  setDraft({
    ...draft,
    customTemplates: (draft.customTemplates ?? []).map((template) => {
      if (template.id !== id) return template;
      const entityChanged = template.entityType !== entityType;
      return {
        ...template,
        name,
        entityType,
        description: description || null,
        fields: entityChanged ? defaultTemplateFields(entityType) : template.fields,
      };
    }),
  });
  cancelTemplateEdit();
}

function removeCustomTemplate(id: string) {
  if (editingTemplateId === id) cancelTemplateEdit();
  setDraft({
    ...draft,
    customTemplates: (draft.customTemplates ?? []).filter((template) => template.id !== id),
  });
}
</script>

<section class="module-schema-panel">
  <header>
    <strong>Types & fields</strong>
    <p>
      Package defaults stay intact. Disable builtins you do not need, and add project-specific types, fields, and
      templates.
    </p>
  </header>

  {#if !projectOpen}
    <p class="muted">Open a project to customize this schema.</p>
  {:else}
    <div class="block">
      <div class="block-heading">
        <h4>Builtin entity types</h4>
        <span class="block-hint">Click to enable or disable</span>
      </div>
      <div class="chip-row">
        {#each packageTypes as type}
          <button
            type="button"
            class="chip"
            class:is-hidden={isDisabled(draft.disabledEntityTypes, type)}
            aria-pressed={!isDisabled(draft.disabledEntityTypes, type)}
            title={type}
            onclick={() => toggleDisabled("disabledEntityTypes", type)}>
            {humanizeId(type)}
          </button>
        {/each}
      </div>
    </div>

    <div class="block">
      <div class="block-heading">
        <h4>Builtin fields</h4>
        <span class="block-hint">Click to enable or disable</span>
      </div>
      <div class="chip-row">
        {#each packageFields as field}
          <button
            type="button"
            class="chip"
            class:is-hidden={isDisabled(draft.disabledFields, field.key)}
            aria-pressed={!isDisabled(draft.disabledFields, field.key)}
            title={`${field.key} · ${fieldTypeLabel(field.type)}`}
            onclick={() => toggleDisabled("disabledFields", field.key)}>
            {field.label || humanizeId(field.key)}
          </button>
        {/each}
      </div>
    </div>

    <div class="block">
      <div class="block-heading">
        <h4>Builtin templates</h4>
        <span class="block-hint">Click to enable or disable</span>
      </div>
      <div class="chip-row">
        {#each packageTemplates as template}
          <button
            type="button"
            class="chip"
            class:is-hidden={isDisabled(draft.disabledTemplates, template.id)}
            aria-pressed={!isDisabled(draft.disabledTemplates, template.id)}
            title={template.description || template.id}
            onclick={() => toggleDisabled("disabledTemplates", template.id)}>
            {template.name}
          </button>
        {/each}
      </div>
    </div>

    <div class="block">
      <div class="block-heading">
        <h4>Custom entity types</h4>
        <span class="block-hint">Project-only types layered on the package</span>
      </div>
      {#if (draft.customEntityTypes ?? []).length === 0}
        <p class="empty">No custom types yet.</p>
      {:else}
        <ul class="list">
          {#each draft.customEntityTypes ?? [] as type}
            <li class="list-item">
              {#if editingTypeId === type}
                <div class="edit-form">
                  <label>
                    <span>Name</span>
                    <input bind:value={editTypeValue} placeholder="Species" />
                  </label>
                  <div class="edit-actions">
                    <button type="button" class="action" onclick={commitTypeEdit}>Save</button>
                    <button type="button" class="quiet" onclick={cancelTypeEdit}>Cancel</button>
                  </div>
                </div>
              {:else}
                <div class="item-main">
                  <strong>{humanizeId(type)}</strong>
                  <code>{type}</code>
                </div>
                <div class="item-actions">
                  <button type="button" class="quiet" onclick={() => startTypeEdit(type)}>Edit</button>
                  <button type="button" class="danger" onclick={() => requestRemoveCustomType(type)}>Remove</button>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      <div class="add-form">
        <label>
          <span>New type</span>
          <input
            bind:value={newType}
            placeholder="Species"
            onkeydown={(event) => event.key === "Enter" && addCustomType()} />
        </label>
        <button type="button" class="action" onclick={addCustomType}>Add type</button>
      </div>
    </div>

    <div class="block">
      <div class="block-heading">
        <h4>Custom fields</h4>
        <span class="block-hint">Choose at least one entity type for each field</span>
      </div>
      {#if (draft.customFields ?? []).length === 0}
        <p class="empty">No custom fields yet.</p>
      {:else}
        <ul class="list">
          {#each draft.customFields ?? [] as field}
            <li class="list-item">
              {#if editingFieldKey === field.key}
                <div class="edit-form">
                  <label>
                    <span>Label</span>
                    <input bind:value={editFieldLabel} placeholder="Word count" />
                  </label>
                  <label>
                    <span>Type</span>
                    <select bind:value={editFieldType}>
                      {#each FIELD_TYPES as type}
                        <option value={type}>{fieldTypeLabel(type)}</option>
                      {/each}
                    </select>
                  </label>
                  <div class="type-select" role="group" aria-label="Applies to entity types">
                    <span class="type-select-label">Applies to <em>(required)</em></span>
                    <div class="chip-row compact">
                      {#each effectiveTypes() as type}
                        <button
                          type="button"
                          class="chip select"
                          class:selected={editFieldEntityTypes.includes(type)}
                          aria-pressed={editFieldEntityTypes.includes(type)}
                          onclick={() => (editFieldEntityTypes = toggleInList(editFieldEntityTypes, type))}>
                          {humanizeId(type)}
                        </button>
                      {/each}
                    </div>
                  </div>
                  <div class="edit-actions">
                    <button
                      type="button"
                      class="action"
                      disabled={!editFieldLabel.trim() || editFieldEntityTypes.length === 0}
                      onclick={commitFieldEdit}>Save</button>
                    <button type="button" class="quiet" onclick={cancelFieldEdit}>Cancel</button>
                  </div>
                </div>
              {:else}
                <div class="item-main">
                  <strong>{field.label}</strong>
                  <span class="meta">
                    {fieldTypeLabel(field.type)}
                    <span aria-hidden="true">·</span>
                    {scopeLabel(field.entityTypes)}
                  </span>
                  <code>{field.key}</code>
                </div>
                <div class="item-actions">
                  <button type="button" class="quiet" onclick={() => startFieldEdit(field)}>Edit</button>
                  <button type="button" class="danger" onclick={() => removeCustomField(field.key)}>Remove</button>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      <div class="add-form stacked">
        <div class="add-row">
          <label>
            <span>Label</span>
            <input
              bind:value={newFieldLabel}
              placeholder="Word count"
              onkeydown={(event) => event.key === "Enter" && addCustomField()} />
          </label>
          <label>
            <span>Type</span>
            <select bind:value={newFieldType}>
              {#each FIELD_TYPES as type}
                <option value={type}>{fieldTypeLabel(type)}</option>
              {/each}
            </select>
          </label>
          <button
            type="button"
            class="action"
            disabled={!newFieldLabel.trim() || newFieldEntityTypes.length === 0}
            onclick={addCustomField}>Add field</button>
        </div>
        <div class="type-select" role="group" aria-label="Applies to entity types">
          <span class="type-select-label">Applies to <em>(required)</em></span>
          <div class="chip-row compact">
            {#each effectiveTypes() as type}
              <button
                type="button"
                class="chip select"
                class:selected={newFieldEntityTypes.includes(type)}
                aria-pressed={newFieldEntityTypes.includes(type)}
                onclick={() => (newFieldEntityTypes = toggleInList(newFieldEntityTypes, type))}>
                {humanizeId(type)}
              </button>
            {/each}
          </div>
        </div>
      </div>
    </div>

    <div class="block">
      <div class="block-heading">
        <h4>Custom templates</h4>
        <span class="block-hint">Create shortcuts with optional descriptions</span>
      </div>
      {#if (draft.customTemplates ?? []).length === 0}
        <p class="empty">No custom templates yet.</p>
      {:else}
        <ul class="list">
          {#each draft.customTemplates ?? [] as template}
            <li class="list-item">
              {#if editingTemplateId === template.id}
                <div class="edit-form">
                  <label>
                    <span>Name</span>
                    <input bind:value={editTemplateName} placeholder="Species profile" />
                  </label>
                  <label>
                    <span>Entity type</span>
                    <select bind:value={editTemplateEntityType}>
                      <option value="">Choose type</option>
                      {#each effectiveTypes() as type}
                        <option value={type}>{humanizeId(type)}</option>
                      {/each}
                    </select>
                  </label>
                  <label class="grow">
                    <span>Description <em>(optional)</em></span>
                    <input bind:value={editTemplateDescription} placeholder="A kind of being in this world." />
                  </label>
                  <div class="edit-actions">
                    <button type="button" class="action" onclick={commitTemplateEdit}>Save</button>
                    <button type="button" class="quiet" onclick={cancelTemplateEdit}>Cancel</button>
                  </div>
                </div>
              {:else}
                <div class="item-main">
                  <strong>{template.name}</strong>
                  <span class="meta">
                    {humanizeId(template.entityType)}
                    {#if template.description}
                      <span aria-hidden="true">·</span>
                      {template.description}
                    {/if}
                  </span>
                </div>
                <div class="item-actions">
                  <button type="button" class="quiet" onclick={() => startTemplateEdit(template)}>Edit</button>
                  <button type="button" class="danger" onclick={() => removeCustomTemplate(template.id)}>Remove</button>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      <div class="add-form stacked">
        <div class="add-row">
          <label>
            <span>Name</span>
            <input bind:value={newTemplateName} placeholder="Species profile" />
          </label>
          <label>
            <span>Entity type</span>
            <select bind:value={newTemplateEntityType}>
              <option value="">Choose type</option>
              {#each effectiveTypes() as type}
                <option value={type}>{humanizeId(type)}</option>
              {/each}
            </select>
          </label>
          <button type="button" class="action" onclick={addCustomTemplate}>Add template</button>
        </div>
        <label class="grow">
          <span>Description <em>(optional)</em></span>
          <input bind:value={newTemplateDescription} placeholder="A kind of being in this world." />
        </label>
      </div>
    </div>

    <div class="actions">
      <button type="button" class="primary" disabled={busy} onclick={() => void save()}>
        {busy ? "Saving…" : "Save schema"}
      </button>
      {#if dirty && !message}
        <small class="dirty">Unsaved changes</small>
      {/if}
      {#if message}
        <small class:error={message.startsWith("Could")}>{message}</small>
      {/if}
    </div>
  {/if}

  {#if typeRemovalPrompt}
    <div class="type-remove-backdrop" role="presentation">
      <div class="type-remove-dialog" role="alertdialog" aria-modal="true" aria-labelledby="type-remove-title">
        <strong id="type-remove-title">Remove {humanizeId(typeRemovalPrompt.typeId)}?</strong>
        <p>
          Fields must keep at least one entity type. Templates for this type, and fields that only target it, are
          removed with the type.
        </p>
        {#if typeRemovalPrompt.templates.length > 0}
          <div class="type-remove-group">
            <span>Templates to remove</span>
            <ul>
              {#each typeRemovalPrompt.templates as template}
                <li>{template.name}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if typeRemovalPrompt.exclusiveFields.length > 0}
          <div class="type-remove-group">
            <span>Fields to remove (only this type)</span>
            <ul>
              {#each typeRemovalPrompt.exclusiveFields as field}
                <li>{field.label}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if typeRemovalPrompt.sharedFields.length > 0}
          <div class="type-remove-group">
            <span>Fields that also target other types</span>
            <ul>
              {#each typeRemovalPrompt.sharedFields as field}
                <li>
                  {field.label}
                  <small>
                    also {(field.entityTypes ?? [])
                      .filter((type) => type !== typeRemovalPrompt?.typeId)
                      .map(humanizeId)
                      .join(", ")}
                  </small>
                </li>
              {/each}
            </ul>
            <label class="type-remove-check">
              <input type="checkbox" bind:checked={typeRemovalPrompt.removeSharedFields} />
              <span>Also remove these shared fields</span>
            </label>
            {#if !typeRemovalPrompt.removeSharedFields}
              <p class="type-remove-note">
                Otherwise this type is dropped from their scope and they keep their other types.
              </p>
            {/if}
          </div>
        {/if}
        <div class="type-remove-actions">
          <button type="button" class="quiet" onclick={cancelTypeRemoval}>Cancel</button>
          <button type="button" class="danger" onclick={confirmTypeRemoval}>Remove</button>
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
.module-schema-panel {
  display: grid;
  gap: 16px;
}
header strong {
  display: block;
  font: 500 18px var(--font-display, Georgia, serif);
}
header p,
.muted,
.empty,
.block-hint {
  margin: 0;
  color: var(--ink-soft, #8f897e);
  font-size: 12px;
  line-height: 1.45;
}
header p,
.muted {
  margin-top: 6px;
}
.block {
  display: grid;
  gap: 12px;
  padding: 16px;
  border: 1px solid var(--line, #e9e1d4);
  border-radius: 12px;
  background: #fffefa;
}
.block-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px 16px;
}
h4 {
  margin: 0;
  font:
    600 12px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.02em;
  color: #62594e;
}
.chip-row,
.add-row,
.edit-actions,
.item-actions,
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.chip-row.compact {
  gap: 6px;
}
.chip,
.action,
.quiet,
.danger,
.primary {
  border: 1px solid #d9cdbd;
  border-radius: 8px;
  padding: 6px 11px;
  background: #fffefa;
  color: #62594e;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  cursor: pointer;
}
.chip {
  border-radius: 999px;
}
.chip.is-hidden {
  opacity: 0.42;
  border-style: dashed;
  text-decoration: line-through;
  text-decoration-thickness: 1px;
}
.chip.select.selected,
.chip[aria-pressed="true"]:not(.is-hidden) {
  border-color: #b7a88f;
  background: #f4eee4;
  color: #3f3830;
}
.chip.select:not(.selected) {
  opacity: 0.78;
}
.action {
  background: #f7f1e7;
}
.primary {
  border-color: #c4b49a;
  background: #302c26;
  color: #fffefa;
}
.action:disabled,
.primary:disabled {
  opacity: 0.55;
  cursor: default;
}
.danger {
  border-color: #d7a59a;
  background: #f8ece8;
  color: #9a4d3f;
}
.danger:hover {
  border-color: #c9897d;
  background: #f3ddd6;
}
.list {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.list-item {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 10px 16px;
  align-items: flex-start;
  padding: 10px 12px;
  border: 1px solid #ebe3d6;
  border-radius: 10px;
  background: #fffcf7;
}
.item-main {
  display: grid;
  gap: 3px;
  min-width: 0;
}
.item-main strong {
  font:
    600 13px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  color: #302c26;
}
.meta {
  color: #8f897e;
  font-size: 12px;
  line-height: 1.4;
}
code {
  width: fit-content;
  padding: 1px 6px;
  border-radius: 5px;
  background: #f1ebe1;
  color: #6f675c;
  font:
    500 10px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
}
.add-form,
.edit-form,
.stacked {
  display: grid;
  gap: 10px;
}
.add-form {
  padding-top: 2px;
}
.add-row {
  align-items: end;
}
label {
  display: grid;
  gap: 4px;
  min-width: 140px;
}
label.grow {
  min-width: min(100%, 280px);
  flex: 1;
}
label span {
  color: #8f897e;
  font:
    600 10px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
label em {
  font-style: normal;
  font-weight: 500;
  text-transform: none;
  letter-spacing: 0;
  color: #b0a89c;
}
.type-select {
  display: grid;
  gap: 6px;
}
.type-select-label {
  color: #8f897e;
  font:
    600 10px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.type-select-label em {
  font-style: normal;
  font-weight: 500;
  text-transform: none;
  letter-spacing: 0;
  color: #b0a89c;
}
input,
select {
  min-width: 140px;
  padding: 8px 10px;
  border: 1px solid #d9cdbd;
  border-radius: 8px;
  background: #fff;
  color: #302c26;
  font:
    12px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.actions {
  align-items: center;
  gap: 12px;
}
.actions small {
  color: #8f897e;
}
.actions small.dirty {
  color: #9a4d3f;
  font-weight: 600;
}
.actions small.error {
  color: #9a4d3f;
}
.type-remove-backdrop {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  background: rgba(48, 44, 38, 0.35);
}
.type-remove-dialog {
  width: min(440px, calc(100vw - 32px));
  display: grid;
  gap: 12px;
  padding: 18px 20px;
  border: 1px solid #d9cdbd;
  border-radius: 14px;
  background: #fffefa;
  box-shadow: 0 18px 40px rgba(48, 44, 38, 0.18);
}
.type-remove-dialog strong {
  font: 600 16px var(--font-display, Georgia, serif);
}
.type-remove-dialog > p,
.type-remove-note {
  margin: 0;
  color: #8f897e;
  font-size: 13px;
  line-height: 1.45;
}
.type-remove-group {
  display: grid;
  gap: 6px;
}
.type-remove-group > span {
  color: #62594e;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.type-remove-group ul {
  display: grid;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.type-remove-group li {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 10px;
  align-items: baseline;
  padding: 7px 9px;
  border: 1px solid #ebe3d6;
  border-radius: 8px;
  background: #fffcf7;
  font-size: 12px;
  color: #302c26;
}
.type-remove-group small {
  color: #8f897e;
}
.type-remove-check {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  margin-top: 2px;
  color: #302c26;
  font:
    600 12px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  text-transform: none;
  letter-spacing: 0;
}
.type-remove-check input {
  min-width: 0;
  width: 14px;
  height: 14px;
  padding: 0;
}
.type-remove-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}
</style>
