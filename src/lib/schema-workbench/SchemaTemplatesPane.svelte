<script lang="ts">
import type { EntityTemplate, FieldDefinition, ModuleSchemaOverlay } from "$lib/project/client";
import { defaultFieldValue, humanizeId } from "$lib/schema-workbench";
import SchemaFieldInput from "./SchemaFieldInput.svelte";
import SchemaTemplatePreview from "./SchemaTemplatePreview.svelte";
import "./schema-pane.css";
import {
  Check,
  ChevronDown,
  ChevronRight,
  EyeOff,
  LayoutTemplate,
  Pencil,
  Plus,
  Sparkles,
  Trash2,
} from "@lucide/svelte";

let {
  draft,
  packageTemplates,
  showAdvanced,
  selectedItemId = $bindable(),
  builtinTemplatesCollapsed = $bindable(),
  editingBuiltinTemplateId = $bindable(),
  editingTemplateId = $bindable(),
  editingTemplateFieldKeys = $bindable(),
  editingTemplateRequiredFields = $bindable(),
  editingTemplateFieldValues = $bindable(),
  editTemplateIncludeDocument = $bindable(),
  editTemplateName = $bindable(),
  editTemplateEntityType = $bindable(),
  editTemplateDescription = $bindable(),
  newTemplateName = $bindable(),
  newTemplateEntityType = $bindable(),
  newTemplateDescription = $bindable(),
  newTemplateFieldKeys = $bindable(),
  newTemplateRequiredFields = $bindable(),
  newTemplateFieldValues = $bindable(),
  newTemplateIncludeDocument = $bindable(),
  templateMatches,
  isDisabled,
  toggleDisabled,
  effectiveFieldsForType,
  editingPreviewFields,
  fieldsForTemplate,
  entityTypeLabel,
  effectiveTypes,
  toggleInList,
  beginTemplateFieldEdit,
  commitBuiltinTemplateEdit,
  cancelTemplateFieldEdit,
  startTemplateEdit,
  commitTemplateEdit,
  cancelTemplateEdit,
  removeCustomTemplate,
  addCustomTemplate,
}: {
  draft: ModuleSchemaOverlay;
  packageTemplates: EntityTemplate[];
  showAdvanced: boolean;
  selectedItemId: string | null;
  builtinTemplatesCollapsed: boolean;
  editingBuiltinTemplateId: string | null;
  editingTemplateId: string | null;
  editingTemplateFieldKeys: string[];
  editingTemplateRequiredFields: string[];
  editingTemplateFieldValues: Record<string, unknown>;
  editTemplateIncludeDocument: boolean;
  editTemplateName: string;
  editTemplateEntityType: string;
  editTemplateDescription: string;
  newTemplateName: string;
  newTemplateEntityType: string;
  newTemplateDescription: string;
  newTemplateFieldKeys: string[];
  newTemplateRequiredFields: string[];
  newTemplateFieldValues: Record<string, unknown>;
  newTemplateIncludeDocument: boolean;
  templateMatches: (template: EntityTemplate, origin: "builtin" | "custom") => boolean;
  isDisabled: (list: string[] | undefined, id: string) => boolean;
  toggleDisabled: (listKey: "disabledEntityTypes" | "disabledFields" | "disabledTemplates", id: string) => void;
  effectiveFieldsForType: (entityType: string) => FieldDefinition[];
  editingPreviewFields: (entityType: string) => Array<{ field: FieldDefinition; required: boolean }>;
  fieldsForTemplate: (template: EntityTemplate, builtin: boolean) => Record<string, unknown>;
  entityTypeLabel: (typeId: string) => string;
  effectiveTypes: () => string[];
  toggleInList: (list: string[], id: string) => string[];
  beginTemplateFieldEdit: (template: EntityTemplate, builtin: boolean) => void;
  commitBuiltinTemplateEdit: () => void;
  cancelTemplateFieldEdit: () => void;
  startTemplateEdit: (template: EntityTemplate) => void;
  commitTemplateEdit: () => void;
  cancelTemplateEdit: () => void;
  removeCustomTemplate: (id: string) => void;
  addCustomTemplate: () => void;
} = $props();

