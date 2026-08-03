import type {
  Capability,
  ModuleContext,
  ModuleManifest,
  EntityRecord,
  EntitySummary,
  EntityQuery,
  DocumentRecord,
  AssetRecord,
  Relationship,
  UUID,
} from "../../../packages/module-api/src/index";
import { project } from "../project/client";
import type { Entity, Document, FieldValue, Relationship as Rel, Asset } from "../project/client";

function toUUID(id: string): UUID {
  return id as UUID;
}

function toEntityRecord(e: Entity): EntityRecord {
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

function toEntitySummary(e: Entity): EntitySummary {
  return {
    id: toUUID(e.id),
    name: e.name,
    type: e.entity_type,
    deleted: e.deleted,
  };
}

function toDocumentRecord(d: Document): DocumentRecord {
  return {
    id: toUUID(d.id),
    entityId: toUUID(d.entity_id),
    format: d.format as DocumentRecord["format"],
    body: d.body,
    updatedAt: d.updated_at,
  };
}

function toRelationship(r: Rel): Relationship {
  return {
    id: toUUID(r.id),
    sourceId: toUUID(r.source_id),
    targetId: toUUID(r.target_id),
    type: r.relationship_type,
    metadata: JSON.parse(r.metadata || "{}"),
  };
}

function toAsset(asset: Asset): AssetRecord {
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

function schemaForField(manifest: ModuleManifest, key: string) {
  return manifest.schemas.flatMap((schema) => schema.fields.map((field) => ({ schema, field }))).find(({ field }) => field.key === key);
}

function validateField(manifest: ModuleManifest, key: string, value: unknown): string {
  const definition = schemaForField(manifest, key);
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

export function buildModuleContext(
  manifest: ModuleManifest
): ModuleContext {
  return {
    module: manifest,
    entities: {
      get: async (id: UUID) => {
        checkCapability(manifest, "entity.read");
        const entities = await project.listEntities();
        const found = entities.find((e) => e.id === id);
        if (!found) return null;
        const record = toEntityRecord(found);
        const docs = await project.listDocuments(id);
        record.documents = docs.map(toDocumentRecord);
        const namespace = manifest.schemas[0]?.namespace;
        const fields = await project.listFields(id);
        for (const f of fields.filter((field) => !namespace || field.namespace === namespace)) record.fields[f.key] = f.value;
        return record;
      },
      list: async (query?: EntityQuery) => {
        checkCapability(manifest, "entity.read");
        const entities = await project.listEntities();
        const filtered = entities.filter((entity) =>
          (!query?.type || entity.entity_type === query.type) &&
          (!query?.text || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(query.text.toLowerCase()))
        );
        return filtered.slice(0, query?.limit ?? filtered.length).map(toEntitySummary);
      },
      create: async (input: { name: string; type?: string }) => {
        checkCapability(manifest, "entity.write");
        const entity = await project.createEntity(input.name, input.type);
        return toEntityRecord(entity);
      },
      update: async (id: UUID, patch: { name?: string; type?: string | null }) => {
        checkCapability(manifest, "entity.write");
        const entity = await project.updateEntity(
          id,
          patch.name ?? null,
          patch.type ?? null
        );
        return toEntityRecord(entity);
      },
      delete: async (id: UUID) => {
        checkCapability(manifest, "entity.write");
        await project.deleteEntity(id);
      },
    },
    documents: {
      save: async (input: {
        entityId: UUID;
        body: string;
        format?: DocumentRecord["format"];
      }) => {
        checkCapability(manifest, "document.write");
        await project.saveDocument(input.entityId, input.body, input.format);
        const docs = await project.listDocuments(input.entityId);
        return docs.map(toDocumentRecord)[0] ?? null;
      },
    },
    fields: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "entity.read");
        const namespace = manifest.schemas[0]?.namespace;
        const fields = await project.listFields(entityId);
        return Object.fromEntries(fields.filter((field) => !namespace || field.namespace === namespace).map((field) => [field.key, field.value]));
      },
      set: async (entityId: UUID, key: string, value: unknown) => {
        checkCapability(manifest, "entity.write");
        const namespace = validateField(manifest, key, value);
        await project.setField({ entity_id: entityId, namespace, key, value });
      },
    },
    relationships: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "relationship.read");
        const rels = await project.listRelationships(entityId);
        return rels.map(toRelationship);
      },
      create: async (input: Omit<Relationship, "id">) => {
        checkCapability(manifest, "relationship.write");
        const rel = await project.createRelationship(
          input.sourceId,
          input.targetId,
          input.type,
          input.metadata
        );
        return toRelationship(rel);
      },
    },
    assets: {
      list: async (entityId: UUID) => {
        checkCapability(manifest, "asset.read");
        return (await project.listAssets(entityId)).filter((asset) => manifest.schemas.some((schema) => schema.namespace === asset.namespace)).map(toAsset);
      },
      register: async (input) => {
        checkCapability(manifest, "asset.write");
        if (!manifest.schemas.some((schema) => schema.namespace === input.namespace)) throw new Error(`Module ${manifest.id} does not own namespace: ${input.namespace}`);
        return toAsset(await project.registerAsset({
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
      const entities = await project.search(query);
      return entities.map(toEntitySummary);
    },
  };
}
