import { type EntityRecord, type PluginManifest, type PluginRpcClient, type PluginRpcTransport } from "@daena-archive/plugin-sdk";
export interface FakePluginHostOptions {
    manifest: PluginManifest;
    projectId?: string;
    sessionId?: string;
    grants?: Iterable<string>;
}
export interface FakeServiceContext {
    pluginId: string;
    projectId: string;
    deadlineMs: number;
}
type ServiceHandler = (payload: unknown, context: FakeServiceContext) => unknown | Promise<unknown>;
/**
 * A deterministic broker double. It deliberately exposes no host filesystem,
 * Tauri object, or caller-selected plugin identity.
 */
export declare class FakePluginHost implements PluginRpcTransport {
    readonly manifest: PluginManifest;
    readonly projectId: string;
    readonly sessionId: string;
    readonly calls: Array<{
        method: string;
        payload: unknown;
        requestId?: string;
    }>;
    private readonly grants;
    private readonly entities;
    private readonly committedRequests;
    private readonly queues;
    private readonly subscriptions;
    private readonly services;
    private readonly aiResults;
    private readonly aiEvents;
    private readonly aiCapabilities;
    private readonly physicalTransfers;
    private nextEntity;
    private nextRevision;
    private revoked;
    private declarativeActive;
    constructor(options: FakePluginHostOptions);
    client(): PluginRpcClient;
    revoke(): void;
    seed(entity: EntityRecord): void;
    registerService(name: string, major: number, handler: ServiceHandler): void;
    activateDeclarative(): void;
    deactivateDeclarative(): void;
    hostView(viewId: string): PluginManifest["views"][number];
    invokeHostCommand(viewId: string, commandId: string, payload?: Record<string, unknown>): {
        type: string;
    };
    call(method: string, payload: unknown, requestId?: string): Promise<unknown>;
    private bootstrap;
    private require;
    private beginPhysicalCreate;
    private commitPhysicalCreate;
    private startAi;
    private pollAi;
    private cancelAi;
    private resultAi;
    private requireAi;
    private requireDynamic;
    private list;
    private query;
    private create;
    private update;
    private remove;
    private revision;
    private checkRevision;
    private publish;
    private subscribe;
    private poll;
    private callService;
}
export interface ConformanceResult {
    name: string;
    passed: boolean;
    detail?: string;
}
export interface PluginLifecycleSnapshot {
    pluginId: string;
    enabled: boolean;
    selectedVersion: string | null;
    installedVersions: string[];
    dataPresent: boolean;
}
/** In-memory lifecycle host for end-to-end author-tool tests. */
export declare class FakePluginLifecycleHost {
    private readonly versions;
    private readonly selected;
    private readonly enabled;
    private readonly data;
    install(manifest: PluginManifest): void;
    enable(pluginId: string): void;
    disable(pluginId: string): void;
    upgrade(pluginId: string, version: string): void;
    rollback(pluginId: string, version: string): void;
    uninstallCode(pluginId: string, version: string): void;
    deleteData(pluginId: string): void;
    snapshot(pluginId: string): PluginLifecycleSnapshot;
    private requireVersions;
}
export declare function runConformance(host: FakePluginHost): Promise<ConformanceResult[]>;
export {};
//# sourceMappingURL=index.d.ts.map