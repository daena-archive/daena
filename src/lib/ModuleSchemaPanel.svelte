<script lang="ts">
import type {
  EntityTypeAppearanceOverride,
  EntityTypeDefinition,
  EntityTemplate,
  FieldDefinition,
  IconRef,
  ModuleSchemaOverlay,
  FieldMetadataOverride,
  MetadataFieldDefinition,
  EntityTypeColor,
  SchemaOverlayPreviewResult,
} from "$lib/project/client";
import { DEFAULT_TYPE_COLOR } from "$lib/entity-colors/presets";
import { FALLBACK_ICON } from "$lib/entity-icons/catalog";
import { onMount } from "svelte";
import { setSchemaEditorDirtyCheck } from "$lib/schemaEditorGuard";
import { confirmDialog } from "$lib/dialogs.svelte";
import {
  FIELD_TYPES,
  METADATA_FIELD_TYPES,
  applyTypeRemovalPlan,
  cloneJson,
  defaultFieldValue,
  draftsFromMetadataFields,
  ensureFieldKey,
  ensureTypeId,
  fieldKindGroupLabel,
  fieldTypeLabel,
  filterSchemaListItems,
  fingerprint,
  flattenPackageSchemas,
  formatOptions,
  humanizeId,
  localTypeId,
  metadataDraftToDefinition,
  mintTypeId,
  normalizeOverlay,
  parseOneOfVariants,
  parseOptions,
  pruneOverlayForRemovedType,
  preserveTypeId,
  qualifyTypeId,
  slugifyTypeId,
  typeRemovalPlanIsComplete,
  validateMetadataDrafts,
  type ExclusiveFieldDisposition,
  type EntityRemovalDisposition,
  type FieldType,
  type MetadataFieldDraft,
  type MetadataFieldType,
  type SchemaListItem,
  type SchemaStatusFilter,
} from "$lib/schema-workbench";
import SchemaTypesPane from "$lib/schema-workbench/SchemaTypesPane.svelte";
import SchemaFieldsPane from "$lib/schema-workbench/SchemaFieldsPane.svelte";
import SchemaTemplatesPane from "$lib/schema-workbench/SchemaTemplatesPane.svelte";
import SchemaImpactReview from "$lib/schema-workbench/SchemaImpactReview.svelte";
import { MUTATION_STATUS_MESSAGES } from "$lib/entity-lifecycle/vocabulary.ts";
import {
  Layers,
  TextQuote,
  Blocks,
  LayoutTemplate,
  Trash2,
  Check,
  X,
  SlidersHorizontal,
  Save,
  AlertTriangle,
} from "@lucide/svelte";

type PackageManifest = {
  schemas: Array<{
    namespace: string;
    entityTypes: EntityTypeDefinition[];
    fields: FieldDefinition[];
  }>;
  templates: EntityTemplate[];
};

let {
  projectOpen,
  packageManifest,
  referenceEntityTypes = [],
  overlay,
  pluginId = null,
  busy = false,
  message = "",
  conflict = false,
  contentRevision = "",
  projectionLabelsForType = () => [],
  entityCountForType = () => null,
  onReassignEntities,
  onPreview,
  onSave,
  onReloadCurrent,
  onFetchCurrent,
  onAdoptCurrentRevision,
  onDirtyChange,
}: {
  projectOpen: boolean;
  packageManifest: PackageManifest;
  referenceEntityTypes?: Array<{ id: string; name: string }>;
  overlay: ModuleSchemaOverlay;
  pluginId?: string | null;
  busy?: boolean;
  message?: string;
  conflict?: boolean;
  /** Opaque content revision currently expected for CAS saves (display/debug). */
  contentRevision?: string;
  projectionLabelsForType?: (typeId: string) => string[];
  entityCountForType?: (typeId: string) => number | null;
  /** Reassign live entities of fromTypeId to toTypeId before the overlay type is removed. */
  onReassignEntities?: (fromTypeId: string, toTypeId: string) => Promise<void>;
  onPreview: (overlay: ModuleSchemaOverlay) => Promise<SchemaOverlayPreviewResult>;
  onSave: (overlay: ModuleSchemaOverlay, options?: { acknowledgeImpact?: boolean }) => Promise<void>;
  onReloadCurrent?: () => Promise<void>;
  onFetchCurrent?: () => Promise<{ overlay: ModuleSchemaOverlay; revision: string }>;
  /** Keep the local draft; adopt the server's opaque revision for the next CAS save. */
  onAdoptCurrentRevision?: (revision: string) => void;
  onDirtyChange?: (dirty: boolean) => void;
} = $props();

const flatPackage = $derived(flattenPackageSchemas(packageManifest));
const packageTypeDefinitions = $derived(flatPackage.entityTypes);
const packageTypes = $derived(packageTypeDefinitions.map((entityType) => entityType.id));
const packageFields = $derived([...flatPackage.fields]);
const packageTemplates = $derived([...(packageManifest.templates ?? [])]);

function appearanceOverride(typeId: string): EntityTypeAppearanceOverride | undefined {
  return (draft.entityTypeAppearanceOverrides ?? []).find((candidate) => candidate.entityTypeId === typeId);
}

function effectivePackageAppearance(type: EntityTypeDefinition) {
  const override = appearanceOverride(type.id);
  return {
    icon: override?.icon ?? type.icon,
    iconColor: override?.iconColor ?? type.iconColor,
  };
}

function packageAppearanceChanged(type: EntityTypeDefinition) {
  const override = appearanceOverride(type.id);
  if (!override) return false;
  const sameIcon = !override.icon || JSON.stringify(override.icon) === JSON.stringify(type.icon);
  const sameColor = !override.iconColor || JSON.stringify(override.iconColor) === JSON.stringify(type.iconColor);
  return !(sameIcon && sameColor);
}

function setPackageAppearanceOverride(
  typeId: string,
  next: { icon: IconRef; iconColor: EntityTypeColor },
  base: EntityTypeDefinition,
) {
  const sameIcon = JSON.stringify(next.icon) === JSON.stringify(base.icon);
  const sameColor = JSON.stringify(next.iconColor) === JSON.stringify(base.iconColor);
  let overrides = [...(draft.entityTypeAppearanceOverrides ?? [])];
  if (sameIcon && sameColor) {
    overrides = overrides.filter((candidate) => candidate.entityTypeId !== typeId);
  } else {
    const entry: EntityTypeAppearanceOverride = { entityTypeId: typeId };
    if (!sameIcon) entry.icon = next.icon;
    if (!sameColor) entry.iconColor = next.iconColor;
    const index = overrides.findIndex((candidate) => candidate.entityTypeId === typeId);
    if (index >= 0) overrides[index] = entry;
    else overrides.push(entry);
    overrides.sort((left, right) => left.entityTypeId.localeCompare(right.entityTypeId));
  }
  setDraft({ ...draft, entityTypeAppearanceOverrides: overrides.length > 0 ? overrides : undefined });
}

function clearPackageAppearanceOverride(typeId: string) {
  const overrides = (draft.entityTypeAppearanceOverrides ?? []).filter(
    (candidate) => candidate.entityTypeId !== typeId,
  );
  setDraft({ ...draft, entityTypeAppearanceOverrides: overrides.length > 0 ? overrides : undefined });
}

function entityTypeLabel(typeId: string): string {
  const defined =
    packageTypeDefinitions.find((type) => type.id === typeId) ??
    (draft.customEntityTypes ?? []).find((type) => type.id === typeId) ??
    referenceEntityTypes.find((type) => type.id === typeId);
  const name = defined?.name.trim();
  return name || humanizeId(typeId);
}

// Remounted by parent `{#key editorRemountKey}`; initial overlay is intentional.
// svelte-ignore state_referenced_locally
const baseline = fingerprint(overlay, { pluginId });
// Plain snapshot — leave checks must not depend on reading $state from external closures.
// svelte-ignore state_referenced_locally
let draftPlain = normalizeOverlay(overlay, { pluginId });
// svelte-ignore state_referenced_locally
let draft = $state<ModuleSchemaOverlay>(draftPlain);
// Keep initial for discard
const initialPlain = draftPlain;
/** Synced on every setDraft; read by the leave guard (plain bool, not $state). */
let dirtyFlag = false;
let dirty = $state(false);
let activeTab = $state<"types" | "fields" | "templates">("types");
let listQuery = $state("");
let statusFilter = $state<SchemaStatusFilter>("all");
let showAdvanced = $state(false);
let selectedItemId = $state<string | null>(null);
let impactPreview = $state<SchemaOverlayPreviewResult | null>(null);
let previewError = $state("");
let conflictCompare = $state<{
  current: ModuleSchemaOverlay;
  currentRevision: string;
  draft: ModuleSchemaOverlay;
} | null>(null);
let conflictActionBusy = $state(false);
let builtinTypesCollapsed = $state(false);
let builtinFieldsCollapsed = $state(false);
let builtinTemplatesCollapsed = $state(false);

let newType = $state("");
let newTypeIcon = $state<IconRef>({ kind: "catalog", id: "concept" });
let newTypeColor = $state<EntityTypeColor>(DEFAULT_TYPE_COLOR);
let newTypeFieldKeys = $state<string[]>([]);
let editingTypeId = $state<string | null>(null);
let editTypeValue = $state("");
let editTypeIcon = $state<IconRef>(FALLBACK_ICON);
let editTypeColor = $state<EntityTypeColor>(DEFAULT_TYPE_COLOR);
let editTypeFieldKeys = $state<string[]>([]);

let newFieldLabel = $state("");
let newFieldType = $state<FieldType>("text");
let newFieldEntityTypes = $state<string[]>([]);
let newFieldOptions = $state("");
let newFieldMultiple = $state(false);
let newFieldTargetEntityTypes = $state<string[]>([]);
let newFieldRelationshipType = $state("");
let newFieldCardinality = $state<"one" | "many">("many");
let newFieldOneOfVariants = $state<Array<{ label: string; type: FieldType; options: string }>>([]);
let newFieldShared = $state(false);
let newFieldTimelineEnabled = $state(false);
let newFieldTimelineRole = $state<"point" | "start" | "end">("point");
let newFieldTimelineGroup = $state("");
let newFieldTimelineLabel = $state("");
let newFieldTimelineLayer = $state<"dates" | "lifelines">("dates");
let editingBuiltinMetadataFieldKey = $state<string | null>(null);
let editBuiltinMetadataDrafts = $state<MetadataFieldDraft[]>([]);
let editingFieldKey = $state<string | null>(null);
let editFieldLabel = $state("");
let editFieldType = $state<FieldType>("text");
let editFieldEntityTypes = $state<string[]>([]);
let editFieldOptions = $state("");
let editFieldMultiple = $state(false);
let editFieldTargetEntityTypes = $state<string[]>([]);
let editFieldRelationshipType = $state("");
let editFieldCardinality = $state<"one" | "many">("many");
let editFieldOneOfVariants = $state<Array<{ label: string; type: FieldType; options: string }>>([]);
let editFieldShared = $state(false);
let editFieldTimelineEnabled = $state(false);
let editFieldTimelineRole = $state<"point" | "start" | "end">("point");
let editFieldTimelineGroup = $state("");
let editFieldTimelineLabel = $state("");
let editFieldTimelineLayer = $state<"dates" | "lifelines">("dates");
let newFieldMetadata = $state<MetadataFieldDraft[]>([]);
let editFieldMetadata = $state<MetadataFieldDraft[]>([]);
let editingBuiltinFieldKey = $state<string | null>(null);
let editingTimelineFieldKey = $state<string | null>(null);
let editTimelineRole = $state<"point" | "start" | "end">("point");
let editTimelineGroup = $state("");
let editTimelineLabel = $state("");
let editTimelineLayer = $state<"dates" | "lifelines">("dates");

let newTemplateName = $state("");
let newTemplateEntityType = $state("");
let newTemplateDescription = $state("");
let newTemplateFieldKeys = $state<string[]>([]);
let newTemplateRequiredFields = $state<string[]>([]);
let newTemplateFieldValues = $state<Record<string, unknown>>({});
let newTemplateIncludeDocument = $state(false);
let editingTemplateId = $state<string | null>(null);
let editTemplateName = $state("");
let editTemplateEntityType = $state("");
let editTemplateDescription = $state("");
let editingBuiltinTemplateId = $state<string | null>(null);
let editingTemplateFieldKeys = $state<string[]>([]);
let editingTemplateRequiredFields = $state<string[]>([]);
let editingTemplateFieldValues = $state<Record<string, unknown>>({});
let editTemplateIncludeDocument = $state(false);

function syncNewTemplateFields() {
  if (!newTemplateEntityType || !effectiveTypes().includes(newTemplateEntityType)) {
    if (newTemplateFieldKeys.length !== 0 || newTemplateRequiredFields.length !== 0) {
      newTemplateFieldKeys = [];
      newTemplateRequiredFields = [];
    }
    return;
  }
  const available = effectiveFieldsForType(newTemplateEntityType)
    .map((f) => f.key)
    .sort();
  const filtered = newTemplateFieldKeys.filter((k) => available.includes(k));
  const filteredReq = newTemplateRequiredFields.filter((k) => filtered.includes(k));
  const filteredValues = Object.fromEntries(
    Object.entries(newTemplateFieldValues).filter(([key]) => filtered.includes(key)),
  );
  if (filtered.length !== newTemplateFieldKeys.length) newTemplateFieldKeys = filtered;
  if (filteredReq.length !== newTemplateRequiredFields.length) newTemplateRequiredFields = filteredReq;
  if (Object.keys(filteredValues).length !== Object.keys(newTemplateFieldValues).length)
    newTemplateFieldValues = filteredValues;
}

