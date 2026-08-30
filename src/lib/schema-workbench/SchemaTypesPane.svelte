<script lang="ts">
import type { EntityTypeColor, EntityTypeDefinition, IconRef, ModuleSchemaOverlay } from "$lib/project/client";
import FieldPicker from "$lib/FieldPicker.svelte";
import EntityGlyph from "$lib/entity-colors/EntityGlyph.svelte";
import TypeAppearancePicker from "$lib/entity-colors/TypeAppearancePicker.svelte";
import TypeColorPicker from "$lib/entity-colors/TypeColorPicker.svelte";
import IconPicker from "$lib/entity-icons/IconPicker.svelte";
import { HOUSES_PLUGIN_ID, summarizeTypeUsage, type FlattenedPackageSchema } from "$lib/schema-workbench";
import "./schema-pane.css";
import {
  Check,
  ChevronDown,
  ChevronRight,
  Eye,
  EyeOff,
  Layers,
  Pencil,
  Plus,
  Sparkles,
  Trash2,
  Type,
} from "@lucide/svelte";

let {
  draft,
  flatPackage,
  packageTypeDefinitions,
  pluginId,
  showAdvanced,
  selectedItemId = $bindable(),
  builtinTypesCollapsed = $bindable(),
  newType = $bindable(),
  newTypeIcon = $bindable(),
  newTypeColor = $bindable(),
  newTypeFieldKeys = $bindable(),
  editingTypeId = $bindable(),
  editTypeValue = $bindable(),
  editTypeIcon = $bindable(),
  editTypeColor = $bindable(),
  editTypeFieldKeys = $bindable(),
  typeMatches,
  isDisabled,
  effectivePackageAppearance,
  packageAppearanceChanged,
  clearPackageAppearanceOverride,
  setPackageAppearanceOverride,
  toggleDisabled,
  selectableFieldOptions,
  effectiveFieldsForType,
  effectiveTemplates,
  projectionLabelsForType = () => [],
  entityCountForType = () => null,
  commitTypeEdit,
  cancelTypeEdit,
  startTypeEdit,
  requestRemoveCustomType,
  addCustomType,
}: {
  draft: ModuleSchemaOverlay;
  flatPackage: FlattenedPackageSchema;
  packageTypeDefinitions: EntityTypeDefinition[];
  pluginId: string | null;
  showAdvanced: boolean;
  selectedItemId: string | null;
  builtinTypesCollapsed: boolean;
  newType: string;
  newTypeIcon: IconRef;
  newTypeColor: EntityTypeColor;
  newTypeFieldKeys: string[];
  editingTypeId: string | null;
  editTypeValue: string;
  editTypeIcon: IconRef;
  editTypeColor: EntityTypeColor;
  editTypeFieldKeys: string[];
  typeMatches: (type: EntityTypeDefinition, origin: "builtin" | "custom") => boolean;
  isDisabled: (list: string[] | undefined, id: string) => boolean;
  effectivePackageAppearance: (type: EntityTypeDefinition) => { icon: IconRef; iconColor: EntityTypeColor };
  packageAppearanceChanged: (type: EntityTypeDefinition) => boolean;
  clearPackageAppearanceOverride: (typeId: string) => void;
  setPackageAppearanceOverride: (
    typeId: string,
    next: { icon: IconRef; iconColor: EntityTypeColor },
    base: EntityTypeDefinition,
  ) => void;
  toggleDisabled: (listKey: "disabledEntityTypes" | "disabledFields" | "disabledTemplates", id: string) => void;
  selectableFieldOptions: () => Array<{ key: string; label: string; hint: string }>;
  effectiveFieldsForType: (typeId: string) => import("$lib/project/client").FieldDefinition[];
  effectiveTemplates: () => import("$lib/project/client").EntityTemplate[];
  projectionLabelsForType?: (typeId: string) => string[];
  entityCountForType?: (typeId: string) => number | null;
  commitTypeEdit: () => void;
  cancelTypeEdit: () => void;
  startTypeEdit: (type: string) => void;
  requestRemoveCustomType: (type: string) => void;
  addCustomType: () => void;
} = $props();

const selectedTypeId = $derived(selectedItemId?.startsWith("type:") ? selectedItemId.slice(5) : null);
const selectedBuiltinType = $derived(packageTypeDefinitions.find((type) => type.id === selectedTypeId));
const selectedCustomType = $derived((draft.customEntityTypes ?? []).find((type) => type.id === selectedTypeId));
const selectedType = $derived(selectedBuiltinType ?? selectedCustomType ?? null);

