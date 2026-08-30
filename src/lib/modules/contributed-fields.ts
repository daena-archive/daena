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
  enabled?: boolean;
  schemas?: SchemaLike[];
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
