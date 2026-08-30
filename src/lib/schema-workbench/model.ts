/**
 * Schema workbench domain: overlay normalization, package flattening,
 * author-facing labels, and list filtering. Pure TypeScript — no Svelte.
 */
import type {
  EntityTemplate,
  EntityTypeDefinition,
  FieldDefinition,
  FieldMetadataOverride,
  ModuleSchemaOverlay,
} from "../project/client.ts";

/** Local defaults avoid importing Svelte/$lib modules from Node tests. */
const FALLBACK_ICON = { kind: "catalog", id: "unknown" } as const;
const DEFAULT_TYPE_COLOR = { kind: "preset", id: "brass" } as const;

export type FieldType = FieldDefinition["type"];

export const FIELD_TYPES: FieldType[] = ["text", "number", "boolean", "date", "enum", "oneof", "relationship"];

/** Author-facing Kind groups for the field editor (plan §5.4). */
export const FIELD_KIND_GROUPS = {
  basic: {
    label: "Basic",
    types: ["text", "number", "boolean", "date", "enum"] as const satisfies readonly FieldType[],
  },
  linking: {
    label: "Linking",
    types: ["relationship"] as const satisfies readonly FieldType[],
  },
  advanced: {
    label: "Advanced",
    types: ["oneof"] as const satisfies readonly FieldType[],
  },
} as const;

export const METADATA_FIELD_TYPES = ["text", "number", "boolean", "date", "enum", "oneof"] as const;
export type MetadataFieldType = (typeof METADATA_FIELD_TYPES)[number];

export type MetadataFieldDraft = {
  key: string;
  label: string;
  type: MetadataFieldType;
  required: boolean;
  options: string;
  oneOf: Array<{
    label: string;
    type: Exclude<MetadataFieldType, "oneof" | "relationship">;
    options: string;
  }>;
};

export type SchemaWorkbenchTab = "types" | "fields" | "templates";

export type SchemaStatusFilter = "all" | "enabled" | "disabled" | "custom" | "builtin";

export type PackageSchemaSlice = {
  namespace: string;
  entityTypes: EntityTypeDefinition[];
  fields: FieldDefinition[];
};

export type PackageManifestSlice = {
  schemas: PackageSchemaSlice[];
  templates: EntityTemplate[];
};

/** Flat package model built from every schema namespace (not schemas[0] only). */
export type FlattenedPackageSchema = {
  namespaces: string[];
  entityTypes: EntityTypeDefinition[];
  fields: FieldDefinition[];
  /** Map entity type id → owning schema namespace. */
  typeNamespace: Record<string, string>;
  /** Map field key → owning schema namespace. */
  fieldNamespace: Record<string, string>;
};

export type SchemaListItemKind = "type" | "field" | "template";

export type SchemaListItem = {
  id: string;
  kind: SchemaListItemKind;
  name: string;
  origin: "builtin" | "custom";
  enabled: boolean;
  namespace?: string;
  hint?: string;
  searchText: string;
};

export function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function localTypeId(value: string): string {
  const colon = value.lastIndexOf(":");
  return colon >= 0 ? value.slice(colon + 1) : value;
}

/** Show `currentOwner` / `word_count` / `daena.timeline:event` as readable labels. */
export function humanizeId(value: string): string {
  const spaced = localTypeId(value)
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[_-]+/g, " ")
    .trim()
    .replace(/\s+/g, " ");
  if (!spaced) return localTypeId(value) || value;
  return spaced.replace(/\b\w/g, (char) => char.toUpperCase());
}

export function fieldTypeLabel(type: FieldType): string {
  if (type === "oneof") return "One of";
  if (type === "boolean") return "Yes/No";
  return type.charAt(0).toUpperCase() + type.slice(1);
}

export function fieldKindGroupLabel(type: FieldType): string {
  if ((FIELD_KIND_GROUPS.basic.types as readonly string[]).includes(type)) {
    return FIELD_KIND_GROUPS.basic.label;
  }
  if ((FIELD_KIND_GROUPS.linking.types as readonly string[]).includes(type)) {
    return FIELD_KIND_GROUPS.linking.label;
  }
  return FIELD_KIND_GROUPS.advanced.label;
}

