<script lang="ts">
import { manifestText } from "$lib/i18n/manifest-copy";
import { ui } from "$lib/i18n/ui";
import type { FieldDefinition, MetadataFieldDefinition, ModuleSchemaOverlay } from "$lib/project/client";
import {
  FIELD_KIND_GROUPS,
  METADATA_FIELD_TYPES,
  ensureFieldKey,
  fieldFormHasErrors,
  fieldTypeLabel,
  humanizeId,
  showAdvancedControl,
  slugifyFieldKey,
  validateFieldForm,
  validateMetadataDrafts,
  type FieldType,
  type FlattenedPackageSchema,
  type MetadataFieldDraft,
} from "$lib/schema-workbench";
import "./schema-pane.css";
import {
  Blocks,
  Check,
  ChevronDown,
  ChevronRight,
  EyeOff,
  Pencil,
  Plus,
  Sparkles,
  TextQuote,
  Trash2,
  X,
} from "@lucide/svelte";

type VariantDraft = { label: string; type: FieldType; options: string };
type TimelineRole = "point" | "start" | "end";
type TimelineLayer = "dates" | "lifelines";

let {
  pluginId,
  draft,
  flatPackage,
  packageFields,
  showAdvanced,
  selectedItemId = $bindable(),
  builtinFieldsCollapsed = $bindable(),
  editingBuiltinFieldKey = $bindable(),
  editingBuiltinMetadataFieldKey = $bindable(),
  editBuiltinMetadataDrafts = $bindable(),
  editingTimelineFieldKey = $bindable(),
  editTimelineRole = $bindable(),
  editTimelineGroup = $bindable(),
  editTimelineLabel = $bindable(),
  editTimelineLayer = $bindable(),
  editingFieldKey = $bindable(),
  editFieldLabel = $bindable(),
  editFieldType = $bindable(),
  editFieldEntityTypes = $bindable(),
  editFieldOptions = $bindable(),
  editFieldMultiple = $bindable(),
  editFieldTargetEntityTypes = $bindable(),
  editFieldRelationshipType = $bindable(),
  editFieldCardinality = $bindable(),
  editFieldOneOfVariants = $bindable(),
  editFieldMetadata = $bindable(),
  editFieldShared = $bindable(),
  editFieldTimelineEnabled = $bindable(),
  editFieldTimelineRole = $bindable(),
  editFieldTimelineGroup = $bindable(),
  editFieldTimelineLabel = $bindable(),
  editFieldTimelineLayer = $bindable(),
  newFieldLabel = $bindable(),
  newFieldType = $bindable(),
  newFieldEntityTypes = $bindable(),
  newFieldOptions = $bindable(),
  newFieldMultiple = $bindable(),
  newFieldTargetEntityTypes = $bindable(),
  newFieldRelationshipType = $bindable(),
  newFieldCardinality = $bindable(),
  newFieldOneOfVariants = $bindable(),
  newFieldMetadata = $bindable(),
  newFieldShared = $bindable(),
  newFieldTimelineEnabled = $bindable(),
  newFieldTimelineRole = $bindable(),
  newFieldTimelineGroup = $bindable(),
  newFieldTimelineLabel = $bindable(),
  newFieldTimelineLayer = $bindable(),
  fieldMatches,
  isDisabled,
  toggleDisabled,
  fieldScopeTypes,
  builtinFieldScope,
  updateBuiltinFieldScope,
  toggleInList,
  entityTypeLabel,
  builtinOriginalMetadata,
  effectiveBuiltinMetadata,
  builtinMetadataFieldExtras,
  removeEditBuiltinMetadata,
  removeEditBuiltinMetadataOneOfVariant,
  addEditBuiltinMetadataOneOfVariant,
  addEditBuiltinMetadata,
  commitBuiltinMetadataEdit,
  cancelBuiltinMetadataEdit,
  commitTimelineEdit,
  cancelTimelineEdit,
  isTimelineEnabled,
  timelineBadge,
  scopeLabel,
  startBuiltinMetadataEdit,
  startTimelineEdit,
  disableTimeline,
  removeEditFieldOneOfVariant,
  addEditFieldOneOfVariant,
  removeEditFieldMetadata,
  removeEditFieldMetadataOneOfVariant,
  addEditFieldMetadataOneOfVariant,
  addEditFieldMetadata,
  canSaveFieldEdit,
  commitFieldEdit,
  cancelFieldEdit,
  fieldExtrasLabel,
  startFieldEdit,
  removeCustomField,
  canAddField,
  addCustomField,
  removeNewFieldOneOfVariant,
  addNewFieldOneOfVariant,
  removeNewFieldMetadata,
  removeNewFieldMetadataOneOfVariant,
  addNewFieldMetadataOneOfVariant,
  addNewFieldMetadata,
}: {
  pluginId: string | null;
  draft: ModuleSchemaOverlay;
  flatPackage: FlattenedPackageSchema;
  packageFields: FieldDefinition[];
  showAdvanced: boolean;
  selectedItemId: string | null;
  builtinFieldsCollapsed: boolean;
  editingBuiltinFieldKey: string | null;
  editingBuiltinMetadataFieldKey: string | null;
  editBuiltinMetadataDrafts: MetadataFieldDraft[];
  editingTimelineFieldKey: string | null;
  editTimelineRole: TimelineRole;
  editTimelineGroup: string;
  editTimelineLabel: string;
  editTimelineLayer: TimelineLayer;
  editingFieldKey: string | null;
  editFieldLabel: string;
  editFieldType: FieldType;
  editFieldEntityTypes: string[];
  editFieldOptions: string;
  editFieldMultiple: boolean;
  editFieldTargetEntityTypes: string[];
  editFieldRelationshipType: string;
  editFieldCardinality: "one" | "many";
  editFieldOneOfVariants: VariantDraft[];
  editFieldMetadata: MetadataFieldDraft[];
  editFieldShared: boolean;
  editFieldTimelineEnabled: boolean;
  editFieldTimelineRole: TimelineRole;
  editFieldTimelineGroup: string;
  editFieldTimelineLabel: string;
  editFieldTimelineLayer: TimelineLayer;
  newFieldLabel: string;
  newFieldType: FieldType;
  newFieldEntityTypes: string[];
  newFieldOptions: string;
  newFieldMultiple: boolean;
  newFieldTargetEntityTypes: string[];
  newFieldRelationshipType: string;
  newFieldCardinality: "one" | "many";
  newFieldOneOfVariants: VariantDraft[];
  newFieldMetadata: MetadataFieldDraft[];
  newFieldShared: boolean;
  newFieldTimelineEnabled: boolean;
  newFieldTimelineRole: TimelineRole;
  newFieldTimelineGroup: string;
  newFieldTimelineLabel: string;
  newFieldTimelineLayer: TimelineLayer;
  fieldMatches: (field: FieldDefinition, origin: "builtin" | "custom") => boolean;
  isDisabled: (list: string[] | undefined, id: string) => boolean;
  toggleDisabled: (key: "disabledEntityTypes" | "disabledFields" | "disabledTemplates", id: string) => void;
  fieldScopeTypes: () => string[];
  builtinFieldScope: (field: FieldDefinition) => string[];
  updateBuiltinFieldScope: (field: FieldDefinition, types: string[]) => void;
  toggleInList: (list: string[], id: string) => string[];
  entityTypeLabel: (id: string) => string;
  builtinOriginalMetadata: (field: FieldDefinition) => MetadataFieldDefinition[];
  effectiveBuiltinMetadata: (field: FieldDefinition) => MetadataFieldDefinition[];
  builtinMetadataFieldExtras: (field: FieldDefinition) => string;
  removeEditBuiltinMetadata: (index: number) => void;
  removeEditBuiltinMetadataOneOfVariant: (metaIndex: number, variantIndex: number) => void;
  addEditBuiltinMetadataOneOfVariant: (metaIndex: number) => void;
  addEditBuiltinMetadata: () => void;
  commitBuiltinMetadataEdit: () => void;
  cancelBuiltinMetadataEdit: () => void;
  commitTimelineEdit: () => void;
  cancelTimelineEdit: () => void;
  isTimelineEnabled: (field: FieldDefinition) => boolean;
  timelineBadge: (field: FieldDefinition) => string;
  scopeLabel: (types: string[] | undefined) => string;
  startBuiltinMetadataEdit: (field: FieldDefinition) => void;
  startTimelineEdit: (field: FieldDefinition) => void;
  disableTimeline: (field: FieldDefinition) => void;
  removeEditFieldOneOfVariant: (index: number) => void;
  addEditFieldOneOfVariant: () => void;
  removeEditFieldMetadata: (index: number) => void;
  removeEditFieldMetadataOneOfVariant: (metaIndex: number, variantIndex: number) => void;
  addEditFieldMetadataOneOfVariant: (metaIndex: number) => void;
  addEditFieldMetadata: () => void;
  canSaveFieldEdit: () => boolean;
  commitFieldEdit: () => void;
  cancelFieldEdit: () => void;
  fieldExtrasLabel: (field: FieldDefinition) => string;
  startFieldEdit: (field: FieldDefinition) => void;
  removeCustomField: (key: string) => void;
  canAddField: () => boolean;
  addCustomField: () => void;
  removeNewFieldOneOfVariant: (index: number) => void;
  addNewFieldOneOfVariant: () => void;
  removeNewFieldMetadata: (index: number) => void;
  removeNewFieldMetadataOneOfVariant: (metaIndex: number, variantIndex: number) => void;
  addNewFieldMetadataOneOfVariant: (metaIndex: number) => void;
  addNewFieldMetadata: () => void;
} = $props();

