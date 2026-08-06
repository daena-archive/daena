export type { PluginManifest, FieldDefinition, SchemaContribution, EntityTemplate, Migration, MigrationOperation, View, ViewComponent, Command, CommandAction, Service, Event, Services, Events } from "../../plugin-sdk/src/generated";
import type { PluginManifest, Migration } from "../../plugin-sdk/src/generated";
import type { MapNavigationService } from "../../plugin-sdk/src/maps";

export type UUID = string & { readonly __brand: "UUID" };
export type ModuleId = string & { readonly __brand: "ModuleId" };
export type Capability = string;

export interface EntitySummary {
  id: UUID;
  name: string;
  type: string | null;
  deleted: boolean;
  revision: string;
}

export interface EntityRecord extends EntitySummary {
  createdAt: string;
  updatedAt: string;
  documents: DocumentRecord[];
  fields: Record<string, unknown>;
}

export interface DocumentRecord {
  id: UUID;
  entityId: UUID;
  format: "markdown" | "plain-text" | "rich-text";
  body: string;
  updatedAt: string;
  revision: string;
}

export interface Relationship {
  id: UUID;
  sourceId: UUID;
  targetId: UUID;
  type: string;
  metadata: Record<string, unknown>;
  revision: string;
}

export interface AssetRecord {
  id: UUID;
  entityId: UUID;
  namespace: string;
  filename: string;
  contentHash: string;
  size: number;
  mimeType: string;
  path: string;
  createdAt: string;
  revision: string;
}

export interface MutationOptions {
  expectedRevision?: string;
  requestId?: string;
}

export interface FieldRecord {
  entityId: UUID;
  namespace: string;
  key: string;
  value: unknown;
  revision: string;
}

export interface EntityQuery {
  type?: string;
  text?: string;
  includeDeleted?: boolean;
  limit?: number;
}

export interface EntityCreateInput {
  name: string;
  type?: string;
  fields?: Record<string, unknown>;
  relationships?: Record<string, UUID[]>;
  document?: {
    body: string;
    format?: DocumentRecord["format"];
  };
}

export interface CalendarDate {
  calendar: "gregorian";
  era: "BCE" | "CE";
  year: number;
  month?: number;
  day?: number;
  precision: "year" | "month" | "day";
}

export type ModuleManifest = Omit<PluginManifest, "id"> & { id: ModuleId };
export type DeclarativeMigration = Migration;

export interface ModuleContext {
  readonly module: ModuleManifest;
  entities: {
    get(id: UUID): Promise<EntityRecord | null>;
    list(query?: EntityQuery): Promise<EntitySummary[]>;
    create(input: EntityCreateInput, options?: MutationOptions): Promise<EntityRecord>;
    update(id: UUID, patch: { name?: string; type?: string | null }, options?: MutationOptions): Promise<EntityRecord>;
    delete(id: UUID, options?: MutationOptions): Promise<void>;
  };
  documents: {
    save(input: { entityId: UUID; body: string; format?: DocumentRecord["format"] }, options?: MutationOptions): Promise<DocumentRecord>;
  };
  fields: {
    list(entityId: UUID): Promise<Record<string, unknown>>;
    listRecords(entityId: UUID): Promise<FieldRecord[]>;
    set(entityId: UUID, key: string, value: unknown, options?: MutationOptions): Promise<void>;
  };
  relationships: {
    list(entityId: UUID): Promise<Relationship[]>;
    create(input: Omit<Relationship, "id" | "revision">, options?: MutationOptions): Promise<Relationship>;
    delete(id: UUID, relationshipType: string, options?: MutationOptions): Promise<void>;
  };
  assets: {
    list(entityId: UUID): Promise<AssetRecord[]>;
    register(input: Omit<AssetRecord, "id" | "createdAt" | "revision" | "entityId"> & { entityId: UUID }, options?: MutationOptions): Promise<AssetRecord>;
  };
  search(query: string): Promise<EntitySummary[]>;
  maps: MapNavigationService;
}

export interface ModuleView {
  id: string;
  title: string;
  mount(element: HTMLElement, context: ModuleContext): () => void;
}

export interface DaenaModule {
  manifest: ModuleManifest;
  views: ModuleView[];
  register?(context: ModuleContext): Promise<void>;
}

export function requireCapabilities(manifest: ModuleManifest, required: Capability[]): void {
  const granted = new Set(manifest.capabilities);
  const missing = required.filter((capability) => !granted.has(capability));
  if (missing.length > 0) {
    throw new Error(`Module ${manifest.id} lacks capabilities: ${missing.join(", ")}`);
  }
}

export function isMigrationContiguous(migrations: DeclarativeMigration[], current: number): boolean {
  return [...migrations].sort((a, b) => a.from - b.from).every((migration) => {
    const valid = migration.from === current;
    current = migration.to;
    return valid && migration.to > migration.from;
  });
}
