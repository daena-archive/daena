export type {
  PluginManifest,
  FieldDefinition,
  MetadataFieldDefinition,
  TimelineFieldContribution,
  TimelineFieldLayer,
  TimelineFieldRole,
  SchemaContribution,
  EntityTemplate,
  Migration,
  MigrationOperation,
  View,
  ViewComponent,
  Command,
  CommandAction,
  Service,
  Event,
  Services,
  Events,
} from "../../plugin-sdk/src/generated";
import type { PluginManifest, Migration } from "../../plugin-sdk/src/generated";
import type { MapNavigationService, MapLocationsMutations } from "../../plugin-sdk/src/maps";

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
  role: "attachment" | "profile";
  referenceScope: "entity" | "project";
  revision: string;
}

export interface MutationOptions {
  expectedRevision?: string;
  requestId?: string;
}

export interface StructuredAiRequest {
  taskId: string;
  userInstruction: string;
  immediateContext: Record<string, unknown>;
  outputContract: Record<string, unknown>;
  deadlineMs?: number;
  retrievalPolicy?: {
    mode: "none" | "explicit_only" | "related" | "project";
    query?: string;
    seedIds: string[];
    allowedSourceKinds: string[];
    relationshipDepth: number;
    passageCount: number;
    includeSharedFields: boolean;
  };
}

export interface StructuredAiHandle {
  requestId: string;
  poll(): Promise<unknown[]>;
  cancel(): Promise<void>;
  result(): Promise<unknown>;
  citations(): Promise<unknown[]>;
}

export interface ProposalPreviewOptions {
  title: string;
  proposal: string;
  original?: string;
  acceptLabel?: string;
  citations?: AiCitation[];
  onAccept(value: string): Promise<void>;
  onDiscard(): void;
}

export interface AiCitation {
  sourceKind: string;
  summary?: string | null;
  entityId?: string | null;
  documentId?: string | null;
  canonicalPath?: string | null;
  revision: string;
  contentHash: string;
  byteStart?: number | null;
  byteEnd?: number | null;
  excerptHash: string;
  stale?: boolean;
}

/** Shared editable proposal surface for bundled and third-party module views. */
export function createProposalPreview(options: ProposalPreviewOptions): HTMLElement {
  const root = document.createElement("div");
  const heading = document.createElement("strong");
  heading.textContent = options.title;
  const editor = document.createElement("textarea");
  editor.value = options.proposal;
  editor.rows = 6;
  editor.setAttribute("aria-label", "Editable AI proposal");
  if (options.original !== undefined) {
    const before = document.createElement("pre");
    before.textContent = options.original;
    before.setAttribute("aria-label", "Original value");
    root.append(before);
  }
  const actions = document.createElement("div");
  actions.className = "lore-graph-toolbar-actions";
  const discard = document.createElement("button");
  discard.type = "button";
  discard.textContent = "Discard";
  discard.onclick = options.onDiscard;
  const accept = document.createElement("button");
  accept.type = "button";
  accept.textContent = options.acceptLabel ?? "Accept proposal";
  accept.onclick = () =>
    void options
      .onAccept(editor.value)
      .then(() => {
        heading.textContent = "Proposal accepted";
        accept.disabled = true;
        discard.disabled = true;
      })
      .catch((cause) => {
        heading.textContent = cause instanceof Error ? cause.message : String(cause);
      });
  actions.append(discard, accept);
  if (options.citations?.length) {
    const inspector = document.createElement("details");
    inspector.className = "ai-citation-inspector";
    const summary = document.createElement("summary");
    summary.textContent = `Sources (${options.citations.length})`;
    inspector.append(summary);
    const list = document.createElement("ul");
    for (const citation of options.citations) {
      const item = document.createElement("li");
      const source = citation.summary ?? citation.canonicalPath ?? citation.sourceKind;
      item.textContent = `${citation.stale ? "Stale · " : ""}${source} · revision ${citation.revision}`;
      if (
        citation.byteStart !== null &&
        citation.byteStart !== undefined &&
        citation.byteEnd !== null &&
        citation.byteEnd !== undefined
      ) {
        item.textContent += ` · bytes ${citation.byteStart}-${citation.byteEnd}`;
      }
      list.append(item);
    }
    inspector.append(list);
    root.append(inspector);
  }
  root.append(heading, editor, actions);
  return root;
}

export interface FieldRecord {
  entityId: UUID;
  namespace: string;
  key: string;
  value: unknown;
  revision: string;
}

export interface ModuleRecord<T = Record<string, unknown>> {
  id: UUID;
  collection: string;
  ownerEntityId: UUID;
  value: T;
  createdAt: string;
  updatedAt: string;
  revision: string;
}