function toggleEditField(field: FieldDefinition) {
  editingTemplateFieldKeys = toggleInList(editingTemplateFieldKeys, field.key);
  if (editingTemplateFieldKeys.includes(field.key)) {
    if (!(field.key in editingTemplateFieldValues)) {
      editingTemplateFieldValues = { ...editingTemplateFieldValues, [field.key]: defaultFieldValue(field) };
    }
  } else {
    editingTemplateRequiredFields = editingTemplateRequiredFields.filter((key) => key !== field.key);
    const { [field.key]: _removed, ...remaining } = editingTemplateFieldValues;
    editingTemplateFieldValues = remaining;
  }
}

function toggleNewField(field: FieldDefinition) {
  newTemplateFieldKeys = toggleInList(newTemplateFieldKeys, field.key);
  if (newTemplateFieldKeys.includes(field.key)) {
    if (!(field.key in newTemplateFieldValues)) {
      newTemplateFieldValues = { ...newTemplateFieldValues, [field.key]: defaultFieldValue(field) };
    }
  } else {
    newTemplateRequiredFields = newTemplateRequiredFields.filter((key) => key !== field.key);
    const { [field.key]: _removed, ...remaining } = newTemplateFieldValues;
    newTemplateFieldValues = remaining;
  }
}

const selectedTemplateId = $derived(selectedItemId?.startsWith("template:") ? selectedItemId.slice(9) : null);
const selectedBuiltinTemplate = $derived(packageTemplates.find((template) => template.id === selectedTemplateId));
const selectedCustomTemplate = $derived(
  (draft.customTemplates ?? []).find((template) => template.id === selectedTemplateId),
);

function selectBuiltinTemplate(template: EntityTemplate) {
  cancelTemplateFieldEdit();
  selectedItemId = `template:${template.id}`;
}
</script>

