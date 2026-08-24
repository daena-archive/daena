import type {
  EntityRecord,
  Event,
  FieldDefinition,
  Migration,
  MigrationAuthoringOptions,
  MigrationOperation,
  PluginManifest,
  PluginRpcError,
  PluginBootstrap,
  Service,
  MutationOptions,
  AiRequestStartPayload,
  AiRequestIdPayload,
  AssetMetadataUpdatePayload,
  AssetDeletePayload,
  EntityPageRecord,
  EntityQueryPayload,
} from "./generated.js";
import { CATALOG_ICON_IDS } from "./generated.js";

export * from "./generated.js";
export type { MetadataFieldDefinition } from "./generated.js";
export * from "./maps.js";

export interface PluginRpcTransport {
  call(method: string, payload: unknown, requestId?: string): Promise<unknown>;
}

export interface BrowserPluginRpcTransportOptions {
  /** Host-assigned package identity. Defaults to `body[data-plugin]`. */
  pluginId?: string;
  /** Current project identity. Defaults to the `project` URL parameter. */
  projectId?: string;
  /** Same-origin broker endpoint. Defaults to `/__rpc`. */
  endpoint?: string;
  /** Injectable for browser tests; defaults to the global fetch function. */
  fetch?: typeof globalThis.fetch;
  /** Matches the host bridge limit and prevents oversized requests client-side. */
  maxRequestBytes?: number;
  /** Matches the host bridge limit and bounds responses before JSON parsing. */
  maxResponseBytes?: number;
}

export interface PluginRpcClient {
  call<T>(method: string, payload: unknown, requestId?: string): Promise<T>;
  bootstrap(): Promise<PluginBootstrap>;
  listEntities(entityType?: string): Promise<EntityRecord[]>;
  queryEntities(query?: EntityQueryPayload): Promise<EntityPageRecord>;
  createEntity(name: string, entityType?: string, options?: MutationOptions): Promise<EntityRecord>;
  updateEntity(id: string, name?: string, entityType?: string | null, options?: MutationOptions): Promise<EntityRecord>;
  deleteEntity(id: string, options?: MutationOptions): Promise<void>;
  publishEvent(name: string, version: number, payload: unknown): Promise<void>;
  subscribeEvent(name: string, version: number): Promise<void>;
  pollEvents<T = unknown>(name: string, version: number): Promise<T[]>;
  callService<T = unknown>(name: string, major: number, payload: unknown, deadlineMs?: number): Promise<T>;
  getAppVersion(): Promise<{ version: string }>;
  beginAssetRead(assetId: string, namespace: string): Promise<AssetReadHandle>;
  updateAssetMetadata(input: AssetMetadataUpdatePayload, options?: MutationOptions): Promise<unknown>;
  deleteAsset(input: AssetDeletePayload, options?: MutationOptions): Promise<void>;
  beginAssetReplace(input: AssetReplaceRequest, options?: MutationOptions): Promise<AssetReplaceHandle>;
  commitAssetReplace(handle: string, contentHash: string, options?: MutationOptions): Promise<unknown>;
  cancelAssetTransfer(handle: string): Promise<void>;
  startAiRequest(request: AiRequestStartPayload): Promise<AiRequestIdPayload>;
  pollAiRequest(requestId: string): Promise<unknown[]>;
  cancelAiRequest(requestId: string): Promise<void>;
  getAiResult(requestId: string): Promise<unknown>;
  getAiCitations(requestId: string): Promise<unknown[]>;
}

export interface AssetReadHandle {
  handle: string;
  url: string;
  size: number;
  contentHash: string;
  mimeType: string;
  revision: string;
}

export interface AssetReplaceRequest {
  assetId: string;
  namespace: string;
  expectedRevision: string;
  size: number;
  mimeType: string;
}

export interface AssetReplaceHandle {
  handle: string;
  url: string;
  maxChunkBytes: number;
  expiresInMs: number;
}

export class PluginRpcException extends Error {
  readonly code: string;
  readonly retryable: boolean;
  readonly details: unknown;

  constructor(error: PluginRpcError) {
    super(error.message);
    this.name = "PluginRpcException";
    this.code = error.code;
    this.retryable = error.retryable;
    this.details = error.details;
  }
}

function qualified(name: string, version: number): string {
  return `${name}@${version}`;
}

async function callTransport<T>(
  transport: PluginRpcTransport,
  method: string,
  payload: unknown,
  requestId?: string,
): Promise<T> {
  try {
    return (await transport.call(method, payload, requestId)) as T;
  } catch (error) {
    if (isRpcError(error)) throw new PluginRpcException(error);
    throw error;
  }
}

function isRpcError(value: unknown): value is PluginRpcError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { code?: unknown }).code === "string" &&
    typeof (value as { message?: unknown }).message === "string" &&
    typeof (value as { retryable?: unknown }).retryable === "boolean"
  );
}

function rpcFailure(code: string, message: string, retryable = false, details?: unknown): PluginRpcError {
  return { code, message, retryable, details };
}

function runtimeValue(name: "pluginId" | "projectId"): string {
  if (name === "pluginId" && typeof document !== "undefined") {
    const value = document.body?.dataset.plugin;
    if (value) return value;
  }
  if (name === "projectId" && typeof location !== "undefined") {
    const value = new URLSearchParams(location.search).get("project");
    if (value) return value;
  }
  throw new Error(`plugin ${name} is not available in the current runtime`);
}

function utf8Length(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}

function responseError(value: unknown, fallback: string): PluginRpcError {
  if (isRecord(value) && isRpcError(value.error)) return value.error;
  if (isRecord(value) && typeof value.error === "string") return rpcFailure("transport.host", value.error);
  return rpcFailure("transport.protocol", fallback);
}

/**
 * Create the production browser transport for an isolated plugin webview.
 *
 * The transport owns the session handshake and request envelope. Plugin code
 * only supplies method names and payloads; it cannot choose the session or
 * host identity used for an RPC request.
 */