const existingFieldKeys = $derived([
  ...packageFields.map((field) => field.key),
  ...(draft.customFields ?? []).map((field) => field.key),
]);
const newFieldErrors = $derived(
  validateFieldForm({
    label: newFieldLabel,
    key: ensureFieldKey(newFieldLabel),
    type: newFieldType,
    optionsText: newFieldOptions,
    oneOfVariants: newFieldOneOfVariants,
    relationshipType: newFieldRelationshipType,
    targetEntityTypes: newFieldTargetEntityTypes,
    cardinality: newFieldCardinality,
    metadataDrafts: newFieldMetadata,
    metadataValid: validateMetadataDrafts(newFieldMetadata),
    shared: newFieldShared,
    timelineEnabled: newFieldTimelineEnabled,
    timelineRole: newFieldTimelineRole,
    timelineGroup: newFieldTimelineGroup,
    existingKeys: existingFieldKeys,
  }),
);
const editFieldErrors = $derived(
  validateFieldForm({
    label: editFieldLabel,
    key: editingFieldKey ?? "",
    type: editFieldType,
    optionsText: editFieldOptions,
    oneOfVariants: editFieldOneOfVariants,
    relationshipType: editFieldRelationshipType,
    targetEntityTypes: editFieldTargetEntityTypes,
    cardinality: editFieldCardinality,
    metadataDrafts: editFieldMetadata,
    metadataValid: validateMetadataDrafts(editFieldMetadata),
    shared: editFieldShared,
    timelineEnabled: editFieldTimelineEnabled,
    timelineRole: editFieldTimelineRole,
    timelineGroup: editFieldTimelineGroup,
    existingKeys: existingFieldKeys,
    editingKey: editingFieldKey,
  }),
);
const selectedFieldKey = $derived(selectedItemId?.startsWith("field:") ? selectedItemId.slice(6) : null);
const selectedBuiltinField = $derived(packageFields.find((field) => field.key === selectedFieldKey));
const selectedCustomField = $derived((draft.customFields ?? []).find((field) => field.key === selectedFieldKey));

function selectBuiltinField(field: FieldDefinition) {
  editingBuiltinFieldKey = null;
  editingBuiltinMetadataFieldKey = null;
  editBuiltinMetadataDrafts = [];
  editingTimelineFieldKey = null;
  editingFieldKey = null;
  selectedItemId = `field:${field.key}`;
}
</script>