export interface ModuleRecordQuery {
  query?: string;
  limit?: number;
  offset?: number;
  sort?: "lemma" | "symbol" | "name" | "title" | "updatedAt" | "status";
  status?: string;
  tag?: string;
  homonymsOnly?: boolean;
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
  calendar: string;
  era: "BCE" | "CE";
  year: number;
  month?: number;
  day?: number;
  hour?: number;
  minute?: number;
  second?: number;
  precision: "year" | "month" | "day" | "hour" | "minute" | "second";
}

export type ModuleManifest = Omit<PluginManifest, "id"> & { id: ModuleId };
export type DeclarativeMigration = Migration;

export interface ModuleContext {
  readonly module: ModuleManifest;
  /** Host-selected entity to preserve when opening a module projection. */
  readonly focusEntityId?: UUID;
  /** True when the host renders this module inside a native workspace panel
   * that owns entity selection and creation, so the module should not draw
   * its own entity list. */
  readonly embedded?: boolean;
  readonly services: {
    isAvailable(name: string, major: number): boolean;
  };
  modules: {
    /** Effective enabled manifests, including project schema overlays. */
    list(): Promise<ModuleManifest[]>;
  };
  entities: {
    get(id: UUID): Promise<EntityRecord | null>;
    list(query?: EntityQuery): Promise<EntitySummary[]>;
    create(input: EntityCreateInput, options?: MutationOptions): Promise<EntityRecord>;
    update(id: UUID, patch: { name?: string; type?: string | null }, options?: MutationOptions): Promise<EntityRecord>;
    delete(id: UUID, options?: MutationOptions): Promise<void>;
  };
  documents: {
    save(
      input: { entityId: UUID; body: string; format?: DocumentRecord["format"] },
      options?: MutationOptions,
    ): Promise<DocumentRecord>;
  };
  fields: {
    list(entityId: UUID): Promise<Record<string, unknown>>;
    listRecords(entityId: UUID): Promise<FieldRecord[]>;
    listShared(entityId: UUID, namespace: string): Promise<FieldRecord[]>;
    set(entityId: UUID, key: string, value: unknown, options?: MutationOptions): Promise<void>;
  };
  records: {
    list<T = Record<string, unknown>>(
      collection: string,
      ownerEntityId: UUID,
      query?: ModuleRecordQuery,
    ): Promise<ModuleRecord<T>[]>;
    create<T = Record<string, unknown>>(
      collection: string,
      ownerEntityId: UUID,
      value: T,
      options?: MutationOptions,
    ): Promise<ModuleRecord<T>>;
    update<T = Record<string, unknown>>(
      collection: string,
      id: UUID,
      ownerEntityId: UUID,
      value: T,
      options: MutationOptions,
    ): Promise<ModuleRecord<T>>;
    delete(collection: string, id: UUID, ownerEntityId: UUID, options: MutationOptions): Promise<void>;
  };
  relationships: {
    list(entityId: UUID): Promise<Relationship[]>;
    create(input: Omit<Relationship, "id" | "revision">, options?: MutationOptions): Promise<Relationship>;
    update(
      input: { id: UUID; metadata?: Record<string, unknown>; targetId?: UUID },
      options?: MutationOptions,
    ): Promise<Relationship>;
    delete(id: UUID, relationshipType: string, options?: MutationOptions): Promise<void>;
  };
  assets: {
    list(entityId: UUID): Promise<AssetRecord[]>;
    register(
      input: Omit<AssetRecord, "id" | "createdAt" | "role" | "referenceScope" | "revision" | "entityId"> & {
        entityId: UUID;
      },
      options?: MutationOptions,
    ): Promise<AssetRecord>;
    updateMetadata(
      asset: Pick<AssetRecord, "id" | "namespace" | "revision">,
      update: {
        filename?: string;
        role?: "attachment" | "profile";
        referenceScope?: "entity" | "project";
      },
      options?: MutationOptions,
    ): Promise<AssetRecord>;
    delete(asset: Pick<AssetRecord, "id" | "namespace" | "revision">, options?: MutationOptions): Promise<void>;
  };
  search(query: string): Promise<EntitySummary[]>;
  ai: {
    startStructured(request: StructuredAiRequest): Promise<StructuredAiHandle>;
  };
  maps: MapNavigationService & MapLocationsMutations;
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
  return [...migrations]
    .sort((a, b) => a.from - b.from)
    .every((migration) => {
      const valid = migration.from === current;
      current = migration.to;
      return valid && migration.to > migration.from;
    });
}
