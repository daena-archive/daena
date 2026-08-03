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
  Relationship,
  UUID,
} from "../../../packages/module-api/src/index";
import { invoke } from "@tauri-apps/api/core";
import { createPluginRpcClient } from "../../../packages/plugin-sdk/src/index";

interface RawEntity { id: string; name: string; entity_type: string | null; deleted: boolean; created_at: string; updated_at: string }
interface RawDocument { id: string; entity_id: string; format: string; body: string; updated_at: string }
interface RawField { entity_id: string; namespace: string; key: string; value: unknown }
interface RawRelationship { id: string; source_id: string; target_id: string; relationship_type: string; metadata: string }
interface RawAsset { id: string; entity_id: string; namespace: string; filename: string; content_hash: string; size: number; mime_type: string; path: string; created_at: string }

function toUUID(id: string): UUID {
  return id as UUID;
}

function toEntityRecord(e: RawEntity): EntityRecord {
  return {
    id: toUUID(e.id),
    name: e.name,
    type: e.entity_type,
    deleted: e.deleted,
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
  };
}

function toDocumentRecord(d: RawDocument): DocumentRecord {
  return {
    id: toUUID(d.id),
    entityId: toUUID(d.entity_id),
    format: d.format as DocumentRecord["format"],
    body: d.body,
    updatedAt: d.updated_at,
  };
}

function toRelationship(r: RawRelationship): Relationship {
  return {
    id: toUUID(r.id),
    sourceId: toUUID(r.source_id),
    targetId: toUUID(r.target_id),
    type: r.relationship_type,
    metadata: JSON.parse(r.metadata || "{}"),
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

export function buildModuleContext(
  manifest: ModuleManifest,
  projectId: string,
): ModuleContext {
  const rpc = createPluginRpcClient({
    call: (method, payload) => invoke("module_rpc", {
      pluginId: manifest.id,
      projectId,
      method,
      payload,
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
      create: async (input: EntityCreateInput) => {
        checkCapability(manifest, "entity.write");
        if (input.document) checkCapability(manifest, "document.write");
        const fields = input.fields ? createFields(manifest, input.fields, input.type) : undefined;
        const entity = await rpc.call<RawEntity>("entity.create", {
          name: input.name,
          type: input.type ?? null,
          fields,
          document: input.document,
        });
        return toEntityRecord(entity);
      },
      update: async (id: UUID, patch: { name?: string; type?: string | null }) => {
        checkCapability(manifest, "entity.write");
        const entity = await rpc.call<RawEntity>("entity.update", { id, name: patch.name ?? null, type: patch.type ?? null });
        return toEntityRecord(entity);
      },
      delete: async (id: UUID) => {
        checkCapability(manifest, "entity.delete");
        await rpc.call<null>("entity.delete", { id });
      },
    },
    documents: {
      save: async (input: {
        entityId: UUID;
        body: string;
        format?: DocumentRecord["format"];
      }) => {
        checkCapability(manifest, "document.write");
        await rpc.call<null>("document.save", input);
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
      set: async (entityId: UUID, key: string, value: unknown) => {
        checkCapability(manifest, "field.write:self");
        const namespace = validateField(manifest, key, value);
        await rpc.call<null>("field.set", { entityId, namespace, key, value });
      },
    },
    relationships: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "relationship.read");
        const rels = await rpc.call<RawRelationship[]>("relationship.list", { entityId });
        return rels.map(toRelationship);
      },
      create: async (input: Omit<Relationship, "id">) => {
        checkCapability(manifest, "relationship.write");
        const rel = await rpc.call<RawRelationship>("relationship.create", {
          source_id: input.sourceId,
          target_id: input.targetId,
          relationship_type: input.type,
          metadata: JSON.stringify(input.metadata ?? {}),
        });
        return toRelationship(rel);
      },
    },
    assets: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "asset.read:self");
        const namespace = manifest.schemas[0]?.namespace;
        return (await rpc.call<RawAsset[]>("asset.list", { entityId, namespace })).filter((asset) => manifest.schemas.some((schema) => schema.namespace === asset.namespace)).map(toAsset);
      },
      register: async (input) => {
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
        }));
      },
    },
    search: async (query: string) => {
      checkCapability(manifest, "search.query");
      const entities = await rpc.call<RawEntity[]>("search.query", { query });
      return entities.map(toEntitySummary);
    },
  };
}