export function createBrowserPluginRpcTransport(options: BrowserPluginRpcTransportOptions = {}): PluginRpcTransport {
  const pluginId = options.pluginId ?? runtimeValue("pluginId");
  const projectId = options.projectId ?? runtimeValue("projectId");
  const endpoint = options.endpoint ?? "/__rpc";
  const requestFetch = options.fetch ?? globalThis.fetch?.bind(globalThis);
  const maxRequestBytes = options.maxRequestBytes ?? 256 * 1024;
  const maxResponseBytes = options.maxResponseBytes ?? 256 * 1024;
  if (!requestFetch) throw new Error("plugin RPC requires fetch");

  let sequence = 0;
  let sessionId: string | undefined;
  let handshake: Promise<PluginBootstrap> | undefined;

  async function post(body: Record<string, unknown>): Promise<unknown> {
    const serialized = JSON.stringify(body);
    if (utf8Length(serialized) > maxRequestBytes) {
      throw rpcFailure("payload.invalid", "plugin RPC request exceeds payload limit");
    }
    let response: Response;
    try {
      response = await requestFetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: serialized,
      });
    } catch (cause) {
      throw rpcFailure("transport.unavailable", cause instanceof Error ? cause.message : String(cause), true);
    }
    const text = await response.text();
    if (utf8Length(text) > maxResponseBytes) {
      throw rpcFailure("transport.protocol", "plugin RPC response exceeds payload limit");
    }
    let value: unknown;
    try {
      value = JSON.parse(text);
    } catch {
      throw rpcFailure("transport.protocol", "plugin RPC returned invalid JSON");
    }
    if (!response.ok) throw responseError(value, `plugin RPC HTTP request failed (${response.status})`);
    return value;
  }

  async function bootstrap(): Promise<PluginBootstrap> {
    const value = await post({ op: "bootstrap", pluginId, projectId });
    if (
      !isRecord(value) ||
      value.rpcVersion !== 1 ||
      value.pluginId !== pluginId ||
      value.projectId !== projectId ||
      typeof value.sessionId !== "string" ||
      !value.sessionId ||
      typeof value.version !== "string" ||
      typeof value.hostApi !== "string" ||
      !Array.isArray(value.grantedCapabilities) ||
      !Array.isArray(value.optionalFeatures)
    ) {
      throw rpcFailure("transport.protocol", "plugin bootstrap response is invalid");
    }
    sessionId = value.sessionId;
    return value as unknown as PluginBootstrap;
  }

  async function ensureSession(): Promise<void> {
    if (sessionId) return;
    handshake ??= bootstrap().finally(() => {
      handshake = undefined;
    });
    await handshake;
  }

  async function call(method: string, payload: unknown, suppliedRequestId?: string): Promise<unknown> {
    if (method === "plugin.bootstrap") return bootstrap();
    await ensureSession();
    const requestId = suppliedRequestId ?? globalThis.crypto?.randomUUID?.() ?? `${pluginId}-${++sequence}`;
    const value = await post({
      op: "rpc",
      request: { rpcVersion: 1, sessionId, requestId, method, payload },
    });
    if (!isRecord(value) || value.rpcVersion !== 1 || value.requestId !== requestId || typeof value.ok !== "boolean") {
      throw rpcFailure("transport.protocol", "plugin RPC response does not match the request");
    }
    if (!value.ok) {
      if (!isRpcError(value.error)) throw rpcFailure("transport.protocol", "plugin RPC returned an invalid error");
      throw value.error;
    }
    if (!("result" in value)) throw rpcFailure("transport.protocol", "plugin RPC success has no result");
    return value.result;
  }

  return { call };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function normalizeEntity(value: unknown): EntityRecord {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    typeof value.deleted !== "boolean" ||
    typeof value.revision !== "string"
  ) {
    throw rpcFailure("transport.protocol", "broker returned an invalid entity record");
  }
  const entityType = value.entityType ?? value.entity_type;
  const createdAt = value.createdAt ?? value.created_at;
  const updatedAt = value.updatedAt ?? value.updated_at;
  if (
    (entityType !== null && typeof entityType !== "string") ||
    typeof createdAt !== "string" ||
    typeof updatedAt !== "string"
  ) {
    throw rpcFailure("transport.protocol", "broker returned an invalid entity record");
  }
  return {
    id: value.id,
    name: value.name,
    entityType: entityType as string | null,
    deleted: value.deleted,
    createdAt,
    updatedAt,
    revision: value.revision,
  };
}

function normalizeEntityPage(value: unknown): EntityPageRecord {
  if (
    !isRecord(value) ||
    !Array.isArray(value.items) ||
    !Array.isArray(value.typeCounts) ||
    typeof value.total !== "number" ||
    typeof value.offset !== "number" ||
    typeof value.limit !== "number" ||
    typeof value.hasMore !== "boolean"
  ) {
    throw rpcFailure("transport.protocol", "broker returned an invalid entity page");
  }
  const typeCounts = value.typeCounts.map((entry) => {
    if (!isRecord(entry) || typeof entry.count !== "number")
      throw rpcFailure("transport.protocol", "broker returned an invalid entity type count");
    const entityType = entry.entityType;
    if (entityType !== null && entityType !== undefined && typeof entityType !== "string")
      throw rpcFailure("transport.protocol", "broker returned an invalid entity type count");
    return { entityType: entityType as string | null | undefined, count: entry.count };
  });
  return {
    items: value.items.map(normalizeEntity),
    total: value.total,
    offset: value.offset,
    limit: value.limit,
    hasMore: value.hasMore,
    typeCounts,
  };
}

function checkKeys(value: Record<string, unknown>, label: string, allowed: string[], errors: string[]): void {
  const known = new Set(allowed);
  for (const key of Object.keys(value)) if (!known.has(key)) errors.push(`unknown ${label} key: ${key}`);
}

