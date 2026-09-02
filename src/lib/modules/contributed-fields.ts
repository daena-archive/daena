import { fieldAppliesToEntity, type SchemaFieldLike, type SchemaOverlayLike } from "./fields.ts";

export type RelationshipDirection = "outgoing" | "incoming" | "undirected";

export type RelationshipFieldLike = SchemaFieldLike & {
  key: string;
  label?: string;
  relationshipType?: string;
  relationshipDirection?: RelationshipDirection | string;
  relationshipConstraints?: { unique?: string };
  metadataFields?: Array<{
    key: string;
    label?: string;
    type: string;
    required?: boolean | null;
    options?: string[] | null;
  }>;
};

export type SchemaLike = {
  entityTypes?: Array<{ id: string }>;
  fields?: RelationshipFieldLike[];
};

export type ManifestLike = {
  id: string;
  name?: string;
  enabled?: boolean;
  schemas?: SchemaLike[];
};

export type RelationshipFieldGroup<T extends RelationshipFieldLike = RelationshipFieldLike> = {
  moduleId: string;
  moduleName: string;
  fields: T[];
};

export type RelationshipModuleGroup<T extends RelationshipLike = RelationshipLike> = {
  moduleId: string;
  moduleName: string;
  relationships: T[];
};

export type RelationshipAttributeRow = {
  key: string;
  label: string;
  raw: unknown;
};

export type RelationshipLike = {
  id: string;
  source_id?: string;
  target_id?: string;
  sourceId?: string;
  targetId?: string;
  relationship_type?: string;
  type?: string;
};

export function relationshipDirection(field: RelationshipFieldLike): RelationshipDirection {
  const declared = field.relationshipDirection;
  if (declared === "incoming" || declared === "outgoing" || declared === "undirected") return declared;
  if (field.relationshipConstraints?.unique === "undirected") return "undirected";
  return "outgoing";
}

export function relationshipSourceId(relationship: RelationshipLike): string {
  return relationship.source_id ?? relationship.sourceId ?? "";
}

export function relationshipTargetId(relationship: RelationshipLike): string {
  return relationship.target_id ?? relationship.targetId ?? "";
}

export function relationshipTypeOf(relationship: RelationshipLike): string {
  return relationship.relationship_type ?? relationship.type ?? "";
}

export function counterpartId(
  entityId: string,
  relationship: RelationshipLike,
  field: RelationshipFieldLike,
): string | null {
  if (relationshipTypeOf(relationship) !== field.relationshipType) return null;
  const source = relationshipSourceId(relationship);
  const target = relationshipTargetId(relationship);
  const direction = relationshipDirection(field);
  if (direction === "incoming") {
    if (target !== entityId || source === entityId) return null;
    return source;
  }
  if (direction === "undirected") {
    if (source === entityId && target !== entityId) return target;
    if (target === entityId && source !== entityId) return source;
    return null;
  }
  if (source !== entityId || target === entityId) return null;
  return target;
}

export function relationshipsForField<T extends RelationshipLike>(
  entityId: string,
  relationships: readonly T[],
  field: RelationshipFieldLike,
): T[] {
  if (!field.relationshipType) return [];
  return relationships.filter((relationship) => counterpartId(entityId, relationship, field) !== null);
}

export function coveredRelationshipIds(
  entityId: string,
  relationships: readonly RelationshipLike[],
  fields: readonly RelationshipFieldLike[],
): Set<string> {
  const ids = new Set<string>();
  for (const field of fields) {
    if (field.type !== "relationship") continue;
    for (const relationship of relationshipsForField(entityId, relationships, field)) {
      ids.add(relationship.id);
    }
  }
  return ids;
}

export function counterpartIds(
  entityId: string,
  relationships: readonly RelationshipLike[],
  field: RelationshipFieldLike,
): string[] {
  const ids: string[] = [];
  for (const relationship of relationships) {
    const id = counterpartId(entityId, relationship, field);
    if (id) ids.push(id);
  }
  return ids;
}