function usageForType(typeId: string) {
  return summarizeTypeUsage({
    typeId,
    fields: effectiveFieldsForType(typeId).map((field) => ({ ...field, entityTypes: [typeId] })),
    templates: effectiveTemplates(),
    projectionLabels: projectionLabelsForType(typeId),
    entityCount: entityCountForType(typeId),
  });
}

function selectBuiltinType(typeId: string) {
  cancelTypeEdit();
  selectedItemId = `type:${typeId}`;
}
</script>

<div class="schema-workbench-pane workbench-split">
  <div class="workbench-list">
    <div class="block elevated">
      <button
        type="button"
        class="block-heading collapsible"
        aria-expanded={!builtinTypesCollapsed}
        onclick={() => (builtinTypesCollapsed = !builtinTypesCollapsed)}>
        <div class="heading-left">
          <span class="heading-icon"><Layers size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h4>Builtin entity types</h4>
          <span class="count-badge">{packageTypeDefinitions.length}</span>
        </div>
        <span class="block-hint"><Eye size={12} strokeWidth={1.8} aria-hidden="true" /> Select for details</span>
        <span class="collapse-icon" aria-hidden="true"
          >{#if builtinTypesCollapsed}<ChevronRight size={14} strokeWidth={1.8} />{:else}<ChevronDown
              size={14}
              strokeWidth={1.8} />{/if}</span>
      </button>
      {#if !builtinTypesCollapsed}
        <ul class="list compact-list">
          {#each packageTypeDefinitions.filter((type) => typeMatches(type, "builtin")) as type}
            {@const disabled = isDisabled(draft.disabledEntityTypes, type.id)}
            {@const appearance = effectivePackageAppearance(type)}
            <li class="list-item compact" class:selected-detail={selectedTypeId === type.id}>
              <button type="button" class="item-select" onclick={() => selectBuiltinType(type.id)}>
                <EntityGlyph icon={appearance.icon} iconColor={appearance.iconColor} {pluginId} size={14} box={24} />
                <span><strong>{type.name}</strong><small>{disabled ? "Disabled" : "Enabled"}</small></span>
              </button>
              <button
                type="button"
                class="chip"
                class:is-hidden={disabled}
                aria-pressed={!disabled}
                onclick={() => toggleDisabled("disabledEntityTypes", type.id)}>
                {#if disabled}<EyeOff size={11} strokeWidth={1.8} aria-hidden="true" /> Enable{:else}<Check
                    size={11}
                    strokeWidth={2.2}
                    aria-hidden="true" /> Enabled{/if}
              </button>
            </li>
          {/each}
        </ul>
        <p class="subtle-note">Disabled types are hidden from create menus and can be enabled again later.</p>
      {/if}
    </div>

    <div class="block elevated">
      <div class="block-heading">
        <div class="heading-left">
          <span class="heading-icon accent"><Sparkles size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h4>Custom entity types</h4>
          <span class="count-badge accent">{(draft.customEntityTypes ?? []).length}</span>
        </div>
      </div>
      {#if (draft.customEntityTypes ?? []).length === 0}
        <div class="empty-inline">
          <Type size={16} strokeWidth={1.7} aria-hidden="true" />
          <div><strong>No custom types yet</strong><span>Create a project-specific entity type below.</span></div>
        </div>
      {:else}
        <ul class="list">
          {#each (draft.customEntityTypes ?? []).filter((type) => typeMatches(type, "custom")) as type}
            <li class="list-item" class:selected-detail={selectedTypeId === type.id}>
              <button type="button" class="item-select" onclick={() => startTypeEdit(type.id)}>
                <EntityGlyph icon={type.icon} iconColor={type.iconColor} {pluginId} size={15} box={24} />
                <strong>{type.name}</strong>
              </button>
              <div class="item-actions">
                <button
                  type="button"
                  class="quiet icon"
                  aria-label="Edit {type.name}"
                  onclick={() => startTypeEdit(type.id)}
                  ><Pencil size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                <button
                  type="button"
                  class="danger icon"
                  aria-label="Remove {type.name}"
                  onclick={() => requestRemoveCustomType(type.id)}
                  ><Trash2 size={14} strokeWidth={1.8} aria-hidden="true" /></button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="add-form">
        <div class="add-row">
          <label><span>New type</span><input bind:value={newType} placeholder="Species" /></label>
          <IconPicker value={newTypeIcon} onChange={(icon) => (newTypeIcon = icon)} />
          <TypeColorPicker value={newTypeColor} onChange={(iconColor) => (newTypeColor = iconColor)} />
          <button type="button" class="action primary-action" onclick={addCustomType}
            ><Plus size={14} strokeWidth={2} aria-hidden="true" /> Add type</button>
        </div>
        <div class="type-select" role="group" aria-label="Fields for the new type">
          <span class="type-select-label">Fields <em>(optional)</em></span>
          <FieldPicker
            options={selectableFieldOptions()}
            selected={newTypeFieldKeys}
            onChange={(keys) => (newTypeFieldKeys = keys)}
            placeholder="Search fields to include…" />
        </div>
      </div>
    </div>
  </div>

  <div class="workbench-detail">
    {#if selectedType}
      {@const type = selectedType}
      {@const isBuiltin = Boolean(selectedBuiltinType)}
      {@const usage = usageForType(type.id)}
      <div class="block elevated detail-card">
        <div class="block-heading">
          <div class="heading-left">
            <EntityGlyph
              icon={isBuiltin ? effectivePackageAppearance(type).icon : type.icon}
              iconColor={isBuiltin ? effectivePackageAppearance(type).iconColor : type.iconColor}
              {pluginId}
              size={16}
              box={30} />
            <h4>{type.name}</h4>
          </div>
          <span class="type-pill">{isBuiltin ? "Built in" : "Project custom"}</span>
        </div>
        {#if isBuiltin}
          {@const appearance = effectivePackageAppearance(type)}
          <TypeAppearancePicker
            value={{ icon: appearance.icon, iconColor: appearance.iconColor }}
            showReset={packageAppearanceChanged(type)}
            onReset={() => clearPackageAppearanceOverride(type.id)}
            onChange={(next) => setPackageAppearanceOverride(type.id, next, type)} />
        {:else if editingTypeId === type.id}
          <div class="edit-form">
            <label><span>Name</span><input bind:value={editTypeValue} placeholder="Species" /></label>
            {#if showAdvanced}
              <label><span>Stable ID</span><input value={editingTypeId} readonly aria-readonly="true" /></label>
            {/if}
            <TypeAppearancePicker
              value={{ icon: editTypeIcon, iconColor: editTypeColor }}
              onChange={(next) => {
                editTypeIcon = next.icon;
                editTypeColor = next.iconColor;
              }} />
            <div class="type-select" role="group" aria-label={`Fields for ${editTypeValue || type.name}`}>
              <span class="type-select-label">Fields</span>
              <FieldPicker
                options={selectableFieldOptions()}
                selected={editTypeFieldKeys}
                onChange={(keys) => (editTypeFieldKeys = keys)}
                placeholder="Search fields…" />
            </div>
          </div>
        {/if}
        <dl class="usage-summary">
          <div>
            <dt>Origin</dt>
            <dd>{isBuiltin ? "Built in" : "Project custom"}</dd>
          </div>
          <div>
            <dt>Status</dt>
            <dd>{isBuiltin && isDisabled(draft.disabledEntityTypes, type.id) ? "Disabled" : "Enabled"}</dd>
          </div>
          <div>
            <dt>Fields that apply</dt>
            <dd>{usage.fieldCount} — {usage.fieldLabels.slice(0, 4).join(", ") || "None"}</dd>
          </div>
          <div>
            <dt>Templates that create it</dt>
            <dd>{usage.templateLabels.join(", ") || "None"}</dd>
          </div>
          <div>
            <dt>Projections</dt>
            <dd>{usage.projectionLabels.join(", ") || "None"}</dd>
          </div>
          <div>
            <dt>Entities</dt>
            <dd>{usage.entityCount ?? "—"}</dd>
          </div>
          {#if showAdvanced}
            <div>
              <dt>Stable ID</dt>
              <dd><code>{type.id}</code></dd>
            </div>
            {#if isBuiltin}<div>
                <dt>Namespace</dt>
                <dd>{flatPackage.typeNamespace[type.id]}</dd>
              </div>{/if}
          {/if}
        </dl>
        {#if pluginId === HOUSES_PLUGIN_ID && usage.projectionLabels.includes("Houses collection only")}
          <p class="subtle-note tree-compat-note">
            Tree only shows Person and House. This custom type stays in the Houses collection and is not a Tree node.
          </p>
        {:else if pluginId === HOUSES_PLUGIN_ID && usage.projectionLabels.includes("Tree")}
          <p class="subtle-note tree-compat-note">Available in the Houses collection and as a Tree root.</p>
        {/if}
        {#if !isBuiltin}
          <div class="edit-actions">
            <button type="button" class="action" onclick={commitTypeEdit}
              ><Check size={14} strokeWidth={2} aria-hidden="true" /> Save</button>
            <button type="button" class="quiet" onclick={cancelTypeEdit}>Cancel</button>
          </div>
        {/if}
      </div>
    {:else}
      <div class="block detail-placeholder">
        <strong>Select a type</strong><span>Choose a type to view usage and edit its settings.</span>
      </div>
    {/if}
  </div>
</div>