const catalogIconIds = new Set<string>(CATALOG_ICON_IDS);

function validateIconRef(value: unknown, label: string, errors: string[]): void {
  if (!isRecord(value)) {
    errors.push(`${label} must be an object`);
    return;
  }
  if (value.kind === "catalog") {
    checkKeys(value, label, ["kind", "id"], errors);
    if (typeof value.id !== "string" || !catalogIconIds.has(value.id))
      errors.push(`${label} uses an unknown catalog icon`);
    return;
  }
  if (value.kind === "plugin-svg") {
    checkKeys(value, label, ["kind", "path"], errors);
    if (typeof value.path !== "string" || !isPackagePath(value.path) || !value.path.toLowerCase().endsWith(".svg"))
      errors.push(`${label} must reference a package-relative SVG file`);
    return;
  }
  errors.push(`${label} has an unknown kind`);
}

/** Framework-neutral SDK boundary. The host owns identity and authorization. */
export function createPluginRpcClient(transport: PluginRpcTransport): PluginRpcClient {
  return {
    call: <T>(method: string, payload: unknown, requestId?: string) =>
      callTransport<T>(transport, method, payload, requestId),
    bootstrap: () => callTransport<PluginBootstrap>(transport, "plugin.bootstrap", {}),
    listEntities: async (entityType?: string) =>
      (await callTransport<unknown[]>(transport, "entity.list", entityType ? { entityType } : {})).map(normalizeEntity),
    queryEntities: async (query: EntityQueryPayload = {}) =>
      normalizeEntityPage(await callTransport<unknown>(transport, "entity.query", query)),
    createEntity: async (name, entityType, options) =>
      normalizeEntity(
        await callTransport<unknown>(
          transport,
          "entity.create",
          { name, type: entityType ?? null },
          options?.requestId,
        ),
      ),
    updateEntity: async (id, name, entityType, options) =>
      normalizeEntity(
        await callTransport<unknown>(
          transport,
          "entity.update",
          { id, name: name ?? null, type: entityType ?? null, expectedRevision: options?.expectedRevision },
          options?.requestId,
        ),
      ),
    deleteEntity: (id, options) =>
      callTransport<void>(
        transport,
        "entity.delete",
        { id, expectedRevision: options?.expectedRevision },
        options?.requestId,
      ),
    publishEvent: (name, version, payload) =>
      callTransport<void>(transport, "event.publish", { type: qualified(name, version), payload }),
    subscribeEvent: (name, version) =>
      callTransport<void>(transport, "event.subscribe", { type: qualified(name, version) }),
    pollEvents: <T>(name: string, version: number) =>
      callTransport<T[]>(transport, "event.poll", { type: qualified(name, version) }),
    callService: <T>(name: string, major: number, payload: unknown, deadlineMs = 5000) =>
      callTransport<T>(transport, "service.call", { name, major, payload, deadlineMs }),
    getAppVersion: () => callTransport<{ version: string }>(transport, "app.version", {}),
    beginAssetRead: (assetId, namespace) =>
      callTransport<AssetReadHandle>(transport, "asset.read.begin", { assetId, namespace }),
    updateAssetMetadata: (input, options) =>
      callTransport<unknown>(transport, "asset.update", input, options?.requestId),
    deleteAsset: (input, options) => callTransport<void>(transport, "asset.delete", input, options?.requestId),
    beginAssetReplace: (input, options) =>
      callTransport<AssetReplaceHandle>(transport, "asset.replace.begin", input, options?.requestId),
    commitAssetReplace: (handle, contentHash, options) =>
      callTransport<unknown>(transport, "asset.replace.commit", { handle, contentHash }, options?.requestId),
    cancelAssetTransfer: (handle) => callTransport<void>(transport, "asset.transfer.cancel", { handle }),
    startAiRequest: (request) => callTransport<AiRequestIdPayload>(transport, "ai.request.start", request),
    pollAiRequest: (requestId) => callTransport<unknown[]>(transport, "ai.request.poll", { requestId }),
    cancelAiRequest: (requestId) => callTransport<void>(transport, "ai.request.cancel", { requestId }),
    getAiResult: (requestId) => callTransport<unknown>(transport, "ai.request.result", { requestId }),
    getAiCitations: (requestId) => callTransport<unknown[]>(transport, "ai.request.citations", { requestId }),
  };
}

export async function uploadAssetChunks(
  transfer: AssetReplaceHandle,
  bytes: Uint8Array,
  fetcher: typeof globalThis.fetch = globalThis.fetch,
): Promise<void> {
  if (bytes.length > 64 * 1024 * 1024) throw new Error("asset exceeds SDK transfer limit");
  for (
    let offset = 0, chunk = 0;
    offset < bytes.length || (bytes.length === 0 && chunk === 0);
    offset += transfer.maxChunkBytes, chunk += 1
  ) {
    const response = await fetcher(`${transfer.url.replace(/\/0\?/, `/${chunk}?`)}`, {
      method: "PUT",
      body: bytes.slice(offset, offset + transfer.maxChunkBytes),
    });
    if (!response.ok) throw new Error(`asset upload failed: ${response.status}`);
    if (bytes.length === 0) break;
  }
}

const knownCapabilities = new Set([
  "ai.text.generate",
  "ai.text.generate-structured",
  "entity.read",
  "entity.write",
  "entity.delete",
  "document.read",
  "document.write",
  "field.read:self",
  "field.read:shared",
  "field.write:self",
  "record.read:self",
  "record.write:self",
  "relationship.read",
  "relationship.write",
  "asset.read:self",
  "asset.read:shared",
  "asset.write:self",
  "asset.register",
  "search.query",
  "schema.overlay",
  "event.publish:<type>",
  "event.subscribe:<type>",
  "host.surface:<name>@<major>",
  "service.provide:<name>",
  "service.call:<name>",
]);

