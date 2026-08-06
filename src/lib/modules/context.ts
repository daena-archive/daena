import type {
  Capability,
  ModuleContext,
  ModuleManifest,
  EntityRecord,
  EntitySummary,
  EntityQuery,
  EntityCreateInput,
  DocumentRecord,
  AssetRecord,
  FieldRecord,
  Relationship,
  UUID,
  MutationOptions,
} from "../../../packages/module-api/src/index";
import { invoke } from "@tauri-apps/api/core";
import { createPluginRpcClient } from "../../../packages/plugin-sdk/src/index";
import type { MapLocationReference } from "../../../packages/plugin-sdk/src/maps";

interface RawEntity { id: string; name: string; entity_type: string | null; deleted: boolean; created_at: string; updated_at: string; revision: string }
interface RawDocument { id: string; entity_id: string; format: string; body: string; updated_at: string; revision: string }
interface RawField { entity_id: string; namespace: string; key: string; value: unknown; revision: string }
interface RawRelationship { id: string; source_id: string; target_id: string; relationship_type: string; metadata: string; revision: string }
interface RawAsset { id: string; entity_id: string; namespace: string; filename: string; content_hash: string; size: number; mime_type: string; path: string; created_at: string; revision: string }

function toUUID(id: string): UUID {
  return id as UUID;
}

function toEntityRecord(e: RawEntity): EntityRecord {
  return {
    id: toUUID(e.id),
    name: e.name,
    type: e.entity_type,
    deleted: e.deleted,
    revision: e.revision,
    createdAt: e.created_at,
    updatedAt: e.updated_at,
    documents: [],
    fields: {},
  };
}

function toEntitySummary(e: RawEntity): EntitySummary {
  return {
    id: toUUID(e.id),
    name: e.name,
    type: e.entity_type,
    deleted: e.deleted,
    revision: e.revision,
  };
}

function toDocumentRecord(d: RawDocument): DocumentRecord {
  return {
    id: toUUID(d.id),
    entityId: toUUID(d.entity_id),
    format: d.format as DocumentRecord["format"],
    body: d.body,
    updatedAt: d.updated_at,
    revision: d.revision,
  };
}

function toRelationship(r: RawRelationship): Relationship {
  return {
    id: toUUID(r.id),
    sourceId: toUUID(r.source_id),
    targetId: toUUID(r.target_id),
    type: r.relationship_type,
    metadata: JSON.parse(r.metadata || "{}"),
    revision: r.revision,
  };
}

function toAsset(asset: RawAsset): AssetRecord {
  return {
    id: toUUID(asset.id),
    entityId: toUUID(asset.entity_id),
    namespace: asset.namespace,
    filename: asset.filename,
    contentHash: asset.content_hash,
    size: asset.size,
    mimeType: asset.mime_type,
    path: asset.path,
    createdAt: asset.created_at,
    revision: asset.revision,
  };
}

function schemaForField(manifest: ModuleManifest, key: string, entityType?: string) {
  return manifest.schemas
    .flatMap((schema) => schema.fields.map((field) => ({ schema, field })))
    .find(({ schema, field }) => field.key === key
      && (entityType === undefined || schema.entityTypes.includes(entityType))
      && (!field.entityTypes || entityType === undefined || field.entityTypes.includes(entityType)));
}

function validateField(manifest: ModuleManifest, key: string, value: unknown, entityType?: string): string {
  const definition = schemaForField(manifest, key, entityType);
  if (!definition) throw new Error(`Module ${manifest.id} does not declare field: ${key}`);
  const { field } = definition;
  if (field.required && (value === null || value === undefined || value === "")) throw new Error(`Field ${key} is required`);
  if (value === null || value === undefined || value === "") return definition.schema.namespace;
  const valid = field.type === "text" ? typeof value === "string"
    : field.type === "number" ? typeof value === "number" && Number.isFinite(value)
    : field.type === "boolean" ? typeof value === "boolean"
    : field.type === "date" ? typeof value === "string" || (typeof value === "object" && value !== null)
    : field.type === "enum" ? typeof value === "string" && !!field.options?.includes(value)
    : field.type === "relationship" ? false
    : typeof value === "string";
  if (!valid) throw new Error(`Invalid value for field ${key}`);
  return definition.schema.namespace;
}

function checkCapability(manifest: ModuleManifest, required: Capability): void {
  if (!manifest.capabilities.includes(required)) {
    throw new Error(
      `Module ${manifest.id} lacks required capability: ${required}`
    );
  }
}