<div class="schema-workbench-pane workbench-split">
  <div class="workbench-list">
    <div class="block elevated">
      <button
        type="button"
        class="block-heading collapsible"
        aria-expanded={!builtinFieldsCollapsed}
        onclick={() => (builtinFieldsCollapsed = !builtinFieldsCollapsed)}>
        <div class="heading-left">
          <span class="heading-icon"><TextQuote size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h4>{$ui("Builtin fields")}</h4>
          <span class="count-badge">{packageFields.length}</span>
        </div>
        <span class="block-hint">{$ui("Enable fields and choose the entity types they apply to")}</span>
        <span class="collapse-icon" aria-hidden="true"
          >{#if builtinFieldsCollapsed}<ChevronRight size={14} strokeWidth={1.8} />{:else}<ChevronDown
              size={14}
              strokeWidth={1.8} />{/if}</span>
      </button>
      {#if !builtinFieldsCollapsed}
        <ul class="list compact-list">
          {#each packageFields.filter((field) => fieldMatches(field, "builtin")) as field}
            <li class="list-item compact" class:selected-detail={selectedItemId === `field:${field.key}`}>
              <button type="button" class="item-select" onclick={() => selectBuiltinField(field)}>
                <div class="item-main">
                  <div class="item-title-row">
                    <strong>{$manifestText(pluginId, "field", field.key, field.label) || humanizeId(field.key)}</strong>
                    <span class="type-pill">{fieldTypeLabel(field.type)}</span>
                    <span class:disabled-pill={isDisabled(draft.disabledFields, field.key)} class="meta">
                      {#if isDisabled(draft.disabledFields, field.key)}
                        <EyeOff size={10} strokeWidth={1.8} aria-hidden="true" /> {$ui("Disabled")} {:else} {$ui("Enabled")} {/if}
                    </span>
                    {#if field.type === "relationship" && effectiveBuiltinMetadata(field).length > 0}
                      <span class="meta">{builtinMetadataFieldExtras(field)}</span>
                    {/if}
                    {#if field.type === "date"}
                      {#if isTimelineEnabled(field)}
                        <span class="meta">{$ui("· Timeline: {0}", timelineBadge(field))}</span>
                      {:else if (field as unknown as Record<string, unknown>).shared}
                        <span class="meta">{$ui("· Shared, not on Timeline")}</span>
                      {/if}
                    {/if}
                  </div>
                  <span class="meta"
                    >{scopeLabel(builtinFieldScope(field))}
                    {#if showAdvanced}
                      <span class="dot">·</span> <code>{field.key}</code>
                      <span class="dot">·</span>
                      <span class="namespace-pill">{flatPackage.fieldNamespace[field.key]}</span>
                    {/if}
                  </span>
                </div>
              </button>
              <button
                type="button"
                class="chip"
                class:is-hidden={isDisabled(draft.disabledFields, field.key)}
                aria-pressed={!isDisabled(draft.disabledFields, field.key)}
                onclick={() => toggleDisabled("disabledFields", field.key)}>
                {isDisabled(draft.disabledFields, field.key) ? $ui("Enable") : $ui("Enabled")}
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
          <h4>{$ui("Custom fields")}</h4>
          <span class="count-badge accent">{(draft.customFields ?? []).length}</span>
        </div>
        <span class="block-hint">{$ui("Choose at least one entity type for each field")}</span>
      </div>
      {#if (draft.customFields ?? []).length === 0}
        <div class="empty-inline">
          <Blocks size={16} strokeWidth={1.7} aria-hidden="true" />
          <div>
            <strong>{$ui("No custom fields yet")}</strong>
            <span>{$ui("Add a field like “Word count” or “Origin”.")}</span>
          </div>
        </div>
      {:else}
        <ul class="list">
          {#each (draft.customFields ?? []).filter((field) => fieldMatches(field, "custom")) as field}
            <li class="list-item" class:selected-detail={selectedItemId === `field:${field.key}`}>
              <button type="button" class="item-select" onclick={() => startFieldEdit(field)}>
                <div class="item-main">
                  <div class="item-title-row">
                    <strong>{$manifestText(pluginId, "field", field.key, field.label)}</strong><span class="type-pill">{fieldTypeLabel(field.type)}</span>
                    {#if fieldExtrasLabel(field)}<span class="meta">{fieldExtrasLabel(field)}</span>{/if}
                    {#if field.type === "date" && (field as unknown as Record<string, unknown>).timeline}
                      <span class="meta">{$ui("· Timeline: {0}", timelineBadge(field))}</span>
                    {:else if field.type === "date" && (field as unknown as Record<string, unknown>).shared}
                      <span class="meta">{$ui("· Shared")}</span>
                    {/if}
                  </div>
                  <span class="meta">
                    {scopeLabel(field.entityTypes)}
                    {#if showAdvanced}<span class="dot">·</span> <code>{field.key}</code>{/if}
                  </span>
                </div>
              </button>
              <div class="item-actions">
                <button
                  type="button"
                  class="quiet icon"
                  aria-label={$ui("Edit {0}", $manifestText(pluginId, "field", field.key, field.label))}
                  onclick={() => startFieldEdit(field)}
                  ><Pencil size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                <button
                  type="button"
                  class="danger icon"
                  aria-label={$ui("Remove {0}", $manifestText(pluginId, "field", field.key, field.label))}
                  onclick={() => removeCustomField(field.key)}
                  ><Trash2 size={14} strokeWidth={1.8} aria-hidden="true" /></button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}

      <div class="add-form stacked">
        <div class="add-row">
          <label>
            <span>{$ui("Label")}</span>
            <input
              bind:value={newFieldLabel}
              placeholder={$ui("Word count")}
              onkeydown={(event) => event.key === "Enter" && addCustomField()} />
          </label>
          {#if showAdvancedControl(showAdvanced, "field-key")}
            <label>
              <span>{$ui("Stable key")}</span><input value={ensureFieldKey(newFieldLabel)} readonly aria-readonly="true" />
            </label>
          {/if}
          <label>
            <span>{$ui("Kind")}</span>
            <select bind:value={newFieldType}>
              {#each Object.values(FIELD_KIND_GROUPS) as group}
                <optgroup label={$ui(group.label)}>
                  {#each group.types as type}<option value={type}>{fieldTypeLabel(type)}</option>{/each}
                </optgroup>
              {/each}
            </select>
          </label>
          <button
            type="button"
            class="action primary-action"
            disabled={fieldFormHasErrors(newFieldErrors) || !canAddField()}
            onclick={addCustomField}><Plus size={14} strokeWidth={2} aria-hidden="true" /> {$ui("Add field")}</button>
        </div>
        {#if newFieldType === "enum"}
          <label>
            <span>{$ui("Options")} <em>{$ui("(comma separated)")}</em></span>
            <input bind:value={newFieldOptions} placeholder={$ui("idea, drafting, revising, complete")} />
            {#if newFieldErrors.choices}<small class="field-error">{newFieldErrors.choices}</small>{/if}
          </label>
          <label class="inline-check">
            <input type="checkbox" bind:checked={newFieldMultiple} /><span>{$ui("Allow multiple values")}</span>
          </label>
        {:else if newFieldType === "oneof"}
          <div class="type-select" role="group" aria-label={$ui("One-of variants")}>
            <span class="type-select-label">{$ui("Choices")} <em>{$ui("(at least one)")}</em></span>
            {#each newFieldOneOfVariants as variant, index}
              <div class="variant-row">
                <input bind:value={variant.label} placeholder={$ui("Variant label")} />
                {#if showAdvancedControl(showAdvanced, "oneof-variants")}
                  <select bind:value={variant.type}>
                    {#each ["text", "number", "boolean", "date", "enum"] as type}
                      <option value={type}>{fieldTypeLabel(type as FieldType)}</option>
                    {/each}
                  </select>
                  {#if variant.type === "enum"}
                    <input bind:value={variant.options} placeholder={$ui("Options, comma separated")} />
                  {/if}
                {/if}
                <button type="button" class="quiet icon" onclick={() => removeNewFieldOneOfVariant(index)}
                  ><X size={14} strokeWidth={1.8} aria-hidden="true" /></button>
              </div>
            {/each}
            <button type="button" class="quiet" onclick={addNewFieldOneOfVariant}
              ><Plus size={14} strokeWidth={1.8} aria-hidden="true" /> {$ui("Add variant")}</button>
            {#if newFieldErrors.oneOf}<small class="field-error">{newFieldErrors.oneOf}</small>{/if}
          </div>
        {:else if newFieldType === "relationship"}
          {#if showAdvancedControl(showAdvanced, "relationship-type")}
            <label>
              <span>{$ui("Relationship type ID")}</span>
              <input bind:value={newFieldRelationshipType} placeholder={$ui("related_to")} />
              {#if newFieldErrors.relationshipType}
                <small class="field-error">{newFieldErrors.relationshipType}</small>
              {/if}
            </label>
          {/if}
          <div class="type-select" role="group" aria-label={$ui("Target entity types")}>
            <span class="type-select-label">{$ui("Target types")} <em>{$ui("(required)")}</em></span>
            <div class="chip-row compact">
              {#each fieldScopeTypes() as type}
                <button
                  type="button"
                  class="chip select"
                  class:selected={newFieldTargetEntityTypes.includes(type)}
                  aria-pressed={newFieldTargetEntityTypes.includes(type)}
                  onclick={() => (newFieldTargetEntityTypes = toggleInList(newFieldTargetEntityTypes, type))}
                  >{entityTypeLabel(type)}</button>
              {/each}
            </div>
            {#if newFieldErrors.targetTypes}<small class="field-error">{newFieldErrors.targetTypes}</small>{/if}
          </div>
          <label>
            <span>{$ui("Cardinality")}</span>
            <select bind:value={newFieldCardinality}>
              <option value="many">{$ui("Many")}</option><option value="one">{$ui("One")}</option>
            </select>
            {#if newFieldErrors.cardinality}<small class="field-error">{newFieldErrors.cardinality}</small>{/if}
          </label>
          <div class="type-select" role="group" aria-label={$ui("Relationship attributes")}>
            <span class="type-select-label">{$ui("Attributes")} <em>{$ui("(custom fields on the relationship)")}</em></span>
            {#if newFieldMetadata.length === 0}
              <span class="meta-hint">{$ui("No attributes yet — add “Since”, “Role”, “Strength” …")}</span>
            {/if}
            {#each newFieldMetadata as meta, metaIdx}
              <div class="metadata-row">
                <div class="metadata-main">
                  <input
                    bind:value={meta.label}
                    placeholder={$ui("Attribute label")}
                    oninput={() => {
                      if (!meta.key || slugifyFieldKey(meta.key) === slugifyFieldKey(meta.label.slice(0, -1))) {
                        meta.key = slugifyFieldKey(meta.label);
                      }
                    }} />
                  {#if showAdvanced}<input bind:value={meta.key} placeholder={$ui("Stable key")} />{/if}
                  <select bind:value={meta.type}>
                    {#each METADATA_FIELD_TYPES as type}<option value={type}>{fieldTypeLabel(type)}</option>{/each}
                  </select>
                  <label class="inline-check small">
                    <input type="checkbox" bind:checked={meta.required} /><span>{$ui("Required")}</span>
                  </label>
                  <button type="button" class="quiet icon" onclick={() => removeNewFieldMetadata(metaIdx)}
                    ><X size={12} strokeWidth={1.8} aria-hidden="true" /></button>
                </div>
                {#if meta.type === "enum"}
                  <input bind:value={meta.options} placeholder={$ui("Options, comma separated")} />
                {:else if meta.type === "oneof"}
                  <div class="oneof-variants nested">
                    {#each meta.oneOf as variant, variantIdx}
                      <div class="variant-row small">
                        <input bind:value={variant.label} placeholder={$ui("Variant label")} />
                        {#if showAdvanced}
                          <select bind:value={variant.type}>
                            {#each ["text", "number", "boolean", "date", "enum"] as type}
                              <option value={type}>{fieldTypeLabel(type as FieldType)}</option>
                            {/each}
                          </select>
                          {#if variant.type === "enum"}
                            <input bind:value={variant.options} placeholder={$ui("Options, comma separated")} />
                          {/if}
                        {/if}
                        <button
                          type="button"
                          class="quiet icon"
                          onclick={() => removeNewFieldMetadataOneOfVariant(metaIdx, variantIdx)}
                          ><X size={12} strokeWidth={1.8} aria-hidden="true" /></button>
                      </div>
                    {/each}
                    <button type="button" class="quiet small" onclick={() => addNewFieldMetadataOneOfVariant(metaIdx)}
                      ><Plus size={12} strokeWidth={1.8} aria-hidden="true" /> {$ui("Add variant")}</button>
                  </div>
                {/if}
              </div>
            {/each}
            <button type="button" class="quiet" onclick={addNewFieldMetadata}
              ><Plus size={12} strokeWidth={1.8} aria-hidden="true" /> {$ui("Add attribute")}</button>
            <span class="meta-hint">{$ui("Each attribute becomes a field in the relationship details dialog.")}</span>
          </div>
        {:else if newFieldType === "date"}
          <label class="inline-check">
            <input type="checkbox" bind:checked={newFieldShared} />
            <span>{$ui("Shared — allow Timeline and other modules to read")}</span>
          </label>
          {#if newFieldShared}
            <label class="inline-check">
              <input type="checkbox" bind:checked={newFieldTimelineEnabled} /><span>{$ui("Show on Timeline")}</span>
            </label>
            {#if newFieldTimelineEnabled}
              <div class="timeline-config">
                {#if showAdvancedControl(showAdvanced, "timeline-role")}
                  <label>
                    <span>{$ui("Timeline role")}</span>
                    <select bind:value={newFieldTimelineRole}>
                      <option value="point">{$ui("Point")}</option><option value="start">{$ui("Start")}</option><option value="end"
                        >{$ui("End")}</option>
                    </select>
                  </label>
                  {#if newFieldTimelineRole !== "point"}
                    <label>
                      <span>{$ui("Timeline group")}</span>
                      <input bind:value={newFieldTimelineGroup} placeholder={$ui("e.g. life, existence")} />
                    </label>
                  {/if}
                {/if}
                <label
                  ><span>{$ui("Label")}</span><input bind:value={newFieldTimelineLabel} placeholder={$ui("Born, Created…")} /></label>
                {#if showAdvancedControl(showAdvanced, "timeline-layer")}
                  <label>
                    <span>{$ui("Timeline layer")}</span>
                    <select bind:value={newFieldTimelineLayer}>
                      <option value="dates">{$ui("Project dates")}</option><option value="lifelines">{$ui("Lifelines")}</option>
                    </select>
                  </label>
                {/if}
              </div>
              {#if newFieldErrors.timelineGroup}<small class="field-error">{newFieldErrors.timelineGroup}</small>{/if}
            {/if}
          {/if}
        {/if}
        <div class="type-select" role="group" aria-label={$ui("Applies to entity types")}>
          <span class="type-select-label">{$ui("Applies to")} <em>{$ui("(optional)")}</em></span>
          <div class="chip-row compact">
            {#each fieldScopeTypes() as type}
              <button
                type="button"
                class="chip select"
                class:selected={newFieldEntityTypes.includes(type)}
                aria-pressed={newFieldEntityTypes.includes(type)}
                onclick={() => (newFieldEntityTypes = toggleInList(newFieldEntityTypes, type))}
                >{entityTypeLabel(type)}</button>
            {/each}
          </div>
        </div>
      </div>
    </div>
  </div>

  <div class="workbench-detail">
    {#if selectedBuiltinField}
      {@const field = selectedBuiltinField}
      <div class="block elevated detail-card">
        <div class="block-heading">
          <div class="heading-left">
            <span class="heading-icon"><TextQuote size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>{$manifestText(pluginId, "field", field.key, field.label) || humanizeId(field.key)}</h4>
          </div>
          <span class="type-pill">{fieldTypeLabel(field.type)}</span>
        </div>
        {#if editingBuiltinFieldKey === field.key}
          <div class="edit-form wide">
            <div class="type-select" role="group" aria-label={$ui("Entity types for {0}", $manifestText(pluginId, "field", field.key, field.label))}>
              <span class="type-select-label">{$ui("Applies to")}</span>
              <div class="chip-row compact">
                {#each fieldScopeTypes() as type}
                  <button
                    type="button"
                    class="chip select"
                    class:selected={builtinFieldScope(field).includes(type)}
                    aria-pressed={builtinFieldScope(field).includes(type)}
                    onclick={() => updateBuiltinFieldScope(field, toggleInList(builtinFieldScope(field), type))}
                    >{entityTypeLabel(type)}</button>
                {/each}
              </div>
            </div>
            <div class="edit-actions">
              <button type="button" class="action" onclick={() => (editingBuiltinFieldKey = null)}
                ><Check size={14} strokeWidth={2} aria-hidden="true" /> {$ui("Done")}</button>
            </div>
          </div>
        {:else if editingBuiltinMetadataFieldKey === field.key}
          <div class="edit-form wide">
            <div class="type-select" role="group" aria-label={$ui("Attributes for {0}", $manifestText(pluginId, "field", field.key, field.label))}>
              <span class="type-select-label">{$ui("Attributes")} <em>{$ui("(additive to builtin)")}</em></span>
              {#if editBuiltinMetadataDrafts.length === 0}
                <span class="meta-hint">{$ui("No attributes yet — add a new one.")}</span>
              {/if}
              {#each editBuiltinMetadataDrafts as meta, metaIdx}
                {@const isBuiltinKey = builtinOriginalMetadata(field).some(
                  (item) => String((item as unknown as Record<string, unknown>).key) === meta.key,
                )}
                <div class="metadata-row">
                  <div class="metadata-main">
                    <input
                      bind:value={meta.label}
                      placeholder={$ui("Attribute label")}
                      oninput={() => {
                        if (!meta.key || slugifyFieldKey(meta.key) === slugifyFieldKey(meta.label.slice(0, -1))) {
                          meta.key = slugifyFieldKey(meta.label);
                        }
                      }} />
                    {#if showAdvanced}
                      <input
                        bind:value={meta.key}
                        placeholder={$ui("Stable key")}
                        disabled={isBuiltinKey}
                        title={isBuiltinKey ? $ui("Builtin key cannot be renamed") : $ui("Metadata key")} />
                    {/if}
                    <select bind:value={meta.type}>
                      {#each METADATA_FIELD_TYPES as type}<option value={type}>{fieldTypeLabel(type)}</option>{/each}
                    </select>
                    <label class="inline-check small">
                      <input type="checkbox" bind:checked={meta.required} />
                      <span>{$ui("Required")}</span>
                    </label>
                    <button
                      type="button"
                      class="quiet icon"
                      disabled={isBuiltinKey}
                      title={isBuiltinKey ? $ui("Builtin attributes cannot be removed (additive)") : $ui("Remove")}
                      onclick={() => removeEditBuiltinMetadata(metaIdx)}
                      ><X size={12} strokeWidth={1.8} aria-hidden="true" /></button>
                  </div>
                  {#if meta.type === "enum"}
                    <input bind:value={meta.options} placeholder={$ui("Options, comma separated")} />
                  {:else if meta.type === "oneof"}
                    <div class="oneof-variants nested">
                      {#each meta.oneOf as variant, variantIdx}
                        <div class="variant-row small">
                          <input bind:value={variant.label} placeholder={$ui("Variant label")} />
                          {#if showAdvanced}
                            <select bind:value={variant.type}>
                              {#each ["text", "number", "boolean", "date", "enum"] as type}
                                <option value={type}>{fieldTypeLabel(type as FieldType)}</option>
                              {/each}
                            </select>
                            {#if variant.type === "enum"}
                              <input bind:value={variant.options} placeholder={$ui("Options, comma separated")} />
                            {/if}
                          {/if}
                          <button
                            type="button"
                            class="quiet icon"
                            onclick={() => removeEditBuiltinMetadataOneOfVariant(metaIdx, variantIdx)}
                            ><X size={12} strokeWidth={1.8} aria-hidden="true" /></button>
                        </div>
                      {/each}
                      <button
                        type="button"
                        class="quiet small"
                        onclick={() => addEditBuiltinMetadataOneOfVariant(metaIdx)}
                        ><Plus size={12} strokeWidth={1.8} aria-hidden="true" /> {$ui("Add variant")}</button>
                    </div>
                  {/if}
                  {#if isBuiltinKey}
                    <span class="meta-hint">{$ui("Builtin attribute — editing overrides the packaged definition.")}</span>
                  {/if}
                </div>
              {/each}
              <button type="button" class="quiet" onclick={addEditBuiltinMetadata}
                ><Plus size={12} strokeWidth={1.8} aria-hidden="true" /> {$ui("Add attribute")}</button>
              <span class="meta-hint"
                >{$ui("Builtin {0} + override {1} = effective {2}. Stored as delta.", builtinOriginalMetadata(field).length, (draft.fieldMetadataOverrides ?? []).find(
                  (override) => override.fieldKey === field.key,
                )?.metadataFields?.length ?? 0, effectiveBuiltinMetadata(field).length)}</span>
            </div>
            <div class="edit-actions">
              <button type="button" class="action" onclick={commitBuiltinMetadataEdit}
                ><Check size={14} strokeWidth={2} aria-hidden="true" /> {$ui("Save attributes")}</button>
              <button type="button" class="quiet" onclick={cancelBuiltinMetadataEdit}>{$ui("Cancel")}</button>
            </div>
          </div>
        {:else if editingTimelineFieldKey === field.key}
          <div class="edit-form wide">
            <div class="type-select" role="group" aria-label={$ui("Timeline for {0}", $manifestText(pluginId, "field", field.key, field.label))}>
              <span class="type-select-label">{$ui("Timeline")}</span>
              {#if showAdvanced}
                <label>
                  <span>{$ui("Timeline role")}</span>
                  <select bind:value={editTimelineRole}>
                    <option value="point">{$ui("Point")}</option>
                    <option value="start">{$ui("Start")}</option>
                    <option value="end">{$ui("End")}</option>
                  </select>
                </label>
                {#if editTimelineRole !== "point"}
                  <label>
                    <span>{$ui("Timeline group")}</span>
                    <input bind:value={editTimelineGroup} placeholder={$ui("e.g. life, existence")} />
                  </label>
                {/if}
              {/if}
              <label><span>{$ui("Label")}</span><input bind:value={editTimelineLabel} placeholder={$ui("Born, Created…")} /></label>
              {#if showAdvanced}
                <label>
                  <span>{$ui("Timeline layer")}</span>
                  <select bind:value={editTimelineLayer}>
                    <option value="dates">{$ui("Project dates")}</option>
                    <option value="lifelines">{$ui("Lifelines")}</option>
                  </select>
                </label>
              {/if}
              <span class="meta-hint"
                >{$ui("Group required for start/end; layer controls Timeline swarm. Shared required — builtin date fields are already shared.")}</span>
            </div>
            <div class="edit-actions">
              <button type="button" class="action" onclick={commitTimelineEdit}
                ><Check size={14} strokeWidth={2} aria-hidden="true" /> {$ui("Save")}</button>
              <button type="button" class="quiet" onclick={cancelTimelineEdit}>{$ui("Cancel")}</button>
            </div>
          </div>
        {:else}
          <dl class="usage-summary">
            <div>
              <dt>{$ui("Kind")}</dt>
              <dd>{fieldTypeLabel(field.type)}</dd>
            </div>
            <div>
              <dt>{$ui("Status")}</dt>
              <dd>{isDisabled(draft.disabledFields, field.key) ? $ui("Disabled") : $ui("Enabled")}</dd>
            </div>
            <div>
              <dt>{$ui("Applies to")}</dt>
              <dd>{scopeLabel(builtinFieldScope(field))}</dd>
            </div>
            {#if field.type === "relationship"}
              <div>
                <dt>{$ui("Attributes")}</dt>
                <dd>{builtinMetadataFieldExtras(field) || $ui("None")}</dd>
              </div>
            {/if}
            {#if field.type === "date"}
              <div>
                <dt>{$ui("Timeline")}</dt>
                <dd>{isTimelineEnabled(field) ? timelineBadge(field) : $ui("Not shown")}</dd>
              </div>
            {/if}
            {#if showAdvanced}
              <div>
                <dt>{$ui("Stable key")}</dt>
                <dd><code>{field.key}</code></dd>
              </div>
              <div>
                <dt>{$ui("Namespace")}</dt>
                <dd>{flatPackage.fieldNamespace[field.key]}</dd>
              </div>
            {/if}
          </dl>
          <div class="edit-actions">
            <button
              type="button"
              class="action"
              onclick={() => {
                editingBuiltinMetadataFieldKey = null;
                editBuiltinMetadataDrafts = [];
                editingTimelineFieldKey = null;
                editingBuiltinFieldKey = field.key;
              }}><Pencil size={14} strokeWidth={1.8} aria-hidden="true" /> {$ui("Edit scope")}</button>
            {#if field.type === "relationship"}
              <button
                type="button"
                class="quiet"
                disabled={isDisabled(draft.disabledFields, field.key)}
                onclick={() => startBuiltinMetadataEdit(field)}>{$ui("Edit attributes")}</button>
            {/if}
            {#if field.type === "date" && (field as unknown as Record<string, unknown>).shared}
              <button
                type="button"
                class="quiet"
                disabled={isDisabled(draft.disabledFields, field.key)}
                onclick={() => startTimelineEdit(field)}
                >{isTimelineEnabled(field) ? $ui("Edit Timeline") : $ui("Enable Timeline")}</button>
              {#if isTimelineEnabled(field)}
                <button
                  type="button"
                  class="quiet"
                  disabled={isDisabled(draft.disabledFields, field.key)}
                  onclick={() => disableTimeline(field)}>{$ui("Disable Timeline")}</button>
              {/if}
            {/if}
          </div>
        {/if}
      </div>
    {:else if selectedCustomField && editingFieldKey === selectedCustomField.key}
      {@const field = selectedCustomField}
      <div class="block elevated detail-card">
        <div class="block-heading">
          <div class="heading-left">
            <span class="heading-icon accent"><Sparkles size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>{$manifestText(pluginId, "field", field.key, field.label)}</h4>
          </div>
          <span class="type-pill">{fieldTypeLabel(field.type)}</span>
        </div>
        <div class="edit-form">
          <label>
            <span>{$ui("Label")}</span><input bind:value={editFieldLabel} placeholder={$ui("Word count")} />
            {#if editFieldErrors.name}<small class="field-error">{editFieldErrors.name}</small>{/if}
          </label>
          {#if showAdvancedControl(showAdvanced, "field-key")}
            <label>
              <span>{$ui("Stable key")}</span><input value={editingFieldKey ?? ""} readonly aria-readonly="true" />
              {#if editFieldErrors.key}<small class="field-error">{editFieldErrors.key}</small>{/if}
            </label>
          {/if}
          <label>
            <span>{$ui("Kind")}</span>
            <select bind:value={editFieldType}>
              {#each Object.values(FIELD_KIND_GROUPS) as group}
                <optgroup label={$ui(group.label)}>
                  {#each group.types as type}<option value={type}>{fieldTypeLabel(type)}</option>{/each}
                </optgroup>
              {/each}
            </select>
          </label>
          {#if editFieldType === "enum"}
            <label>
              <span>{$ui("Options")} <em>{$ui("(comma separated)")}</em></span>
              <input bind:value={editFieldOptions} placeholder={$ui("idea, drafting, revising, complete")} />
              {#if editFieldErrors.choices}<small class="field-error">{editFieldErrors.choices}</small>{/if}
            </label>
            <label class="inline-check">
              <input type="checkbox" bind:checked={editFieldMultiple} /><span>{$ui("Allow multiple values")}</span>
            </label>
          {:else if editFieldType === "oneof"}
            <div class="type-select" role="group" aria-label={$ui("One-of variants")}>
              <span class="type-select-label">{$ui("Choices")} <em>{$ui("(at least one)")}</em></span>
              {#each editFieldOneOfVariants as variant, index}
                <div class="variant-row">
                  <input bind:value={variant.label} placeholder={$ui("Variant label")} />
                  {#if showAdvancedControl(showAdvanced, "oneof-variants")}
                    <select bind:value={variant.type}>
                      {#each ["text", "number", "boolean", "date", "enum"] as type}
                        <option value={type}>{fieldTypeLabel(type as FieldType)}</option>
                      {/each}
                    </select>
                    {#if variant.type === "enum"}
                      <input bind:value={variant.options} placeholder={$ui("Options, comma separated")} />
                    {/if}
                  {/if}
                  <button type="button" class="quiet icon" onclick={() => removeEditFieldOneOfVariant(index)}
                    ><X size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                </div>
              {/each}
              <button type="button" class="quiet" onclick={addEditFieldOneOfVariant}
                ><Plus size={14} strokeWidth={1.8} aria-hidden="true" /> {$ui("Add variant")}</button>
              {#if editFieldErrors.oneOf}<small class="field-error">{editFieldErrors.oneOf}</small>{/if}
            </div>
          {:else if editFieldType === "relationship"}
            {#if showAdvancedControl(showAdvanced, "relationship-type")}
              <label>
                <span>{$ui("Relationship type ID")}</span>
                <input bind:value={editFieldRelationshipType} placeholder={$ui("related_to")} />
                {#if editFieldErrors.relationshipType}
                  <small class="field-error">{editFieldErrors.relationshipType}</small>
                {/if}
              </label>
            {/if}
            <div class="type-select" role="group" aria-label={$ui("Target entity types")}>
              <span class="type-select-label">{$ui("Target types")} <em>{$ui("(required)")}</em></span>
              <div class="chip-row compact">
                {#each fieldScopeTypes() as type}
                  <button
                    type="button"
                    class="chip select"
                    class:selected={editFieldTargetEntityTypes.includes(type)}
                    aria-pressed={editFieldTargetEntityTypes.includes(type)}
                    onclick={() => (editFieldTargetEntityTypes = toggleInList(editFieldTargetEntityTypes, type))}
                    >{entityTypeLabel(type)}</button>
                {/each}
              </div>
              {#if editFieldErrors.targetTypes}<small class="field-error">{editFieldErrors.targetTypes}</small>{/if}
            </div>
            <label>
              <span>{$ui("Cardinality")}</span>
              <select bind:value={editFieldCardinality}>
                <option value="many">{$ui("Many")}</option><option value="one">{$ui("One")}</option>
              </select>
              {#if editFieldErrors.cardinality}<small class="field-error">{editFieldErrors.cardinality}</small>{/if}
            </label>
            <div class="type-select" role="group" aria-label={$ui("Relationship attributes")}>
              <span class="type-select-label">{$ui("Attributes")} <em>{$ui("(custom fields on the relationship)")}</em></span>
              {#if editFieldMetadata.length === 0}
                <span class="meta-hint">{$ui("No attributes yet — e.g., “Since”, “Strength”, “Role”.")}</span>
              {/if}
              {#each editFieldMetadata as meta, metaIdx}
                <div class="metadata-row">
                  <div class="metadata-main">
                    <input
                      bind:value={meta.label}
                      placeholder={$ui("Attribute label")}
                      oninput={() => {
                        if (!meta.key || slugifyFieldKey(meta.key) === slugifyFieldKey(meta.label.slice(0, -1))) {
                          meta.key = slugifyFieldKey(meta.label);
                        }
                      }} />
                    {#if showAdvanced}<input bind:value={meta.key} placeholder={$ui("Stable key")} />{/if}
                    <select bind:value={meta.type}>
                      {#each METADATA_FIELD_TYPES as type}<option value={type}>{fieldTypeLabel(type)}</option>{/each}
                    </select>
                    <label class="inline-check small">
                      <input type="checkbox" bind:checked={meta.required} /><span>{$ui("Required")}</span>
                    </label>
                    <button type="button" class="quiet icon" onclick={() => removeEditFieldMetadata(metaIdx)}
                      ><X size={12} strokeWidth={1.8} aria-hidden="true" /></button>
                  </div>
                  {#if meta.type === "enum"}
                    <input bind:value={meta.options} placeholder={$ui("Options, comma separated e.g. weak, strong")} />
                  {:else if meta.type === "oneof"}
                    <div class="oneof-variants nested">
                      {#each meta.oneOf as variant, variantIdx}
                        <div class="variant-row small">
                          <input bind:value={variant.label} placeholder={$ui("Variant label")} />
                          {#if showAdvanced}
                            <select bind:value={variant.type}>
                              {#each ["text", "number", "boolean", "date", "enum"] as type}
                                <option value={type}>{fieldTypeLabel(type as FieldType)}</option>
                              {/each}
                            </select>
                            {#if variant.type === "enum"}
                              <input bind:value={variant.options} placeholder={$ui("Options, comma separated")} />
                            {/if}
                          {/if}
                          <button
                            type="button"
                            class="quiet icon"
                            onclick={() => removeEditFieldMetadataOneOfVariant(metaIdx, variantIdx)}
                            ><X size={12} strokeWidth={1.8} aria-hidden="true" /></button>
                        </div>
                      {/each}
                      <button
                        type="button"
                        class="quiet small"
                        onclick={() => addEditFieldMetadataOneOfVariant(metaIdx)}
                        ><Plus size={12} strokeWidth={1.8} aria-hidden="true" /> {$ui("Add variant")}</button>
                    </div>
                  {/if}
                  {#if meta.label.trim() && !ensureFieldKey(meta.key)}
                    <span class="field-error">{$ui("Key must start with a letter.")}</span>
                  {/if}
                </div>
              {/each}
              <button type="button" class="quiet" onclick={addEditFieldMetadata}
                ><Plus size={12} strokeWidth={1.8} aria-hidden="true" /> {$ui("Add attribute")}</button>
            </div>
          {:else if editFieldType === "date"}
            <label class="inline-check">
              <input type="checkbox" bind:checked={editFieldShared} />
              <span>{$ui("Shared — allow Timeline and other modules to read")}</span>
            </label>
            {#if editFieldShared}
              <label class="inline-check">
                <input type="checkbox" bind:checked={editFieldTimelineEnabled} /><span>{$ui("Show on Timeline")}</span>
              </label>
              {#if editFieldTimelineEnabled}
                <div class="timeline-config">
                  {#if showAdvancedControl(showAdvanced, "timeline-role")}
                    <label>
                      <span>{$ui("Timeline role")}</span>
                      <select bind:value={editFieldTimelineRole}>
                        <option value="point">{$ui("Point")}</option><option value="start">{$ui("Start")}</option><option value="end"
                          >{$ui("End")}</option>
                      </select>
                    </label>
                    {#if editFieldTimelineRole !== "point"}
                      <label>
                        <span>{$ui("Timeline group")}</span>
                        <input bind:value={editFieldTimelineGroup} placeholder={$ui("e.g. life, existence")} />
                      </label>
                    {/if}
                  {/if}
                  <label
                    ><span>{$ui("Label")}</span><input
                      bind:value={editFieldTimelineLabel}
                      placeholder={$ui("Born, Created…")} /></label>
                  {#if showAdvancedControl(showAdvanced, "timeline-layer")}
                    <label>
                      <span>{$ui("Timeline layer")}</span>
                      <select bind:value={editFieldTimelineLayer}>
                        <option value="dates">{$ui("Project dates")}</option><option value="lifelines">{$ui("Lifelines")}</option>
                      </select>
                    </label>
                  {/if}
                </div>
                {#if editFieldErrors.timelineGroup}
                  <small class="field-error">{editFieldErrors.timelineGroup}</small>
                {/if}
              {/if}
            {/if}
          {/if}
          <div class="type-select" role="group" aria-label={$ui("Applies to entity types")}>
            <span class="type-select-label">{$ui("Applies to")} <em>{$ui("(optional)")}</em></span>
            <div class="chip-row compact">
              {#each fieldScopeTypes() as type}
                <button
                  type="button"
                  class="chip select"
                  class:selected={editFieldEntityTypes.includes(type)}
                  aria-pressed={editFieldEntityTypes.includes(type)}
                  onclick={() => (editFieldEntityTypes = toggleInList(editFieldEntityTypes, type))}
                  >{entityTypeLabel(type)}</button>
              {/each}
            </div>
          </div>
          <div class="edit-actions">
            <button
              type="button"
              class="action"
              disabled={fieldFormHasErrors(editFieldErrors) || !canSaveFieldEdit()}
              onclick={commitFieldEdit}><Check size={14} strokeWidth={2} aria-hidden="true" /> {$ui("Save")}</button>
            <button type="button" class="quiet" onclick={cancelFieldEdit}>{$ui("Cancel")}</button>
          </div>
        </div>
      </div>
    {:else}
      <div class="block detail-placeholder">
        <strong>{$ui("Select a field")}</strong>
        <span>{$ui("Choose a field to view its scope and edit its settings.")}</span>
      </div>
    {/if}
  </div>
</div>