export function endpointsForCreate(
  entityId: string,
  otherId: string,
  field: RelationshipFieldLike,
): { sourceId: string; targetId: string } {
  const direction = relationshipDirection(field);
  if (direction === "incoming") return { sourceId: otherId, targetId: entityId };
  if (direction === "undirected") {
    return entityId < otherId ? { sourceId: entityId, targetId: otherId } : { sourceId: otherId, targetId: entityId };
  }
  return { sourceId: entityId, targetId: otherId };
}

export function defaultRelationshipMetadata(field: RelationshipFieldLike): Record<string, unknown> {
  const metadata: Record<string, unknown> = {};
  for (const meta of field.metadataFields ?? []) {
    if (!meta.required) continue;
    if (meta.type === "enum" && meta.options?.[0]) metadata[meta.key] = meta.options[0];
  }
  return metadata;
}

export function fieldsApplyingToEntity<T extends RelationshipFieldLike>(
  manifest: ManifestLike,
  entityType: string | null | undefined,
  enabledEntityTypes: ReadonlySet<string> | null,
  overlay: SchemaOverlayLike | null = null,
): T[] {
  const fields: T[] = [];
  for (const schema of manifest.schemas ?? []) {
    for (const field of schema.fields ?? []) {
      if (fieldAppliesToEntity(field, entityType, enabledEntityTypes, overlay)) fields.push(field as T);
    }
  }
  return fields;
}

export function contributedRelationshipFields<T extends RelationshipFieldLike>(
  activeManifest: ManifestLike | null | undefined,
  entityType: string | null | undefined,
  enabledManifests: readonly ManifestLike[],
  enabledEntityTypes: ReadonlySet<string> | null,
  overlay: SchemaOverlayLike | null = null,
): T[] {
  const own = activeManifest ? fieldsApplyingToEntity<T>(activeManifest, entityType, enabledEntityTypes, overlay) : [];
  const keys = new Set(own.map((field) => field.key));
  const contributed: T[] = [];
  for (const manifest of enabledManifests) {
    if (!activeManifest) break;
    if (manifest.id === activeManifest.id) continue;
    if (manifest.enabled === false) continue;
    for (const field of fieldsApplyingToEntity<T>(manifest, entityType, enabledEntityTypes, null)) {
      if (field.type !== "relationship" || keys.has(field.key)) continue;
      keys.add(field.key);
      contributed.push(field);
    }
  }
  return [...own, ...contributed];
}

export function manifestOwningRelationshipType(
  relationshipType: string,
  manifests: readonly ManifestLike[],
): ManifestLike | null {
  for (const manifest of manifests) {
    if (manifest.enabled === false) continue;
    for (const schema of manifest.schemas ?? []) {
      if (schema.fields?.some((field) => field.relationshipType === relationshipType)) return manifest;
    }
  }
  return null;
}

function moduleDisplayName(manifest: ManifestLike | null | undefined, fallbackId: string): string {
  const name = manifest?.name?.trim();
  return name || fallbackId;
}

export function groupedRelationshipFields<T extends RelationshipFieldLike>(
  fields: readonly T[],
  manifests: readonly ManifestLike[],
  populatedCount: (field: T) => number,
  options?: { sortByPopulated?: boolean },
): RelationshipFieldGroup<T>[] {
  const sortByPopulated = options?.sortByPopulated !== false;
  const buckets = new Map<string, RelationshipFieldGroup<T>>();
  for (const field of fields) {
    if (field.type !== "relationship") continue;
    const owner = field.relationshipType ? manifestOwningRelationshipType(field.relationshipType, manifests) : null;
    const moduleId = owner?.id ?? "unknown";
    let bucket = buckets.get(moduleId);
    if (!bucket) {
      bucket = { moduleId, moduleName: moduleDisplayName(owner, moduleId), fields: [] };
      buckets.set(moduleId, bucket);
    }
    bucket.fields.push(field);
  }
  const groups = [...buckets.values()].map((bucket) => ({
    ...bucket,
    fields: sortByPopulated
      ? [...bucket.fields].sort((left, right) => {
          const leftFilled = populatedCount(left) > 0;
          const rightFilled = populatedCount(right) > 0;
          if (leftFilled === rightFilled) return 0;
          return leftFilled ? -1 : 1;
        })
      : bucket.fields,
  }));
  groups.sort((left, right) => {
    if (sortByPopulated) {
      const leftFilled = left.fields.some((field) => populatedCount(field) > 0);
      const rightFilled = right.fields.some((field) => populatedCount(field) > 0);
      if (leftFilled !== rightFilled) return leftFilled ? -1 : 1;
    }
    return left.moduleName.localeCompare(right.moduleName);
  });
  return groups;
}