<div class="schema-workbench-pane workbench-split">
  <div class="workbench-list">
    <div class="block elevated">
      <button
        type="button"
        class="block-heading collapsible"
        aria-expanded={!builtinTemplatesCollapsed}
        onclick={() => (builtinTemplatesCollapsed = !builtinTemplatesCollapsed)}>
        <div class="heading-left">
          <span class="heading-icon"><LayoutTemplate size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h4>Builtin templates</h4>
          <span class="count-badge">{packageTemplates.length}</span>
        </div>
        <span class="block-hint">Enable templates and choose their included fields</span>
        <span class="collapse-icon" aria-hidden="true"
          >{#if builtinTemplatesCollapsed}<ChevronRight size={14} strokeWidth={1.8} />{:else}<ChevronDown
              size={14}
              strokeWidth={1.8} />{/if}</span>
      </button>
      {#if !builtinTemplatesCollapsed}
        <ul class="list compact-list">
          {#each packageTemplates.filter((template) => templateMatches(template, "builtin")) as template}
            <li class="list-item compact" class:selected-detail={selectedItemId === `template:${template.id}`}>
              <button type="button" class="item-select" onclick={() => selectBuiltinTemplate(template)}>
                <div class="item-main">
                  <div class="item-title-row">
                    <strong>{template.name}</strong>
                    <span class="type-pill ghost">{entityTypeLabel(template.entityType)}</span>
                    {#if isDisabled(draft.disabledTemplates, template.id)}<span class="disabled-pill"
                        ><EyeOff size={10} strokeWidth={1.8} aria-hidden="true" /> Disabled</span
                      >{/if}
                  </div>
                  <span class="meta"
                    >{Object.keys(fieldsForTemplate(template, true)).length} fields <span class="dot">·</span>
                    {template.description || "Builtin template"}
                    {#if showAdvanced}<span class="dot">·</span> <code>{template.id}</code>{/if}
                  </span>
                </div>
              </button>
              <button
                type="button"
                class="chip"
                class:is-hidden={isDisabled(draft.disabledTemplates, template.id)}
                aria-pressed={!isDisabled(draft.disabledTemplates, template.id)}
                onclick={() => toggleDisabled("disabledTemplates", template.id)}>
                {isDisabled(draft.disabledTemplates, template.id) ? "Enable" : "Enabled"}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="block elevated">
      <div class="block-heading">
        <div class="heading-left">
          <span class="heading-icon accent"><Sparkles size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h4>Custom templates</h4>
          <span class="count-badge accent">{(draft.customTemplates ?? []).length}</span>
        </div>
        <span class="block-hint">Create shortcuts with optional descriptions</span>
      </div>
      {#if (draft.customTemplates ?? []).length === 0}
        <div class="empty-inline">
          <LayoutTemplate size={16} strokeWidth={1.7} aria-hidden="true" />
          <div>
            <strong>No custom templates yet</strong>
            <span>Templates bundle fields for quick creation — e.g., “Species profile” for a custom Species type.</span>
          </div>
        </div>
      {:else}
        <ul class="list">
          {#each (draft.customTemplates ?? []).filter((template) => templateMatches(template, "custom")) as template}
            <li class="list-item" class:selected-detail={selectedItemId === `template:${template.id}`}>
              <button type="button" class="item-select" onclick={() => startTemplateEdit(template)}>
                <div class="item-main">
                  <div class="item-title-row">
                    <strong>{template.name}</strong>
                    <span class="type-pill ghost">{entityTypeLabel(template.entityType)}</span>
                  </div>
                  <span class="meta">
                    {entityTypeLabel(template.entityType)}
                    {#if template.description}
                      <span class="dot">·</span>
                      {template.description}
                    {/if}
                    {#if showAdvanced}<span class="dot">·</span> <code>{template.id}</code>{/if}
                  </span>
                </div>
              </button>
              <div class="item-actions">
                <button
                  type="button"
                  class="quiet icon"
                  aria-label="Edit {template.name}"
                  onclick={() => startTemplateEdit(template)}
                  ><Pencil size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                <button
                  type="button"
                  class="danger icon"
                  aria-label="Remove {template.name}"
                  onclick={() => removeCustomTemplate(template.id)}
                  ><Trash2 size={14} strokeWidth={1.8} aria-hidden="true" /></button>
              </div>
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
                <option value={type}>{entityTypeLabel(type)}</option>
              {/each}
            </select>
          </label>
        </div>
        {#if newTemplateEntityType && effectiveTypes().includes(newTemplateEntityType)}
          <div class="type-select" role="group" aria-label="Included fields">
            <span class="type-select-label">Included fields</span>
            <div class="chip-row compact">
              {#each effectiveFieldsForType(newTemplateEntityType) as field}
                <button
                  type="button"
                  class="chip select"
                  class:selected={newTemplateFieldKeys.includes(field.key)}
                  aria-pressed={newTemplateFieldKeys.includes(field.key)}
                  onclick={() => toggleNewField(field)}>{field.label || humanizeId(field.key)}</button>
              {/each}
            </div>
          </div>
          <div class="type-select" role="group" aria-label="Required fields">
            <span class="type-select-label">Required fields</span>
            <div class="chip-row compact">
              {#each effectiveFieldsForType(newTemplateEntityType).filter( (f) => newTemplateFieldKeys.includes(f.key) ) as field}
                <button
                  type="button"
                  class="chip select"
                  class:selected={newTemplateRequiredFields.includes(field.key)}
                  aria-pressed={newTemplateRequiredFields.includes(field.key)}
                  onclick={() => (newTemplateRequiredFields = toggleInList(newTemplateRequiredFields, field.key))}
                  >{field.label || humanizeId(field.key)}</button>
              {/each}
              {#if newTemplateFieldKeys.length === 0}
                <span class="meta">Select at least one included field to mark as required.</span>
              {/if}
            </div>
          </div>
          <div class="type-select" aria-label="Template field defaults">
            <span class="type-select-label">Defaults</span>
            {#each effectiveFieldsForType(newTemplateEntityType).filter( (field) => newTemplateFieldKeys.includes(field.key) ) as field}
              <SchemaFieldInput
                {field}
                value={newTemplateFieldValues[field.key]}
                readOnly={false}
                idPrefix="new-template-default"
                onChange={(value) => (newTemplateFieldValues = { ...newTemplateFieldValues, [field.key]: value })} />
            {/each}
            {#if newTemplateFieldKeys.length === 0}<span class="meta">Select included fields to set defaults.</span
              >{/if}
          </div>
        {/if}
        <label class="grow">
          <span>Description <em>(optional)</em></span>
          <input bind:value={newTemplateDescription} placeholder="A kind of being in this world." />
        </label>
        <label class="inline-check">
          <input type="checkbox" bind:checked={newTemplateIncludeDocument} />
          <span>Include opening note</span>
        </label>
        <button
          type="button"
          class="action primary-action"
          onclick={addCustomTemplate}
          disabled={!newTemplateName.trim() || !newTemplateEntityType.trim()}
          ><Plus size={14} strokeWidth={2} aria-hidden="true" /> Add template</button>
      </div>
    </div>
  </div>

  <div class="workbench-detail">
    {#if selectedBuiltinTemplate}
      {@const template = selectedBuiltinTemplate}
      <div class="block elevated detail-card">
        <div class="block-heading">
          <div class="heading-left">
            <span class="heading-icon"><LayoutTemplate size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>{template.name}</h4>
          </div>
          <span class="type-pill">Built in</span>
        </div>
        {#if editingBuiltinTemplateId === template.id}
          <div class="edit-form wide">
            <div class="type-select" role="group" aria-label={`Fields for ${template.name}`}>
              <span class="type-select-label">Included fields</span>
              <div class="chip-row compact">
                {#each effectiveFieldsForType(template.entityType) as field}
                  <button
                    type="button"
                    class="chip select"
                    class:selected={editingTemplateFieldKeys.includes(field.key)}
                    aria-pressed={editingTemplateFieldKeys.includes(field.key)}
                    onclick={() => toggleEditField(field)}>{field.label || humanizeId(field.key)}</button>
                {/each}
              </div>
            </div>
            <div class="type-select" aria-label={`Defaults for ${template.name}`}>
              <span class="type-select-label">Defaults</span>
              {#each effectiveFieldsForType(template.entityType).filter( (field) => editingTemplateFieldKeys.includes(field.key) ) as field}
                <SchemaFieldInput
                  {field}
                  value={editingTemplateFieldValues[field.key]}
                  readOnly={false}
                  idPrefix={`builtin-default-${template.id}`}
                  onChange={(value) =>
                    (editingTemplateFieldValues = { ...editingTemplateFieldValues, [field.key]: value })} />
              {/each}
            </div>
            <div class="type-select" role="group" aria-label={`Required fields for ${template.name}`}>
              <span class="type-select-label">Required fields</span>
              <div class="chip-row compact">
                {#each effectiveFieldsForType(template.entityType).filter( (field) => editingTemplateFieldKeys.includes(field.key) ) as field}
                  <button
                    type="button"
                    class="chip select"
                    class:selected={editingTemplateRequiredFields.includes(field.key)}
                    aria-pressed={editingTemplateRequiredFields.includes(field.key)}
                    onclick={() =>
                      (editingTemplateRequiredFields = toggleInList(editingTemplateRequiredFields, field.key))}
                    >{field.label || humanizeId(field.key)}</button>
                {/each}
              </div>
            </div>
            <SchemaTemplatePreview
              {template}
              fields={editingPreviewFields(template.entityType)}
              values={editingTemplateFieldValues}
              showDocument={Boolean(template.document !== undefined)}
              readOnly={true}
              idPrefix={`builtin-template-${template.id}`} />
            <div class="edit-actions">
              <button type="button" class="action" onclick={commitBuiltinTemplateEdit}
                ><Check size={14} strokeWidth={2} aria-hidden="true" /> Save fields</button>
              <button type="button" class="quiet" onclick={cancelTemplateFieldEdit}>Cancel</button>
            </div>
          </div>
        {:else}
          <p class="subtle-note">{template.description || "Builtin template"}</p>
          <SchemaTemplatePreview
            {template}
            fields={effectiveFieldsForType(template.entityType)
              .filter((field) => Object.keys(fieldsForTemplate(template, true)).includes(field.key))
              .map((field) => ({
                field,
                required: Boolean(template.requiredFields?.includes(field.key)),
              }))}
            values={fieldsForTemplate(template, true)}
            showDocument={Boolean(template.document !== undefined)}
            readOnly={true}
            idPrefix={`builtin-template-summary-${template.id}`} />
          <div class="edit-actions">
            <button type="button" class="action" onclick={() => beginTemplateFieldEdit(template, true)}
              ><Pencil size={14} strokeWidth={1.8} aria-hidden="true" /> Customize fields</button>
          </div>
        {/if}
      </div>
    {:else if selectedCustomTemplate && editingTemplateId === selectedCustomTemplate.id}
      {@const template = selectedCustomTemplate}
      <div class="block elevated detail-card">
        <div class="block-heading">
          <div class="heading-left">
            <span class="heading-icon accent"><Sparkles size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>{template.name}</h4>
          </div>
          <span class="type-pill">Project custom</span>
        </div>
        <div class="edit-form">
          <label>
            <span>Name</span>
            <input bind:value={editTemplateName} placeholder="Species profile" />
          </label>
          <label>
            <span>Entity type</span>
            <select bind:value={editTemplateEntityType}>
              <option value="">Choose type</option>
              {#each effectiveTypes() as type}<option value={type}>{entityTypeLabel(type)}</option>{/each}
            </select>
          </label>
          <div class="type-select" role="group" aria-label={`Fields for ${editTemplateName || template.name}`}>
            <span class="type-select-label">Included fields</span>
            <div class="chip-row compact">
              {#each effectiveFieldsForType(editTemplateEntityType) as field}
                <button
                  type="button"
                  class="chip select"
                  class:selected={editingTemplateFieldKeys.includes(field.key)}
                  aria-pressed={editingTemplateFieldKeys.includes(field.key)}
                  onclick={() => toggleEditField(field)}>{field.label || humanizeId(field.key)}</button>
              {/each}
            </div>
          </div>
          <div class="type-select" role="group" aria-label={`Required fields for ${template.name}`}>
            <span class="type-select-label">Required fields</span>
            <div class="chip-row compact">
              {#each effectiveFieldsForType(editTemplateEntityType).filter( (field) => editingTemplateFieldKeys.includes(field.key) ) as field}
                <button
                  type="button"
                  class="chip select"
                  class:selected={editingTemplateRequiredFields.includes(field.key)}
                  aria-pressed={editingTemplateRequiredFields.includes(field.key)}
                  onclick={() =>
                    (editingTemplateRequiredFields = toggleInList(editingTemplateRequiredFields, field.key))}
                  >{field.label || humanizeId(field.key)}</button>
              {/each}
            </div>
          </div>
          <div class="type-select" aria-label={`Defaults for ${template.name}`}>
            <span class="type-select-label">Defaults</span>
            {#each effectiveFieldsForType(editTemplateEntityType).filter( (field) => editingTemplateFieldKeys.includes(field.key) ) as field}
              <SchemaFieldInput
                {field}
                value={editingTemplateFieldValues[field.key]}
                readOnly={false}
                idPrefix={`custom-default-${template.id}`}
                onChange={(value) =>
                  (editingTemplateFieldValues = { ...editingTemplateFieldValues, [field.key]: value })} />
            {/each}
          </div>
          <label class="grow">
            <span>Description <em>(optional)</em></span>
            <input bind:value={editTemplateDescription} placeholder="A kind of being in this world." />
          </label>
          <label class="inline-check">
            <input type="checkbox" bind:checked={editTemplateIncludeDocument} />
            <span>Include opening note</span>
          </label>
          <SchemaTemplatePreview
            template={{
              ...template,
              name: editTemplateName || template.name,
              entityType: editTemplateEntityType || template.entityType,
              description: editTemplateDescription.trim() || null,
            }}
            fields={editingPreviewFields(editTemplateEntityType || template.entityType)}
            values={editingTemplateFieldValues}
            showDocument={editTemplateIncludeDocument}
            readOnly={true}
            idPrefix={`custom-template-${template.id}`} />
          <div class="edit-actions">
            <button type="button" class="action" onclick={commitTemplateEdit}
              ><Check size={14} strokeWidth={2} aria-hidden="true" /> Save</button>
            <button type="button" class="quiet" onclick={cancelTemplateEdit}>Cancel</button>
          </div>
        </div>
      </div>
    {:else if newTemplateEntityType && effectiveTypes().includes(newTemplateEntityType)}
      <div class="block elevated detail-card">
        <div class="block-heading">
          <div class="heading-left">
            <span class="heading-icon accent"><Plus size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>{newTemplateName || "New template"}</h4>
          </div>
          <span class="type-pill">Preview</span>
        </div>
        <SchemaTemplatePreview
          template={{
            id: "new-template",
            name: newTemplateName || "New template",
            entityType: newTemplateEntityType,
            description: newTemplateDescription.trim() || null,
            fields: newTemplateFieldValues,
            requiredFields: newTemplateRequiredFields,
            ...(newTemplateIncludeDocument ? { document: "" } : {}),
          }}
          fields={effectiveFieldsForType(newTemplateEntityType)
            .filter((field) => newTemplateFieldKeys.includes(field.key))
            .map((field) => ({ field, required: newTemplateRequiredFields.includes(field.key) }))}
          values={newTemplateFieldValues}
          showDocument={newTemplateIncludeDocument}
          readOnly={true}
          idPrefix="new-template-preview" />
      </div>
    {:else}
      <div class="block detail-placeholder">
        <strong>Select a template</strong>
        <span>Choose a template to preview it or customize its fields and defaults.</span>
      </div>
    {/if}
  </div>
</div>