function syncEditTemplateFields() {
  if (!editingTemplateId || !editTemplateEntityType) return;
  if (!effectiveTypes().includes(editTemplateEntityType)) return;
  const original = (draft.customTemplates ?? []).find((t) => t.id === editingTemplateId);
  const originalType = original?.entityType;
  const available = effectiveFieldsForType(editTemplateEntityType)
    .map((f) => f.key)
    .sort();
  if (originalType && originalType !== editTemplateEntityType) {
    const filtered = editingTemplateFieldKeys.filter((k) => available.includes(k));
    const filteredReq = editingTemplateRequiredFields.filter((k) => filtered.includes(k));
    editingTemplateFieldKeys = filtered;
    editingTemplateRequiredFields = filteredReq;
    return;
  }
  const filtered = editingTemplateFieldKeys.filter((k) => available.includes(k));
  const filteredReq = editingTemplateRequiredFields.filter((k) => filtered.includes(k));
  const filteredValues = Object.fromEntries(
    Object.entries(editingTemplateFieldValues).filter(([key]) => filtered.includes(key)),
  );
  if (filtered.length !== editingTemplateFieldKeys.length) editingTemplateFieldKeys = filtered;
  if (filteredReq.length !== editingTemplateRequiredFields.length) editingTemplateRequiredFields = filteredReq;
  if (Object.keys(filteredValues).length !== Object.keys(editingTemplateFieldValues).length)
    editingTemplateFieldValues = filteredValues;
}

$effect(() => {
  // Track entity type and available types to keep field selection valid
  void newTemplateEntityType;
  void effectiveTypes();
  syncNewTemplateFields();
});

$effect(() => {
  void editTemplateEntityType;
  void editingTemplateId;
  void effectiveTypes();
  if (editingTemplateId) syncEditTemplateFields();
});

function reportDirty(next: boolean) {
  dirtyFlag = next;
  dirty = next;
  onDirtyChange?.(next);
}