export function isPluginIdentifier(value: string): boolean {
  return value.length > 0 && value.split(".").every((part) => /^[a-z0-9][a-z0-9_-]*$/.test(part));
}

export function isHostSurfaceId(value: string): boolean {
  const [namespace, surface, ...extra] = value.split("/");
  return (
    extra.length === 0 &&
    typeof namespace === "string" &&
    isPluginIdentifier(namespace) &&
    typeof surface === "string" &&
    /^[a-z0-9][a-z0-9_.-]*$/.test(surface)
  );
}

export function isSemanticVersion(value: string): boolean {
  return /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value);
}

export function isHostApiRange(value: string): boolean {
  return (
    value.trim().split(/\s+/).length > 0 &&
    value
      .trim()
      .split(/\s+/)
      .every((part) =>
        /^(?:\^|~|>=|<=|>|<|=)?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(
          part,
        ),
      )
  );
}

export function isPackagePath(value: string): boolean {
  return (
    value.length > 0 &&
    !value.startsWith("/") &&
    !value.includes("\\") &&
    !value.split("/").some((part) => !part || part === ".." || part === ".")
  );
}

function validateCommandSchema(value: unknown, label: string, errors: string[]): void {
  if (!isRecord(value)) {
    errors.push(`${label} must be an object`);
    return;
  }
  checkKeys(value, label, ["type", "properties", "required", "additionalProperties"], errors);
  if (value.type !== "object") errors.push(`${label} type must be object`);
  if (value.properties !== undefined && !isRecord(value.properties))
    errors.push(`${label} properties must be an object`);
  if (value.required !== undefined && !Array.isArray(value.required)) errors.push(`${label} required must be an array`);
  if (typeof value.additionalProperties !== "undefined" && typeof value.additionalProperties !== "boolean")
    errors.push(`${label} additionalProperties must be boolean`);
  if (Array.isArray(value.required) && isRecord(value.properties)) {
    const required = new Set<string>();
    for (const key of value.required) {
      if (typeof key !== "string" || !key.trim() || required.has(key) || !(key in value.properties))
        errors.push(`${label} has invalid required property`);
      required.add(key);
    }
  }
  if (isRecord(value.properties))
    for (const [key, property] of Object.entries(value.properties)) {
      if (!isRecord(property)) {
        errors.push(`${label} property ${key} must be an object`);
        continue;
      }
      checkKeys(property, `${label} property`, ["type"], errors);
      if (!["object", "string", "number", "boolean", "array", "null"].includes(String(property.type)))
        errors.push(`${label} property ${key} has an invalid type`);
    }
}