export function parseRelationshipMetadata(raw: unknown): Record<string, unknown> {
  if (raw && typeof raw === "object" && !Array.isArray(raw)) return raw as Record<string, unknown>;
  if (typeof raw !== "string" || !raw.trim()) return {};
  try {
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : {};
  } catch {
    return {};
  }
}

export function relationshipFieldForType(
  relationshipType: string,
  manifests: readonly ManifestLike[],
): RelationshipFieldLike | null {
  let fallback: RelationshipFieldLike | null = null;
  for (const manifest of manifests) {
    if (manifest.enabled === false) continue;
    for (const schema of manifest.schemas ?? []) {
      for (const field of schema.fields ?? []) {
        if (field.type !== "relationship" || field.relationshipType !== relationshipType) continue;
        if (field.metadataFields?.length) return field;
        fallback ??= field;
      }
    }
  }
  return fallback;
}

function metadataValuePresent(value: unknown): boolean {
  if (value === null || value === undefined) return false;
  if (typeof value === "string" && value.trim() === "") return false;
  if (Array.isArray(value) && value.length === 0) return false;
  return true;
}

export function relationshipAttributeRows(
  metadataRaw: unknown,
  definition: RelationshipFieldLike | null,
): RelationshipAttributeRow[] {
  const metadata = parseRelationshipMetadata(metadataRaw);
  const fields = definition?.metadataFields ?? [];
  if (fields.length > 0) {
    const rows: RelationshipAttributeRow[] = [];
    for (const field of fields) {
      const raw = metadata[field.key];
      if (!metadataValuePresent(raw)) continue;
      rows.push({
        key: field.key,
        label: field.label?.trim() || field.key,
        raw,
      });
    }
    return rows;
  }
  return Object.entries(metadata)
    .filter(([, raw]) => metadataValuePresent(raw))
    .map(([key, raw]) => ({ key, label: key, raw }));
}

export function groupRelationshipsByOwningModule<T extends RelationshipLike>(
  relationships: readonly T[],
  manifests: readonly ManifestLike[],
): RelationshipModuleGroup<T>[] {
  const buckets = new Map<string, RelationshipModuleGroup<T>>();
  for (const relationship of relationships) {
    const type = relationshipTypeOf(relationship);
    const owner = type ? manifestOwningRelationshipType(type, manifests) : null;
    const moduleId = owner?.id ?? "unknown";
    let bucket = buckets.get(moduleId);
    if (!bucket) {
      bucket = { moduleId, moduleName: moduleDisplayName(owner, moduleId), relationships: [] };
      buckets.set(moduleId, bucket);
    }
    bucket.relationships.push(relationship);
  }
  return [...buckets.values()].sort((left, right) => left.moduleName.localeCompare(right.moduleName));
}

export function partitionPopulatedFields<T>(
  fields: readonly T[],
  populatedCount: (field: T) => number,
): { filled: T[]; empty: T[] } {
  const filled: T[] = [];
  const empty: T[] = [];
  for (const field of fields) {
    if (populatedCount(field) > 0) filled.push(field);
    else empty.push(field);
  }
  return { filled, empty };
}
