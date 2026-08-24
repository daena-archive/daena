import type {
  Capability,
  ModuleContext,
  ModuleManifest,
  EntityRecord,
  EntitySummary,
  EntityQuery,
  EntityPage,
  EntityCreateInput,
  DocumentRecord,
  AssetRecord,
  FieldRecord,
  ModuleRecord,
  ModuleRecordQuery,
  Relationship,
  UUID,
  MutationOptions,
  StructuredAiHandle,
  StructuredAiRequest,
} from "../../../packages/module-api/src/index";
import { invoke } from "@tauri-apps/api/core";
import { createPluginRpcClient } from "../../../packages/plugin-sdk/src/index";
import type { MapLocationReference } from "../../../packages/plugin-sdk/src/maps";

interface RawEntity {
  id: string;
  name: string;
  entity_type: string | null;
  deleted: boolean;
  created_at: string;
  updated_at: string;
  revision: string;
}
interface RawDocument {
  id: string;
  entity_id: string;
  format: string;
  body: string;
  updated_at: string;
  revision: string;
}
interface RawField {
  entity_id: string;
  namespace: string;
  key: string;
  value: unknown;
  revision: string;
}
interface RawRelationship {
  id: string;
  source_id: string;
  target_id: string;
  relationship_type: string;
  metadata: string;
  revision: string;
}
interface RawAsset {
  id: string;
  entity_id: string;
  namespace: string;
  filename: string;
  content_hash: string;
  size: number;
  mime_type: string;
  path: string;
  created_at: string;
  role: "attachment" | "profile";
  reference_scope: "entity" | "project";
  revision: string;
}
interface RawModuleRecord {
  id: string;
  collection: string;
  owner_entity_id: string;
  value: Record<string, unknown>;
  created_at: string;
  updated_at: string;
  revision: string;
}

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

function toPluginEntitySummary(e: {
  id: string;
  name: string;
  entityType?: string | null;
  deleted: boolean;
  revision: string;
}): EntitySummary {
  return {
    id: toUUID(e.id),
    name: e.name,
    type: e.entityType ?? null,
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
    role: asset.role,
    referenceScope: asset.reference_scope,
    revision: asset.revision,
  };
}

function toModuleRecord<T>(record: RawModuleRecord): ModuleRecord<T> {
  return {
    id: toUUID(record.id),
    collection: record.collection,
    ownerEntityId: toUUID(record.owner_entity_id),
    value: record.value as T,
    createdAt: record.created_at,
    updatedAt: record.updated_at,
    revision: record.revision,
  };
}

function schemaForField(manifest: ModuleManifest, key: string, entityType?: string) {
  return manifest.schemas
    .flatMap((schema) => schema.fields.map((field) => ({ schema, field })))
    .find(
      ({ schema, field }) =>
        field.key === key &&
        (entityType === undefined || schema.entityTypes.some((definition) => definition.id === entityType)) &&
        (!field.entityTypes || entityType === undefined || field.entityTypes.includes(entityType)),
    );
}

function validateField(manifest: ModuleManifest, key: string, value: unknown, entityType?: string): string {
  const definition = schemaForField(manifest, key, entityType);
  if (!definition) throw new Error(`Module ${manifest.id} does not declare field: ${key}`);
  const { field } = definition;
  if (field.required && (value === null || value === undefined || value === ""))
    throw new Error(`Field ${key} is required`);
  if (value === null || value === undefined || value === "") return definition.schema.namespace;
  const valid =
    field.type === "text"
      ? typeof value === "string"
      : field.type === "number"
        ? typeof value === "number" && Number.isFinite(value)
        : field.type === "boolean"
          ? typeof value === "boolean"
          : field.type === "date"
            ? typeof value === "string" || (typeof value === "object" && value !== null)
            : field.type === "enum"
              ? typeof value === "string" && !!field.options?.includes(value)
              : field.type === "relationship"
                ? false
                : typeof value === "string";
  if (!valid) throw new Error(`Invalid value for field ${key}`);
  return definition.schema.namespace;
}

function checkCapability(manifest: ModuleManifest, required: Capability): void {
  if (!manifest.capabilities.includes(required)) {
    throw new Error(`Module ${manifest.id} lacks required capability: ${required}`);
  }
}