export function validatePluginManifest(manifest: PluginManifest): string[] {
  const errors: string[] = [];
  const knownManifestKeys = new Set([
    "manifestVersion",
    "id",
    "name",
    "version",
    "publisher",
    "enabledByDefault",
    "stability",
    "hostApi",
    "kind",
    "entrypoints",
    "capabilities",
    "dependencies",
    "namespaces",
    "schemas",
    "templates",
    "records",
    "views",
    "commands",
    "services",
    "events",
    "migrations",
  ]);
  const value = manifest as unknown as Record<string, unknown>;
  for (const key of Object.keys(value)) if (!knownManifestKeys.has(key)) errors.push(`unknown manifest key: ${key}`);
  for (const key of knownManifestKeys)
    if (key !== "enabledByDefault" && key !== "stability" && key !== "records" && !(key in value))
      errors.push(`missing manifest key: ${key}`);
  if (value.manifestVersion !== 1) errors.push("manifestVersion must be 1");
  if (typeof value.id !== "string" || !isPluginIdentifier(value.id)) errors.push("id is invalid");
  if (typeof value.publisher !== "string" || !isPluginIdentifier(value.publisher)) errors.push("publisher is invalid");
  if (typeof value.name !== "string" || !value.name.trim()) errors.push("name is required");
  if (typeof value.version !== "string" || !isSemanticVersion(value.version)) errors.push("version is invalid");
  if (typeof value.hostApi !== "string" || !isHostApiRange(value.hostApi)) errors.push("hostApi is invalid");
  if (value.enabledByDefault !== undefined && typeof value.enabledByDefault !== "boolean")
    errors.push("enabledByDefault must be a boolean");
  if (value.stability !== undefined && !["stable", "beta", "experimental"].includes(value.stability as string))
    errors.push("stability is invalid");
  if (value.kind !== "declarative" && value.kind !== "sandboxed") errors.push("kind is invalid");
  const entrypoints = value.entrypoints;
  if (!entrypoints || typeof entrypoints !== "object" || Array.isArray(entrypoints))
    errors.push("entrypoints must be an object");
  else {
    for (const key of Object.keys(entrypoints))
      if (key !== "ui" && key !== "wasm") errors.push(`unknown entrypoint key: ${key}`);
    if (!("ui" in entrypoints) && !("wasm" in entrypoints)) errors.push("an entrypoint is required");
    if ("ui" in entrypoints && entrypoints.ui !== undefined && typeof entrypoints.ui !== "string")
      errors.push("entrypoint ui must be a package path");
    if ("wasm" in entrypoints && entrypoints.wasm !== undefined && typeof entrypoints.wasm !== "string")
      errors.push("entrypoint wasm must be a package path");
  }
  const capabilities = value.capabilities;
  const dependencies = value.dependencies;
  const namespaces = value.namespaces;
  const schemas = value.schemas;
  const templates = value.templates;
  const records = value.records;
  const views = value.views;
  const commands = value.commands;
  const services = value.services;
  const events = value.events;
  const migrations = value.migrations;
  if (!Array.isArray(capabilities)) errors.push("capabilities must be an array");
  if (!dependencies || typeof dependencies !== "object" || Array.isArray(dependencies))
    errors.push("dependencies must be an object");
  if (!Array.isArray(namespaces)) errors.push("namespaces must be an array");
  if (!Array.isArray(schemas)) errors.push("schemas must be an array");
  if (!Array.isArray(templates)) errors.push("templates must be an array");
  if (records !== undefined && !Array.isArray(records)) errors.push("records must be an array");
  if (!Array.isArray(views)) errors.push("views must be an array");
  if (!Array.isArray(commands)) errors.push("commands must be an array");
  if (!services || typeof services !== "object" || Array.isArray(services)) errors.push("services must be an object");
  if (!events || typeof events !== "object" || Array.isArray(events)) errors.push("events must be an object");
  if (!Array.isArray(migrations)) errors.push("migrations must be an array");
  if (Array.isArray(schemas))
    for (const schema of schemas) {
      if (!isRecord(schema)) {
        errors.push("schemas must contain objects");
        continue;
      }
      checkKeys(schema, "schema", ["namespace", "entityTypes", "fields"], errors);
      if (!Array.isArray(schema.entityTypes) || !Array.isArray(schema.fields))
        errors.push("schema entityTypes and fields must be arrays");
      else {
        for (const entityType of schema.entityTypes) {
          if (!isRecord(entityType)) {
            errors.push("schema entityTypes must contain objects");
            continue;
          }
          checkKeys(entityType, "entity type", ["id", "name", "icon"], errors);
          if (typeof entityType.id !== "string" || !entityType.id.trim()) errors.push("entity type id is required");
          if (typeof entityType.name !== "string" || !entityType.name.trim())
            errors.push(`entity type ${String(entityType.id)} name is required`);
          validateIconRef(entityType.icon, `entity type ${String(entityType.id)} icon`, errors);
        }
        for (const field of schema.fields) {
          if (!isRecord(field)) {
            errors.push("schema fields must contain objects");
            continue;
          }
          checkKeys(
            field,
            "field",
            [
              "key",
              "label",
              "type",
              "required",
              "options",
              "entityTypes",
              "relationshipType",
              "targetEntityTypes",
              "shared",
              "metadataFields",
              "timeline",
            ],
            errors,
          );
          if (field.shared !== undefined && typeof field.shared !== "boolean")
            errors.push(`field ${String(field.key)} shared must be boolean`);
          if (field.timeline !== undefined) {
            if (!isRecord(field.timeline)) {
              errors.push(`field ${String(field.key)} timeline must be an object`);
            } else {
              checkKeys(field.timeline, "timeline contribution", ["role", "group", "label", "layer"], errors);
              if (field.type !== "date")
                errors.push(`field ${String(field.key)} timeline contribution requires a date field`);
              if (field.shared !== true) errors.push(`field ${String(field.key)} timeline contribution must be shared`);
              if (!["point", "start", "end"].includes(String(field.timeline.role)))
                errors.push(`field ${String(field.key)} timeline contribution role is invalid`);
              if (
                (field.timeline.role === "start" || field.timeline.role === "end") &&
                (typeof field.timeline.group !== "string" || !field.timeline.group.trim())
              )
                errors.push(`field ${String(field.key)} timeline start/end contribution requires a group`);
              if (
                field.timeline.label !== undefined &&
                (typeof field.timeline.label !== "string" || !field.timeline.label.trim())
              )
                errors.push(`field ${String(field.key)} timeline contribution label is invalid`);
              if (field.timeline.layer !== undefined && !["dates", "lifelines"].includes(String(field.timeline.layer)))
                errors.push(`field ${String(field.key)} timeline contribution layer is invalid`);
            }
          }
          if (field.metadataFields !== undefined) {
            if (field.type !== "relationship") {
              errors.push(`non-relationship field ${String(field.key)} cannot declare metadataFields`);
            } else if (!Array.isArray(field.metadataFields)) {
              errors.push(`relationship field ${String(field.key)} metadataFields must be an array`);
            } else {
              const metadataKeys = new Set<string>();
              for (const metadataField of field.metadataFields) {
                if (!isRecord(metadataField)) {
                  errors.push(`relationship field ${String(field.key)} metadataFields must contain objects`);
                  continue;
                }
                checkKeys(metadataField, "metadata field", ["key", "label", "type", "required", "options"], errors);
                if (typeof metadataField.key !== "string" || !metadataField.key.trim())
                  errors.push(`relationship field ${String(field.key)} has an invalid metadata key`);
                else if (metadataKeys.has(metadataField.key))
                  errors.push(
                    `relationship field ${String(field.key)} has duplicate metadata key: ${metadataField.key}`,
                  );
                else metadataKeys.add(metadataField.key);
                if (typeof metadataField.label !== "string" || !metadataField.label.trim())
                  errors.push(`relationship metadata field ${String(metadataField.key)} requires a label`);
                if (!["text", "number", "boolean", "date", "enum"].includes(String(metadataField.type)))
                  errors.push(`relationship metadata field ${String(metadataField.key)} has an unsupported type`);
                if (
                  metadataField.type === "enum" &&
                  (!Array.isArray(metadataField.options) || metadataField.options.length === 0)
                )
                  errors.push(`relationship metadata enum field ${String(metadataField.key)} requires options`);
              }
            }
          }
        }
      }
    }
  if (Array.isArray(templates))
    for (const template of templates) {
      if (!isRecord(template)) {
        errors.push("templates must contain objects");
        continue;
      }
      checkKeys(
        template,
        "template",
        ["id", "name", "entityType", "description", "icon", "fields", "requiredFields", "document"],
        errors,
      );
      if (!isRecord(template.fields)) errors.push("template fields must be an object");
      if (template.icon !== undefined && template.icon !== null)
        validateIconRef(template.icon, `template ${String(template.id)} icon`, errors);
    }
  for (const [label, list] of [
    ["views", views],
    ["commands", commands],
  ] as const)
    if (Array.isArray(list))
      for (const item of list) {
        if (!isRecord(item)) {
          errors.push(`${label} must contain objects`);
          continue;
        }
        checkKeys(
          item,
          label.slice(0, -1),
          label === "views"
            ? ["id", "title", "renderer", "components"]
            : ["id", "title", "action", "input", "output", "capabilities", "exposure"],
          errors,
        );
        if (label === "views" && item.renderer !== undefined) {
          if (!isRecord(item.renderer) || typeof item.renderer.type !== "string") {
            errors.push(`view ${String(item.id)} renderer must be a typed object`);
          } else {
            checkKeys(item.renderer, "view renderer", ["type", "id", "major"], errors);
            if (item.renderer.type === "host-surface") {
              if (typeof item.renderer.id !== "string" || !isHostSurfaceId(item.renderer.id))
                errors.push(`view ${String(item.id)} host surface id is invalid`);
              if (
                typeof item.renderer.major !== "number" ||
                !Number.isInteger(item.renderer.major) ||
                item.renderer.major < 1
              )
                errors.push(`view ${String(item.id)} host surface major is invalid`);
            } else if (item.renderer.type !== "declarative" && item.renderer.type !== "sandboxed") {
              errors.push(`view ${String(item.id)} renderer type is invalid`);
            } else if (item.renderer.id !== undefined || item.renderer.major !== undefined) {
              errors.push(`view ${String(item.id)} renderer has unexpected host surface fields`);
            }
          }
        }
        if (label === "commands") {
          if (item.input !== undefined) validateCommandSchema(item.input, `command ${String(item.id)} input`, errors);
          if (item.output !== undefined)
            validateCommandSchema(item.output, `command ${String(item.id)} output`, errors);
          if (item.capabilities !== undefined) {
            if (!Array.isArray(item.capabilities))
              errors.push(`command ${String(item.id)} capabilities must be an array`);
            else
              for (const capability of item.capabilities) {
                if (
                  typeof capability !== "string" ||
                  !Array.isArray(capabilities) ||
                  !capabilities.includes(capability)
                )
                  errors.push(`command ${String(item.id)} requires an undeclared capability`);
              }
          }
          if (item.exposure !== undefined) {
            if (!Array.isArray(item.exposure)) errors.push(`command ${String(item.id)} exposure must be an array`);
            else if (item.exposure.some((exposure) => !["view", "broker"].includes(String(exposure))))
              errors.push(`command ${String(item.id)} exposure is invalid`);
          }
        }
        if (label !== "views" || item.components === undefined) continue;
        if (!Array.isArray(item.components)) {
          errors.push("view components must be an array");
          continue;
        }
        for (const component of item.components) {
          if (!isRecord(component) || typeof component.type !== "string") {
            errors.push("view components must contain typed objects");
            continue;
          }
          if (component.type === "heading" || component.type === "text")
            checkKeys(component, "view component", ["type", "id", "text"], errors);
          else if (component.type === "entity-list")
            checkKeys(component, "view component", ["type", "id", "title", "entityType", "limit"], errors);
          else if (component.type === "entity-detail")
            checkKeys(component, "view component", ["type", "id", "title", "source"], errors);
          else if (component.type === "field-form")
            checkKeys(
              component,
              "view component",
              ["type", "id", "title", "source", "namespace", "fields", "editable"],
              errors,
            );
          else if (component.type === "button")
            checkKeys(component, "view component", ["type", "id", "label", "command"], errors);
          else errors.push(`unknown view component type: ${component.type}`);
        }
      }
  if (isRecord(services)) {
    checkKeys(services, "services", ["provides", "consumes"], errors);
    if (!Array.isArray(services.provides) || !Array.isArray(services.consumes))
      errors.push("services provides and consumes must be arrays");
  }
  if (isRecord(events)) {
    checkKeys(events, "events", ["publishes", "subscribes"], errors);
    if (!Array.isArray(events.publishes) || !Array.isArray(events.subscribes))
      errors.push("events publishes and subscribes must be arrays");
  }
  if (isRecord(dependencies))
    for (const [id, dependency] of Object.entries(dependencies)) {
      if (!isRecord(dependency)) {
        errors.push(`dependency ${id} must be an object`);
        continue;
      }
      checkKeys(dependency, "dependency", ["version", "required"], errors);
    }
  if (Array.isArray(migrations))
    for (const item of migrations) {
      if (!isRecord(item)) {
        errors.push("migrations must contain objects");
        continue;
      }
      checkKeys(item, "migration", ["id", "from", "to", "recovery", "operations"], errors);
      if (!Array.isArray(item.operations)) errors.push("migration operations must be an array");
    }
  if (Array.isArray(commands))
    for (const command of commands) {
      if (!isRecord(command) || command.action === undefined) continue;
      if (!isRecord(command.action) || command.action.type !== "refresh-view")
        errors.push(`command ${String(command.id)} has an unsupported action`);
      else checkKeys(command.action, "command action", ["type"], errors);
    }
  if (Array.isArray(commands)) {
    const commandIds = new Set<string>();
    for (const command of commands)
      if (isRecord(command)) {
        if (typeof command.id !== "string" || !command.id.trim() || commandIds.has(command.id))
          errors.push(`duplicate or invalid command id: ${String(command.id)}`);
        commandIds.add(String(command.id));
        if (typeof command.title !== "string" || !command.title.trim())
          errors.push(`command ${String(command.id)} title must not be empty`);
        if (Array.isArray(command.exposure)) {
          const exposures = new Set(command.exposure.map(String));
          if (exposures.size !== command.exposure.length)
            errors.push(`command ${String(command.id)} has duplicate exposure`);
        }
      }
  }
  if (errors.length) return [...new Set(errors)];
  const entrypointRecord = entrypoints as { ui?: unknown; wasm?: unknown };
  const capabilityList = capabilities as unknown[];
  const namespaceList = namespaces as string[];
  const entrypointValues = [entrypointRecord.ui, entrypointRecord.wasm].filter(
    (item): item is string => typeof item === "string",
  );
  for (const path of entrypointValues) {
    if (!isPackagePath(path)) errors.push(`invalid package path: ${path}`);
  }
  for (const capability of capabilityList) {
    if (typeof capability !== "string") {
      errors.push("capabilities must contain strings");
      continue;
    }
    if (
      !knownCapabilities.has(capability) &&
      !/^(event\.(publish|subscribe)|service\.(provide|call)):.+$/.test(capability) &&
      !/^host\.surface:[^@]+@\d+$/.test(capability)
    )
      errors.push(`unknown capability: ${capability}`);
  }
  if (new Set(capabilityList).size !== capabilityList.length) errors.push("duplicate capability");
  if (new Set(namespaceList).size !== namespaceList.length) errors.push("duplicate namespace");
  const owned = new Set(namespaceList);
  for (const schema of schemas as PluginManifest["schemas"])
    if (!owned.has(schema.namespace)) errors.push(`unowned schema namespace: ${schema.namespace}`);
  const entityTypeDefinitions = (schemas as PluginManifest["schemas"]).flatMap((schema) => schema.entityTypes);
  const entityTypes = new Set(entityTypeDefinitions.map((entityType) => entityType.id));
  if (entityTypes.size !== entityTypeDefinitions.length) errors.push("duplicate entity type");
  if (Array.isArray(records)) {
    const recordIds = new Set<string>();
    for (const record of records) {
      if (!isRecord(record)) {
        errors.push("record collections must be objects");
        continue;
      }
      checkKeys(record, "record collection", ["id", "ownerEntityTypes", "schema"], errors);
      if (typeof record.id !== "string" || !isPluginIdentifier(record.id))
        errors.push("record collection id is invalid");
      else if (recordIds.has(record.id)) errors.push(`duplicate record collection: ${record.id}`);
      else recordIds.add(record.id);
      if (
        !Array.isArray(record.ownerEntityTypes) ||
        record.ownerEntityTypes.length === 0 ||
        record.ownerEntityTypes.some((type) => typeof type !== "string" || !entityTypes.has(type))
      )
        errors.push(`record collection ${String(record.id)} has invalid owner entity types`);
      validateCommandSchema(record.schema, `record collection ${String(record.id)} schema`, errors);
    }
  }
  const fields = new Map<string, FieldDefinition>();
  for (const schema of schemas as PluginManifest["schemas"])
    for (const field of schema.fields) {
      if (fields.has(field.key)) errors.push(`duplicate field key: ${field.key}`);
      fields.set(field.key, field);
      if (field.entityTypes?.some((type) => !entityTypes.has(type)))
        errors.push(`field ${field.key} uses an unknown entity type`);
      if (
        field.entityTypes &&
        (field.entityTypes.length === 0 || new Set(field.entityTypes).size !== field.entityTypes.length)
      )
        errors.push(`field ${field.key} has empty or duplicate entity types`);
      if (field.type === "relationship" && (!field.relationshipType || !field.targetEntityTypes?.length))
        errors.push(`relationship field ${field.key} is incomplete`);
      if (
        field.type === "relationship" &&
        field.targetEntityTypes &&
        new Set(field.targetEntityTypes).size !== field.targetEntityTypes.length
      )
        errors.push(`relationship field ${field.key} has duplicate target entity types`);
      if (field.type !== "relationship" && (field.relationshipType || field.targetEntityTypes))
        errors.push(`non-relationship field ${field.key} has relationship metadata`);
    }
  const templateIds = new Set<string>();
  for (const template of templates as PluginManifest["templates"]) {
    if (templateIds.has(template.id)) errors.push(`duplicate template id: ${template.id}`);
    templateIds.add(template.id);
    if (!entityTypes.has(template.entityType)) errors.push(`template ${template.id} uses an unknown entity type`);
    for (const key of Object.keys(template.fields)) {
      const field = fields.get(key);
      if (!field) errors.push(`template ${template.id} uses undeclared field: ${key}`);
      else if (field.entityTypes && !field.entityTypes.includes(template.entityType))
        errors.push(`template ${template.id} uses an inapplicable field: ${key}`);
      else {
        const preset = template.fields[key];
        if (preset !== null && preset !== "") {
          let valid = false;
          switch (field.type) {
            case "text":
              valid = typeof preset === "string";
              break;
            case "relationship":
              valid = Array.isArray(preset) && preset.every((target) => typeof target === "string");
              break;
            case "number":
              valid = typeof preset === "number";
              break;
            case "boolean":
              valid = typeof preset === "boolean";
              break;
            case "date":
              valid = typeof preset === "string" || isRecord(preset);
              break;
            case "enum":
              valid = field.multiple
                ? Array.isArray(preset) &&
                  preset.every((value) => typeof value === "string" && (field.options?.includes(value) ?? false))
                : typeof preset === "string" && (field.options?.includes(preset) ?? false);
              break;
          }
          if (!valid) errors.push(`template ${template.id} has invalid preset for field: ${key}`);
        }
      }
    }
    for (const key of template.requiredFields ?? [])
      if (!fields.has(key)) errors.push(`template ${template.id} requires undeclared field: ${key}`);
  }
  const viewIds = new Set<string>();
  for (const view of views as PluginManifest["views"]) {
    if (viewIds.has(view.id)) errors.push(`duplicate view id: ${view.id}`);
    viewIds.add(view.id);
    if (typeof view.id !== "string" || !view.id.trim()) errors.push(`view id must not be empty`);
    if (typeof view.title !== "string" || !view.title.trim()) errors.push(`view ${view.id} title must not be empty`);
    const componentIds = new Set<string>();
    for (const component of view.components ?? []) {
      if (typeof component.id !== "string" || !component.id.trim())
        errors.push(`view ${view.id} component id must not be empty`);
      if (componentIds.has(component.id)) errors.push(`duplicate view component id: ${component.id}`);
      componentIds.add(component.id);
      if (component.type === "heading" || component.type === "text") {
        if (typeof component.text !== "string" || !component.text.trim())
          errors.push(`view ${view.id} ${component.type} text must not be empty`);
      } else if (component.type === "entity-list") {
        if (typeof component.title !== "string" || !component.title.trim())
          errors.push(`view ${view.id} entity list title must not be empty`);
        if (!entityTypes.has(component.entityType)) errors.push(`view ${view.id} lists an unknown entity type`);
        if (!capabilityList.includes("entity.read")) errors.push(`view ${view.id} entity list requires entity.read`);
        if (!Number.isInteger(component.limit) || component.limit < 1 || component.limit > 100)
          errors.push(`view ${view.id} entity list limit is invalid`);
      } else if (component.type === "entity-detail") {
        if (typeof component.title !== "string" || !component.title.trim())
          errors.push(`view ${view.id} entity detail title must not be empty`);
        if (typeof component.source !== "string" || !component.source.trim())
          errors.push(`view ${view.id} entity detail source must not be empty`);
        if (
          !view.components?.some((candidate) => candidate.type === "entity-list" && candidate.id === component.source)
        )
          errors.push(`view ${view.id} detail references an unknown entity list`);
        if (!capabilityList.includes("entity.read")) errors.push(`view ${view.id} entity detail requires entity.read`);
      } else if (component.type === "field-form") {
        const source = view.components?.find(
          (
            candidate,
          ): candidate is Extract<
            NonNullable<PluginManifest["views"][number]["components"]>[number],
            { type: "entity-list" }
          > => candidate.type === "entity-list" && candidate.id === component.source,
        );
        if (!source) errors.push(`view ${view.id} form references an unknown entity list`);
        if (typeof component.title !== "string" || !component.title.trim())
          errors.push(`view ${view.id} field form title must not be empty`);
        if (typeof component.source !== "string" || !component.source.trim())
          errors.push(`view ${view.id} field form source must not be empty`);
        if (typeof component.namespace !== "string" || !component.namespace.trim())
          errors.push(`view ${view.id} field form namespace must not be empty`);
        if (
          !Array.isArray(component.fields) ||
          component.fields.length === 0 ||
          component.fields.some((key) => typeof key !== "string" || !key.trim())
        )
          errors.push(`view ${view.id} field form must list at least one non-empty field`);
        if (!owned.has(component.namespace)) errors.push(`view ${view.id} form uses an unowned namespace`);
        if (Array.isArray(component.fields)) {
          for (const key of component.fields) {
            const field = fields.get(key);
            if (!field || (field.entityTypes?.length && source && !field.entityTypes.includes(source.entityType)))
              errors.push(`view ${view.id} form uses an invalid field: ${key}`);
          }
        }
        if (!capabilityList.includes("field.read:self"))
          errors.push(`view ${view.id} field form requires field.read:self`);
        if (component.editable && !capabilityList.includes("field.write:self"))
          errors.push(`view ${view.id} editable form requires field.write:self`);
      } else if (component.type === "button") {
        if (typeof component.label !== "string" || !component.label.trim())
          errors.push(`view ${view.id} button label must not be empty`);
        const command = (commands as PluginManifest["commands"]).find(
          (candidate) => candidate.id === component.command,
        );
        if (typeof component.command !== "string" || !component.command.trim())
          errors.push(`view ${view.id} button command must not be empty`);
        else if (!command?.action) errors.push(`view ${view.id} button references a command without a host action`);
        else if (command.exposure?.length && !command.exposure.includes("view"))
          errors.push(`view ${view.id} button references a command not exposed to views`);
      }
    }
  }
  errors.push(...validateMigrationChain(migrations as PluginManifest["migrations"], namespaceList));
  return [...new Set(errors)];
}