function createFields(manifest: ModuleManifest, fields: Record<string, unknown>, entityType?: string) {
  const entries = Object.entries(fields)
    .filter(([, value]) => value !== "" && value !== null && value !== undefined)
  if (entries.length === 0) return [];
  checkCapability(manifest, "field.write:self");
  return entries
    .map(([key, value]) => ({
      namespace: validateField(manifest, key, value, entityType),
      key,
      value,
    }));
}

function createRelationships(manifest: ModuleManifest, relationships: Record<string, string[]>, entityType?: string) {
  const entries = Object.entries(relationships);
  if (entries.length === 0) return [];
  checkCapability(manifest, "relationship.write");
  return entries.map(([key, targetIds]) => {
    const definition = schemaForField(manifest, key, entityType);
    if (!definition || definition.field.type !== "relationship" || !definition.field.relationshipType) {
      throw new Error(`Module ${manifest.id} does not declare relationship field: ${key}`);
    }
    if (!Array.isArray(targetIds) || targetIds.some((targetId) => typeof targetId !== "string" || !targetId)) {
      throw new Error(`Invalid targets for relationship field ${key}`);
    }
    return {
      relationship_type: definition.field.relationshipType,
      target_ids: [...new Set(targetIds)],
    };
  });
}

export function buildModuleContext(
  manifest: ModuleManifest,
  projectId: string,
): ModuleContext {
  void projectId;
  const rpc = createPluginRpcClient({
    call: (method, payload, requestId) => invoke("trusted_module_rpc", {
      method,
      payload,
      requestId: requestId ?? crypto.randomUUID(),
    }),
  });
  return {
    module: manifest,
    entities: {
      get: async (id: UUID) => {
        checkCapability(manifest, "entity.read");
        const entities = await rpc.call<RawEntity[]>("entity.list", {});
        const found = entities.find((e) => e.id === id);
        if (!found) return null;
        const record = toEntityRecord(found);
        const docs = await rpc.call<RawDocument[]>("document.list", { entityId: id });
        record.documents = docs.map(toDocumentRecord);
        const namespace = manifest.schemas[0]?.namespace;
        const fields = await rpc.call<RawField[]>("field.list", { entityId: id, namespace });
        for (const f of fields.filter((field) => !namespace || field.namespace === namespace)) record.fields[f.key] = f.value;
        return record;
      },
      list: async (query?: EntityQuery) => {
        checkCapability(manifest, "entity.read");
        const entities = await rpc.call<RawEntity[]>("entity.list", {});
        const filtered = entities.filter((entity) =>
          (!query?.type || entity.entity_type === query.type) &&
          (!query?.text || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(query.text.toLowerCase()))
        );
        return filtered.slice(0, query?.limit ?? filtered.length).map(toEntitySummary);
      },
      create: async (input: EntityCreateInput, options?: MutationOptions) => {
        checkCapability(manifest, "entity.write");
        if (input.document) checkCapability(manifest, "document.write");
        const fields = input.fields ? createFields(manifest, input.fields, input.type) : undefined;
        const relationships = input.relationships ? createRelationships(manifest, input.relationships, input.type) : undefined;
        const entity = await rpc.call<RawEntity>("entity.create", {
          name: input.name,
          type: input.type ?? null,
          fields,
          relationships,
          document: input.document,
          expectedRevision: undefined,
        }, options?.requestId);
        return toEntityRecord(entity);
      },
      update: async (id: UUID, patch: { name?: string; type?: string | null }, options?: MutationOptions) => {
        checkCapability(manifest, "entity.write");
        const entity = await rpc.call<RawEntity>("entity.update", { id, name: patch.name ?? null, type: patch.type ?? null, expectedRevision: options?.expectedRevision }, options?.requestId);
        return toEntityRecord(entity);
      },
      delete: async (id: UUID, options?: MutationOptions) => {
        checkCapability(manifest, "entity.delete");
        await rpc.call<null>("entity.delete", { id, expectedRevision: options?.expectedRevision }, options?.requestId);
      },
    },
    documents: {
      save: async (input: {
        entityId: UUID;
        body: string;
        format?: DocumentRecord["format"];
      }, options?: MutationOptions) => {
        checkCapability(manifest, "document.write");
        await rpc.call<null>("document.save", { ...input, expectedRevision: options?.expectedRevision }, options?.requestId);
        const docs = await rpc.call<RawDocument[]>("document.list", { entityId: input.entityId });
        return docs.map(toDocumentRecord)[0] ?? null;
      },
    },
    fields: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "field.read:self");
        const namespace = manifest.schemas[0]?.namespace;
        const fields = await rpc.call<RawField[]>("field.list", { entityId, namespace });
        return Object.fromEntries(fields.filter((field) => !namespace || field.namespace === namespace).map((field) => [field.key, field.value]));
      },
      listRecords: async (entityId: UUID): Promise<FieldRecord[]> => {
        checkCapability(manifest, "field.read:self");
        const namespace = manifest.schemas[0]?.namespace;
        const fields = await rpc.call<RawField[]>("field.list", { entityId, namespace });
        return fields
          .filter((field) => !namespace || field.namespace === namespace)
          .map((field) => ({ entityId: toUUID(field.entity_id), namespace: field.namespace, key: field.key, value: field.value, revision: field.revision }));
      },
      set: async (entityId: UUID, key: string, value: unknown, options?: MutationOptions) => {
        checkCapability(manifest, "field.write:self");
        const namespace = validateField(manifest, key, value);
        await rpc.call<null>("field.set", { entityId, namespace, key, value, expectedRevision: options?.expectedRevision }, options?.requestId);
      },
    },
    relationships: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "relationship.read");
        const rels = await rpc.call<RawRelationship[]>("relationship.list", { entityId });
        return rels.map(toRelationship);
      },
      create: async (input: Omit<Relationship, "id" | "revision">, options?: MutationOptions) => {
        checkCapability(manifest, "relationship.write");
        const rel = await rpc.call<RawRelationship>("relationship.create", {
          source_id: input.sourceId,
          target_id: input.targetId,
          relationship_type: input.type,
          metadata: JSON.stringify(input.metadata ?? {}),
          expectedRevision: options?.expectedRevision,
        }, options?.requestId);
        return toRelationship(rel);
      },
      delete: async (id: UUID, relationshipType: string, options?: MutationOptions) => {
        checkCapability(manifest, "relationship.write");
        if (!manifest.schemas.some((schema) => schema.fields.some((field) => field.relationshipType === relationshipType))) {
          throw new Error(`Module ${manifest.id} does not declare relationship type: ${relationshipType}`);
        }
        await rpc.call<null>("relationship.delete", { id, relationship_type: relationshipType, expectedRevision: options?.expectedRevision }, options?.requestId);
      },
    },
    assets: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "asset.read:self");
        const namespace = manifest.schemas[0]?.namespace;
        return (await rpc.call<RawAsset[]>("asset.list", { entityId, namespace })).filter((asset) => manifest.schemas.some((schema) => schema.namespace === asset.namespace)).map(toAsset);
      },
      register: async (input, options?: MutationOptions) => {
        checkCapability(manifest, "asset.import");
        if (!manifest.schemas.some((schema) => schema.namespace === input.namespace)) throw new Error(`Module ${manifest.id} does not own namespace: ${input.namespace}`);
        return toAsset(await rpc.call<RawAsset>("asset.register", {
          entity_id: input.entityId,
          namespace: input.namespace,
          filename: input.filename,
          content_hash: input.contentHash,
          size: input.size,
          mime_type: input.mimeType,
          path: input.path,
          expectedRevision: options?.expectedRevision,
        }, options?.requestId));
      },
    },
    search: async (query: string) => {
      checkCapability(manifest, "search.query");
      const entities = await rpc.call<RawEntity[]>("search.query", { query });
      return entities.map(toEntitySummary);
    },
    maps: {
      openMap: async (input) => { await invoke("maps_navigation", { operation: "openMap", map_entity_id: input.mapEntityId, link_id: input.linkId ?? null, entity_id: null }); },
      focusEntity: async (input) => { await invoke("maps_navigation", { operation: "focusEntity", map_entity_id: input.mapEntityId ?? null, entity_id: input.entityId, link_id: null }); },
      setDate: async (input) => { await invoke("maps_navigation", { operation: "setDate", map_entity_id: null, entity_id: null, link_id: null, date: input.date }); },
      showResults: async (input) => { await invoke("maps_navigation", { operation: "showResults", map_entity_id: input.mapEntityId ?? null, entity_id: null, link_id: null, entity_ids: input.entityIds }); },
      listLocations: async (input) => await invoke<readonly MapLocationReference[]>("project_list_map_locations", { entityId: input.entityId }),
    },
  };
}