function setDraft(next: ModuleSchemaOverlay) {
  draftPlain = normalizeOverlay(next, { pluginId });
  draft = draftPlain;
  reportDirty(fingerprint(draftPlain, { pluginId }) !== baseline);
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

function matchesWorkbenchItem(item: SchemaListItem): boolean {
  return filterSchemaListItems([item], listQuery, statusFilter).length > 0;
}

function typeMatches(type: EntityTypeDefinition, origin: "builtin" | "custom"): boolean {
  return matchesWorkbenchItem({
    id: type.id,
    kind: "type",
    name: type.name,
    origin,
    enabled: origin === "custom" || !isDisabled(draft.disabledEntityTypes, type.id),
    namespace: origin === "builtin" ? flatPackage.typeNamespace[type.id] : undefined,
    hint: localTypeId(type.id),
    searchText: [type.name, type.id, flatPackage.typeNamespace[type.id] ?? "", "type"].join(" "),
  });
}

function fieldMatches(field: FieldDefinition, origin: "builtin" | "custom"): boolean {
  return matchesWorkbenchItem({
    id: field.key,
    kind: "field",
    name: field.label || humanizeId(field.key),
    origin,
    enabled: origin === "custom" || !isDisabled(draft.disabledFields, field.key),
    namespace: origin === "builtin" ? flatPackage.fieldNamespace[field.key] : undefined,
    hint: fieldTypeLabel(field.type),
    searchText: [
      field.label,
      field.key,
      field.type,
      fieldKindGroupLabel(field.type),
      flatPackage.fieldNamespace[field.key] ?? "",
      scopeLabel(field.entityTypes),
    ].join(" "),
  });
}

function templateMatches(template: EntityTemplate, origin: "builtin" | "custom"): boolean {
  return matchesWorkbenchItem({
    id: template.id,
    kind: "template",
    name: template.name,
    origin,
    enabled: origin === "custom" || !isDisabled(draft.disabledTemplates, template.id),
    hint: entityTypeLabel(template.entityType),
    searchText: [
      template.name,
      template.id,
      template.description ?? "",
      template.entityType,
      entityTypeLabel(template.entityType),
      "template",
    ].join(" "),
  });
}

function editingPreviewFields(entityType: string): Array<{ field: FieldDefinition; required: boolean }> {
  return editingTemplateFieldKeys
    .map((key) => {
      const field = effectiveFieldsForType(entityType).find((candidate) => candidate.key === key);
      return field ? { field, required: editingTemplateRequiredFields.includes(key) } : null;
    })
    .filter((item): item is { field: FieldDefinition; required: boolean } => item !== null);
}

function closeSelectedItem() {
  cancelTypeEdit();
  cancelFieldEdit();
  cancelTemplateFieldEdit();
  cancelBuiltinMetadataEdit();
  cancelTimelineEdit();
  editingBuiltinFieldKey = null;
  selectedItemId = null;
}

function toggleDisabled(listKey: "disabledEntityTypes" | "disabledFields" | "disabledTemplates", id: string) {
  const current = new Set(draft[listKey] ?? []);
  if (current.has(id)) current.delete(id);
  else current.add(id);
  const next = { ...draft, [listKey]: [...current].sort() } as ModuleSchemaOverlay;
  if (listKey === "disabledFields" && current.has(id)) {
    // Remove scope and metadata overrides for disabled field
    next.fieldScopeOverrides = (next.fieldScopeOverrides ?? []).filter(
      (ov: { fieldKey: string }) => ov.fieldKey !== id,
    );
    next.fieldMetadataOverrides = (next.fieldMetadataOverrides ?? []).filter(
      (ov: { fieldKey: string }) => ov.fieldKey !== id,
    );
    (next as unknown as Record<string, unknown>).fieldTimelineOverrides = (
      ((next as unknown as Record<string, unknown>).fieldTimelineOverrides as
        Array<{ fieldKey: string }> | undefined) ?? []
    ).filter((ov) => ov.fieldKey !== id);
    if (
      ((next as unknown as Record<string, unknown>).fieldTimelineOverrides as Array<unknown> | undefined)?.length === 0
    )
      delete (next as unknown as Record<string, unknown>).fieldTimelineOverrides;
    if (editingBuiltinFieldKey === id) editingBuiltinFieldKey = null;
    if (editingBuiltinMetadataFieldKey === id) cancelBuiltinMetadataEdit();
    if (editingTimelineFieldKey === id) editingTimelineFieldKey = null;
  }
  if (listKey === "disabledEntityTypes" && current.has(id)) {
    const overrides = (next.entityTypeAppearanceOverrides ?? []).filter((ov) => ov.entityTypeId !== id);
    next.entityTypeAppearanceOverrides = overrides.length > 0 ? overrides : undefined;
  }
  setDraft(next);
}

function effectiveTypes() {
  return [
    ...packageTypes.filter((type) => !isDisabled(draft.disabledEntityTypes, type)),
    ...(draft.customEntityTypes ?? []).map((entityType) => entityType.id),
  ];
}

function fieldScopeTypes() {
  const owned = effectiveTypes();
  const extras = referenceEntityTypes.map((type) => type.id).filter((id) => !owned.includes(id));
  return [...owned, ...extras];
}

function builtinFieldScope(field: FieldDefinition): string[] {
  const override = (draft.fieldScopeOverrides ?? []).find((scope) => scope.fieldKey === field.key);
  if (override) return [...override.entityTypes];
  if (field.entityTypes?.length) return [...field.entityTypes];
  // Builtin fields with no explicit scope historically meant "all builtin types" — not automatically new custom types
  return [...packageTypes.filter((type) => !isDisabled(draft.disabledEntityTypes, type))];
}

function fieldAppliesTo(field: FieldDefinition, entityType: string): boolean {
  const scope = packageFields.some((candidate) => candidate.key === field.key)
    ? builtinFieldScope(field)
    : field.entityTypes && field.entityTypes.length
      ? field.entityTypes
      : effectiveTypes();
  return scope.includes(entityType);
}

function effectiveFieldsForType(entityType: string): FieldDefinition[] {
  return [...packageFields, ...(draft.customFields ?? [])].filter(
    (field) => !isDisabled(draft.disabledFields, field.key) && fieldAppliesTo(field, entityType),
  );
}

function effectiveTemplates(): EntityTemplate[] {
  return [...packageTemplates, ...(draft.customTemplates ?? [])].filter(
    (template) => !isDisabled(draft.disabledTemplates, template.id),
  );
}

/** All enabled fields (builtin + custom) as searchable picker options. */
function selectableFieldOptions() {
  return [...packageFields, ...(draft.customFields ?? [])]
    .filter((field) => !isDisabled(draft.disabledFields, field.key))
    .map((field) => ({
      key: field.key,
      label: field.label || humanizeId(field.key),
      hint: fieldTypeLabel(field.type),
    }))
    .sort((left, right) => left.label.localeCompare(right.label));
}

function effectiveTimeline(
  field: FieldDefinition,
): { role: string; group?: string; label?: string; layer?: string } | null | undefined {
  const override = (
    draft as unknown as { fieldTimelineOverrides?: Array<{ fieldKey: string; timeline: unknown }> }
  ).fieldTimelineOverrides?.find((ov) => ov.fieldKey === field.key);
  if (override !== undefined)
    return override.timeline as unknown as { role: string; group?: string; label?: string; layer?: string } | null;
  const raw = (field as unknown as Record<string, unknown>).timeline as unknown as
    { role: string; group?: string; label?: string; layer?: string } | undefined;
  return raw ?? null;
}

function timelineBadge(field: FieldDefinition): string {
  const tl = effectiveTimeline(field);
  if (!tl) return "";
  const role = tl.role ?? "point";
  const layer = tl.layer ?? "dates";
  const group = tl.group ? `· ${tl.group}` : "";
  const label = tl.label ? `· ${tl.label}` : "";
  return `${role} · ${layer} ${group} ${label}`.trim();
}

function isTimelineEnabled(field: FieldDefinition): boolean {
  return effectiveTimeline(field) != null;
}

function startTimelineEdit(field: FieldDefinition) {
  if (field.type !== "date") return;
  editingBuiltinFieldKey = null;
  editingBuiltinMetadataFieldKey = null;
  editingFieldKey = null;
  editingTimelineFieldKey = field.key;
  selectedItemId = `field:${field.key}`;
  const tl = effectiveTimeline(field);
  if (tl) {
    editTimelineRole = (tl.role as "point" | "start" | "end") ?? "point";
    editTimelineGroup = tl.group ?? "";
    editTimelineLabel = tl.label ?? "";
    editTimelineLayer = (tl.layer as "dates" | "lifelines") ?? "dates";
  } else {
    editTimelineRole = "point";
    editTimelineGroup = "";
    editTimelineLabel = "";
    editTimelineLayer = "dates";
  }
}

function cancelTimelineEdit() {
  editingTimelineFieldKey = null;
  selectedItemId = null;
}

function commitTimelineEdit() {
  const key = editingTimelineFieldKey;
  if (!key) return;
  const field = packageFields.find((f) => f.key === key) ?? (draft.customFields ?? []).find((f) => f.key === key);
  if (!field || field.type !== "date") return;
  if (editTimelineRole === "start" || editTimelineRole === "end") {
    if (!editTimelineGroup.trim()) return;
  }
  const isBuiltin = packageFields.some((f) => f.key === key);
  const isCustom = !isBuiltin && (draft.customFields ?? []).some((f) => f.key === key);
  if (isCustom) {
    // Custom date fields must be shared to carry a timeline (validated in Rust); mirror guard in UI.
    const shared = (field as unknown as Record<string, unknown>).shared as boolean | undefined;
    if (!shared) return;
  }
  const timeline = {
    role: editTimelineRole,
    ...(editTimelineGroup.trim() ? { group: editTimelineGroup.trim() } : {}),
    ...(editTimelineLabel.trim() ? { label: editTimelineLabel.trim() } : {}),
    layer: editTimelineLayer,
  };
  const existing =
    ((draft as unknown as Record<string, unknown>).fieldTimelineOverrides as
      Array<{ fieldKey: string; timeline: unknown }> | undefined) ?? [];
  const nextOverrides = existing.filter((ov) => ov.fieldKey !== key);
  // Custom fields: single source of truth is field.timeline on draft.customFields.
  // Do not emit a fieldTimelineOverrides entry; mutate the custom field directly and prune any stale override.
  if (isCustom) {
    const nextCustomFields = (draft.customFields ?? []).map((f) => {
      if (f.key !== key) return f;
      return { ...f, timeline: timeline as unknown as FieldDefinition["timeline"] } as FieldDefinition;
    });
    const next: unknown = {
      ...(draft as unknown as Record<string, unknown>),
      customFields: nextCustomFields,
      fieldTimelineOverrides: nextOverrides.length ? nextOverrides : undefined,
    };
    const cleaned = next as Record<string, unknown>;
    if (!nextOverrides.length) delete cleaned.fieldTimelineOverrides;
    setDraft(cleaned as unknown as ModuleSchemaOverlay);
    cancelTimelineEdit();
    return;
  }
  const builtinTimeline = (field as unknown as Record<string, unknown>).timeline as unknown as
    Record<string, unknown> | undefined;
  // For builtin, compare to packaged timeline to decide whether to store override or clear
  let shouldStore = true;
  if (isBuiltin && builtinTimeline) {
    const packaged = {
      role: String((builtinTimeline as Record<string, unknown>).role ?? "").toLowerCase(),
      group:
        (builtinTimeline as Record<string, unknown>).group != null
          ? String((builtinTimeline as Record<string, unknown>).group)
          : undefined,
      label:
        (builtinTimeline as Record<string, unknown>).label != null
          ? String((builtinTimeline as Record<string, unknown>).label)
          : undefined,
      layer:
        (builtinTimeline as Record<string, unknown>).layer != null
          ? String((builtinTimeline as Record<string, unknown>).layer).toLowerCase()
          : "dates",
    };
    const nextNorm = {
      role: timeline.role,
      group: (timeline as Record<string, unknown>).group as string | undefined,
      label: (timeline as Record<string, unknown>).label as string | undefined,
      layer: timeline.layer ?? "dates",
    };
    if (JSON.stringify(packaged) === JSON.stringify(nextNorm)) shouldStore = false;
  } else if (isBuiltin && !builtinTimeline) {
    // builtin without timeline — storing point defaults would be an enable, so store
    shouldStore = true;
  }
  if (shouldStore) {
    nextOverrides.push({ fieldKey: key, timeline });
    nextOverrides.sort((a, b) => a.fieldKey.localeCompare(b.fieldKey));
  }
  {
    const next: unknown = {
      ...(draft as unknown as Record<string, unknown>),
      fieldTimelineOverrides: nextOverrides.length ? nextOverrides : undefined,
    };
    const cleaned = next as Record<string, unknown>;
    if (!nextOverrides.length) delete cleaned.fieldTimelineOverrides;
    setDraft(cleaned as unknown as ModuleSchemaOverlay);
  }
  cancelTimelineEdit();
}

function disableTimeline(field: FieldDefinition) {
  const key = field.key;
  const existing =
    ((draft as unknown as Record<string, unknown>).fieldTimelineOverrides as
      Array<{ fieldKey: string; timeline: unknown }> | undefined) ?? [];
  const nextOverrides = existing.filter((ov) => ov.fieldKey !== key);
  const isBuiltin = packageFields.some((f) => f.key === key);
  const isCustom = !isBuiltin && (draft.customFields ?? []).some((f) => f.key === key);
  if (isCustom) {
    const nextCustomFields = (draft.customFields ?? []).map((f) => {
      if (f.key !== key) return f;
      const nf = { ...f } as Record<string, unknown>;
      delete nf.timeline;
      return nf as unknown as FieldDefinition;
    });
    const next: unknown = {
      ...(draft as unknown as Record<string, unknown>),
      customFields: nextCustomFields,
      fieldTimelineOverrides: nextOverrides.length ? nextOverrides : undefined,
    };
    const cleaned = next as Record<string, unknown>;
    if (!nextOverrides.length) delete cleaned.fieldTimelineOverrides;
    setDraft(cleaned as unknown as ModuleSchemaOverlay);
    if (editingTimelineFieldKey === key) editingTimelineFieldKey = null;
    return;
  }
  const hasBuiltinTimeline = !!(field as unknown as Record<string, unknown>).timeline;
  if (hasBuiltinTimeline) {
    // disabling builtin timeline → store null override
    nextOverrides.push({ fieldKey: key, timeline: null as unknown as null });
    nextOverrides.sort((a, b) => a.fieldKey.localeCompare(b.fieldKey));
    {
      const next: unknown = { ...(draft as unknown as Record<string, unknown>), fieldTimelineOverrides: nextOverrides };
      setDraft(next as unknown as ModuleSchemaOverlay);
    }
  } else {
    // builtin without timeline but with an existing override (e.g. previously enabled) → just remove override
    if (nextOverrides.length === 0) {
      const d = { ...(draft as unknown as Record<string, unknown>) } as unknown as Record<string, unknown>;
      delete d.fieldTimelineOverrides;
      setDraft(d as unknown as ModuleSchemaOverlay);
    } else {
      const next: unknown = { ...(draft as unknown as Record<string, unknown>), fieldTimelineOverrides: nextOverrides };
      setDraft(next as unknown as ModuleSchemaOverlay);
    }
  }
  if (editingTimelineFieldKey === key) editingTimelineFieldKey = null;
}

function updateBuiltinFieldScope(field: FieldDefinition, entityTypes: string[]) {
  const nextTypes = [...new Set(entityTypes)].sort();
  const baseline = field.entityTypes?.length ? [...field.entityTypes].sort() : effectiveTypes().sort();
  const fieldScopeOverrides = (draft.fieldScopeOverrides ?? []).filter((scope) => scope.fieldKey !== field.key);
  const disabledFields = new Set(draft.disabledFields ?? []);
  if (nextTypes.length === 0) {
    disabledFields.add(field.key);
  } else {
    disabledFields.delete(field.key);
    if (JSON.stringify(nextTypes) !== JSON.stringify(baseline)) {
      fieldScopeOverrides.push({ fieldKey: field.key, entityTypes: nextTypes });
    }
  }
  const applies = (key: string, type: string) => {
    if (key === field.key) return nextTypes.includes(type);
    const candidate = [...packageFields, ...(draft.customFields ?? [])].find((item) => item.key === key);
    return candidate ? fieldAppliesTo(candidate, type) : false;
  };
  const pruneTemplate = (template: EntityTemplate) => {
    const fields = { ...(template.fields as Record<string, unknown>) };
    for (const key of Object.keys(fields)) if (!applies(key, template.entityType)) delete fields[key];
    return {
      ...template,
      fields,
      requiredFields: template.requiredFields?.filter((key) => key in fields) ?? null,
    };
  };
  setDraft({
    ...draft,
    disabledFields: [...disabledFields].sort(),
    fieldScopeOverrides,
    customTemplates: (draft.customTemplates ?? []).map(pruneTemplate),
    templateOverrides: (draft.templateOverrides ?? []).map((override) => {
      const template = packageTemplates.find((candidate) => candidate.id === override.templateId);
      if (!template) return override;
      const fields = { ...override.fields };
      for (const key of Object.keys(fields)) if (!applies(key, template.entityType)) delete fields[key];
      return { ...override, fields, requiredFields: override.requiredFields?.filter((key) => key in fields) ?? null };
    }),
  });
  editingBuiltinFieldKey = null;
}

/**
 * Diff `desiredKeys` against the fields that currently apply to `typeId` and rewrite
 * custom-field scopes / builtin scope overrides accordingly, pruning templates whose
 * fields no longer apply.
 */
function applyTypeFieldSelection(typeId: string, desiredKeys: string[]) {
  const desired = new Set(desiredKeys);
  const current = new Set(effectiveFieldsForType(typeId).map((field) => field.key));
  const changedKeys = [...desired, ...current].filter((key) => desired.has(key) !== current.has(key));
  if (changedKeys.length === 0) return;

  const allTypes = effectiveTypes().sort();
  const typesWithout = allTypes.filter((type) => type !== typeId);

  const customFields = (draft.customFields ?? []).map((field) => {
    if (!changedKeys.includes(field.key)) return field;
    // Empty entityTypes means "all types" — materialize the full list before removing one.
    const base = field.entityTypes?.length ? [...field.entityTypes] : [...allTypes];
    const next = desired.has(field.key) ? [...base, typeId] : base.filter((type) => type !== typeId);
    const unique = [...new Set(next)];
    return { ...field, entityTypes: (unique.length ? unique : typesWithout).sort() };
  });

  const overrides = new Map((draft.fieldScopeOverrides ?? []).map((scope) => [scope.fieldKey, scope.entityTypes]));
  const disabledFields = new Set(draft.disabledFields ?? []);
  for (const field of packageFields) {
    if (!changedKeys.includes(field.key)) continue;
    const base = builtinFieldScope(field);
    const next = desired.has(field.key) ? [...base, typeId] : base.filter((type) => type !== typeId);
    const unique = [...new Set(next)].sort();
    const baseline = field.entityTypes?.length ? [...field.entityTypes].sort() : allTypes;
    if (unique.length === 0) {
      disabledFields.add(field.key);
      overrides.delete(field.key);
    } else if (JSON.stringify(unique) === JSON.stringify(baseline)) {
      disabledFields.delete(field.key);
      overrides.delete(field.key);
    } else {
      disabledFields.delete(field.key);
      overrides.set(field.key, unique);
    }
  }

  const customByKey = new Map(customFields.map((field) => [field.key, field]));
  const enabledPackageTypes = packageTypes.filter((type) => !isDisabled(draft.disabledEntityTypes, type));
  const appliesFinal = (key: string, type: string): boolean => {
    const custom = customByKey.get(key);
    if (custom) {
      const scope = custom.entityTypes?.length ? custom.entityTypes : allTypes;
      return scope.includes(type);
    }
    const builtin = packageFields.find((candidate) => candidate.key === key);
    if (!builtin || disabledFields.has(key)) return false;
    const override = overrides.get(key);
    if (override) return override.includes(type);
    if (builtin.entityTypes?.length) return builtin.entityTypes.includes(type);
    return enabledPackageTypes.includes(type);
  };

  setDraft({
    ...draft,
    customFields,
    disabledFields: [...disabledFields].sort(),
    fieldScopeOverrides: [...overrides.entries()]
      .map(([fieldKey, entityTypes]) => ({ fieldKey, entityTypes }))
      .sort((left, right) => left.fieldKey.localeCompare(right.fieldKey)),
    customTemplates: (draft.customTemplates ?? []).map((template) => {
      const fields = { ...(template.fields as Record<string, unknown>) };
      for (const key of Object.keys(fields)) if (!appliesFinal(key, template.entityType)) delete fields[key];
      return {
        ...template,
        fields,
        requiredFields: template.requiredFields?.filter((key) => key in fields) ?? null,
      };
    }),
    templateOverrides: (draft.templateOverrides ?? []).map((override) => {
      const template = packageTemplates.find((candidate) => candidate.id === override.templateId);
      if (!template) return override;
      const fields = { ...override.fields };
      for (const key of Object.keys(fields)) if (!appliesFinal(key, template.entityType)) delete fields[key];
      return { ...override, fields, requiredFields: override.requiredFields?.filter((key) => key in fields) ?? null };
    }),
  });
}

function scopeLabel(entityTypes: string[] | undefined): string {
  if (!entityTypes || entityTypes.length === 0) return "All types";
  return entityTypes.map(entityTypeLabel).join(", ");
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

function fieldsForTemplate(template: EntityTemplate, builtin: boolean): Record<string, unknown> {
  if (builtin) {
    const override = (draft.templateOverrides ?? []).find((candidate) => candidate.templateId === template.id);
    if (override) return { ...override.fields };
  }
  return { ...(template.fields as Record<string, unknown>) };
}

function beginTemplateFieldEdit(template: EntityTemplate, builtin: boolean) {
  editingTypeId = null;
  editingFieldKey = null;
  editingBuiltinFieldKey = null;
  editingBuiltinMetadataFieldKey = null;
  editingTemplateId = builtin ? null : template.id;
  editingBuiltinTemplateId = builtin ? template.id : null;
  selectedItemId = `template:${template.id}`;
  editTemplateName = template.name;
  editTemplateEntityType = template.entityType;
  editTemplateDescription = template.description ?? "";
  const fields = fieldsForTemplate(template, builtin);
  editingTemplateFieldKeys = Object.keys(fields).sort();
  editingTemplateRequiredFields = [
    ...((builtin
      ? (draft.templateOverrides ?? []).find((candidate) => candidate.templateId === template.id)?.requiredFields
      : template.requiredFields) ?? []),
  ].sort();
  editingTemplateFieldValues = { ...fields };
  editTemplateIncludeDocument = !builtin && template.document !== undefined && template.document !== null;
}

function templateFieldsFromSelection(template: EntityTemplate): Record<string, unknown> {
  const existing = fieldsForTemplate(template, Boolean(editingBuiltinTemplateId));
  return Object.fromEntries(
    editingTemplateFieldKeys.map((key) => {
      const field = effectiveFieldsForType(template.entityType).find((candidate) => candidate.key === key);
      return [
        key,
        key in editingTemplateFieldValues
          ? editingTemplateFieldValues[key]
          : key in existing
            ? existing[key]
            : field
              ? defaultFieldValue(field)
              : "",
      ];
    }),
  );
}

function cancelTemplateFieldEdit() {
  cancelTemplateEdit();
  editingBuiltinTemplateId = null;
  editingTemplateFieldKeys = [];
  editingTemplateRequiredFields = [];
  editingTemplateFieldValues = {};
  editTemplateIncludeDocument = false;
  selectedItemId = null;
}

async function save() {
  const candidate = normalizeOverlay(draft, { pluginId });
  previewError = "";
  try {
    const preview = await onPreview(candidate);
    if (!preview.ok || preview.requiresAcknowledgement) {
      impactPreview = preview;
      return;
    }
    await onSave(candidate);
  } catch (cause) {
    previewError = cause instanceof Error ? cause.message : String(cause);
  }
}

async function confirmImpactSave() {
  if (!impactPreview?.ok) return;
  const candidate = normalizeOverlay(draft, { pluginId });
  impactPreview = null;
  previewError = "";
  await onSave(candidate, { acknowledgeImpact: true });
}

function cancelImpactReview() {
  impactPreview = null;
}

async function reloadCurrentOverlay() {
  if (!onReloadCurrent) return;
  conflictCompare = null;
  await onReloadCurrent();
}

async function compareWithCurrent() {
  if (!onFetchCurrent) return;
  conflictActionBusy = true;
  previewError = "";
  try {
    const current = await onFetchCurrent();
    conflictCompare = {
      current: current.overlay,
      currentRevision: current.revision,
      draft: normalizeOverlay(draft, { pluginId }),
    };
  } catch (cause) {
    previewError = cause instanceof Error ? cause.message : String(cause);
  } finally {
    conflictActionBusy = false;
  }
}

async function reapplyDraftOntoCurrent() {
  if (!onFetchCurrent || !onAdoptCurrentRevision) return;
  conflictActionBusy = true;
  previewError = "";
  try {
    const current = await onFetchCurrent();
    // Keep the local draft; only adopt the server's opaque revision for CAS.
    onAdoptCurrentRevision(current.revision);
    conflictCompare = null;
  } catch (cause) {
    previewError = cause instanceof Error ? cause.message : String(cause);
  } finally {
    conflictActionBusy = false;
  }
}

function overlaySummary(value: ModuleSchemaOverlay): string {
  const types = value.customEntityTypes?.length ?? 0;
  const fields = value.customFields?.length ?? 0;
  const templates = value.customTemplates?.length ?? 0;
  const disabledTypes = value.disabledEntityTypes?.length ?? 0;
  const disabledFields = value.disabledFields?.length ?? 0;
  return [
    `custom types ${types}`,
    `custom fields ${fields}`,
    `custom templates ${templates}`,
    `disabled types ${disabledTypes}`,
    `disabled fields ${disabledFields}`,
  ].join(" · ");
}

async function discardChanges() {
  if (!dirty) return;
  const confirmed = await confirmDialog({
    title: "Discard unsaved changes?",
    message: "All unsaved types, fields, and templates will be reverted to the last saved schema.",
    confirmLabel: "Discard",
    danger: true,
  });
  if (!confirmed) return;
  setDraft(initialPlain);
  newType = "";
  newTypeFieldKeys = [];
  editingTypeId = null;
  editTypeValue = "";
  editTypeFieldKeys = [];
  newFieldLabel = "";
  newFieldType = "text";
  newFieldEntityTypes = [];
  newFieldOptions = "";
  newFieldMultiple = false;
  newFieldTargetEntityTypes = [];
  newFieldRelationshipType = "";
  newFieldCardinality = "many";
  newFieldOneOfVariants = [];
  newFieldMetadata = [];
  newFieldShared = false;
  newFieldTimelineEnabled = false;
  newFieldTimelineRole = "point";
  newFieldTimelineGroup = "";
  newFieldTimelineLabel = "";
  newFieldTimelineLayer = "dates";
  editingFieldKey = null;
  editFieldLabel = "";
  editFieldType = "text";
  editFieldEntityTypes = [];
  editFieldOptions = "";
  editFieldMultiple = false;
  editFieldTargetEntityTypes = [];
  editFieldRelationshipType = "";
  editFieldCardinality = "many";
  editFieldOneOfVariants = [];
  editFieldMetadata = [];
  editFieldShared = false;
  editFieldTimelineEnabled = false;
  editFieldTimelineRole = "point";
  editFieldTimelineGroup = "";
  editFieldTimelineLabel = "";
  editFieldTimelineLayer = "dates";
  editingBuiltinFieldKey = null;
  editingBuiltinMetadataFieldKey = null;
  editBuiltinMetadataDrafts = [];
  editingTimelineFieldKey = null;
  editTimelineRole = "point";
  editTimelineGroup = "";
  editTimelineLabel = "";
  editTimelineLayer = "dates";
  newTemplateName = "";
  newTemplateEntityType = "";
  newTemplateDescription = "";
  newTemplateFieldKeys = [];
  newTemplateRequiredFields = [];
  newTemplateFieldValues = {};
  newTemplateIncludeDocument = false;
  editingTemplateId = null;
  editTemplateName = "";
  editTemplateEntityType = "";
  editTemplateDescription = "";
  editingBuiltinTemplateId = null;
  editingTemplateFieldKeys = [];
  editingTemplateRequiredFields = [];
  editingTemplateFieldValues = {};
  editTemplateIncludeDocument = false;
  typeRemovalPrompt = null;
}

function cancelTypeEdit() {
  editingTypeId = null;
  editTypeValue = "";
  editTypeIcon = FALLBACK_ICON;
  editTypeColor = DEFAULT_TYPE_COLOR;
  editTypeFieldKeys = [];
  selectedItemId = null;
}

function startTypeEdit(type: string) {
  editingFieldKey = null;
  editingTemplateId = null;
  editingTypeId = type;
  selectedItemId = `type:${type}`;
  const definition = (draft.customEntityTypes ?? []).find((candidate) => candidate.id === type);
  editTypeValue = definition?.name ?? humanizeId(type);
  editTypeIcon = definition?.icon ?? FALLBACK_ICON;
  editTypeColor = definition?.iconColor ?? DEFAULT_TYPE_COLOR;
  editTypeFieldKeys = effectiveFieldsForType(type)
    .map((field) => field.key)
    .sort();
}

function addCustomType() {
  const name = newType.trim();
  if (!name) return;
  const id = mintTypeId(name, pluginId);
  if (packageTypes.includes(id) || (draft.customEntityTypes ?? []).some((entityType) => entityType.id === id)) return;
  setDraft({
    ...draft,
    customEntityTypes: [
      ...(draft.customEntityTypes ?? []),
      { id, name, icon: newTypeIcon, iconColor: newTypeColor },
    ].sort((left, right) => left.id.localeCompare(right.id)),
  });
  if (newTypeFieldKeys.length > 0) applyTypeFieldSelection(id, newTypeFieldKeys);
  newType = "";
  newTypeIcon = { kind: "catalog", id: "concept" };
  newTypeColor = DEFAULT_TYPE_COLOR;
  newTypeFieldKeys = [];
}

function commitTypeEdit() {
  if (!editingTypeId) return;
  const stableId = editingTypeId;
  if (!editTypeValue.trim()) {
    cancelTypeEdit();
    return;
  }
  setDraft({
    ...draft,
    customEntityTypes: (draft.customEntityTypes ?? [])
      .map((item) =>
        item.id === stableId
          ? { ...item, name: editTypeValue.trim(), icon: editTypeIcon, iconColor: editTypeColor }
          : item,
      )
      .sort((left, right) => left.id.localeCompare(right.id)),
  });
  applyTypeFieldSelection(stableId, editTypeFieldKeys);
  cancelTypeEdit();
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
  exclusiveDispositions: Record<string, ExclusiveFieldDisposition | undefined>;
  entityCount: number | null;
  entityDisposition: EntityRemovalDisposition | undefined;
  busy: boolean;
  error: string;
};

let typeRemovalPrompt = $state<TypeRemovalPrompt | null>(null);

function requestRemoveCustomType(name: string) {
  const { exclusiveFields, sharedFields, templates } = dependentsForType(name);
  const entityCount = entityCountForType(name);
  const available = effectiveTypes().filter((typeId) => typeId !== name);
  const needsEntityResolution = entityCount === null || entityCount > 0;
  if (exclusiveFields.length === 0 && sharedFields.length === 0 && templates.length === 0 && !needsEntityResolution) {
    if (editingTypeId === name) cancelTypeEdit();
    setDraft(
      applyTypeRemovalPlan(draft, {
        typeId: name,
        exclusiveDispositions: {},
        removeSharedFieldKeys: [],
        entityDisposition: { action: "none" },
        entityCount: 0,
      }),
    );
    return;
  }
  typeRemovalPrompt = {
    typeId: name,
    exclusiveFields,
    sharedFields,
    templates,
    removeSharedFields: false,
    exclusiveDispositions: {},
    entityCount,
    entityDisposition: needsEntityResolution
      ? available[0]
        ? { action: "reassign", toTypeId: available[0] }
        : undefined
      : { action: "none" },
    busy: false,
    error: "",
  };
}

function updateExclusiveDisposition(fieldKey: string, action: "remove" | "disable" | "reassign") {
  const prompt = typeRemovalPrompt;
  if (!prompt) return;
  const available = effectiveTypes().filter((typeId) => typeId !== prompt.typeId);
  const disposition: ExclusiveFieldDisposition =
    action === "reassign" ? { action, toTypeId: available[0] ?? "" } : { action };
  typeRemovalPrompt = {
    ...prompt,
    exclusiveDispositions: { ...prompt.exclusiveDispositions, [fieldKey]: disposition },
  };
}

function updateExclusiveReassignment(fieldKey: string, toTypeId: string) {
  const prompt = typeRemovalPrompt;
  if (!prompt) return;
  typeRemovalPrompt = {
    ...prompt,
    exclusiveDispositions: {
      ...prompt.exclusiveDispositions,
      [fieldKey]: { action: "reassign", toTypeId },
    },
  };
}

function exclusiveReassignmentTarget(prompt: TypeRemovalPrompt, fieldKey: string): string {
  const disposition = prompt.exclusiveDispositions[fieldKey];
  return disposition?.action === "reassign" ? disposition.toTypeId : "";
}

function updateEntityDisposition(action: "none" | "reassign") {
  const prompt = typeRemovalPrompt;
  if (!prompt) return;
  const available = effectiveTypes().filter((typeId) => typeId !== prompt.typeId);
  typeRemovalPrompt = {
    ...prompt,
    entityDisposition: action === "reassign" ? { action, toTypeId: available[0] ?? "" } : { action: "none" },
    error: "",
  };
}

function updateEntityReassignment(toTypeId: string) {
  const prompt = typeRemovalPrompt;
  if (!prompt) return;
  typeRemovalPrompt = {
    ...prompt,
    entityDisposition: { action: "reassign", toTypeId },
    error: "",
  };
}

function removalPlanComplete(prompt: TypeRemovalPrompt): boolean {
  return typeRemovalPlanIsComplete(
    prompt.exclusiveFields.map((field) => field.key),
    prompt.exclusiveDispositions,
    effectiveTypes().filter((typeId) => typeId !== prompt.typeId),
    {
      entityCount: prompt.entityCount,
      entityDisposition: prompt.entityDisposition,
    },
  );
}

function cancelTypeRemoval() {
  if (typeRemovalPrompt?.busy) return;
  typeRemovalPrompt = null;
}

async function confirmTypeRemoval() {
  const prompt = typeRemovalPrompt;
  if (!prompt || !removalPlanComplete(prompt) || prompt.busy) return;
  const { typeId, entityDisposition } = prompt;
  if (editingTypeId === typeId) cancelTypeEdit();
  const exclusiveDispositions = Object.fromEntries(
    Object.entries(prompt.exclusiveDispositions).filter(
      (entry): entry is [string, ExclusiveFieldDisposition] => entry[1] !== undefined,
    ),
  );

  typeRemovalPrompt = { ...prompt, busy: true, error: "" };
  try {
    if (entityDisposition?.action === "reassign") {
      if (!onReassignEntities) {
        throw new Error("Entity reassignment is unavailable in this session.");
      }
      await onReassignEntities(typeId, entityDisposition.toTypeId);
    }
    setDraft(
      applyTypeRemovalPlan(draft, {
        typeId,
        exclusiveDispositions,
        removeSharedFieldKeys: prompt.removeSharedFields ? prompt.sharedFields.map((field) => field.key) : [],
        entityDisposition: entityDisposition ?? { action: "none" },
        entityCount: prompt.entityCount,
      }),
    );
    typeRemovalPrompt = null;
  } catch (cause) {
    typeRemovalPrompt = {
      ...prompt,
      busy: false,
      error: cause instanceof Error ? cause.message : String(cause),
    };
  }
}

function cancelFieldEdit() {
  editingFieldKey = null;
  editFieldLabel = "";
  editFieldType = "text";
  editFieldEntityTypes = [];
  editFieldOptions = "";
  editFieldMultiple = false;
  editFieldTargetEntityTypes = [];
  editFieldRelationshipType = "";
  editFieldCardinality = "many";
  editFieldOneOfVariants = [];
  editFieldMetadata = [];
  editFieldShared = false;
  editFieldTimelineEnabled = false;
  editFieldTimelineRole = "point";
  editFieldTimelineGroup = "";
  editFieldTimelineLabel = "";
  editFieldTimelineLayer = "dates";
  selectedItemId = null;
}

function startFieldEdit(field: FieldDefinition) {
  editingTypeId = null;
  editingTemplateId = null;
  editingBuiltinFieldKey = null;
  editingBuiltinMetadataFieldKey = null;
  editingTimelineFieldKey = null;
  editingFieldKey = field.key;
  selectedItemId = `field:${field.key}`;
  editFieldLabel = field.label;
  editFieldType = FIELD_TYPES.includes(field.type) ? field.type : "text";
  editFieldEntityTypes = [...(field.entityTypes ?? [])].sort();
  editFieldOptions = formatOptions(field.options as string[] | undefined);
  editFieldMultiple = Boolean((field as any).multiple);
  editFieldTargetEntityTypes = [...((field as any).targetEntityTypes ?? [])].sort();
  editFieldRelationshipType = humanizeId((field as any).relationshipType ?? "");
  editFieldCardinality = (field as any).cardinality === "one" ? "one" : "many";
  const rawOneOf = (field as any).oneOf as Array<{ label: string; type: string; options?: string[] }> | undefined;
  editFieldOneOfVariants = (rawOneOf ?? []).map((v) => ({
    label: v.label,
    type: (FIELD_TYPES.includes(v.type as FieldType) ? v.type : "text") as FieldType,
    options: formatOptions(v.options as string[] | undefined),
  }));
  editFieldMetadata = draftsFromMetadataFields((field as unknown as Record<string, unknown>).metadataFields);
  editFieldShared = Boolean((field as unknown as Record<string, unknown>).shared);
  const tl = (field as unknown as Record<string, unknown>).timeline as Record<string, unknown> | undefined;
  if (tl && typeof tl === "object") {
    editFieldTimelineEnabled = true;
    editFieldTimelineRole =
      (String((tl as Record<string, unknown>).role ?? "point").toLowerCase() as "point" | "start" | "end") ?? "point";
    editFieldTimelineGroup = tl.group != null ? String(tl.group) : "";
    editFieldTimelineLabel = tl.label != null ? String(tl.label) : "";
    editFieldTimelineLayer = tl.layer != null ? (String(tl.layer).toLowerCase() as "dates" | "lifelines") : "dates";
  } else {
    editFieldTimelineEnabled = false;
    editFieldTimelineRole = "point";
    editFieldTimelineGroup = "";
    editFieldTimelineLabel = "";
    editFieldTimelineLayer = "dates";
  }
}

function addCustomField() {
  const label = newFieldLabel.trim();
  if (!label) return;
  const key = ensureFieldKey(label);
  if (packageFields.some((field) => field.key === key)) return;
  if ((draft.customFields ?? []).some((field) => field.key === key)) return;

  const base: any = {
    key,
    label,
    type: newFieldType,
    entityTypes: newFieldEntityTypes.length ? [...newFieldEntityTypes].sort() : undefined,
  };

  if (newFieldType === "enum") {
    const options = parseOptions(newFieldOptions);
    if (options.length === 0) return;
    base.options = options;
    if (newFieldMultiple) base.multiple = true;
  } else if (newFieldType === "oneof") {
    if (newFieldOneOfVariants.length === 0) return;
    const oneOf = newFieldOneOfVariants
      .filter((v) => v.label.trim())
      .map((v) => {
        const variant: any = { label: v.label.trim(), type: v.type };
        if (v.type === "enum") {
          const opts = parseOptions(v.options);
          if (opts.length === 0) return null;
          variant.options = opts;
        }
        return variant;
      })
      .filter(Boolean);
    if (oneOf.length === 0) return;
    // Validate enum variants have options
    for (const v of oneOf) if (v.type === "enum" && (!v.options || v.options.length === 0)) return;
    // oneof and relationship not allowed as variant type already enforced by UI
    base.oneOf = oneOf;
  } else if (newFieldType === "relationship") {
    const relType = ensureFieldKey(newFieldRelationshipType.trim() || label);
    if (!relType || newFieldTargetEntityTypes.length === 0) return;
    base.relationshipType = relType;
    base.targetEntityTypes = [...newFieldTargetEntityTypes].sort();
    base.cardinality = newFieldCardinality;
    if (newFieldMetadata.length > 0) {
      if (!validateMetadataDrafts(newFieldMetadata)) return;
      const defs = newFieldMetadata.map(metadataDraftToDefinition).filter(Boolean) as Record<string, unknown>[];
      if (defs.length !== newFieldMetadata.length) return;
      base.metadataFields = defs;
    }
  } else if (newFieldType === "date") {
    if (newFieldShared) {
      base.shared = true;
      if (newFieldTimelineEnabled) {
        if ((newFieldTimelineRole === "start" || newFieldTimelineRole === "end") && !newFieldTimelineGroup.trim())
          return;
        const tl: Record<string, unknown> = { role: newFieldTimelineRole, layer: newFieldTimelineLayer };
        if (newFieldTimelineGroup.trim()) tl.group = newFieldTimelineGroup.trim();
        else if (newFieldTimelineRole !== "point") return;
        if (newFieldTimelineLabel.trim()) tl.label = newFieldTimelineLabel.trim();
        base.timeline = tl as unknown as FieldDefinition["timeline"];
      }
    }
  }

  const field: FieldDefinition = base as FieldDefinition;
  setDraft({ ...draft, customFields: [...(draft.customFields ?? []), field] });
  newFieldLabel = "";
  newFieldEntityTypes = [];
  newFieldType = "text";
  newFieldOptions = "";
  newFieldMultiple = false;
  newFieldTargetEntityTypes = [];
  newFieldRelationshipType = "";
  newFieldCardinality = "many";
  newFieldOneOfVariants = [];
  newFieldMetadata = [];
  newFieldShared = false;
  newFieldTimelineEnabled = false;
  newFieldTimelineRole = "point";
  newFieldTimelineGroup = "";
  newFieldTimelineLabel = "";
  newFieldTimelineLayer = "dates";
}

function commitFieldEdit() {
  if (!editingFieldKey) return;
  const key = editingFieldKey;
  const label = editFieldLabel.trim();
  if (!label) return;

  // Validate per type
  let extra: Record<string, unknown> = {};
  if (editFieldType === "enum") {
    const options = parseOptions(editFieldOptions);
    if (options.length === 0) return;
    extra.options = options;
    extra.multiple = editFieldMultiple ? true : undefined;
    extra.targetEntityTypes = undefined;
    extra.relationshipType = undefined;
    extra.cardinality = undefined;
    extra.oneOf = undefined;
  } else if (editFieldType === "oneof") {
    if (editFieldOneOfVariants.length === 0) return;
    const oneOf = editFieldOneOfVariants
      .filter((v) => v.label.trim())
      .map((v) => {
        const variant: any = { label: v.label.trim(), type: v.type };
        if (v.type === "enum") {
          const opts = parseOptions(v.options);
          if (opts.length === 0) return null;
          variant.options = opts;
        }
        return variant;
      })
      .filter(Boolean);
    if (oneOf.length === 0) return;
    for (const v of oneOf as any[]) if (v.type === "enum" && (!v.options || v.options.length === 0)) return;
    extra.oneOf = oneOf;
    extra.options = undefined;
    extra.multiple = undefined;
    extra.targetEntityTypes = undefined;
    extra.relationshipType = undefined;
    extra.cardinality = undefined;
  } else if (editFieldType === "relationship") {
    const relType = ensureFieldKey(editFieldRelationshipType.trim() || label);
    if (!relType || editFieldTargetEntityTypes.length === 0) return;
    if (!validateMetadataDrafts(editFieldMetadata)) return;
    const defs = editFieldMetadata.map(metadataDraftToDefinition).filter(Boolean) as Record<string, unknown>[];
    if (defs.length !== editFieldMetadata.length) return;
    extra.relationshipType = relType;
    extra.targetEntityTypes = [...editFieldTargetEntityTypes].sort();
    extra.cardinality = editFieldCardinality;
    extra.metadataFields = defs.length ? defs : undefined;
    extra.options = undefined;
    extra.multiple = undefined;
    extra.oneOf = undefined;
    extra.shared = undefined;
    extra.timeline = undefined;
  } else if (editFieldType === "date") {
    extra.options = undefined;
    extra.multiple = undefined;
    extra.targetEntityTypes = undefined;
    extra.relationshipType = undefined;
    extra.cardinality = undefined;
    extra.oneOf = undefined;
    extra.metadataFields = undefined;
    if (editFieldShared) {
      extra.shared = true;
      if (editFieldTimelineEnabled) {
        if ((editFieldTimelineRole === "start" || editFieldTimelineRole === "end") && !editFieldTimelineGroup.trim())
          return;
        const tl: Record<string, unknown> = { role: editFieldTimelineRole, layer: editFieldTimelineLayer };
        if (editFieldTimelineGroup.trim()) tl.group = editFieldTimelineGroup.trim();
        else if (editFieldTimelineRole !== "point") return;
        if (editFieldTimelineLabel.trim()) tl.label = editFieldTimelineLabel.trim();
        extra.timeline = tl;
      } else {
        extra.timeline = undefined;
      }
    } else {
      extra.shared = undefined;
      extra.timeline = undefined;
    }
  } else {
    extra.options = undefined;
    extra.multiple = undefined;
    extra.targetEntityTypes = undefined;
    extra.relationshipType = undefined;
    extra.cardinality = undefined;
    extra.oneOf = undefined;
    extra.metadataFields = undefined;
    extra.shared = undefined;
    extra.timeline = undefined;
  }

  const nextCustomFields = (draft.customFields ?? []).map((field) => {
    if (field.key !== key) return field;
    const next: any = {
      ...field,
      label,
      type: editFieldType,
      entityTypes: editFieldEntityTypes.length ? [...editFieldEntityTypes].sort() : undefined,
    };
    // Clear previous type-specific keys then apply new
    delete next.options;
    delete next.multiple;
    delete next.targetEntityTypes;
    delete next.relationshipType;
    delete next.cardinality;
    delete next.oneOf;
    delete next.metadataFields;
    delete next.shared;
    delete next.timeline;
    Object.assign(next, extra);
    // Remove undefined
    for (const k of Object.keys(next)) if (next[k] === undefined) delete next[k];
    return next;
  });
  // Prune any stale fieldTimelineOverrides entry for this custom field — timeline is now owned by customFields.
  const existingOverridesForEdit =
    ((draft as unknown as Record<string, unknown>).fieldTimelineOverrides as
      Array<{ fieldKey: string; timeline: unknown }> | undefined) ?? [];
  const nextOverridesForEdit = existingOverridesForEdit.filter((ov) => ov.fieldKey !== key);
  {
    const nextDraft: Record<string, unknown> = {
      ...(draft as unknown as Record<string, unknown>),
      customFields: nextCustomFields,
      fieldTimelineOverrides: nextOverridesForEdit.length ? nextOverridesForEdit : undefined,
    };
    if (!nextOverridesForEdit.length) delete nextDraft.fieldTimelineOverrides;
    setDraft(nextDraft as unknown as ModuleSchemaOverlay);
  }
  cancelFieldEdit();
}

function removeCustomField(key: string) {
  if (editingFieldKey === key) cancelFieldEdit();
  const nextOverrides = (
    ((draft as unknown as Record<string, unknown>).fieldTimelineOverrides as
      Array<{ fieldKey: string; timeline: unknown }> | undefined) ?? []
  ).filter((ov) => ov.fieldKey !== key);
  const nextDraft: Record<string, unknown> = {
    ...(draft as unknown as Record<string, unknown>),
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
    fieldTimelineOverrides: nextOverrides.length ? nextOverrides : undefined,
  };
  if (!nextOverrides.length) delete nextDraft.fieldTimelineOverrides;
  setDraft(nextDraft as unknown as ModuleSchemaOverlay);
}

function addNewFieldOneOfVariant() {
  newFieldOneOfVariants = [...newFieldOneOfVariants, { label: "", type: "text" as FieldType, options: "" }];
}
function removeNewFieldOneOfVariant(index: number) {
  newFieldOneOfVariants = newFieldOneOfVariants.filter((_, i) => i !== index);
}
function addEditFieldOneOfVariant() {
  editFieldOneOfVariants = [...editFieldOneOfVariants, { label: "", type: "text" as FieldType, options: "" }];
}
function removeEditFieldOneOfVariant(index: number) {
  editFieldOneOfVariants = editFieldOneOfVariants.filter((_, i) => i !== index);
}

function addNewFieldMetadata() {
  newFieldMetadata = [
    ...newFieldMetadata,
    { key: "", label: "", type: "text", required: false, options: "", oneOf: [] },
  ];
}
function removeNewFieldMetadata(index: number) {
  newFieldMetadata = newFieldMetadata.filter((_, i) => i !== index);
}
function addNewFieldMetadataOneOfVariant(metaIndex: number) {
  const copy = newFieldMetadata.map((m, i) =>
    i === metaIndex ? { ...m, oneOf: [...m.oneOf, { label: "", type: "text" as const, options: "" }] } : m,
  );
  newFieldMetadata = copy;
}
function removeNewFieldMetadataOneOfVariant(metaIndex: number, variantIndex: number) {
  const copy = newFieldMetadata.map((m, i) =>
    i === metaIndex ? { ...m, oneOf: m.oneOf.filter((_, j) => j !== variantIndex) } : m,
  );
  newFieldMetadata = copy;
}
function addEditFieldMetadata() {
  editFieldMetadata = [
    ...editFieldMetadata,
    { key: "", label: "", type: "text", required: false, options: "", oneOf: [] },
  ];
}
function removeEditFieldMetadata(index: number) {
  editFieldMetadata = editFieldMetadata.filter((_, i) => i !== index);
}
function addEditFieldMetadataOneOfVariant(metaIndex: number) {
  const copy = editFieldMetadata.map((m, i) =>
    i === metaIndex ? { ...m, oneOf: [...m.oneOf, { label: "", type: "text" as const, options: "" }] } : m,
  );
  editFieldMetadata = copy;
}
function removeEditFieldMetadataOneOfVariant(metaIndex: number, variantIndex: number) {
  const copy = editFieldMetadata.map((m, i) =>
    i === metaIndex ? { ...m, oneOf: m.oneOf.filter((_, j) => j !== variantIndex) } : m,
  );
  editFieldMetadata = copy;
}

function builtinOriginalMetadata(field: FieldDefinition): MetadataFieldDefinition[] {
  const builtin = packageFields.find((f) => f.key === field.key);
  return (
    ((builtin as unknown as Record<string, unknown>)?.metadataFields as MetadataFieldDefinition[] | undefined) ?? []
  );
}

function effectiveBuiltinMetadata(field: FieldDefinition): MetadataFieldDefinition[] {
  const original = builtinOriginalMetadata(field);
  const override = (draft.fieldMetadataOverrides ?? []).find((ov) => ov.fieldKey === field.key);
  if (!override) return original as unknown as MetadataFieldDefinition[];
  const map = new Map<string, MetadataFieldDefinition>();
  for (const mf of original as unknown as MetadataFieldDefinition[])
    map.set((mf as unknown as Record<string, unknown>).key as string, mf as unknown as MetadataFieldDefinition);
  for (const mf of override.metadataFields as unknown as MetadataFieldDefinition[])
    map.set((mf as unknown as Record<string, unknown>).key as string, mf as unknown as MetadataFieldDefinition);
  return [...map.values()].sort((a, b) =>
    String((a as unknown as Record<string, unknown>).key).localeCompare(
      String((b as unknown as Record<string, unknown>).key),
    ),
  );
}

function builtinMetadataFieldExtras(field: FieldDefinition): string {
  const eff = effectiveBuiltinMetadata(field);
  if (eff.length === 0) return "";
  return `${eff.length} attribute${eff.length === 1 ? "" : "s"}`;
}

function startBuiltinMetadataEdit(field: FieldDefinition) {
  if (field.type !== "relationship") return;
  editingBuiltinFieldKey = null;
  editingFieldKey = null;
  editingBuiltinMetadataFieldKey = field.key;
  selectedItemId = `field:${field.key}`;
  const eff = effectiveBuiltinMetadata(field);
  editBuiltinMetadataDrafts = eff.map((mf) => {
    const r = mf as unknown as Record<string, unknown>;
    return {
      key: String(r.key ?? ""),
      label: String(r.label ?? ""),
      type: (METADATA_FIELD_TYPES as readonly string[]).includes(String(r.type))
        ? (r.type as MetadataFieldType)
        : "text",
      required: Boolean(r.required),
      options: formatOptions(r.options as string[] | undefined),
      oneOf: Array.isArray(r.oneOf)
        ? (r.oneOf as Array<Record<string, unknown>>).map((v) => {
            const vt = String(v.type ?? "");
            const allowed = ["text", "number", "boolean", "date", "enum"] as const as readonly string[];
            return {
              label: String(v.label ?? ""),
              type: (allowed.includes(vt) ? vt : "text") as Exclude<MetadataFieldType, "oneof" | "relationship">,
              options: formatOptions(v.options as string[] | undefined),
            };
          })
        : [],
    };
  });
}

function cancelBuiltinMetadataEdit() {
  editingBuiltinMetadataFieldKey = null;
  editBuiltinMetadataDrafts = [];
  selectedItemId = null;
}

function commitBuiltinMetadataEdit() {
  const fieldKey = editingBuiltinMetadataFieldKey;
  if (!fieldKey) return;
  const field = packageFields.find((f) => f.key === fieldKey);
  if (!field || field.type !== "relationship") return;
  if (editBuiltinMetadataDrafts.some((d) => !d.label.trim())) return;
  if (editBuiltinMetadataDrafts.length > 0 && !validateMetadataDrafts(editBuiltinMetadataDrafts)) return;
  // Reject duplicate keys (including a new draft colliding with an existing builtin
  // metadata key), which would otherwise fail server-side as a conflicting field type.
  const seenKeys = new Set<string>();
  for (const d of editBuiltinMetadataDrafts) {
    const k = ensureFieldKey(d.key.trim() || d.label.trim(), "field");
    if (!k || seenKeys.has(k)) return;
    seenKeys.add(k);
  }
  const effDefs = editBuiltinMetadataDrafts
    .map(metadataDraftToDefinition)
    .filter(Boolean) as unknown as MetadataFieldDefinition[];
  const original = builtinOriginalMetadata(field) as unknown as MetadataFieldDefinition[];
  const originalMap = new Map<string, string>();
  for (const mf of original)
    originalMap.set(String((mf as unknown as Record<string, unknown>).key), JSON.stringify(mf));
  const effMap = new Map<string, string>();
  for (const mf of effDefs) effMap.set(String((mf as unknown as Record<string, unknown>).key), JSON.stringify(mf));
  // delta = effDefs that are new or changed vs original
  const delta: MetadataFieldDefinition[] = [];
  for (const mf of effDefs) {
    const k = String((mf as unknown as Record<string, unknown>).key);
    const origJson = originalMap.get(k);
    const effJson = effMap.get(k)!;
    if (origJson !== effJson) delta.push(mf);
  }
  // If delta empty and effective equals original (no change), remove override
  let nextOverrides = [...(draft.fieldMetadataOverrides ?? [])].filter((ov) => ov.fieldKey !== fieldKey);
  if (delta.length > 0) {
    // store delta sorted; validation will ensure conflicting type not present (we already check), but we must keep delta sorted
    delta.sort((a, b) =>
      String((a as unknown as Record<string, unknown>).key).localeCompare(
        String((b as unknown as Record<string, unknown>).key),
      ),
    );
    nextOverrides.push({ fieldKey, metadataFields: delta as unknown as FieldMetadataOverride["metadataFields"] });
    nextOverrides.sort((a, b) => a.fieldKey.localeCompare(b.fieldKey));
  }
  setDraft({ ...draft, fieldMetadataOverrides: nextOverrides });
  cancelBuiltinMetadataEdit();
}

function addEditBuiltinMetadata() {
  editBuiltinMetadataDrafts = [
    ...editBuiltinMetadataDrafts,
    { key: "", label: "", type: "text", required: false, options: "", oneOf: [] },
  ];
}
function removeEditBuiltinMetadata(index: number) {
  const fieldKey = editingBuiltinMetadataFieldKey;
  const field = fieldKey ? packageFields.find((f) => f.key === fieldKey) : null;
  const originalKeys = new Set(
    (builtinOriginalMetadata(field as FieldDefinition) as unknown as MetadataFieldDefinition[]).map((mf) =>
      String((mf as unknown as Record<string, unknown>).key),
    ),
  );
  const draftKey = editBuiltinMetadataDrafts[index]?.key;
  if (draftKey && originalKeys.has(draftKey)) return; // prevent removing builtin keys (additive)
  editBuiltinMetadataDrafts = editBuiltinMetadataDrafts.filter((_, i) => i !== index);
}
function addEditBuiltinMetadataOneOfVariant(metaIndex: number) {
  const copy = editBuiltinMetadataDrafts.map((m, i) =>
    i === metaIndex ? { ...m, oneOf: [...m.oneOf, { label: "", type: "text" as const, options: "" }] } : m,
  );
  editBuiltinMetadataDrafts = copy;
}
function removeEditBuiltinMetadataOneOfVariant(metaIndex: number, variantIndex: number) {
  const copy = editBuiltinMetadataDrafts.map((m, i) =>
    i === metaIndex ? { ...m, oneOf: m.oneOf.filter((_, j) => j !== variantIndex) } : m,
  );
  editBuiltinMetadataDrafts = copy;
}

function canAddField(): boolean {
  if (!newFieldLabel.trim()) return false;
  if (newFieldType === "enum") return parseOptions(newFieldOptions).length > 0;
  if (newFieldType === "oneof")
    return newFieldOneOfVariants.some(
      (v) => v.label.trim() && (v.type !== "enum" || parseOptions(v.options).length > 0),
    );
  if (newFieldType === "relationship") {
    if (!Boolean(ensureTypeId(newFieldRelationshipType.trim() || newFieldLabel.trim(), "relationship"))) return false;
    if (newFieldTargetEntityTypes.length === 0) return false;
    if (newFieldMetadata.length > 0 && !validateMetadataDrafts(newFieldMetadata)) return false;
    // enforce metadata keys validity even when empty options etc. Already validated; if any draft invalid block
    if (newFieldMetadata.some((d) => !d.label.trim())) return false;
    return true;
  }
  if (newFieldType === "date" && newFieldShared && newFieldTimelineEnabled) {
    if ((newFieldTimelineRole === "start" || newFieldTimelineRole === "end") && !newFieldTimelineGroup.trim())
      return false;
  }
  return true;
}

function canSaveFieldEdit(): boolean {
  if (!editingFieldKey || !editFieldLabel.trim()) return false;
  if (editFieldType === "enum") return parseOptions(editFieldOptions).length > 0;
  if (editFieldType === "oneof")
    return editFieldOneOfVariants.some(
      (v) => v.label.trim() && (v.type !== "enum" || parseOptions(v.options).length > 0),
    );
  if (editFieldType === "relationship") {
    if (!Boolean(ensureTypeId(editFieldRelationshipType.trim() || editFieldLabel.trim(), "relationship"))) return false;
    if (editFieldTargetEntityTypes.length === 0) return false;
    if (editFieldMetadata.length > 0 && !validateMetadataDrafts(editFieldMetadata)) return false;
    if (editFieldMetadata.some((d) => !d.label.trim())) return false;
    return true;
  }
  if (editFieldType === "date" && editFieldShared && editFieldTimelineEnabled) {
    if ((editFieldTimelineRole === "start" || editFieldTimelineRole === "end") && !editFieldTimelineGroup.trim())
      return false;
  }
  return true;
}

function fieldExtrasLabel(field: FieldDefinition): string {
  const f: any = field;
  if (field.type === "enum") {
    const opts = (f.options as string[] | undefined) ?? [];
    return `${opts.length} options${f.multiple ? " · multiple" : ""}`;
  }
  if (field.type === "oneof") {
    const variants = (f.oneOf as any[] | undefined) ?? [];
    return `${variants.length} variants`;
  }

  if (field.type === "relationship") {
    const rel = f.relationshipType ? humanizeId(f.relationshipType) : "Relationship";
    const targets = (f.targetEntityTypes as string[] | undefined) ?? [];
    const card = f.cardinality ?? "many";
    const meta = (f.metadataFields as unknown[] | undefined)?.length ?? 0;
    const metaSuffix = meta ? ` · ${meta} attribute${meta === 1 ? "" : "s"}` : "";
    return `${rel} → ${targets.map(entityTypeLabel).join(", ") || "any"} · ${card}${metaSuffix}`;
  }
  return "";
}

function cancelTemplateEdit() {
  editingTemplateId = null;
  editTemplateName = "";
  editTemplateEntityType = "";
  editTemplateDescription = "";
}

function startTemplateEdit(template: EntityTemplate) {
  beginTemplateFieldEdit(template, false);
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
    id = ensureTypeId(`${name}-${localTypeId(entityType)}`, "template");
  }
  if (taken(id)) {
    let suffix = 2;
    while (taken(`${id}-${suffix}`)) suffix += 1;
    id = `${id}-${suffix}`;
  }
  const description = newTemplateDescription.trim();
  const availableFields = effectiveFieldsForType(entityType);
  const fields: Record<string, unknown> = {};
  for (const key of newTemplateFieldKeys) {
    const field = availableFields.find((f) => f.key === key);
    if (!field) continue;
    fields[key] = key in newTemplateFieldValues ? newTemplateFieldValues[key] : defaultFieldValue(field);
  }
  const finalFields = fields;
  const finalRequired = newTemplateRequiredFields.filter((k) => k in finalFields);
  const template: EntityTemplate = {
    id,
    name,
    entityType,
    description: description || null,
    fields: finalFields,
    requiredFields: finalRequired,
    ...(newTemplateIncludeDocument ? { document: "" } : {}),
  };
  setDraft({ ...draft, customTemplates: [...(draft.customTemplates ?? []), template] });
  newTemplateName = "";
  newTemplateEntityType = "";
  newTemplateDescription = "";
  newTemplateFieldKeys = [];
  newTemplateRequiredFields = [];
  newTemplateFieldValues = {};
  newTemplateIncludeDocument = false;
}

function commitTemplateEdit() {
  if (!editingTemplateId) return;
  const id = editingTemplateId;
  const name = editTemplateName.trim();
  const entityType = editTemplateEntityType.trim();
  if (!name || !entityType || !effectiveTypes().includes(entityType)) return;
  const description = editTemplateDescription.trim();
  const availableFields = effectiveFieldsForType(entityType);
  const fields: Record<string, unknown> = {};
  for (const key of editingTemplateFieldKeys) {
    if (!availableFields.some((f) => f.key === key)) continue;
    const field = availableFields.find((f) => f.key === key)!;
    fields[key] = key in editingTemplateFieldValues ? editingTemplateFieldValues[key] : defaultFieldValue(field);
  }
  const finalRequired = editingTemplateRequiredFields.filter((k) => k in fields);
  setDraft({
    ...draft,
    customTemplates: (draft.customTemplates ?? []).map((template) => {
      if (template.id !== id) return template;
      return {
        ...template,
        name,
        entityType,
        description: description || null,
        fields,
        requiredFields: finalRequired,
        document: editTemplateIncludeDocument ? (template.document ?? "") : undefined,
      };
    }),
  });
  cancelTemplateFieldEdit();
}

function commitBuiltinTemplateEdit() {
  if (!editingBuiltinTemplateId) return;
  const template = packageTemplates.find((candidate) => candidate.id === editingBuiltinTemplateId);
  if (!template) return;
  const fields = templateFieldsFromSelection(template);
  const templateOverrides = (draft.templateOverrides ?? []).filter((candidate) => candidate.templateId !== template.id);
  templateOverrides.push({
    templateId: template.id,
    fields,
    requiredFields: editingTemplateRequiredFields.filter((key) => key in fields),
  });
  setDraft({ ...draft, templateOverrides });
  cancelTemplateFieldEdit();
}

function removeCustomTemplate(id: string) {
  if (editingTemplateId === id) cancelTemplateEdit();
  setDraft({
    ...draft,
    customTemplates: (draft.customTemplates ?? []).filter((template) => template.id !== id),
  });
}
</script>

<section class="module-schema-panel" class:detail-selected={selectedItemId !== null}>
  <header class="panel-hero">
    <div class="hero-icon">
      <SlidersHorizontal size={18} strokeWidth={1.8} aria-hidden="true" />
    </div>
    <div class="hero-copy">
      <span class="kicker">PROJECT STRUCTURE</span>
      <strong>Fields &amp; Types</strong>
      <p>
        Choose what authors see in this project. Extension defaults stay intact while project-specific types, fields,
        and templates travel with the project.
      </p>
    </div>
    <div class="hero-stats" aria-label="Overlay summary">
      <span class="stat-pill"
        ><Layers size={12} strokeWidth={1.8} aria-hidden="true" /> {effectiveTypes().length} types</span>
      <span class="stat-pill"
        ><TextQuote size={12} strokeWidth={1.8} aria-hidden="true" />
        {(draft.customFields ?? []).length + packageFields.length - (draft.disabledFields?.length ?? 0)} fields</span>
      <span class="stat-pill"
        ><LayoutTemplate size={12} strokeWidth={1.8} aria-hidden="true" />
        {(draft.customTemplates ?? []).length + packageTemplates.length - (draft.disabledTemplates?.length ?? 0)} templates</span>
    </div>
  </header>

  {#if !projectOpen}
    <div class="empty-card">
      <div class="empty-icon"><Blocks size={20} strokeWidth={1.7} aria-hidden="true" /></div>
      <strong>Open a project to customize Fields &amp; Types</strong>
      <p>Project structure choices are saved inside the project and travel with it.</p>
    </div>
  {:else}
    <div class="tab-bar" role="tablist" aria-label="Fields and Types sections">
      <button
        type="button"
        role="tab"
        class="tab"
        class:active={activeTab === "types"}
        aria-selected={activeTab === "types"}
        onclick={() => {
          closeSelectedItem();
          activeTab = "types";
        }}>
        <Layers size={14} strokeWidth={1.8} aria-hidden="true" />
        Types
        <span class="tab-count">{effectiveTypes().length}</span>
      </button>
      <button
        type="button"
        role="tab"
        class="tab"
        class:active={activeTab === "fields"}
        aria-selected={activeTab === "fields"}
        onclick={() => {
          closeSelectedItem();
          activeTab = "fields";
        }}>
        <TextQuote size={14} strokeWidth={1.8} aria-hidden="true" />
        Fields
        <span class="tab-count"
          >{[...packageFields, ...(draft.customFields ?? [])].filter((f) => !isDisabled(draft.disabledFields, f.key))
            .length}</span>
      </button>
      <button
        type="button"
        role="tab"
        class="tab"
        class:active={activeTab === "templates"}
        aria-selected={activeTab === "templates"}
        onclick={() => {
          closeSelectedItem();
          activeTab = "templates";
        }}>
        <LayoutTemplate size={14} strokeWidth={1.8} aria-hidden="true" />
        Templates
        <span class="tab-count"
          >{[...packageTemplates, ...(draft.customTemplates ?? [])].filter(
            (t) => !isDisabled(draft.disabledTemplates, t.id),
          ).length}</span>
      </button>
    </div>

    <div class="workbench-toolbar" aria-label="Schema workbench filters">
      {#if selectedItemId}
        <button type="button" class="quiet narrow-back" onclick={closeSelectedItem}>← Back to list</button>
      {/if}
      <label class="workbench-search">
        <span>Search</span>
        <input type="search" bind:value={listQuery} placeholder={`Search ${activeTab}…`} />
      </label>
      <label class="workbench-status">
        <span>Status</span>
        <select bind:value={statusFilter}>
          <option value="all">All</option>
          <option value="enabled">Enabled</option>
          <option value="disabled">Disabled</option>
          <option value="custom">Custom</option>
          <option value="builtin">Builtin</option>
        </select>
      </label>
      <label class="advanced-toggle">
        <input type="checkbox" bind:checked={showAdvanced} />
        <span>Advanced</span>
      </label>
    </div>

    {#if activeTab === "types"}
      <SchemaTypesPane
        {draft}
        {flatPackage}
        {packageTypeDefinitions}
        {pluginId}
        {showAdvanced}
        bind:selectedItemId
        bind:builtinTypesCollapsed
        bind:newType
        bind:newTypeIcon
        bind:newTypeColor
        bind:newTypeFieldKeys
        bind:editingTypeId
        bind:editTypeValue
        bind:editTypeIcon
        bind:editTypeColor
        bind:editTypeFieldKeys
        {typeMatches}
        {isDisabled}
        {effectivePackageAppearance}
        {packageAppearanceChanged}
        {clearPackageAppearanceOverride}
        {setPackageAppearanceOverride}
        {toggleDisabled}
        {selectableFieldOptions}
        {effectiveFieldsForType}
        {effectiveTemplates}
        {projectionLabelsForType}
        {entityCountForType}
        {commitTypeEdit}
        {cancelTypeEdit}
        {startTypeEdit}
        {requestRemoveCustomType}
        {addCustomType} />
    {:else if activeTab === "fields"}
      <SchemaFieldsPane
        {draft}
        {flatPackage}
        {packageFields}
        {showAdvanced}
        bind:selectedItemId
        bind:builtinFieldsCollapsed
        bind:editingBuiltinFieldKey
        bind:editingBuiltinMetadataFieldKey
        bind:editBuiltinMetadataDrafts
        bind:editingTimelineFieldKey
        bind:editTimelineRole
        bind:editTimelineGroup
        bind:editTimelineLabel
        bind:editTimelineLayer
        bind:editingFieldKey
        bind:editFieldLabel
        bind:editFieldType
        bind:editFieldEntityTypes
        bind:editFieldOptions
        bind:editFieldMultiple
        bind:editFieldTargetEntityTypes
        bind:editFieldRelationshipType
        bind:editFieldCardinality
        bind:editFieldOneOfVariants
        bind:editFieldMetadata
        bind:editFieldShared
        bind:editFieldTimelineEnabled
        bind:editFieldTimelineRole
        bind:editFieldTimelineGroup
        bind:editFieldTimelineLabel
        bind:editFieldTimelineLayer
        bind:newFieldLabel
        bind:newFieldType
        bind:newFieldEntityTypes
        bind:newFieldOptions
        bind:newFieldMultiple
        bind:newFieldTargetEntityTypes
        bind:newFieldRelationshipType
        bind:newFieldCardinality
        bind:newFieldOneOfVariants
        bind:newFieldMetadata
        bind:newFieldShared
        bind:newFieldTimelineEnabled
        bind:newFieldTimelineRole
        bind:newFieldTimelineGroup
        bind:newFieldTimelineLabel
        bind:newFieldTimelineLayer
        {fieldMatches}
        {isDisabled}
        {toggleDisabled}
        {fieldScopeTypes}
        {builtinFieldScope}
        {updateBuiltinFieldScope}
        {toggleInList}
        {entityTypeLabel}
        {builtinOriginalMetadata}
        {effectiveBuiltinMetadata}
        {builtinMetadataFieldExtras}
        {removeEditBuiltinMetadata}
        {removeEditBuiltinMetadataOneOfVariant}
        {addEditBuiltinMetadataOneOfVariant}
        {addEditBuiltinMetadata}
        {commitBuiltinMetadataEdit}
        {cancelBuiltinMetadataEdit}
        {commitTimelineEdit}
        {cancelTimelineEdit}
        {isTimelineEnabled}
        {timelineBadge}
        {scopeLabel}
        {startBuiltinMetadataEdit}
        {startTimelineEdit}
        {disableTimeline}
        {removeEditFieldOneOfVariant}
        {addEditFieldOneOfVariant}
        {removeEditFieldMetadata}
        {removeEditFieldMetadataOneOfVariant}
        {addEditFieldMetadataOneOfVariant}
        {addEditFieldMetadata}
        {canSaveFieldEdit}
        {commitFieldEdit}
        {cancelFieldEdit}
        {fieldExtrasLabel}
        {startFieldEdit}
        {removeCustomField}
        {canAddField}
        {addCustomField}
        {removeNewFieldOneOfVariant}
        {addNewFieldOneOfVariant}
        {removeNewFieldMetadata}
        {removeNewFieldMetadataOneOfVariant}
        {addNewFieldMetadataOneOfVariant}
        {addNewFieldMetadata} />
    {:else}
      <SchemaTemplatesPane
        {draft}
        {packageTemplates}
        {showAdvanced}
        bind:selectedItemId
        bind:builtinTemplatesCollapsed
        bind:editingBuiltinTemplateId
        bind:editingTemplateId
        bind:editingTemplateFieldKeys
        bind:editingTemplateRequiredFields
        bind:editingTemplateFieldValues
        bind:editTemplateIncludeDocument
        bind:editTemplateName
        bind:editTemplateEntityType
        bind:editTemplateDescription
        bind:newTemplateName
        bind:newTemplateEntityType
        bind:newTemplateDescription
        bind:newTemplateFieldKeys
        bind:newTemplateRequiredFields
        bind:newTemplateFieldValues
        bind:newTemplateIncludeDocument
        {templateMatches}
        {isDisabled}
        {toggleDisabled}
        {effectiveFieldsForType}
        {editingPreviewFields}
        {fieldsForTemplate}
        {entityTypeLabel}
        {effectiveTypes}
        {toggleInList}
        {beginTemplateFieldEdit}
        {commitBuiltinTemplateEdit}
        {cancelTemplateFieldEdit}
        {startTemplateEdit}
        {commitTemplateEdit}
        {cancelTemplateEdit}
        {removeCustomTemplate}
        {addCustomTemplate} />
    {/if}

    <div class="save-bar" class:has-dirty={dirty} class:is-busy={busy} class:has-conflict={conflict}>
      <div class="save-copy">
        {#if conflict}
          <AlertTriangle size={13} strokeWidth={1.8} aria-hidden="true" />
          <strong>{MUTATION_STATUS_MESSAGES.conflictTitle}</strong>
          <span>{message || MUTATION_STATUS_MESSAGES.conflictBody}</span>
        {:else if previewError}
          <AlertTriangle size={13} strokeWidth={1.8} aria-hidden="true" />
          <strong>Preview failed</strong>
          <span>{previewError}</span>
        {:else if dirty}
          <span class="dirty-dot" aria-hidden="true"></span>
          <strong>Unsaved changes</strong>
          <span>Review types, fields, and templates before saving.</span>
        {:else if message}
          <span class="save-message" class:error={message.startsWith("Could")}>
            {#if message.startsWith("Could")}<AlertTriangle size={13} strokeWidth={1.8} aria-hidden="true" />{/if}
            {message}
          </span>
        {:else}
          <Check size={13} strokeWidth={2} aria-hidden="true" />
          <span>All changes saved to the project overlay.</span>
        {/if}
      </div>
      <div class="save-actions">
        {#if conflict}
          {#if onFetchCurrent}
            <button
              type="button"
              class="quiet"
              disabled={busy || conflictActionBusy}
              onclick={() => void compareWithCurrent()}>{MUTATION_STATUS_MESSAGES.conflictCompare}</button>
          {/if}
          {#if onReloadCurrent}
            <button
              type="button"
              class="quiet"
              disabled={busy || conflictActionBusy}
              onclick={() => void reloadCurrentOverlay()}>{MUTATION_STATUS_MESSAGES.conflictReload}</button>
          {/if}
          {#if onFetchCurrent && onAdoptCurrentRevision}
            <button
              type="button"
              class="quiet"
              disabled={busy || conflictActionBusy}
              onclick={() => void reapplyDraftOntoCurrent()}>{MUTATION_STATUS_MESSAGES.conflictReapply}</button>
          {/if}
        {/if}
        <button type="button" class="quiet" disabled={busy || !dirty} onclick={() => void discardChanges()}
          ><X size={14} strokeWidth={1.8} aria-hidden="true" /> Discard</button>
        <button type="button" class="primary save-button" disabled={busy || !dirty} onclick={() => void save()}>
          {#if busy}<span class="spinner" aria-hidden="true"></span> Saving…{:else}<Save
              size={14}
              strokeWidth={2}
              aria-hidden="true" /> Save schema{/if}
        </button>
      </div>
    </div>
    {#if conflictCompare}
      <details class="schema-conflict-compare" open>
        <summary>Compare current vs draft</summary>
        <div class="compare-grid">
          <div>
            <strong>Current</strong>
            <p class="compare-meta">{conflictCompare.currentRevision || "unknown revision"}</p>
            <p>{overlaySummary(conflictCompare.current)}</p>
            <pre>{JSON.stringify(conflictCompare.current, null, 2)}</pre>
          </div>
          <div>
            <strong>Draft</strong>
            <p class="compare-meta">{contentRevision || "pending revision"}</p>
            <p>{overlaySummary(conflictCompare.draft)}</p>
            <pre>{JSON.stringify(conflictCompare.draft, null, 2)}</pre>
          </div>
        </div>
      </details>
    {/if}
  {/if}

  {#if impactPreview}
    <SchemaImpactReview
      preview={impactPreview}
      {busy}
      onCancel={cancelImpactReview}
      onConfirm={() => void confirmImpactSave()} />
  {/if}

  {#if typeRemovalPrompt}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="type-remove-backdrop"
      role="presentation"
      tabindex="-1"
      onclick={() => cancelTypeRemoval()}
      onkeydown={(e) => e.key === "Escape" && cancelTypeRemoval()}>
      <!-- svelte-ignore a11y_autofocus -->
      <div
        class="type-remove-dialog"
        role="alertdialog"
        aria-modal="true"
        tabindex="-1"
        aria-labelledby="type-remove-title"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.key === "Escape" && cancelTypeRemoval()}>
        <div class="dialog-icon warn">
          <AlertTriangle size={18} strokeWidth={1.8} aria-hidden="true" />
        </div>
        <strong id="type-remove-title">Remove {entityTypeLabel(typeRemovalPrompt.typeId)}?</strong>
        <p>
          Templates for this type are removed with it. Choose what should happen to every field that only applies to
          this type before continuing.
        </p>
        {#if typeRemovalPrompt.templates.length > 0}
          <div class="type-remove-group">
            <span>Templates to remove</span>
            <ul>
              {#each typeRemovalPrompt.templates as template}
                <li><LayoutTemplate size={12} strokeWidth={1.7} aria-hidden="true" /> {template.name}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if typeRemovalPrompt.exclusiveFields.length > 0}
          <div class="type-remove-group">
            <span>Fields that only target this type</span>
            <ul>
              {#each typeRemovalPrompt.exclusiveFields as field}
                <li>
                  <span class="field-label"
                    ><TextQuote size={12} strokeWidth={1.7} aria-hidden="true" /> {field.label}</span>
                  <code>{field.key}</code>
                  <label class="type-remove-choice">
                    <span>Disposition</span>
                    <select
                      value={typeRemovalPrompt.exclusiveDispositions[field.key]?.action ?? ""}
                      onchange={(event) =>
                        updateExclusiveDisposition(
                          field.key,
                          event.currentTarget.value as "remove" | "disable" | "reassign",
                        )}>
                      <option value="" disabled>Choose…</option>
                      <option value="remove">Remove field</option>
                      <option value="disable">Disable field</option>
                      <option value="reassign">Reassign</option>
                    </select>
                  </label>
                  {#if typeRemovalPrompt.exclusiveDispositions[field.key]?.action === "reassign"}
                    <label class="type-remove-choice">
                      <span>New type</span>
                      <select
                        value={exclusiveReassignmentTarget(typeRemovalPrompt, field.key)}
                        onchange={(event) => updateExclusiveReassignment(field.key, event.currentTarget.value)}>
                        {#each effectiveTypes().filter((typeId) => typeId !== typeRemovalPrompt?.typeId) as typeId}
                          <option value={typeId}>{entityTypeLabel(typeId)}</option>
                        {/each}
                      </select>
                    </label>
                  {/if}
                </li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if typeRemovalPrompt.entityCount === null || (typeRemovalPrompt.entityCount ?? 0) > 0}
          <div class="type-remove-group">
            <span>Existing entities</span>
            <p class="type-remove-note">
              {#if typeRemovalPrompt.entityCount == null}
                Entity count is unavailable. Choose a destination type to reassign any entities that still use this
                type, or confirm there are none.
              {:else}
                {typeRemovalPrompt.entityCount}
                {typeRemovalPrompt.entityCount === 1 ? "entity uses" : "entities use"} this type and must be reassigned before
                removal.
              {/if}
            </p>
            {#if typeRemovalPrompt.entityCount == null}
              <label class="type-remove-choice">
                <span>Entities</span>
                <select
                  value={typeRemovalPrompt.entityDisposition?.action ?? ""}
                  disabled={typeRemovalPrompt.busy}
                  onchange={(event) => updateEntityDisposition(event.currentTarget.value as "none" | "reassign")}>
                  <option value="" disabled>Choose…</option>
                  <option value="reassign">Reassign to another type</option>
                  <option value="none">Confirm no entities use this type</option>
                </select>
              </label>
            {/if}
            {#if typeRemovalPrompt.entityDisposition?.action === "reassign" || (typeRemovalPrompt.entityCount ?? 0) > 0}
              <label class="type-remove-choice">
                <span>Reassign entities to</span>
                <select
                  value={typeRemovalPrompt.entityDisposition?.action === "reassign"
                    ? typeRemovalPrompt.entityDisposition.toTypeId
                    : ""}
                  disabled={typeRemovalPrompt.busy ||
                    effectiveTypes().filter((typeId) => typeId !== typeRemovalPrompt?.typeId).length === 0}
                  onchange={(event) => updateEntityReassignment(event.currentTarget.value)}>
                  {#each effectiveTypes().filter((typeId) => typeId !== typeRemovalPrompt?.typeId) as typeId}
                    <option value={typeId}>{entityTypeLabel(typeId)}</option>
                  {/each}
                </select>
              </label>
              {#if effectiveTypes().filter((typeId) => typeId !== typeRemovalPrompt?.typeId).length === 0}
                <p class="type-remove-note">
                  Create or enable another type first — entities cannot be left without a type.
                </p>
              {/if}
            {/if}
          </div>
        {:else}
          <div class="type-remove-group">
            <span>Existing entities</span>
            <p class="type-remove-note">No entities currently use this type.</p>
          </div>
        {/if}
        {#if typeRemovalPrompt.sharedFields.length > 0}
          <div class="type-remove-group">
            <span>Fields that also target other types</span>
            <ul>
              {#each typeRemovalPrompt.sharedFields as field}
                <li>
                  <span class="field-label">{field.label}</span>
                  <small>
                    also {(field.entityTypes ?? [])
                      .filter((type) => type !== typeRemovalPrompt?.typeId)
                      .map(entityTypeLabel)
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
        {#if typeRemovalPrompt.error}
          <p class="type-remove-note" role="alert">{typeRemovalPrompt.error}</p>
        {/if}
        <div class="type-remove-actions">
          <button type="button" class="quiet" disabled={typeRemovalPrompt.busy} onclick={cancelTypeRemoval}
            >Keep type</button>
          <button
            type="button"
            class="danger"
            disabled={typeRemovalPrompt.busy || !removalPlanComplete(typeRemovalPrompt)}
            onclick={() => void confirmTypeRemoval()}
            >{#if typeRemovalPrompt.busy}Reassigning…{:else}<Trash2 size={14} strokeWidth={1.8} aria-hidden="true" /> Remove{/if}</button>
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
.module-schema-panel {
  display: grid;
  gap: 18px;
}
.panel-hero {
  display: grid;
  grid-template-columns: 40px 1fr;
  gap: 14px;
  padding: 16px 16px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.hero-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.hero-copy .kicker {
  color: var(--accent);
  font:
    700 10px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.hero-copy strong {
  display: block;
  margin-top: 3px;
  color: var(--ink);
  font: 600 16px/1.15 var(--font-display, Georgia, serif);
}
.hero-copy p {
  margin: 6px 0 0;
  max-width: 640px;
  color: var(--ink-soft);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.hero-stats {
  grid-column: 2;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 2px;
}
.stat-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.empty-card {
  display: grid;
  gap: 10px;
  justify-items: start;
  padding: 22px 18px;
  border: 1px dashed var(--line-strong);
  border-radius: 14px;
  background: var(--surface-quiet);
}
.empty-card .empty-icon {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
}
.empty-card strong {
  color: var(--ink);
  font: 600 15px var(--font-display, Georgia, serif);
}
.empty-card p {
  margin: 0;
  max-width: 520px;
  color: var(--ink-soft);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.tab-bar {
  display: flex;
  gap: 6px;
  padding: 6px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
  overflow-x: auto;
}
.tab {
  flex: 1 1 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  min-height: 36px;
  padding: 7px 12px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: transparent;
  color: var(--ink-muted);
  font:
    600 13px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.14s ease;
}
.tab:hover {
  background: var(--theme-warning-bg, #efe8d9);
  color: var(--ink-muted);
}
.tab.active {
  background: var(--accent-dark);
  border-color: var(--accent-dark);
  color: var(--on-accent);
  box-shadow: 0 1px 0 rgba(48, 44, 38, 0.12);
}
.tab-count {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  padding: 0 6px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.14);
  color: inherit;
  font:
    700 11px Inter,
    sans-serif;
}
.tab.active .tab-count {
  background: rgba(255, 255, 255, 0.18);
}
.workbench-toolbar {
  display: grid;
  grid-template-columns: minmax(180px, 1fr) minmax(130px, 180px) auto;
  gap: 10px;
  align-items: end;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
}
.workbench-search,
.workbench-status {
  min-width: 0;
}
.advanced-toggle {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
  min-height: 34px;
  padding: 0 4px;
  cursor: pointer;
}
.advanced-toggle input {
  margin: 0;
  width: 16px;
  height: 16px;
  min-width: 0;
  min-height: 0;
  padding: 0;
  accent-color: var(--accent-dark);
  flex: 0 0 auto;
}
.quiet.narrow-back {
  display: none;
}

.quiet,
.danger,
.primary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 11px;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink-muted);
  font:
    600 11px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  cursor: pointer;
  transition: all 0.14s ease;
}
.quiet:hover {
  border-color: var(--theme-warning-border, #b7a88f);
  background: var(--surface-warm);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(48, 44, 38, 0.06);
}
.primary {
  border-color: var(--accent-dark);
  background: var(--accent-dark);
  color: var(--on-accent);
}
.primary:hover {
  background: #4a6b57;
}
.danger {
  border-color: var(--theme-danger-border, #e0b8ad);
  background: var(--danger-bg);
  color: var(--danger);
}
.danger:hover {
  border-color: var(--theme-danger-border, #c9897d);
  background: var(--theme-danger-bg, #f3ddd6);
}
.primary:disabled,
.quiet:disabled,
.danger:disabled {
  opacity: 0.45;
  cursor: default;
  transform: none;
  box-shadow: none;
}
label {
  display: grid;
  gap: 5px;
  min-width: 140px;
}
label span {
  color: var(--ink-muted);
  font:
    600 10px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
input:not([type="checkbox"]):not([type="radio"]):not([type="hidden"]),
select {
  box-sizing: border-box;
  min-width: 140px;
  height: 36px;
  padding: 0 11px;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: var(--theme-surface-bg, #fff);
  color: var(--ink);
  font:
    400 13px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
select {
  padding-right: 28px;
  appearance: none;
  -webkit-appearance: none;
  background-image:
    linear-gradient(45deg, transparent 50%, var(--ink-muted) 50%),
    linear-gradient(135deg, var(--ink-muted) 50%, transparent 50%);
  background-position:
    calc(100% - 16px) calc(50% - 2px),
    calc(100% - 11px) calc(50% - 2px);
  background-size:
    5px 5px,
    5px 5px;
  background-repeat: no-repeat;
}
input:not([type="checkbox"]):not([type="radio"]):not([type="hidden"]):focus,
select:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.12);
}
code {
  width: fit-content;
  padding: 2px 6px;
  border: 1px solid var(--line-soft);
  border-radius: 6px;
  background: var(--theme-warning-bg, #f1ebe1);
  color: var(--theme-neutral-text-soft, #6f675c);
  font:
    500 11px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
}

.save-bar {
  position: sticky;
  bottom: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  margin-top: 6px;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--surface) 96%, transparent);
  backdrop-filter: blur(10px);
  box-shadow: 0 8px 24px rgba(48, 44, 38, 0.08);
}
.save-bar.has-dirty {
  border-color: var(--danger-line);
  background: linear-gradient(var(--surface), var(--theme-danger-bg, #fff6f1));
}
.schema-conflict-compare {
  margin: 0 0 12px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
}
.schema-conflict-compare summary {
  cursor: pointer;
  font:
    600 12.5px Inter,
    sans-serif;
}
.compare-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
  margin-top: 10px;
}
.compare-grid pre {
  max-height: 14rem;
  overflow: auto;
  margin: 0.4rem 0 0;
  padding: 8px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--ink) 4%, var(--surface));
  font:
    400 11px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
}
.compare-meta {
  margin: 0.2rem 0;
  color: var(--ink-muted);
  font:
    400 11.5px Inter,
    sans-serif;
}
.save-copy {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  min-width: 0;
  color: var(--ink-muted);
  font:
    400 12.5px Inter,
    sans-serif;
}
.save-copy strong {
  color: var(--danger);
  font:
    700 12.5px Inter,
    sans-serif;
}
.save-copy .dirty-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: #c35a46;
  box-shadow: 0 0 0 4px rgba(195, 90, 70, 0.14);
}
.save-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}
.save-message {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--ink-muted);
}
.save-message.error {
  color: var(--danger);
}
.save-button {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  min-height: 38px;
  padding: 8px 16px;
  border-radius: 9px;
  font:
    700 13px Inter,
    sans-serif;
}
.spinner {
  width: 14px;
  height: 14px;
  border-radius: 999px;
  border: 2px solid rgba(255, 255, 255, 0.35);
  border-top-color: var(--theme-neutral-border, #fff);
  animation: spin 0.7s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.type-remove-backdrop {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  padding: 16px;
  background: rgba(48, 44, 38, 0.38);
  backdrop-filter: blur(4px);
}
.type-remove-dialog {
  width: min(460px, calc(100vw - 32px));
  display: grid;
  gap: 12px;
  padding: 20px 20px 16px;
  border: 1px solid var(--line-strong);
  border-radius: 16px;
  background: var(--surface);
  box-shadow: 0 20px 44px rgba(48, 44, 38, 0.2);
  animation: dialogIn 0.16s ease;
}
@keyframes dialogIn {
  from {
    opacity: 0;
    transform: translateY(6px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
.dialog-icon.warn {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: var(--danger-bg);
  border: 1px solid var(--danger-line);
  color: var(--danger);
}
.type-remove-dialog strong {
  font: 600 16px var(--font-display, Georgia, serif);
  color: var(--ink);
}
.type-remove-dialog > p,
.type-remove-note {
  margin: 0;
  color: var(--ink-muted);
  font:
    400 13px/1.5 Inter,
    sans-serif;
}
.type-remove-group {
  display: grid;
  gap: 7px;
  padding: 12px;
  border: 1px solid var(--theme-warning-border, #f0e8d9);
  border-radius: 11px;
  background: var(--surface-quiet);
}
.type-remove-group > span {
  color: var(--ink-muted);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}
.type-remove-group ul {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.type-remove-group li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--theme-warning-border, #ebe3d6);
  border-radius: 9px;
  background: var(--theme-surface-bg, #fff);
  font:
    500 12px Inter,
    sans-serif;
  color: var(--ink);
}
.type-remove-group li code {
  margin-left: auto;
  font:
    500 10px ui-monospace,
    monospace;
}
.type-remove-choice {
  flex: 1 1 150px;
  min-width: 140px;
}
.type-remove-choice select {
  width: 100%;
  min-width: 0;
}
.type-remove-group .field-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.field-label {
  font-weight: 600;
}
.type-remove-group small {
  color: var(--ink-muted);
  font:
    400 11px Inter,
    sans-serif;
}
.type-remove-check {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  margin-top: 4px;
  color: var(--ink);
  font:
    600 12px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  text-transform: none;
  letter-spacing: 0;
  cursor: pointer;
}
.type-remove-check input {
  min-width: 0;
  width: 16px;
  height: 16px;
  padding: 0;
  accent-color: var(--danger);
}
.type-remove-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}
@media (max-width: 899px) {
  .panel-hero {
    grid-template-columns: 1fr;
  }
  .hero-stats {
    grid-column: 1;
  }
  .tab {
    flex: 1 1 0;
    min-height: 34px;
    padding: 6px 10px;
    font-size: 12.5px;
  }
  .workbench-toolbar {
    grid-template-columns: 1fr;
  }
  .quiet.narrow-back {
    display: inline-flex;
    justify-self: start;
    width: auto;
    max-width: max-content;
  }
  .save-bar {
    flex-direction: column;
    align-items: stretch;
  }
  .save-button {
    width: 100%;
    justify-content: center;
  }
}
</style>