export function assertValidPluginManifest(manifest: PluginManifest): asserts manifest is PluginManifest {
  const errors = validatePluginManifest(manifest);
  if (errors.length) throw new Error(`Invalid plugin manifest: ${errors.join("; ")}`);
}

/** Stable JSON representation for reproducible package digests and review. */
export function canonicalize(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.entries(value as Record<string, unknown>)
      .filter(([, item]) => item !== undefined)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([key, item]) => `${JSON.stringify(key)}:${canonicalize(item)}`)
      .join(",")}}`;
  }
  return value === undefined ? "null" : JSON.stringify(value);
}

export function canonicalManifestJson(manifest: PluginManifest): string {
  assertValidPluginManifest(manifest);
  return `${canonicalize(manifest)}\n`;
}

export function migration(
  options: { id: string; from: number; to: number; operations: MigrationOperation[] } & MigrationAuthoringOptions,
): Migration {
  const result: Migration = {
    id: options.id,
    from: options.from,
    to: options.to,
    recovery: options.recovery ?? "backup",
    operations: options.operations,
  };
  if (result.from < 0 || result.to <= result.from || !result.id.trim())
    throw new Error("migration must have a non-empty ID and increasing versions");
  return result;
}

export function validateMigrationChain(migrations: Migration[], namespaces: string[] = []): string[] {
  const errors: string[] = [];
  let current = 0;
  const ids = new Set<string>();
  for (const item of [...migrations].sort((a, b) => a.from - b.from)) {
    if (item.from !== current || item.to <= item.from || ids.has(item.id)) errors.push("migration chain is invalid");
    ids.add(item.id);
    current = item.to;
    for (const operation of item.operations) {
      const namespace = "namespace" in operation ? operation.namespace : "";
      if (namespaces.length && !namespaces.includes(namespace))
        errors.push(`migration uses unowned namespace: ${namespace}`);
    }
  }
  return [...new Set(errors)];
}

export function createMigrationOperation(
  kind: MigrationOperation["kind"],
  namespace: string,
  value: Omit<MigrationOperation, "kind" | "namespace"> = {},
): MigrationOperation {
  return { kind, namespace, ...value } as MigrationOperation;
}

export function service(name: string, major: number): Service {
  return { name, major };
}
export function event(name: string, version: number): Event {
  return { name, version };
}