function createFields(manifest: ModuleManifest, fields: Record<string, unknown>, entityType?: string) {
  const entries = Object.entries(fields).filter(([, value]) => value !== "" && value !== null && value !== undefined);
  if (entries.length === 0) return [];
  checkCapability(manifest, "field.write:self");
  return entries.map(([key, value]) => ({
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
  options?: {
    focusEntityId?: UUID;
    availableServices?: ReadonlySet<string>;
    embedded?: boolean;
    onEntityDeleted?: () => void;
  },
): ModuleContext {
  void projectId;
  const onEntityDeleted = options?.onEntityDeleted;
  const rpc = createPluginRpcClient({
    call: (method, payload, requestId) =>
      invoke("trusted_module_rpc", {
        method,
        payload,
        requestId: requestId ?? crypto.randomUUID(),
        pluginId: manifest.id,
      }),
  });
  const queryEntities = async (query: EntityQuery = {}): Promise<EntityPage> => {
    checkCapability(manifest, "entity.read");
    const entityTypes = query.types ?? (query.type ? [query.type] : []);
    const page = await rpc.queryEntities({
      query: query.text,
      entityTypes,
      excludedEntityTypes: query.excludedTypes ?? [],
      sortField: query.sortField,
      sortDirection: query.sortDirection,
      offset: query.offset,
      limit: query.limit,
    });
    return {
      items: page.items.map(toPluginEntitySummary),
      total: page.total,
      offset: page.offset,
      limit: page.limit,
      hasMore: page.hasMore,
      typeCounts: page.typeCounts.map((count) => ({ type: count.entityType ?? null, count: count.count })),
    };
  };
  return {
    module: manifest,
    focusEntityId: options?.focusEntityId,
    embedded: options?.embedded,
    services: {
      isAvailable: (name, major) => options?.availableServices?.has(`${name}@${major}`) ?? false,
    },
    modules: {
      list: async () => {
        const manifests = await invoke<Array<ModuleManifest & { enabled?: boolean }>>("module_list_manifests");
        return manifests.filter((candidate) => candidate.enabled !== false);
      },
    },
    entities: {
      get: async (id: UUID) => {
        checkCapability(manifest, "entity.read");
        const found = await rpc.call<RawEntity | null>("entity.get", { id });
        if (!found) return null;
        const record = toEntityRecord(found);
        const docs = await rpc.call<RawDocument[]>("document.list", { entityId: id });
        record.documents = docs.map(toDocumentRecord);
        const namespace = manifest.schemas[0]?.namespace;
        const fields = await rpc.call<RawField[]>("field.list", { entityId: id, namespace });
        for (const f of fields.filter((field) => !namespace || field.namespace === namespace))
          record.fields[f.key] = f.value;
        return record;
      },
      query: queryEntities,
      list: async (query?: EntityQuery) => {
        checkCapability(manifest, "entity.read");
        const requested = query ?? {};
        if (requested.limit !== undefined) return (await queryEntities(requested)).items;
        const entities: EntitySummary[] = [];
        let offset = requested.offset ?? 0;
        while (true) {
          const page = await queryEntities({ ...requested, offset, limit: 200 });
          entities.push(...page.items);
          if (!page.hasMore) return entities;
          offset += page.items.length;
        }
      },
      create: async (input: EntityCreateInput, options?: MutationOptions) => {
        checkCapability(manifest, "entity.write");
        if (input.document) checkCapability(manifest, "document.write");
        const fields = input.fields ? createFields(manifest, input.fields, input.type) : undefined;
        const relationships = input.relationships
          ? createRelationships(manifest, input.relationships, input.type)
          : undefined;
        const entity = await rpc.call<RawEntity>(
          "entity.create",
          {
            name: input.name,
            type: input.type ?? null,
            fields,
            relationships,
            document: input.document,
            expectedRevision: undefined,
          },
          options?.requestId,
        );
        return toEntityRecord(entity);
      },
      update: async (id: UUID, patch: { name?: string; type?: string | null }, options?: MutationOptions) => {
        checkCapability(manifest, "entity.write");
        const entity = await rpc.call<RawEntity>(
          "entity.update",
          { id, name: patch.name ?? null, type: patch.type ?? null, expectedRevision: options?.expectedRevision },
          options?.requestId,
        );
        return toEntityRecord(entity);
      },
      delete: async (id: UUID, options?: MutationOptions) => {
        checkCapability(manifest, "entity.delete");
        await rpc.call<null>("entity.delete", { id, expectedRevision: options?.expectedRevision }, options?.requestId);
        onEntityDeleted?.();
      },
    },
    documents: {
      save: async (
        input: {
          entityId: UUID;
          body: string;
          format?: DocumentRecord["format"];
        },
        options?: MutationOptions,
      ) => {
        checkCapability(manifest, "document.write");
        await rpc.call<null>(
          "document.save",
          { ...input, expectedRevision: options?.expectedRevision },
          options?.requestId,
        );
        const docs = await rpc.call<RawDocument[]>("document.list", { entityId: input.entityId });
        return docs.map(toDocumentRecord)[0] ?? null;
      },
    },
    fields: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "field.read:self");
        const namespace = manifest.schemas[0]?.namespace;
        const fields = await rpc.call<RawField[]>("field.list", { entityId, namespace });
        return Object.fromEntries(
          fields
            .filter((field) => !namespace || field.namespace === namespace)
            .map((field) => [field.key, field.value]),
        );
      },
      listRecords: async (entityId: UUID): Promise<FieldRecord[]> => {
        checkCapability(manifest, "field.read:self");
        const namespace = manifest.schemas[0]?.namespace;
        const fields = await rpc.call<RawField[]>("field.list", { entityId, namespace });
        return fields
          .filter((field) => !namespace || field.namespace === namespace)
          .map((field) => ({
            entityId: toUUID(field.entity_id),
            namespace: field.namespace,
            key: field.key,
            value: field.value,
            revision: field.revision,
          }));
      },
      listShared: async (entityId: UUID, namespace: string): Promise<FieldRecord[]> => {
        checkCapability(manifest, "field.read:shared");
        if (!namespace.trim()) throw new Error("Shared field namespace is required");
        const fields = await rpc.call<RawField[]>("field.list", {
          entityId,
          namespace,
          sharedOnly: true,
        });
        return fields.map((field) => ({
          entityId: toUUID(field.entity_id),
          namespace: field.namespace,
          key: field.key,
          value: field.value,
          revision: field.revision,
        }));
      },
      set: async (entityId: UUID, key: string, value: unknown, options?: MutationOptions) => {
        checkCapability(manifest, "field.write:self");
        const namespace = validateField(manifest, key, value);
        await rpc.call<null>(
          "field.set",
          { entityId, namespace, key, value, expectedRevision: options?.expectedRevision },
          options?.requestId,
        );
      },
    },
    records: {
      list: async <T>(collection: string, ownerEntityId: UUID, query: ModuleRecordQuery = {}) => {
        checkCapability(manifest, "record.read:self");
        const records = await rpc.call<RawModuleRecord[]>("record.list", {
          collection,
          ownerEntityId,
          query: query.query,
          limit: query.limit,
          offset: query.offset,
          sort: query.sort,
          status: query.status,
          tag: query.tag,
          homonymsOnly: query.homonymsOnly,
        });
        return records.map((record) => toModuleRecord<T>(record));
      },
      create: async <T>(collection: string, ownerEntityId: UUID, value: T, options?: MutationOptions) => {
        checkCapability(manifest, "record.write:self");
        const record = await rpc.call<RawModuleRecord>(
          "record.create",
          { collection, ownerEntityId, value },
          options?.requestId,
        );
        return toModuleRecord<T>(record);
      },
      update: async <T>(collection: string, id: UUID, ownerEntityId: UUID, value: T, options: MutationOptions) => {
        checkCapability(manifest, "record.write:self");
        const record = await rpc.call<RawModuleRecord>(
          "record.update",
          { collection, id, ownerEntityId, value, expectedRevision: options.expectedRevision },
          options.requestId,
        );
        return toModuleRecord<T>(record);
      },
      delete: async (collection: string, id: UUID, ownerEntityId: UUID, options: MutationOptions) => {
        checkCapability(manifest, "record.write:self");
        await rpc.call<null>(
          "record.delete",
          { collection, id, ownerEntityId, expectedRevision: options.expectedRevision },
          options.requestId,
        );
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
        const rel = await rpc.call<RawRelationship>(
          "relationship.create",
          {
            source_id: input.sourceId,
            target_id: input.targetId,
            relationship_type: input.type,
            metadata: JSON.stringify(input.metadata ?? {}),
            expectedRevision: options?.expectedRevision,
          },
          options?.requestId,
        );
        return toRelationship(rel);
      },
      update: async (
        input: { id: UUID; metadata?: Record<string, unknown>; targetId?: UUID },
        options?: MutationOptions,
      ) => {
        checkCapability(manifest, "relationship.write");
        const rel = await rpc.call<RawRelationship>(
          "relationship.update",
          {
            id: input.id,
            metadata: input.metadata ? JSON.stringify(input.metadata) : undefined,
            target_id: input.targetId,
            expectedRevision: options?.expectedRevision,
          },
          options?.requestId,
        );
        return toRelationship(rel);
      },
      delete: async (id: UUID, relationshipType: string, options?: MutationOptions) => {
        checkCapability(manifest, "relationship.write");
        if (
          !manifest.schemas.some((schema) => schema.fields.some((field) => field.relationshipType === relationshipType))
        ) {
          throw new Error(`Module ${manifest.id} does not declare relationship type: ${relationshipType}`);
        }
        await rpc.call<null>(
          "relationship.delete",
          { id, relationship_type: relationshipType, expectedRevision: options?.expectedRevision },
          options?.requestId,
        );
      },
    },
    assets: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "asset.read:self");
        const namespace = manifest.schemas[0]?.namespace;
        return (await rpc.call<RawAsset[]>("asset.list", { entityId, namespace }))
          .filter((asset) => manifest.schemas.some((schema) => schema.namespace === asset.namespace))
          .map(toAsset);
      },
      register: async (input, options?: MutationOptions) => {
        checkCapability(manifest, "asset.register");
        if (!manifest.schemas.some((schema) => schema.namespace === input.namespace))
          throw new Error(`Module ${manifest.id} does not own namespace: ${input.namespace}`);
        return toAsset(
          await rpc.call<RawAsset>(
            "asset.register",
            {
              entity_id: input.entityId,
              namespace: input.namespace,
              filename: input.filename,
              content_hash: input.contentHash,
              size: input.size,
              mime_type: input.mimeType,
              path: input.path,
              expectedRevision: options?.expectedRevision,
            },
            options?.requestId,
          ),
        );
      },
      updateMetadata: async (asset, update, options?: MutationOptions) => {
        checkCapability(manifest, "asset.write:self");
        if (!manifest.schemas.some((schema) => schema.namespace === asset.namespace))
          throw new Error(`Module ${manifest.id} does not own namespace: ${asset.namespace}`);
        return toAsset(
          await rpc.call<RawAsset>(
            "asset.update",
            {
              assetId: asset.id,
              namespace: asset.namespace,
              filename: update.filename,
              role: update.role,
              referenceScope: update.referenceScope,
              expectedRevision: options?.expectedRevision ?? asset.revision,
            },
            options?.requestId,
          ),
        );
      },
      delete: async (asset, options?: MutationOptions) => {
        checkCapability(manifest, "asset.write:self");
        if (!manifest.schemas.some((schema) => schema.namespace === asset.namespace))
          throw new Error(`Module ${manifest.id} does not own namespace: ${asset.namespace}`);
        await rpc.call<void>(
          "asset.delete",
          {
            assetId: asset.id,
            namespace: asset.namespace,
            expectedRevision: options?.expectedRevision ?? asset.revision,
          },
          options?.requestId,
        );
      },
    },
    search: async (query: string) => {
      checkCapability(manifest, "search.query");
      const entities = await rpc.call<RawEntity[]>("search.query", { query });
      return entities.map(toEntitySummary);
    },
    ai: {
      startStructured: async (request: StructuredAiRequest): Promise<StructuredAiHandle> => {
        checkCapability(manifest, "ai.text.generate-structured");
        const started = await rpc.startAiRequest({
          operation: "generate_structured",
          taskId: request.taskId,
          userInstruction: request.userInstruction,
          immediateContext: request.immediateContext,
          outputContract: request.outputContract,
          deadlineMs: request.deadlineMs,
          retrievalPolicy: request.retrievalPolicy,
        });
        return {
          requestId: started.requestId,
          poll: () => rpc.pollAiRequest(started.requestId),
          cancel: () => rpc.cancelAiRequest(started.requestId),
          result: () => rpc.getAiResult(started.requestId),
          citations: () => rpc.getAiCitations(started.requestId),
        };
      },
    },
    maps: {
      openMap: async (input) =>
        await invoke("maps_navigation", {
          operation: "openMap",
          mapEntityId: input.mapEntityId,
          linkId: input.linkId ?? null,
          entityId: null,
        }),
      focusEntity: async (input) =>
        await invoke("maps_navigation", {
          operation: "focusEntity",
          mapEntityId: input.mapEntityId ?? null,
          entityId: input.entityId,
          linkId: null,
        }),
      setDate: async (input) =>
        await invoke("maps_navigation", {
          operation: "setDate",
          mapEntityId: null,
          entityId: null,
          linkId: null,
          date: input.date,
        }),
      showResults: async (input) =>
        await invoke("maps_navigation", {
          operation: "showResults",
          mapEntityId: input.mapEntityId ?? null,
          entityId: null,
          linkId: null,
          entityIds: input.entityIds,
        }),
      listLocations: async (input) =>
        await invoke<readonly MapLocationReference[]>("project_list_map_locations", { entityId: input.entityId }),
      upsertLocation: async (input, options?: MutationOptions) => {
        await invoke<void>("project_upsert_map_location", {
          entityId: input.entityId,
          location: input.location,
          request_id: options?.requestId ?? null,
        });
      },
      unlinkLocation: async (input, options?: MutationOptions) => {
        await invoke<void>("project_unlink_map_location", {
          entityId: input.entityId,
          locationId: input.locationId,
          request_id: options?.requestId ?? null,
        });
      },
    },
  };
}
