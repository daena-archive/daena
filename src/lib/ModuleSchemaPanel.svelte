<script lang="ts">
import type { EntityTemplate, FieldDefinition, ModuleSchemaOverlay } from "$lib/project/client";
import { onMount } from "svelte";
import { setSchemaEditorDirtyCheck } from "$lib/schemaEditorGuard";
import { confirmDialog } from "$lib/dialogs.svelte";
import {
  Layers,
  Type,
  TextQuote,
  Blocks,
  LayoutTemplate,
  Plus,
  Pencil,
  Trash2,
  Check,
  X,
  SlidersHorizontal,
  Settings2,
  Sparkles,
  Eye,
  EyeOff,
  ChevronDown,
  ChevronRight,
  Save,
  AlertTriangle,
} from "@lucide/svelte";

type PackageManifest = {
  schemas: Array<{
    namespace: string;
    entityTypes: string[];
    fields: FieldDefinition[];
  }>;
  templates: EntityTemplate[];
};

type FieldType = FieldDefinition["type"];

const FIELD_TYPES: FieldType[] = ["text", "number", "boolean", "date", "enum", "oneof", "relationship"];

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
  if (type === "oneof") return "One of";
  return type.charAt(0).toUpperCase() + type.slice(1);
}

function parseOptions(value: string): string[] {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

function formatOptions(options?: string[] | null): string {
  return (options ?? []).join(", ");
}

function parseOneOfVariants(value: string): Array<{ label: string; type: FieldType; options?: string[] }> | null {
  if (!value.trim()) return [];
  try {
    const parsed = JSON.parse(value);
    if (Array.isArray(parsed)) return parsed;
    return null;
  } catch {
    return null;
  }
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
  const fieldScopeOverrides = cloneJson(value.fieldScopeOverrides ?? [])
    .map((scope) => ({
      fieldKey: scope.fieldKey,
      entityTypes: [...new Set(scope.entityTypes.map((name) => ensureTypeId(name, "type")).filter(Boolean))].sort(),
    }))
    .sort((left, right) => left.fieldKey.localeCompare(right.fieldKey));
  const templateOverrides = cloneJson(value.templateOverrides ?? []).sort((left, right) =>
    left.templateId.localeCompare(right.templateId),
  );
  return {
    version: value.version || 1,
    disabledEntityTypes: [...(value.disabledEntityTypes ?? [])].sort(),
    disabledFields: [...(value.disabledFields ?? [])].sort(),
    disabledTemplates: [...(value.disabledTemplates ?? [])].sort(),
    customEntityTypes,
    customFields,
    customTemplates,
    fieldScopeOverrides,
    templateOverrides,
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
// Keep initial for discard
const initialPlain = draftPlain;
/** Synced on every setDraft; read by the leave guard (plain bool, not $state). */
let dirtyFlag = false;
let dirty = $state(false);
let activeTab = $state<"types" | "fields" | "templates">("types");
let builtinTypesCollapsed = $state(false);
let builtinFieldsCollapsed = $state(false);
let builtinTemplatesCollapsed = $state(false);

let newType = $state("");
let editingTypeId = $state<string | null>(null);
let editTypeValue = $state("");

let newFieldLabel = $state("");
let newFieldType = $state<FieldType>("text");
let newFieldEntityTypes = $state<string[]>([]);
let newFieldOptions = $state("");
let newFieldMultiple = $state(false);
let newFieldTargetEntityTypes = $state<string[]>([]);
let newFieldRelationshipType = $state("");
let newFieldCardinality = $state<"one" | "many">("many");
let newFieldOneOfVariants = $state<Array<{ label: string; type: FieldType; options: string }>>([]);
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
let editingBuiltinFieldKey = $state<string | null>(null);

let newTemplateName = $state("");
let newTemplateEntityType = $state("");
let newTemplateDescription = $state("");
let newTemplateFieldKeys = $state<string[]>([]);
let newTemplateRequiredFields = $state<string[]>([]);
let editingTemplateId = $state<string | null>(null);
let editTemplateName = $state("");
let editTemplateEntityType = $state("");
let editTemplateDescription = $state("");
let editingBuiltinTemplateId = $state<string | null>(null);
let editingTemplateFieldKeys = $state<string[]>([]);
let editingTemplateRequiredFields = $state<string[]>([]);

function syncNewTemplateFields() {
  if (!newTemplateEntityType || !effectiveTypes().includes(newTemplateEntityType)) {
    if (newTemplateFieldKeys.length !== 0 || newTemplateRequiredFields.length !== 0) {
      newTemplateFieldKeys = [];
      newTemplateRequiredFields = [];
    }
    return;
  }
  const available = effectiveFieldsForType(newTemplateEntityType).map((f) => f.key).sort();
  const filtered = newTemplateFieldKeys.filter((k) => available.includes(k));
  const filteredReq = newTemplateRequiredFields.filter((k) => filtered.includes(k));
  if (filtered.length !== newTemplateFieldKeys.length) newTemplateFieldKeys = filtered;
  if (filteredReq.length !== newTemplateRequiredFields.length) newTemplateRequiredFields = filteredReq;
}

function syncEditTemplateFields() {
  if (!editingTemplateId || !editTemplateEntityType) return;
  if (!effectiveTypes().includes(editTemplateEntityType)) return;
  const original = (draft.customTemplates ?? []).find((t) => t.id === editingTemplateId);
  const originalType = original?.entityType;
  const available = effectiveFieldsForType(editTemplateEntityType).map((f) => f.key).sort();
  if (originalType && originalType !== editTemplateEntityType) {
    const filtered = editingTemplateFieldKeys.filter((k) => available.includes(k));
    const filteredReq = editingTemplateRequiredFields.filter((k) => filtered.includes(k));
    editingTemplateFieldKeys = filtered;
    editingTemplateRequiredFields = filteredReq;
    return;
  }
  const filtered = editingTemplateFieldKeys.filter((k) => available.includes(k));
  const filteredReq = editingTemplateRequiredFields.filter((k) => filtered.includes(k));
  if (filtered.length !== editingTemplateFieldKeys.length) editingTemplateFieldKeys = filtered;
  if (filteredReq.length !== editingTemplateRequiredFields.length) editingTemplateRequiredFields = filteredReq;
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
    : (field.entityTypes && field.entityTypes.length ? field.entityTypes : effectiveTypes());
  return scope.includes(entityType);
}

function effectiveFieldsForType(entityType: string): FieldDefinition[] {
  return [...packageFields, ...(draft.customFields ?? [])].filter(
    (field) => !isDisabled(draft.disabledFields, field.key) && fieldAppliesTo(field, entityType),
  );
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

function scopeLabel(entityTypes: string[] | undefined): string {
  if (!entityTypes || entityTypes.length === 0) return "All types";
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

function defaultFieldValue(field: FieldDefinition): unknown {
  if (field.type === "boolean") return false;
  if (field.type === "relationship") return [];
  return "";
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
  editingTemplateId = builtin ? null : template.id;
  editingBuiltinTemplateId = builtin ? template.id : null;
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
}

function templateFieldsFromSelection(template: EntityTemplate): Record<string, unknown> {
  const existing = fieldsForTemplate(template, Boolean(editingBuiltinTemplateId));
  return Object.fromEntries(
    editingTemplateFieldKeys.map((key) => {
      const field = effectiveFieldsForType(template.entityType).find((candidate) => candidate.key === key);
      return [key, key in existing ? existing[key] : field ? defaultFieldValue(field) : ""];
    }),
  );
}

function cancelTemplateFieldEdit() {
  cancelTemplateEdit();
  editingBuiltinTemplateId = null;
  editingTemplateFieldKeys = [];
  editingTemplateRequiredFields = [];
}

async function save() {
  await onSave(normalizeOverlay(draft));
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
  editingTypeId = null;
  editTypeValue = "";
  newFieldLabel = "";
  newFieldType = "text";
  newFieldEntityTypes = [];
  newFieldOptions = "";
  newFieldMultiple = false;
  newFieldTargetEntityTypes = [];
  newFieldRelationshipType = "";
  newFieldCardinality = "many";
  newFieldOneOfVariants = [];
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
  editingBuiltinFieldKey = null;
  newTemplateName = "";
  newTemplateEntityType = "";
  newTemplateDescription = "";
  newTemplateFieldKeys = [];
  newTemplateRequiredFields = [];
  editingTemplateId = null;
  editTemplateName = "";
  editTemplateEntityType = "";
  editTemplateDescription = "";
  editingBuiltinTemplateId = null;
  editingTemplateFieldKeys = [];
  editingTemplateRequiredFields = [];
  typeRemovalPrompt = null;
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
        if (scoped.length === 1) return false;
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
  editFieldOptions = "";
  editFieldMultiple = false;
  editFieldTargetEntityTypes = [];
  editFieldRelationshipType = "";
  editFieldCardinality = "many";
  editFieldOneOfVariants = [];
}

function startFieldEdit(field: FieldDefinition) {
  editingTypeId = null;
  editingTemplateId = null;
  editingFieldKey = field.key;
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
    extra.relationshipType = relType;
    extra.targetEntityTypes = [...editFieldTargetEntityTypes].sort();
    extra.cardinality = editFieldCardinality;
    extra.options = undefined;
    extra.multiple = undefined;
    extra.oneOf = undefined;
  } else {
    extra.options = undefined;
    extra.multiple = undefined;
    extra.targetEntityTypes = undefined;
    extra.relationshipType = undefined;
    extra.cardinality = undefined;
    extra.oneOf = undefined;
  }

  setDraft({
    ...draft,
    customFields: (draft.customFields ?? []).map((field) => {
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
      Object.assign(next, extra);
      // Remove undefined
      for (const k of Object.keys(next)) if (next[k] === undefined) delete next[k];
      return next;
    }),
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

function canAddField(): boolean {
  if (!newFieldLabel.trim()) return false;
  if (newFieldType === "enum") return parseOptions(newFieldOptions).length > 0;
  if (newFieldType === "oneof")
    return newFieldOneOfVariants.some((v) => v.label.trim() && (v.type !== "enum" || parseOptions(v.options).length > 0));
  if (newFieldType === "relationship")
    return Boolean(ensureTypeId(newFieldRelationshipType.trim() || newFieldLabel.trim(), "relationship")) && newFieldTargetEntityTypes.length > 0;
  return true;
}

function canSaveFieldEdit(): boolean {
  if (!editingFieldKey || !editFieldLabel.trim()) return false;
  if (editFieldType === "enum") return parseOptions(editFieldOptions).length > 0;
  if (editFieldType === "oneof")
    return editFieldOneOfVariants.some((v) => v.label.trim() && (v.type !== "enum" || parseOptions(v.options).length > 0));
  if (editFieldType === "relationship")
    return Boolean(ensureTypeId(editFieldRelationshipType.trim() || editFieldLabel.trim(), "relationship")) && editFieldTargetEntityTypes.length > 0;
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
    return `${rel} → ${targets.map(humanizeId).join(", ") || "any"} · ${card}`;
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
    id = ensureTypeId(`${name}-${entityType}`, "template");
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
    fields[key] = defaultFieldValue(field);
  }
  const finalFields = fields;
  const finalRequired = newTemplateRequiredFields.filter((k) => k in finalFields);
  const template: EntityTemplate = {
    id,
    name,
    entityType,
    description: description || null,
    icon: name.slice(0, 1).toUpperCase(),
    fields: finalFields,
    requiredFields: finalRequired,
  };
  setDraft({ ...draft, customTemplates: [...(draft.customTemplates ?? []), template] });
  newTemplateName = "";
  newTemplateEntityType = "";
  newTemplateDescription = "";
  newTemplateFieldKeys = [];
  newTemplateRequiredFields = [];
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
    const existing = (draft.customTemplates ?? []).find((t) => t.id === id)?.fields as Record<string, unknown> | undefined;
    const hasExisting = existing && key in existing;
    fields[key] = hasExisting ? (existing as Record<string, unknown>)[key] : defaultFieldValue(field);
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
<section class="module-schema-panel">
  <header class="panel-hero">
    <div class="hero-icon">
      <SlidersHorizontal size={18} strokeWidth={1.8} aria-hidden="true" />
    </div>
    <div class="hero-copy">
      <span class="kicker">PROJECT OVERLAY</span>
      <strong>Types &amp; fields</strong>
      <p>Package defaults stay intact. Disable what you don’t need and layer project-specific types, fields, and templates. Nothing you do here modifies the installed plugin.</p>
    </div>
    <div class="hero-stats" aria-label="Overlay summary">
      <span class="stat-pill"><Layers size={12} strokeWidth={1.8} aria-hidden="true" /> {effectiveTypes().length} types</span>
      <span class="stat-pill"><TextQuote size={12} strokeWidth={1.8} aria-hidden="true" /> {(draft.customFields ?? []).length + packageFields.length - (draft.disabledFields?.length ?? 0)} fields</span>
      <span class="stat-pill"><LayoutTemplate size={12} strokeWidth={1.8} aria-hidden="true" /> {(draft.customTemplates ?? []).length + packageTemplates.length - (draft.disabledTemplates?.length ?? 0)} templates</span>
    </div>
  </header>

  {#if !projectOpen}
    <div class="empty-card">
      <div class="empty-icon"><Blocks size={20} strokeWidth={1.7} aria-hidden="true" /></div>
      <strong>Open a project to customize this schema</strong>
      <p>Schema overlays are saved inside the project’s folder and travel with the project.</p>
    </div>
  {:else}
    <div class="tab-bar" role="tablist" aria-label="Schema sections">
      <button
        type="button"
        role="tab"
        class="tab"
        class:active={activeTab === "types"}
        aria-selected={activeTab === "types"}
        onclick={() => (activeTab = "types")}>
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
        onclick={() => (activeTab = "fields")}>
        <TextQuote size={14} strokeWidth={1.8} aria-hidden="true" />
        Fields
        <span class="tab-count">{[...packageFields, ...(draft.customFields ?? [])].filter(f=>!isDisabled(draft.disabledFields,f.key)).length}</span>
      </button>
      <button
        type="button"
        role="tab"
        class="tab"
        class:active={activeTab === "templates"}
        aria-selected={activeTab === "templates"}
        onclick={() => (activeTab = "templates")}>
        <LayoutTemplate size={14} strokeWidth={1.8} aria-hidden="true" />
        Templates
        <span class="tab-count">{[...packageTemplates, ...(draft.customTemplates ?? [])].filter(t=>!isDisabled(draft.disabledTemplates,t.id)).length}</span>
      </button>
    </div>

    {#if activeTab === "types"}
      <div class="block elevated">
        <button type="button" class="block-heading collapsible" aria-expanded={!builtinTypesCollapsed} onclick={() => (builtinTypesCollapsed = !builtinTypesCollapsed)}>
          <div class="heading-left">
            <span class="heading-icon"><Layers size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>Builtin entity types</h4>
            <span class="count-badge">{packageTypes.length}</span>
          </div>
          <span class="block-hint"><Eye size={12} strokeWidth={1.8} aria-hidden="true" /> Click to enable or disable</span>
          <span class="collapse-icon" aria-hidden="true">{#if builtinTypesCollapsed}<ChevronRight size={14} strokeWidth={1.8} />{:else}<ChevronDown size={14} strokeWidth={1.8} />{/if}</span>
        </button>
        {#if !builtinTypesCollapsed}
          <div class="chip-row">
            {#each packageTypes as type}
              {@const disabled = isDisabled(draft.disabledEntityTypes, type)}
              <button
                type="button"
                class="chip"
                class:is-hidden={disabled}
                aria-pressed={!disabled}
                title={type}
                onclick={() => toggleDisabled("disabledEntityTypes", type)}>
                {#if !disabled}<Check size={11} strokeWidth={2.2} aria-hidden="true" />{:else}<EyeOff size={11} strokeWidth={1.8} aria-hidden="true" />{/if}
                {humanizeId(type)}
              </button>
            {/each}
          </div>
          <p class="subtle-note">Disabled types and their exclusive fields/templates are hidden from create menus. Re-enable to bring them back.</p>
        {/if}
      </div>

      <div class="block elevated">
        <div class="block-heading">
          <div class="heading-left">
            <span class="heading-icon accent"><Sparkles size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>Custom entity types</h4>
            <span class="count-badge accent">{(draft.customEntityTypes ?? []).length}</span>
          </div>
          <span class="block-hint">Project-only types layered on the package</span>
        </div>
        {#if (draft.customEntityTypes ?? []).length === 0}
          <div class="empty-inline">
            <Type size={16} strokeWidth={1.7} aria-hidden="true" />
            <div>
              <strong>No custom types yet</strong>
              <span>Create a project type like “Species”, “Artifact” or “Language”.</span>
            </div>
          </div>
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
                      <button type="button" class="action" onclick={commitTypeEdit}><Check size={14} strokeWidth={2} aria-hidden="true" /> Save</button>
                      <button type="button" class="quiet" onclick={cancelTypeEdit}>Cancel</button>
                    </div>
                  </div>
                {:else}
                  <div class="item-main">
                    <strong>{humanizeId(type)}</strong>
                    <code>{type}</code>
                  </div>
                  <div class="item-actions">
                    <button type="button" class="quiet icon" aria-label="Edit {type}" onclick={() => startTypeEdit(type)}><Pencil size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                    <button type="button" class="danger icon" aria-label="Remove {type}" onclick={() => requestRemoveCustomType(type)}><Trash2 size={14} strokeWidth={1.8} aria-hidden="true" /></button>
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
          <button type="button" class="action primary-action" onclick={addCustomType}><Plus size={14} strokeWidth={2} aria-hidden="true" /> Add type</button>
        </div>
      </div>
    {:else if activeTab === "fields"}
      <div class="block elevated">
        <button type="button" class="block-heading collapsible" aria-expanded={!builtinFieldsCollapsed} onclick={() => (builtinFieldsCollapsed = !builtinFieldsCollapsed)}>
          <div class="heading-left">
            <span class="heading-icon"><TextQuote size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>Builtin fields</h4>
            <span class="count-badge">{packageFields.length}</span>
          </div>
          <span class="block-hint">Enable fields and choose the entity types they apply to</span>
          <span class="collapse-icon" aria-hidden="true">{#if builtinFieldsCollapsed}<ChevronRight size={14} strokeWidth={1.8} />{:else}<ChevronDown size={14} strokeWidth={1.8} />{/if}</span>
        </button>
        {#if !builtinFieldsCollapsed}
          <div class="chip-row">
            {#each packageFields as field}
              {@const disabled = isDisabled(draft.disabledFields, field.key)}
              <button
                type="button"
                class="chip"
                class:is-hidden={disabled}
                aria-pressed={!disabled}
                title={`${field.key} · ${fieldTypeLabel(field.type)}`}
                onclick={() => toggleDisabled("disabledFields", field.key)}>
                {#if !disabled}<Check size={11} strokeWidth={2.2} aria-hidden="true" />{:else}<EyeOff size={11} strokeWidth={1.8} aria-hidden="true" />{/if}
                {field.label || humanizeId(field.key)}
              </button>
            {/each}
          </div>
          <ul class="list compact-list">
          {#each packageFields as field}
            <li class="list-item compact">
              {#if editingBuiltinFieldKey === field.key}
                <div class="edit-form wide">
                  <div class="type-select" role="group" aria-label={`Entity types for ${field.label}`}>
                    <span class="type-select-label">Applies to</span>
                    <div class="chip-row compact">
                      {#each effectiveTypes() as type}
                        <button
                          type="button"
                          class="chip select"
                          class:selected={builtinFieldScope(field).includes(type)}
                          aria-pressed={builtinFieldScope(field).includes(type)}
                          onclick={() => updateBuiltinFieldScope(field, toggleInList(builtinFieldScope(field), type))}
                          >{humanizeId(type)}</button>
                      {/each}
                    </div>
                  </div>
                  <div class="edit-actions">
                    <button type="button" class="action" onclick={() => (editingBuiltinFieldKey = null)}><Check size={14} strokeWidth={2} aria-hidden="true" /> Done</button>
                  </div>
                </div>
              {:else}
                <div class="item-main">
                  <div class="item-title-row">
                    <strong>{field.label || humanizeId(field.key)}</strong>
                    <span class="type-pill">{fieldTypeLabel(field.type)}</span>
                    {#if isDisabled(draft.disabledFields, field.key)}<span class="disabled-pill"><EyeOff size={10} strokeWidth={1.8} aria-hidden="true" /> Disabled</span>{/if}
                  </div>
                  <span class="meta">{scopeLabel(builtinFieldScope(field))} <span class="dot">·</span> <code>{field.key}</code></span>
                </div>
                <div class="item-actions">
                  <button type="button" class="quiet" onclick={() => (editingBuiltinFieldKey = field.key)}>Edit scope</button>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
        {/if}
      </div>

      <div class="block elevated">
        <div class="block-heading">
          <div class="heading-left">
            <span class="heading-icon accent"><Sparkles size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>Custom fields</h4>
            <span class="count-badge accent">{(draft.customFields ?? []).length}</span>
          </div>
          <span class="block-hint">Choose at least one entity type for each field</span>
        </div>
        {#if (draft.customFields ?? []).length === 0}
          <div class="empty-inline">
            <Blocks size={16} strokeWidth={1.7} aria-hidden="true" />
            <div>
              <strong>No custom fields yet</strong>
              <span>Add a field like “Word count” or “Origin”.</span>
            </div>
          </div>
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
                    {#if editFieldType === "enum"}
                      <label>
                        <span>Options <em>(comma separated)</em></span>
                        <input bind:value={editFieldOptions} placeholder="idea, drafting, revising, complete" />
                      </label>
                      <label class="inline-check">
                        <input type="checkbox" bind:checked={editFieldMultiple} />
                        <span>Allow multiple values</span>
                      </label>
                    {:else if editFieldType === "oneof"}
                      <div class="type-select" role="group" aria-label="One-of variants">
                        <span class="type-select-label">Variants <em>(at least one)</em></span>
                        {#each editFieldOneOfVariants as variant, idx}
                          <div class="variant-row">
                            <input bind:value={variant.label} placeholder="Variant label" />
                            <select bind:value={variant.type}>
                              {#each ["text","number","boolean","date","enum"] as vt}
                                <option value={vt}>{fieldTypeLabel(vt as FieldType)}</option>
                              {/each}
                            </select>
                            {#if variant.type === "enum"}
                              <input bind:value={variant.options} placeholder="Options, comma separated" />
                            {/if}
                            <button type="button" class="quiet icon" onclick={() => removeEditFieldOneOfVariant(idx)}><X size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                          </div>
                        {/each}
                        <button type="button" class="quiet" onclick={addEditFieldOneOfVariant}><Plus size={14} strokeWidth={1.8} aria-hidden="true" /> Add variant</button>
                      </div>

                    {:else if editFieldType === "relationship"}
                      <label>
                        <span>Relationship type</span>
                        <input bind:value={editFieldRelationshipType} placeholder="Related to" />
                      </label>
                      <div class="type-select" role="group" aria-label="Target entity types">
                        <span class="type-select-label">Target types <em>(required)</em></span>
                        <div class="chip-row compact">
                          {#each effectiveTypes() as type}
                            <button
                              type="button"
                              class="chip select"
                              class:selected={editFieldTargetEntityTypes.includes(type)}
                              aria-pressed={editFieldTargetEntityTypes.includes(type)}
                              onclick={() => (editFieldTargetEntityTypes = toggleInList(editFieldTargetEntityTypes, type))}>
                              {humanizeId(type)}
                            </button>
                          {/each}
                        </div>
                      </div>
                      <label>
                        <span>Cardinality</span>
                        <select bind:value={editFieldCardinality}>
                          <option value="many">Many</option>
                          <option value="one">One</option>
                        </select>
                      </label>
                    {/if}
                    <div class="type-select" role="group" aria-label="Applies to entity types">
                      <span class="type-select-label">Applies to <em>(optional)</em></span>
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
                        disabled={!canSaveFieldEdit()}
                        onclick={commitFieldEdit}><Check size={14} strokeWidth={2} aria-hidden="true" /> Save</button>
                      <button type="button" class="quiet" onclick={cancelFieldEdit}>Cancel</button>
                    </div>
                  </div>
                {:else}
                  <div class="item-main">
                    <div class="item-title-row">
                      <strong>{field.label}</strong>
                      <span class="type-pill">{fieldTypeLabel(field.type)}</span>
                      {#if fieldExtrasLabel(field)}
                        <span class="meta">{fieldExtrasLabel(field)}</span>
                      {/if}
                    </div>
                    <span class="meta">{scopeLabel(field.entityTypes)} <span class="dot">·</span> <code>{field.key}</code></span>
                  </div>
                  <div class="item-actions">
                    <button type="button" class="quiet icon" aria-label="Edit {field.label}" onclick={() => startFieldEdit(field)}><Pencil size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                    <button type="button" class="danger icon" aria-label="Remove {field.label}" onclick={() => removeCustomField(field.key)}><Trash2 size={14} strokeWidth={1.8} aria-hidden="true" /></button>
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
              class="action primary-action"
              disabled={!canAddField()}
              onclick={addCustomField}><Plus size={14} strokeWidth={2} aria-hidden="true" /> Add field</button>
          </div>
          {#if newFieldType === "enum"}
            <label>
              <span>Options <em>(comma separated)</em></span>
              <input bind:value={newFieldOptions} placeholder="idea, drafting, revising, complete" />
            </label>
            <label class="inline-check">
              <input type="checkbox" bind:checked={newFieldMultiple} />
              <span>Allow multiple values</span>
            </label>
          {:else if newFieldType === "oneof"}
            <div class="type-select" role="group" aria-label="One-of variants">
              <span class="type-select-label">Variants <em>(at least one)</em></span>
              {#each newFieldOneOfVariants as variant, idx}
                <div class="variant-row">
                  <input bind:value={variant.label} placeholder="Variant label" />
                  <select bind:value={variant.type}>
                    {#each ["text","number","boolean","date","enum"] as vt}
                      <option value={vt}>{fieldTypeLabel(vt as FieldType)}</option>
                    {/each}
                  </select>
                  {#if variant.type === "enum"}
                    <input bind:value={variant.options} placeholder="Options, comma separated" />
                  {/if}
                  <button type="button" class="quiet icon" onclick={() => removeNewFieldOneOfVariant(idx)}><X size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                </div>
              {/each}
              <button type="button" class="quiet" onclick={addNewFieldOneOfVariant}><Plus size={14} strokeWidth={1.8} aria-hidden="true" /> Add variant</button>
            </div>

          {:else if newFieldType === "relationship"}
            <label>
              <span>Relationship type</span>
              <input bind:value={newFieldRelationshipType} placeholder="Related to" />
            </label>
            <div class="type-select" role="group" aria-label="Target entity types">
              <span class="type-select-label">Target types <em>(required)</em></span>
              <div class="chip-row compact">
                {#each effectiveTypes() as type}
                  <button
                    type="button"
                    class="chip select"
                    class:selected={newFieldTargetEntityTypes.includes(type)}
                    aria-pressed={newFieldTargetEntityTypes.includes(type)}
                    onclick={() => (newFieldTargetEntityTypes = toggleInList(newFieldTargetEntityTypes, type))}>
                    {humanizeId(type)}
                  </button>
                {/each}
              </div>
            </div>
            <label>
              <span>Cardinality</span>
              <select bind:value={newFieldCardinality}>
                <option value="many">Many</option>
                <option value="one">One</option>
              </select>
            </label>
          {/if}
          <div class="type-select" role="group" aria-label="Applies to entity types">
            <span class="type-select-label">Applies to <em>(optional)</em></span>
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
    {:else}
      <div class="block elevated">
        <button type="button" class="block-heading collapsible" aria-expanded={!builtinTemplatesCollapsed} onclick={() => (builtinTemplatesCollapsed = !builtinTemplatesCollapsed)}>
          <div class="heading-left">
            <span class="heading-icon"><LayoutTemplate size={14} strokeWidth={1.8} aria-hidden="true" /></span>
            <h4>Builtin templates</h4>
            <span class="count-badge">{packageTemplates.length}</span>
          </div>
          <span class="block-hint">Enable templates and choose their included fields</span>
          <span class="collapse-icon" aria-hidden="true">{#if builtinTemplatesCollapsed}<ChevronRight size={14} strokeWidth={1.8} />{:else}<ChevronDown size={14} strokeWidth={1.8} />{/if}</span>
        </button>
        {#if !builtinTemplatesCollapsed}
          <div class="chip-row">
            {#each packageTemplates as template}
              {@const disabled = isDisabled(draft.disabledTemplates, template.id)}
            <button
              type="button"
              class="chip"
              class:is-hidden={disabled}
              aria-pressed={!disabled}
              title={template.description || template.id}
              onclick={() => toggleDisabled("disabledTemplates", template.id)}>
              {#if !disabled}<Check size={11} strokeWidth={2.2} aria-hidden="true" />{:else}<EyeOff size={11} strokeWidth={1.8} aria-hidden="true" />{/if}
              {template.name}
            </button>
          {/each}
        </div>
        <ul class="list compact-list">
          {#each packageTemplates as template}
            <li class="list-item compact">
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
                          onclick={() => {
                            editingTemplateFieldKeys = toggleInList(editingTemplateFieldKeys, field.key);
                            if (!editingTemplateFieldKeys.includes(field.key))
                              editingTemplateRequiredFields = editingTemplateRequiredFields.filter(
                                (key) => key !== field.key,
                              );
                          }}>{field.label || humanizeId(field.key)}</button>
                      {/each}
                    </div>
                  </div>
                  <div class="type-select" role="group" aria-label={`Required fields for ${template.name}`}>
                    <span class="type-select-label">Required fields</span>
                    <div class="chip-row compact">
                      {#each effectiveFieldsForType(template.entityType).filter((f) => editingTemplateFieldKeys.includes(f.key)) as field}
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
                  <div class="edit-actions">
                    <button type="button" class="action" onclick={commitBuiltinTemplateEdit}><Check size={14} strokeWidth={2} aria-hidden="true" /> Save fields</button><button
                      type="button"
                      class="quiet"
                      onclick={cancelTemplateFieldEdit}>Cancel</button>
                  </div>
                </div>
              {:else}
                <div class="item-main">
                  <div class="item-title-row">
                    <strong>{template.name}</strong>
                    <span class="type-pill ghost">{humanizeId(template.entityType)}</span>
                    {#if isDisabled(draft.disabledTemplates, template.id)}<span class="disabled-pill"><EyeOff size={10} strokeWidth={1.8} aria-hidden="true" /> Disabled</span>{/if}
                  </div>
                  <span class="meta">{Object.keys(fieldsForTemplate(template, true)).length} fields <span class="dot">·</span> {template.description || template.id}</span>
                </div>
                <div class="item-actions">
                  <button type="button" class="quiet" onclick={() => beginTemplateFieldEdit(template, true)}>Customize fields</button>
                </div>
              {/if}
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
                    <div class="type-select" role="group" aria-label={`Fields for ${editTemplateName || template.name}`}>
                      <span class="type-select-label">Included fields</span>
                      <div class="chip-row compact">
                        {#each effectiveFieldsForType(editTemplateEntityType) as field}
                          <button
                            type="button"
                            class="chip select"
                            class:selected={editingTemplateFieldKeys.includes(field.key)}
                            aria-pressed={editingTemplateFieldKeys.includes(field.key)}
                            onclick={() => {
                              editingTemplateFieldKeys = toggleInList(editingTemplateFieldKeys, field.key);
                              if (!editingTemplateFieldKeys.includes(field.key))
                                editingTemplateRequiredFields = editingTemplateRequiredFields.filter(
                                  (key) => key !== field.key,
                                );
                            }}>{field.label || humanizeId(field.key)}</button>
                        {/each}
                      </div>
                    </div>
                    <div class="type-select" role="group" aria-label={`Required fields for ${template.name}`}>
                      <span class="type-select-label">Required fields</span>
                      <div class="chip-row compact">
                        {#each effectiveFieldsForType(editTemplateEntityType).filter((f) => editingTemplateFieldKeys.includes(f.key)) as field}
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
                    <div class="edit-actions">
                      <button type="button" class="action" onclick={commitTemplateEdit}><Check size={14} strokeWidth={2} aria-hidden="true" /> Save</button>
                      <button type="button" class="quiet" onclick={cancelTemplateEdit}>Cancel</button>
                    </div>
                  </div>
                {:else}
                  <div class="item-main">
                    <div class="item-title-row">
                      <strong>{template.name}</strong>
                      <span class="type-pill ghost">{humanizeId(template.entityType)}</span>
                    </div>
                    <span class="meta">
                      {humanizeId(template.entityType)}
                      {#if template.description}
                        <span class="dot">·</span>
                        {template.description}
                      {/if}
                    </span>
                  </div>
                  <div class="item-actions">
                    <button type="button" class="quiet icon" aria-label="Edit {template.name}" onclick={() => startTemplateEdit(template)}><Pencil size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                    <button type="button" class="danger icon" aria-label="Remove {template.name}" onclick={() => removeCustomTemplate(template.id)}><Trash2 size={14} strokeWidth={1.8} aria-hidden="true" /></button>
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
            <button type="button" class="action primary-action" onclick={addCustomTemplate} disabled={!newTemplateName.trim() || !newTemplateEntityType.trim()}><Plus size={14} strokeWidth={2} aria-hidden="true" /> Add template</button>
          </div>
          <label class="grow">
            <span>Description <em>(optional)</em></span>
            <input bind:value={newTemplateDescription} placeholder="A kind of being in this world." />
          </label>
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
                    onclick={() => {
                      newTemplateFieldKeys = toggleInList(newTemplateFieldKeys, field.key);
                      if (!newTemplateFieldKeys.includes(field.key))
                        newTemplateRequiredFields = newTemplateRequiredFields.filter((k) => k !== field.key);
                    }}>{field.label || humanizeId(field.key)}</button>
                {/each}
              </div>
            </div>
            <div class="type-select" role="group" aria-label="Required fields">
              <span class="type-select-label">Required fields</span>
              <div class="chip-row compact">
                {#each effectiveFieldsForType(newTemplateEntityType).filter((f) => newTemplateFieldKeys.includes(f.key)) as field}
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
          {/if}
        </div>
      </div>
    {/if}

    <div class="save-bar" class:has-dirty={dirty} class:is-busy={busy}>
      <div class="save-copy">
        {#if dirty}
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
        <button type="button" class="quiet" disabled={busy || !dirty} onclick={() => void discardChanges()}><X size={14} strokeWidth={1.8} aria-hidden="true" /> Discard</button>
        <button type="button" class="primary save-button" disabled={busy || !dirty} onclick={() => void save()}>
          {#if busy}<span class="spinner" aria-hidden="true"></span> Saving…{:else}<Save size={14} strokeWidth={2} aria-hidden="true" /> Save schema{/if}
        </button>
      </div>
    </div>
  {/if}

  {#if typeRemovalPrompt}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="type-remove-backdrop" role="presentation" tabindex="-1" onclick={() => cancelTypeRemoval()} onkeydown={(e)=> e.key==="Escape" && cancelTypeRemoval()}>
      <!-- svelte-ignore a11y_autofocus -->
      <div class="type-remove-dialog" role="alertdialog" aria-modal="true" tabindex="-1" aria-labelledby="type-remove-title" onclick={(e)=>e.stopPropagation()} onkeydown={(e)=>e.key==="Escape" && cancelTypeRemoval()}>
        <div class="dialog-icon warn">
          <AlertTriangle size={18} strokeWidth={1.8} aria-hidden="true" />
        </div>
        <strong id="type-remove-title">Remove {humanizeId(typeRemovalPrompt.typeId)}?</strong>
        <p>
          Templates for this type are removed with the type. Fields that only target it will be kept and will now
          apply to all types until you reassign them.
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
            <span>Fields that only target this type (will be kept)</span>
            <ul>
              {#each typeRemovalPrompt.exclusiveFields as field}
                <li><TextQuote size={12} strokeWidth={1.7} aria-hidden="true" /> {field.label} <code>{field.key}</code> <small>— now applies to all types</small></li>
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
                  <span class="field-label">{field.label}</span>
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
          <button type="button" class="quiet" onclick={cancelTypeRemoval}>Keep type</button>
          <button type="button" class="danger" onclick={confirmTypeRemoval}><Trash2 size={14} strokeWidth={1.8} aria-hidden="true" /> Remove</button>
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
  border: 1px solid var(--line, #e9e1d4);
  border-radius: 14px;
  background: var(--surface, #fffefa);
}
.hero-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: #fffefa;
}
.hero-copy .kicker {
  color: #b4773f;
  font: 700 10px/1 Inter, ui-sans-serif, system-ui, sans-serif;
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
  color: var(--ink-soft, #8f897e);
  font: 400 12.5px/1.5 Inter, ui-sans-serif, system-ui, sans-serif;
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
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #62594e;
  font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif;
}
.empty-card {
  display: grid;
  gap: 10px;
  justify-items: start;
  padding: 22px 18px;
  border: 1px dashed #d9cdbd;
  border-radius: 14px;
  background: #fffcf7;
}
.empty-card .empty-icon {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #8f897e;
}
.empty-card strong {
  color: var(--ink);
  font: 600 15px var(--font-display, Georgia, serif);
}
.empty-card p {
  margin: 0;
  max-width: 520px;
  color: var(--ink-soft, #8f897e);
  font: 400 12.5px/1.5 Inter, ui-sans-serif, system-ui, sans-serif;
}
.tab-bar {
  display: flex;
  gap: 6px;
  padding: 6px;
  border: 1px solid var(--line, #e9e1d4);
  border-radius: 12px;
  background: #f7f3ec;
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
  color: #8f897e;
  font: 600 13px Inter, ui-sans-serif, system-ui, sans-serif;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.14s ease;
}
.tab:hover {
  background: #efe8d9;
  color: #62594e;
}
.tab.active {
  background: var(--accent-dark);
  border-color: var(--accent-dark);
  color: #fffefa;
  box-shadow: 0 1px 0 rgba(48,44,38,0.12);
}
.tab-count {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  padding: 0 6px;
  border-radius: 999px;
  background: rgba(255,255,255,0.14);
  color: inherit;
  font: 700 11px Inter, sans-serif;
}
.tab.active .tab-count {
  background: rgba(255,255,255,0.18);
}
.block {
  display: grid;
  gap: 14px;
  padding: 18px;
  border: 1px solid var(--line, #e9e1d4);
  border-radius: 14px;
  background: #fffefa;
}
.block.elevated {
  box-shadow: 0 1px 0 rgba(48,44,38,0.03), 0 8px 24px rgba(48,44,38,0.04);
}
.block-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid #f0e8d9;
}
.block-heading.collapsible {
  width: 100%;
  background: transparent;
  border: none;
  border-bottom: 1px solid #f0e8d9;
  padding: 0 0 12px;
  text-align: left;
  cursor: pointer;
  border-radius: 0;
}
.block-heading.collapsible:hover {
  opacity: 0.92;
}
.collapse-icon {
  margin-left: auto;
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  color: var(--ink-soft, #8f897e);
  flex: 0 0 24px;
}
.block-heading.collapsible:hover .collapse-icon {
  background: #f4eee3;
  color: var(--ink);
}
.heading-left {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}
.heading-icon {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #62594e;
}
.heading-icon.accent {
  background: var(--accent-dark);
  border-color: var(--accent-dark);
  color: #fffefa;
}
.block-heading h4 {
  margin: 0;
  font: 600 13px Inter, ui-sans-serif, system-ui, sans-serif;
  color: var(--ink);
  letter-spacing: -0.01em;
}
.count-badge {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  padding: 0 7px;
  border-radius: 999px;
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #62594e;
  font: 700 11px Inter, sans-serif;
}
.count-badge.accent {
  background: #fff3df;
  border-color: #e9c9a6;
  color: #8b5c2e;
}
.block-hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--ink-faint, #b0a89c);
  font: 500 11.5px Inter, ui-sans-serif, system-ui, sans-serif;
}
.subtle-note {
  margin: 0;
  padding: 9px 11px;
  border-radius: 9px;
  background: #f7f3ec;
  border: 1px solid #f0e8d9;
  color: #8f897e;
  font: 400 11.5px/1.5 Inter, sans-serif;
}
.empty-inline {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 14px 14px;
  border: 1px dashed #d9cdbd;
  border-radius: 11px;
  background: #fffcf7;
  color: #8f897e;
}
.empty-inline strong {
  display: block;
  color: var(--ink);
  font: 600 13px Inter, sans-serif;
  margin-bottom: 3px;
}
.empty-inline span {
  font: 400 12px/1.5 Inter, sans-serif;
}


.chip-row,
.add-row,
.edit-actions,
.item-actions {
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
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid #d9cdbd;
  border-radius: 9px;
  padding: 7px 11px;
  background: #fffefa;
  color: #62594e;
  font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif;
  line-height: 1;
  cursor: pointer;
  transition: all 0.14s ease;
}
.chip {
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 30px;
  padding: 6px 12px;
  font: 600 12px Inter, sans-serif;
}
.chip:hover,
.action:hover,
.quiet:hover {
  border-color: #b7a88f;
  background: #f4eee4;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(48,44,38,0.06);
}
.chip:active,
.action:active,
.quiet:active,
.danger:active {
  transform: translateY(0);
  box-shadow: none;
}
.chip:focus-visible,
.action:focus-visible,
.quiet:focus-visible,
.danger:focus-visible,
.primary:focus-visible {
  outline: 2px solid #b4773f;
  outline-offset: 2px;
}
.chip.is-hidden {
  opacity: 0.52;
  border-style: dashed;
  text-decoration: line-through;
  text-decoration-thickness: 1px;
  background: #fdf8ef;
}
.chip.select.selected,
.chip[aria-pressed="true"]:not(.is-hidden) {
  border-color: #b7a88f;
  background: #f4eee4;
  color: #3f3830;
  box-shadow: 0 1px 0 rgba(48,44,38,0.12);
}
.chip.select:not(.selected) {
  opacity: 0.9;
  background: #fff;
}
.action {
  background: #f7f1e7;
  border-color: #e9dfd0;
}
.action.primary-action {
  background: var(--accent-dark);
  border-color: var(--accent-dark);
  color: #fffefa;
}
.action.primary-action:hover {
  background: #4a6b57;
  border-color: #4a6b57;
}
.primary {
  border-color: var(--accent-dark);
  background: var(--accent-dark);
  color: #fffefa;
}
.primary:hover {
  background: #4a6b57;
}
.action:disabled,
.primary:disabled,
.quiet:disabled,
.danger:disabled,
.chip:disabled {
  opacity: 0.45;
  cursor: default;
  transform: none;
  box-shadow: none;
}
.danger {
  border-color: #e0b8ad;
  background: #fdf2ef;
  color: #9a4d3f;
}
.danger:hover {
  border-color: #c9897d;
  background: #f3ddd6;
}
.quiet.icon,
.danger.icon {
  width: 32px;
  height: 32px;
  padding: 0;
  display: grid;
  place-items: center;
  border-radius: 9px;
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
  align-items: center;
  padding: 12px 14px;
  border: 1px solid #ebe3d6;
  border-radius: 12px;
  background: #fffcf7;
  transition: border-color 0.14s ease, box-shadow 0.14s ease;
}
.list-item:hover {
  border-color: #e0d6c4;
  box-shadow: 0 4px 14px rgba(48,44,38,0.05);
}
.list-item.compact {
  padding: 10px 14px;
}
.item-main {
  display: grid;
  gap: 4px;
  min-width: 0;
  flex: 1;
}
.item-title-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.item-main strong {
  font: 600 13px Inter, ui-sans-serif, system-ui, sans-serif;
  color: var(--ink);
}
.meta {
  color: #8f897e;
  font: 400 12px/1.4 Inter, sans-serif;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.meta code {
  padding: 2px 6px;
  border-radius: 6px;
  background: #f1ebe1;
  border: 1px solid #e9e1d4;
  color: #6f675c;
  font: 500 11px ui-monospace, monospace;
}
.type-pill {
  display: inline-flex;
  align-items: center;
  padding: 2px 7px;
  border-radius: 999px;
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #62594e;
  font: 600 10px Inter, sans-serif;
  letter-spacing: 0.02em;
  text-transform: uppercase;
}
.type-pill.ghost {
  background: #fff;
  border-color: #e9e1d4;
  color: #8f897e;
}
.disabled-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 7px;
  border-radius: 999px;
  background: #fdf2ef;
  border: 1px solid #e7c4bc;
  color: #9a4d3f;
  font: 600 10px Inter, sans-serif;
}
.dot {
  color: #cbbda9;
}
code {
  width: fit-content;
  padding: 2px 6px;
  border-radius: 6px;
  background: #f1ebe1;
  border: 1px solid #e9e1d4;
  color: #6f675c;
  font: 500 11px ui-monospace, SFMono-Regular, Menlo, monospace;
}
.add-form,
.edit-form,
.stacked {
  display: grid;
  gap: 12px;
}
.add-form {
  padding: 14px;
  border: 1px dashed #d9cdbd;
  border-radius: 11px;
  background: #fdf8ef;
  margin-top: 2px;
}
.edit-form {
  width: 100%;
}
.edit-form.wide {
  gap: 10px;
}
.add-row {
  align-items: end;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}
.add-row > label {
  flex: 1 1 160px;
}
label {
  display: grid;
  gap: 5px;
  min-width: 140px;
}
label.grow {
  min-width: min(100%, 280px);
  flex: 1;
}
label span {
  color: #8f897e;
  font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif;
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
  gap: 7px;
}
.type-select-label {
  color: #8f897e;
  font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif;
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
  height: 36px;
  padding: 0 11px;
  border: 1px solid #d9cdbd;
  border-radius: 9px;
  background: #fff;
  color: var(--ink);
  font: 400 13px Inter, ui-sans-serif, system-ui, sans-serif;
  line-height: 1;
  box-sizing: border-box;
  transition: border-color 0.14s ease, box-shadow 0.14s ease;
}
select {
  padding-right: 28px;
  appearance: none;
  -webkit-appearance: none;
  background-image:
    linear-gradient(45deg, transparent 50%, #8f897e 50%),
    linear-gradient(135deg, #8f897e 50%, transparent 50%);
  background-position:
    calc(100% - 16px) calc(50% - 2px),
    calc(100% - 11px) calc(50% - 2px);
  background-size: 5px 5px, 5px 5px;
  background-repeat: no-repeat;
}
input:focus,
select:focus {
  outline: none;
  border-color: #b4773f;
  box-shadow: 0 0 0 3px rgba(180,119,63,0.12);
}
input::placeholder {
  color: #b0a89c;
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
  border: 1px solid var(--line, #e9e1d4);
  border-radius: 12px;
  background: rgba(255,254,250,0.96);
  backdrop-filter: blur(10px);
  box-shadow: 0 8px 24px rgba(48,44,38,0.08);
}
.save-bar.has-dirty {
  border-color: #e7c4bc;
  background: linear-gradient(#fffefa, #fff6f1);
}
.save-copy {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  min-width: 0;
  color: #8f897e;
  font: 400 12.5px Inter, sans-serif;
}
.save-copy strong {
  color: #9a4d3f;
  font: 700 12.5px Inter, sans-serif;
}
.save-copy .dirty-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: #c35a46;
  box-shadow: 0 0 0 4px rgba(195,90,70,0.14);
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
  color: #62594e;
}
.save-message.error {
  color: #9a4d3f;
}
.save-button {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  min-height: 38px;
  padding: 8px 16px;
  border-radius: 9px;
  font: 700 13px Inter, sans-serif;
}
.spinner {
  width: 14px;
  height: 14px;
  border-radius: 999px;
  border: 2px solid rgba(255,255,255,0.35);
  border-top-color: #fff;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
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
  border: 1px solid #d9cdbd;
  border-radius: 16px;
  background: #fffefa;
  box-shadow: 0 20px 44px rgba(48, 44, 38, 0.2);
  animation: dialogIn 0.16s ease;
}
@keyframes dialogIn { from { opacity: 0; transform: translateY(6px) scale(0.98); } to { opacity: 1; transform: translateY(0) scale(1); } }
.dialog-icon.warn {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: #fdf2ef;
  border: 1px solid #e7c4bc;
  color: #9a4d3f;
}
.type-remove-dialog strong {
  font: 600 16px var(--font-display, Georgia, serif);
  color: var(--ink);
}
.type-remove-dialog > p,
.type-remove-note {
  margin: 0;
  color: #8f897e;
  font: 400 13px/1.5 Inter, sans-serif;
}
.type-remove-group {
  display: grid;
  gap: 7px;
  padding: 12px;
  border: 1px solid #f0e8d9;
  border-radius: 11px;
  background: #fffcf7;
}
.type-remove-group > span {
  color: #62594e;
  font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif;
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
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid #ebe3d6;
  border-radius: 9px;
  background: #fff;
  font: 500 12px Inter, sans-serif;
  color: var(--ink);
}
.type-remove-group li code {
  margin-left: auto;
  font: 500 10px ui-monospace, monospace;
}
.field-label {
  font-weight: 600;
}
.type-remove-group small {
  color: #8f897e;
  font: 400 11px Inter, sans-serif;
}
.type-remove-check {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  margin-top: 4px;
  color: var(--ink);
  font: 600 12px Inter, ui-sans-serif, system-ui, sans-serif;
  text-transform: none;
  letter-spacing: 0;
  cursor: pointer;
}
.type-remove-check input {
  min-width: 0;
  width: 16px;
  height: 16px;
  padding: 0;
  accent-color: #9a4d3f;
}
.inline-check {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  color: var(--ink);
  font: 500 12px Inter, ui-sans-serif, system-ui, sans-serif;
  cursor: pointer;
}
.inline-check input {
  width: 16px;
  height: 16px;
  min-width: 0;
  accent-color: var(--accent-dark);
}
.variant-row {
  display: grid;
  grid-template-columns: 1fr 140px 1fr auto;
  gap: 8px;
  align-items: center;
}
.variant-row input,
.variant-row select {
  min-width: 0;
}
@media (max-width: 720px) {
  .variant-row {
    grid-template-columns: 1fr;
  }
}
.type-remove-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}
@media (max-width: 720px) {
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
  .save-bar {
    flex-direction: column;
    align-items: stretch;
  }
  .save-button {
    width: 100%;
    justify-content: center;
  }
  .add-row {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
