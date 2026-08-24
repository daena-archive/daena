import type { EntityRecord, Event, Migration, MigrationAuthoringOptions, MigrationOperation, PluginManifest, PluginRpcError, PluginBootstrap, Service, MutationOptions, AiRequestStartPayload, AiRequestIdPayload, AssetMetadataUpdatePayload, AssetDeletePayload, EntityPageRecord, EntityQueryPayload } from "./generated.js";
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
    getAppVersion(): Promise<{
        version: string;
    }>;
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
export declare class PluginRpcException extends Error {
    readonly code: string;
    readonly retryable: boolean;
    readonly details: unknown;
    constructor(error: PluginRpcError);
}
/**
 * Create the production browser transport for an isolated plugin webview.
 *
 * The transport owns the session handshake and request envelope. Plugin code
 * only supplies method names and payloads; it cannot choose the session or
 * host identity used for an RPC request.
 */
export declare function createBrowserPluginRpcTransport(options?: BrowserPluginRpcTransportOptions): PluginRpcTransport;
/** Framework-neutral SDK boundary. The host owns identity and authorization. */
export declare function createPluginRpcClient(transport: PluginRpcTransport): PluginRpcClient;
export declare function uploadAssetChunks(transfer: AssetReplaceHandle, bytes: Uint8Array, fetcher?: typeof globalThis.fetch): Promise<void>;
export declare function isPluginIdentifier(value: string): boolean;
export declare function isHostSurfaceId(value: string): boolean;
export declare function isSemanticVersion(value: string): boolean;
export declare function isHostApiRange(value: string): boolean;
export declare function isPackagePath(value: string): boolean;
export declare function validatePluginManifest(manifest: PluginManifest): string[];
export declare function assertValidPluginManifest(manifest: PluginManifest): asserts manifest is PluginManifest;
/** Stable JSON representation for reproducible package digests and review. */
export declare function canonicalize(value: unknown): string;
export declare function canonicalManifestJson(manifest: PluginManifest): string;
export declare function migration(options: {
    id: string;
    from: number;
    to: number;
    operations: MigrationOperation[];
} & MigrationAuthoringOptions): Migration;
export declare function validateMigrationChain(migrations: Migration[], namespaces?: string[]): string[];
export declare function createMigrationOperation(kind: MigrationOperation["kind"], namespace: string, value?: Omit<MigrationOperation, "kind" | "namespace">): MigrationOperation;
export declare function service(name: string, major: number): Service;
export declare function event(name: string, version: number): Event;
//# sourceMappingURL=index.d.ts.map