export function parseOptions(value: string): string[] {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

export function formatOptions(options?: string[] | null): string {
  return (options ?? []).join(", ");
}

export function parseOneOfVariants(
  value: string,
): Array<{ label: string; type: FieldType; options?: string[] }> | null {
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
export function slugifyTypeId(value: string): string {
  return value
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** IDs must start with a-z per host validation. */
export function ensureTypeId(value: string, fallback = "custom"): string {
  let id = slugifyTypeId(localTypeId(value));
  if (!id) id = fallback;
  if (!/^[a-z]/.test(id)) id = `${fallback}-${id}`;
  return id;
}

export function qualifyTypeId(id: string, pluginId?: string | null): string {
  const trimmed = id.trim();
  if (!trimmed) return "";
  if (trimmed.includes(":")) return trimmed;
  return pluginId ? `${pluginId}:${trimmed}` : trimmed;
}

export function preserveTypeId(id: string, pluginId?: string | null): string {
  const trimmed = id.trim();
  if (!trimmed) return "";
  if (trimmed.includes(":")) return trimmed;
  return qualifyTypeId(ensureTypeId(trimmed, "type"), pluginId);
}

export function mintTypeId(name: string, pluginId?: string | null): string {
  return qualifyTypeId(ensureTypeId(name, "type"), pluginId);
}

/** Field keys: lowercase snake (`WordCount` → `word_count`). */
export function slugifyFieldKey(value: string): string {
  return value
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_+|_+$/g, "");
}

export function ensureFieldKey(value: string, fallback = "field"): string {
  let key = slugifyFieldKey(value);
  if (!key) key = fallback;
  if (!/^[a-z]/.test(key)) key = `${fallback}_${key}`;
  return key;
}

export function metadataDraftToDefinition(draft: MetadataFieldDraft): Record<string, unknown> | null {
  const label = draft.label.trim();
  if (!label) return null;
  const key = ensureFieldKey(draft.key.trim() || label, "field");
  if (!key) return null;
  const base: Record<string, unknown> = { key, label, type: draft.type };
  if (draft.required) base.required = true;
  if (draft.type === "enum") {
    const opts = parseOptions(draft.options);
    if (opts.length === 0) return null;
    base.options = opts;
  } else if (draft.type === "oneof") {
    if (draft.oneOf.length === 0) return null;
    const oneOf = draft.oneOf
      .filter((v) => v.label.trim())
      .map((v) => {
        const variant: Record<string, unknown> = { label: v.label.trim(), type: v.type };
        if (v.type === "enum") {
          const opts = parseOptions(v.options);
          if (opts.length === 0) return null;
          variant.options = opts;
        }
        return variant;
      })
      .filter(Boolean) as Array<Record<string, unknown>>;
    if (oneOf.length === 0) return null;
    base.oneOf = oneOf;
  }
  return base;
}

export function validateMetadataDrafts(drafts: MetadataFieldDraft[]): boolean {
  if (drafts.length === 0) return true;
  const keys = new Set<string>();
  for (const d of drafts) {
    const def = metadataDraftToDefinition(d);
    if (!def) return false;
    const k = def.key as string;
    if (keys.has(k)) return false;
    keys.add(k);
    if (def.type === "enum" && !(def.options as string[]).length) return false;
    if (def.type === "oneof") {
      const variants = def.oneOf as unknown[];
      if (!variants.length) return false;
      for (const v of d.oneOf) {
        if (!v.label.trim()) return false;
        if (v.type === "enum" && !parseOptions(v.options).length) return false;
      }
    }
  }
  return true;
}

export function draftsFromMetadataFields(fields: unknown): MetadataFieldDraft[] {
  if (!Array.isArray(fields)) return [];
  return (fields as Array<Record<string, unknown>>).map((f) => ({
    key: String(f.key ?? ""),
    label: String(f.label ?? ""),
    type: (METADATA_FIELD_TYPES as readonly string[]).includes(String(f.type)) ? (f.type as MetadataFieldType) : "text",
    required: Boolean(f.required),
    options: formatOptions(f.options as string[] | undefined),
    oneOf: Array.isArray(f.oneOf)
      ? (f.oneOf as Array<Record<string, unknown>>).map((v) => {
          const vt = String(v.type ?? "");
          const allowed = ["text", "number", "boolean", "date", "enum"] as const as readonly string[];
          return {
            label: String(v.label ?? ""),
            type: (allowed.includes(vt) ? vt : "text") as Exclude<MetadataFieldType, "oneof" | "relationship">,
            options: formatOptions(v.options as string[] | undefined),
          };
        })
      : [],
  }));
}

function normalizeMetadataFields(raw: unknown[]): unknown[] {
  const normalized = raw
    .map((entry) => {
      const e = entry as Record<string, unknown>;
      const label = String(e.label ?? "").trim();
      const rawKey = String(e.key ?? "").trim() || label;
      const key = ensureFieldKey(rawKey, "field");
      const t = String(e.type ?? "text");
      const allowed = METADATA_FIELD_TYPES as readonly string[];
      const type = allowed.includes(t) ? t : "text";
      const out: Record<string, unknown> = { key, label, type };
      if (e.required) out.required = true;
      if (type === "enum" && Array.isArray(e.options)) out.options = (e.options as string[]).filter(Boolean);
      if (type === "oneof" && Array.isArray(e.oneOf)) {
        out.oneOf = (e.oneOf as Array<Record<string, unknown>>)
          .filter((v) => String(v.label ?? "").trim())
          .map((v) => {
            const vl = String(v.label ?? "").trim();
            const vt = String(v.type ?? "text");
            const ok = ["text", "number", "boolean", "date", "enum"].includes(vt) ? vt : "text";
            const ov: Record<string, unknown> = { label: vl, type: ok };
            if (ok === "enum" && Array.isArray(v.options)) ov.options = (v.options as string[]).filter(Boolean);
            return ov;
          });
      }
      if (type === "enum" && Array.isArray(out.options)) {
        out.options = [...new Set(out.options as string[])];
      }
      return out;
    })
    .filter((e) => (e as Record<string, unknown>).label && (e as Record<string, unknown>).key)
    .sort((a, b) =>
      String((a as Record<string, unknown>).key).localeCompare(String((b as Record<string, unknown>).key)),
    );
  const seen = new Set<string>();
  const deduped: unknown[] = [];
  for (const e of normalized) {
    const k = String((e as Record<string, unknown>).key);
    if (seen.has(k)) continue;
    seen.add(k);
    deduped.push(e);
  }
  return deduped;
}

export type NormalizeOverlayOptions = {
  pluginId?: string | null;
};

/**
 * Canonicalize an overlay draft for dirty checks and save.
 * Must remain byte-stable: normalize(normalize(x)) === normalize(x).
 */
export function normalizeOverlay(
  value: ModuleSchemaOverlay,
  options: NormalizeOverlayOptions = {},
): ModuleSchemaOverlay {
  const pluginId = options.pluginId ?? null;
  const customEntityTypes = cloneJson(value.customEntityTypes ?? [])
    .map((entityType) => ({
      id: preserveTypeId(entityType.id, pluginId),
      name: entityType.name.trim() || humanizeId(entityType.id),
      icon: entityType.icon ?? FALLBACK_ICON,
      iconColor: entityType.iconColor ?? DEFAULT_TYPE_COLOR,
    }))
    .filter((entityType, index, all) => all.findIndex((candidate) => candidate.id === entityType.id) === index)
    .sort((left, right) => left.id.localeCompare(right.id));
  const customFields = cloneJson(value.customFields ?? []).map((field) => {
    const f = field as unknown as Record<string, unknown>;
    const next: Record<string, unknown> = {
      ...f,
      key: ensureFieldKey(String(f.key ?? "")),
      entityTypes: (f.entityTypes as string[] | undefined)?.map((id) => preserveTypeId(id, pluginId)).filter(Boolean),
      targetEntityTypes: (f.targetEntityTypes as string[] | undefined)
        ?.map((id) => preserveTypeId(id, pluginId))
        .filter(Boolean),
    };
    if (f.type === "relationship" && Array.isArray(f.metadataFields)) {
      const deduped = normalizeMetadataFields(f.metadataFields as unknown[]);
      (next as Record<string, unknown>).metadataFields = deduped;
      if (deduped.length === 0) delete (next as Record<string, unknown>).metadataFields;
    } else {
      delete (next as Record<string, unknown>).metadataFields;
    }
    return next as unknown as FieldDefinition;
  });
  const customTemplates = cloneJson(value.customTemplates ?? []).map((template) => ({
    ...template,
    id: ensureTypeId(template.id || template.name, "template"),
    entityType: preserveTypeId(template.entityType, pluginId),
    description: template.description?.trim() ? template.description.trim() : null,
  }));
  const fieldScopeOverrides = cloneJson(value.fieldScopeOverrides ?? [])
    .map((scope) => ({
      fieldKey: scope.fieldKey,
      entityTypes: [...new Set(scope.entityTypes.map((id) => preserveTypeId(id, pluginId)).filter(Boolean))].sort(),
    }))
    .sort((left, right) => left.fieldKey.localeCompare(right.fieldKey));
  const templateOverrides = cloneJson(value.templateOverrides ?? []).sort((left, right) =>
    left.templateId.localeCompare(right.templateId),
  );
  const fieldMetadataOverrides = cloneJson(value.fieldMetadataOverrides ?? [])
    .map((ov) => {
      const rawFields = (ov as unknown as Record<string, unknown>).metadataFields as unknown[] | undefined;
      const deduped = normalizeMetadataFields(rawFields ?? []);
      return {
        fieldKey: String((ov as unknown as Record<string, unknown>).fieldKey ?? ""),
        metadataFields: deduped as unknown as FieldMetadataOverride["metadataFields"],
      };
    })
    .filter((ov) => ov.fieldKey && ov.metadataFields && ov.metadataFields.length > 0)
    .sort((a, b) => a.fieldKey.localeCompare(b.fieldKey));
  const disabledEntityTypes = new Set(value.disabledEntityTypes ?? []);
  const entityTypeAppearanceOverrides = cloneJson(value.entityTypeAppearanceOverrides ?? [])
    .map((override) => ({
      entityTypeId: preserveTypeId(override.entityTypeId, pluginId),
      ...(override.icon ? { icon: override.icon } : {}),
      ...(override.iconColor ? { iconColor: override.iconColor } : {}),
    }))
    .filter((override) => override.entityTypeId && (override.icon || override.iconColor))
    .filter((override) => !disabledEntityTypes.has(override.entityTypeId))
    .sort((left, right) => left.entityTypeId.localeCompare(right.entityTypeId));
  const fieldTimelineOverrides = cloneJson(
    ((value as unknown as Record<string, unknown>).fieldTimelineOverrides as unknown[] | undefined) ?? [],
  )
    .map((ov: unknown) => {
      const raw = ov as unknown as Record<string, unknown>;
      const fieldKey = ensureFieldKey(String(raw.fieldKey ?? ""));
      if (!fieldKey) return null;
      const t = raw.timeline as Record<string, unknown> | null | undefined;
      if (t === null) return { fieldKey, timeline: null as unknown as null };
      if (t === undefined) return { fieldKey, timeline: null as unknown as null };
      if (typeof t !== "object") return null;
      const role = String((t as Record<string, unknown>).role ?? "")
        .trim()
        .toLowerCase();
      if (!["point", "start", "end"].includes(role)) return null;
      const group =
        (t as Record<string, unknown>).group != null ? String((t as Record<string, unknown>).group).trim() : undefined;
      const label =
        (t as Record<string, unknown>).label != null ? String((t as Record<string, unknown>).label).trim() : undefined;
      const layerRaw =
        (t as Record<string, unknown>).layer != null
          ? String((t as Record<string, unknown>).layer)
              .trim()
              .toLowerCase()
          : undefined;
      const layer = layerRaw && ["dates", "lifelines"].includes(layerRaw) ? layerRaw : undefined;
      const timeline: Record<string, unknown> = { role };
      if (group) timeline.group = group;
      if (label) timeline.label = label;
      if (layer) timeline.layer = layer;
      return {
        fieldKey,
        timeline: timeline as unknown as { role: string; group?: string; label?: string; layer?: string },
      };
    })
    .filter((ov: unknown): ov is NonNullable<typeof ov> => ov !== null)
    .filter(
      (ov: unknown, index: number, all: unknown[]) =>
        (all as Array<{ fieldKey: string }>).findIndex((c) => c.fieldKey === (ov as { fieldKey: string }).fieldKey) ===
        index,
    )
    .sort((left: unknown, right: unknown) =>
      (left as { fieldKey: string }).fieldKey.localeCompare((right as { fieldKey: string }).fieldKey),
    ) as unknown as Array<Record<string, unknown>>;
  return {
    version: value.version || 1,
    disabledEntityTypes: [...(value.disabledEntityTypes ?? [])]
      .map((id) => preserveTypeId(id, pluginId))
      .filter(Boolean)
      .sort(),
    disabledFields: [...(value.disabledFields ?? [])].sort(),
    disabledTemplates: [...(value.disabledTemplates ?? [])].sort(),
    customEntityTypes,
    customFields,
    customTemplates,
    fieldScopeOverrides,
    templateOverrides,
    fieldMetadataOverrides,
    entityTypeAppearanceOverrides,
    fieldTimelineOverrides,
  } as unknown as ModuleSchemaOverlay;
}

export function fingerprint(value: ModuleSchemaOverlay, options: NormalizeOverlayOptions = {}): string {
  return JSON.stringify(normalizeOverlay(value, options));
}

/** True when the overlay differs from an empty default (author has customized). */
export function overlayIsCustomized(overlay: ModuleSchemaOverlay | null | undefined): boolean {
  if (!overlay) return false;
  const normalized = normalizeOverlay(overlay);
  if ((normalized.disabledEntityTypes?.length ?? 0) > 0) return true;
  if ((normalized.disabledFields?.length ?? 0) > 0) return true;
  if ((normalized.disabledTemplates?.length ?? 0) > 0) return true;
  if ((normalized.customEntityTypes?.length ?? 0) > 0) return true;
  if ((normalized.customFields?.length ?? 0) > 0) return true;
  if ((normalized.customTemplates?.length ?? 0) > 0) return true;
  if ((normalized.fieldScopeOverrides?.length ?? 0) > 0) return true;
  if ((normalized.templateOverrides?.length ?? 0) > 0) return true;
  if ((normalized.fieldMetadataOverrides?.length ?? 0) > 0) return true;
  if ((normalized.entityTypeAppearanceOverrides?.length ?? 0) > 0) return true;
  if ((normalized.fieldTimelineOverrides?.length ?? 0) > 0) return true;
  return false;
}

/**
 * Build a normalized package view from all schema namespaces.
 * First-seen id/key wins; later duplicates are skipped.
 */
export function flattenPackageSchemas(manifest: PackageManifestSlice): FlattenedPackageSchema {
  const namespaces: string[] = [];
  const entityTypes: EntityTypeDefinition[] = [];
  const fields: FieldDefinition[] = [];
  const typeNamespace: Record<string, string> = {};
  const fieldNamespace: Record<string, string> = {};
  const seenTypes = new Set<string>();
  const seenFields = new Set<string>();

  for (const schema of manifest.schemas ?? []) {
    const ns = schema.namespace?.trim() || "default";
    if (!namespaces.includes(ns)) namespaces.push(ns);
    for (const entityType of schema.entityTypes ?? []) {
      if (seenTypes.has(entityType.id)) continue;
      seenTypes.add(entityType.id);
      entityTypes.push(entityType);
      typeNamespace[entityType.id] = ns;
    }
    for (const field of schema.fields ?? []) {
      if (seenFields.has(field.key)) continue;
      seenFields.add(field.key);
      fields.push(field);
      fieldNamespace[field.key] = ns;
    }
  }

  entityTypes.sort((left, right) => left.name.localeCompare(right.name));
  fields.sort((left, right) => left.label.localeCompare(right.label));

  return { namespaces, entityTypes, fields, typeNamespace, fieldNamespace };
}

export function defaultFieldValue(field: FieldDefinition): unknown {
  if (field.type === "boolean") return false;
  if (field.type === "relationship") return [];
  if (field.type === "enum" && field.multiple) return [];
  if (field.type === "number") return "";
  return "";
}

export function matchesSchemaSearch(item: SchemaListItem, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return item.searchText.toLowerCase().includes(q);
}

export function matchesStatusFilter(item: SchemaListItem, filter: SchemaStatusFilter): boolean {
  switch (filter) {
    case "all":
      return true;
    case "enabled":
      return item.enabled;
    case "disabled":
      return !item.enabled;
    case "custom":
      return item.origin === "custom";
    case "builtin":
      return item.origin === "builtin";
    default:
      return true;
  }
}

export function filterSchemaListItems(
  items: SchemaListItem[],
  query: string,
  status: SchemaStatusFilter,
): SchemaListItem[] {
  return items.filter((item) => matchesSchemaSearch(item, query) && matchesStatusFilter(item, status));
}

export function summarizePackageCounts(manifest: PackageManifestSlice, overlay: ModuleSchemaOverlay) {
  const flat = flattenPackageSchemas(manifest);
  const disabledTypes = new Set(overlay.disabledEntityTypes ?? []);
  const disabledFields = new Set(overlay.disabledFields ?? []);
  const disabledTemplates = new Set(overlay.disabledTemplates ?? []);
  const activeTypes =
    flat.entityTypes.filter((t) => !disabledTypes.has(t.id)).length + (overlay.customEntityTypes?.length ?? 0);
  const activeFields =
    flat.fields.filter((f) => !disabledFields.has(f.key)).length + (overlay.customFields?.length ?? 0);
  const packageTemplates = manifest.templates ?? [];
  const activeTemplates =
    packageTemplates.filter((t) => !disabledTemplates.has(t.id)).length + (overlay.customTemplates?.length ?? 0);
  return {
    types: activeTypes,
    fields: activeFields,
    templates: activeTemplates,
    namespaces: flat.namespaces.length,
    customized: overlayIsCustomized(overlay),
  };
}

/** Author-facing advanced property keys hidden behind progressive disclosure. */
export const ADVANCED_AUTHOR_TERMS = [
  "stable id",
  "key",
  "relationship type",
  "metadata key",
  "namespace",
  "qualification",
  "timeline role",
  "timeline group",
  "timeline layer",
  "one-of variant",
] as const;

export type AdvancedControlId =
  | "stable-id"
  | "field-key"
  | "relationship-type"
  | "metadata-key"
  | "namespace"
  | "qualification"
  | "timeline-role"
  | "timeline-group"
  | "timeline-layer"
  | "oneof-variants";

/** Map Advanced toggle → which controls become visible. */
export function showAdvancedControl(showAdvanced: boolean, control: AdvancedControlId): boolean {
  if (showAdvanced) return true;
  // Default author path hides contract internals listed in ADVANCED_AUTHOR_TERMS.
  void ADVANCED_AUTHOR_TERMS;
  return false;
}

export type ExclusiveFieldDisposition =
  { action: "remove" } | { action: "disable" } | { action: "reassign"; toTypeId: string };

/** How to handle live entities that still use the type being removed. */
export type EntityRemovalDisposition = { action: "none" } | { action: "reassign"; toTypeId: string };

export type TypeRemovalPlan = {
  typeId: string;
  exclusiveDispositions: Record<string, ExclusiveFieldDisposition>;
  removeSharedFieldKeys: string[];
  /** Required when entityCount > 0; optional/none when zero. */
  entityDisposition?: EntityRemovalDisposition;
  entityCount?: number | null;
};

/** Drop every overlay reference to a removed custom type id. */
export function pruneOverlayForRemovedType(
  overlay: ModuleSchemaOverlay,
  typeId: string,
  options: {
    removeFieldKeys?: Set<string>;
    disableFieldKeys?: Set<string>;
  } = {},
): ModuleSchemaOverlay {
  const removeFieldKeys = options.removeFieldKeys ?? new Set<string>();
  const disableFieldKeys = options.disableFieldKeys ?? new Set<string>();
  const removedTemplateIds = new Set(
    (overlay.customTemplates ?? []).filter((template) => template.entityType === typeId).map((template) => template.id),
  );

  const customFields = (overlay.customFields ?? [])
    .filter((field) => !removeFieldKeys.has(field.key))
    .map((field) => {
      if (!field.entityTypes) return field;
      const remaining = field.entityTypes.filter((type) => type !== typeId);
      // Empty remaining would broaden scope to all types — drop instead.
      if (remaining.length === 0) return null;
      return { ...field, entityTypes: remaining };
    })
    .filter((field): field is FieldDefinition => field !== null);

  const disabledFields = [...new Set([...(overlay.disabledFields ?? []), ...disableFieldKeys])].sort();

  return {
    ...overlay,
    disabledEntityTypes: (overlay.disabledEntityTypes ?? []).filter((id) => id !== typeId),
    disabledFields,
    customEntityTypes: (overlay.customEntityTypes ?? []).filter((item) => item.id !== typeId),
    customFields,
    customTemplates: (overlay.customTemplates ?? [])
      .filter((template) => template.entityType !== typeId)
      .map((template) => {
        if (removeFieldKeys.size === 0) return template;
        const fields = { ...(template.fields as Record<string, unknown>) };
        for (const key of removeFieldKeys) delete fields[key];
        return {
          ...template,
          fields,
          requiredFields: template.requiredFields?.filter((item) => !removeFieldKeys.has(item)) ?? null,
        };
      }),
    fieldScopeOverrides: (overlay.fieldScopeOverrides ?? [])
      .map((scope) => ({
        ...scope,
        entityTypes: scope.entityTypes.filter((id) => id !== typeId),
      }))
      .filter((scope) => scope.entityTypes.length > 0),
    templateOverrides: (overlay.templateOverrides ?? []).filter(
      (override) => !removedTemplateIds.has(override.templateId),
    ),
    fieldMetadataOverrides: (overlay.fieldMetadataOverrides ?? []).filter(
      (override) => !removeFieldKeys.has(override.fieldKey),
    ),
    entityTypeAppearanceOverrides: (overlay.entityTypeAppearanceOverrides ?? []).filter(
      (override) => override.entityTypeId !== typeId,
    ),
    fieldTimelineOverrides: (overlay.fieldTimelineOverrides ?? []).filter(
      (override) => !removeFieldKeys.has(override.fieldKey),
    ),
  };
}

/**
 * Apply a validated type-removal plan. Callers must ensure every exclusive field
 * has an explicit disposition before invoking.
 */
export function applyTypeRemovalPlan(overlay: ModuleSchemaOverlay, plan: TypeRemovalPlan): ModuleSchemaOverlay {
  const removeFieldKeys = new Set<string>();
  const disableFieldKeys = new Set<string>();
  const reassignments = new Map<string, string>();

  for (const [fieldKey, disposition] of Object.entries(plan.exclusiveDispositions)) {
    if (disposition.action === "remove") removeFieldKeys.add(fieldKey);
    else if (disposition.action === "disable") {
      disableFieldKeys.add(fieldKey);
      removeFieldKeys.add(fieldKey);
    } else if (disposition.action === "reassign") {
      reassignments.set(fieldKey, disposition.toTypeId);
    }
  }
  for (const key of plan.removeSharedFieldKeys) removeFieldKeys.add(key);

  let next: ModuleSchemaOverlay = {
    ...overlay,
    customFields: (overlay.customFields ?? []).map((field) => {
      const target = reassignments.get(field.key);
      if (target) return { ...field, entityTypes: [target] };
      if (!field.entityTypes?.includes(plan.typeId)) return field;
      const remaining = field.entityTypes.filter((type) => type !== plan.typeId);
      // Shared fields keep remaining scopes; exclusive without disposition are invalid.
      return { ...field, entityTypes: remaining };
    }),
  };

  next = pruneOverlayForRemovedType(next, plan.typeId, { removeFieldKeys, disableFieldKeys });
  return next;
}

export function typeRemovalPlanIsComplete(
  exclusiveFieldKeys: string[],
  dispositions: Record<string, ExclusiveFieldDisposition | undefined>,
  availableTypeIds: string[],
  options: {
    entityCount?: number | null;
    entityDisposition?: EntityRemovalDisposition | undefined;
  } = {},
): boolean {
  for (const key of exclusiveFieldKeys) {
    const disposition = dispositions[key];
    if (!disposition) return false;
    if (disposition.action === "reassign") {
      if (!disposition.toTypeId || !availableTypeIds.includes(disposition.toTypeId)) return false;
    }
  }
  const entityCount = options.entityCount;
  const entityDisposition = options.entityDisposition;
  if (entityCount != null && entityCount > 0) {
    if (entityDisposition?.action !== "reassign") return false;
    if (!entityDisposition.toTypeId || !availableTypeIds.includes(entityDisposition.toTypeId)) return false;
  } else if (entityCount === null) {
    // Count unknown: author must either reassign to a valid type or explicitly confirm none.
    if (!entityDisposition) return false;
    if (entityDisposition.action === "reassign") {
      if (!entityDisposition.toTypeId || !availableTypeIds.includes(entityDisposition.toTypeId)) return false;
    }
  }
  return true;
}

export type TypeEditorUsage = {
  fieldCount: number;
  templateCount: number;
  fieldLabels: string[];
  templateLabels: string[];
  projectionLabels: string[];
  entityCount: number | null;
};

export function summarizeTypeUsage(options: {
  typeId: string;
  fields: FieldDefinition[];
  templates: EntityTemplate[];
  projectionLabels?: string[];
  entityCount?: number | null;
}): TypeEditorUsage {
  const applying = options.fields.filter(
    (field) => !field.entityTypes || field.entityTypes.length === 0 || field.entityTypes.includes(options.typeId),
  );
  const creating = options.templates.filter((template) => template.entityType === options.typeId);
  return {
    fieldCount: applying.length,
    templateCount: creating.length,
    fieldLabels: applying.map((field) => field.label || field.key),
    templateLabels: creating.map((template) => template.name || template.id),
    projectionLabels: options.projectionLabels ?? [],
    entityCount: options.entityCount ?? null,
  };
}

export type FieldFormErrors = {
  name?: string;
  key?: string;
  choices?: string;
  oneOf?: string;
  relationshipType?: string;
  targetTypes?: string;
  cardinality?: string;
  metadata?: string;
  timelineGroup?: string;
  appliesTo?: string;
};

export function validateFieldForm(input: {
  label: string;
  key?: string;
  type: FieldType;
  optionsText?: string;
  oneOfVariants?: Array<{ label: string; type: string; options: string }>;
  relationshipType?: string;
  targetEntityTypes?: string[];
  cardinality?: "one" | "many" | string | null;
  metadataValid?: boolean;
  metadataDrafts?: MetadataFieldDraft[];
  shared?: boolean;
  timelineEnabled?: boolean;
  timelineRole?: "point" | "start" | "end";
  timelineGroup?: string;
  existingKeys?: string[];
  editingKey?: string | null;
}): FieldFormErrors {
  const errors: FieldFormErrors = {};
  if (!input.label.trim()) errors.name = "Name is required.";
  const key = ensureFieldKey(input.key?.trim() || input.label, "field");
  if (!key) errors.key = "Key is required.";
  const existing = new Set((input.existingKeys ?? []).filter((candidate) => candidate !== input.editingKey));
  if (key && existing.has(key)) errors.key = "Key must be unique.";

  if (input.type === "enum") {
    const opts = parseOptions(input.optionsText ?? "");
    if (opts.length === 0) errors.choices = "Add at least one choice.";
    else if (new Set(opts).size !== opts.length) errors.choices = "Choices must be unique.";
  }
  if (input.type === "oneof") {
    const variants = input.oneOfVariants ?? [];
    if (!variants.some((v) => v.label.trim())) errors.oneOf = "Add at least one variant.";
    for (const variant of variants) {
      if (!variant.label.trim()) {
        errors.oneOf = "Each variant needs a label.";
        break;
      }
      if (variant.type === "enum" && parseOptions(variant.options).length === 0) {
        errors.oneOf = `Variant “${variant.label}” needs choices.`;
        break;
      }
    }
  }
  if (input.type === "relationship") {
    if (!ensureTypeId(input.relationshipType?.trim() || input.label, "relationship")) {
      errors.relationshipType = "Relationship type is required.";
    }
    if (!(input.targetEntityTypes?.length ?? 0)) errors.targetTypes = "Choose at least one target type.";
    const cardinality = String(input.cardinality ?? "").trim();
    if (cardinality !== "one" && cardinality !== "many") {
      errors.cardinality = "Cardinality must be one or many.";
    }
    if (input.metadataDrafts && input.metadataDrafts.length > 0 && input.metadataValid === false) {
      errors.metadata = "Fix metadata attributes (unique keys, choices, variants).";
    }
  }
  if (input.type === "date" && input.shared && input.timelineEnabled) {
    if ((input.timelineRole === "start" || input.timelineRole === "end") && !input.timelineGroup?.trim()) {
      errors.timelineGroup = "Start/end dates need a Timeline group.";
    }
  }
  return errors;
}

export function fieldFormHasErrors(errors: FieldFormErrors): boolean {
  return Object.keys(errors).length > 0;
}

/** Prefer a namespace that owns the field/type; else first package schema namespace. */
export function primarySchemaNamespace(
  schemas:
    Array<{ namespace: string; entityTypes?: Array<{ id: string }>; fields?: Array<{ key: string }> }> | undefined,
  options?: { entityType?: string | null; fieldKey?: string | null; fallback?: string },
): string {
  const list = schemas ?? [];
  if (options?.fieldKey) {
    const match = list.find((schema) => schema.fields?.some((field) => field.key === options.fieldKey));
    if (match?.namespace) return match.namespace;
  }
  if (options?.entityType) {
    const match = list.find((schema) => schema.entityTypes?.some((type) => type.id === options.entityType));
    if (match?.namespace) return match.namespace;
  }
  if (list[0]?.namespace) return list[0].namespace;
  return options?.fallback ?? "default";
}

export function overlayValidationStatus(overlay: ModuleSchemaOverlay | null | undefined): {
  status: "ok" | "error" | "unknown";
  message?: string;
} {
  if (!overlay) return { status: "unknown" };
  try {
    const normalized = normalizeOverlay(overlay);
    const emptyKeys = (normalized.customFields ?? []).filter((field) => !field.key || !field.label);
    if (emptyKeys.length > 0) {
      return { status: "error", message: `${emptyKeys.length} field(s) missing name or key` };
    }
    const emptyTypes = (normalized.customEntityTypes ?? []).filter((type) => !type.id || !type.name.trim());
    if (emptyTypes.length > 0) {
      return { status: "error", message: `${emptyTypes.length} type(s) missing name or id` };
    }
    return { status: "ok" };
  } catch (cause) {
    return { status: "error", message: cause instanceof Error ? cause.message : "Invalid overlay" };
  }
}
