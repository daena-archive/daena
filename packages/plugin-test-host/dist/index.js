import { assertValidPluginManifest, createPluginRpcClient, } from "@daena-archive/plugin-sdk";
function failure(code, message, retryable = false, details) {
    return { code, message, retryable, details };
}
/**
 * A deterministic broker double. It deliberately exposes no host filesystem,
 * Tauri object, or caller-selected plugin identity.
 */
export class FakePluginHost {
    manifest;
    projectId;
    sessionId;
    calls = [];
    grants;
    entities = new Map();
    committedRequests = new Map();
    queues = new Map();
    subscriptions = new Set();
    services = new Map();
    aiResults = new Map();
    aiEvents = new Map();
    aiCapabilities = new Map();
    physicalTransfers = new Map();
    nextEntity = 1;
    nextRevision = 1;
    revoked = false;
    declarativeActive = false;
    constructor(options) {
        assertValidPluginManifest(options.manifest);
        this.manifest = options.manifest;
        this.projectId = options.projectId ?? "test-project";
        this.sessionId = options.sessionId ?? "test-session";
        this.grants = new Set(options.grants ?? options.manifest.capabilities);
    }
    client() {
        return createPluginRpcClient(this);
    }
    revoke() {
        this.revoked = true;
    }
    seed(entity) {
        this.entities.set(entity.id, structuredClone(entity));
    }
    registerService(name, major, handler) {
        this.services.set(`${name}@${major}`, handler);
    }
    activateDeclarative() {
        if (this.manifest.kind !== "declarative")
            throw new Error("only declarative plugins use the host renderer");
        this.declarativeActive = true;
    }
    deactivateDeclarative() {
        this.declarativeActive = false;
    }
    hostView(viewId) {
        if (!this.declarativeActive)
            throw new Error("declarative plugin is not active");
        const view = this.manifest.views.find((candidate) => candidate.id === viewId);
        if (!view)
            throw new Error("declarative view is not declared");
        return structuredClone(view);
    }
    invokeHostCommand(viewId, commandId, payload = {}) {
        const view = this.hostView(viewId);
        const command = this.manifest.commands.find((candidate) => candidate.id === commandId);
        if (!command?.action)
            throw new Error("declarative command is not executable");
        if (command.exposure?.length && !command.exposure.includes("view"))
            throw new Error("declarative command is not exposed to views");
        if (command.capabilities?.some((capability) => !this.grants.has(capability)))
            throw new Error("declarative command capability is not granted");
        if (!view.components?.some((component) => component.type === "button" && component.command === commandId))
            throw new Error("declarative command is not exposed by this view");
        if (Object.keys(payload).length > 0 && !command.input)
            throw new Error("declarative command does not accept input");
        return { type: command.action.type };
    }
    async call(method, payload, requestId) {
        this.calls.push({ method, payload: structuredClone(payload), requestId });
        if (this.revoked)
            throw failure("session-revoked", "plugin session has been revoked");
        const replay = requestId ? this.committedRequests.get(requestId) : undefined;
        if (replay) {
            if (replay.method !== method)
                throw failure("request-id-reuse", "request ID was already used for another method");
            return structuredClone(replay.result);
        }
        try {
            const result = await (async () => {
                switch (method) {
                    case "plugin.bootstrap":
                        return this.bootstrap();
                    case "entity.list":
                        this.require("entity.read");
                        return this.list(payload);
                    case "entity.create":
                        this.require("entity.write");
                        return this.create(payload);
                    case "entity.update":
                        this.require("entity.write");
                        return this.update(payload);
                    case "entity.delete":
                        this.require("entity.delete");
                        return this.remove(payload);
                    case "event.publish":
                        this.requireDynamic("event.publish", payload);
                        return this.publish(payload);
                    case "event.subscribe":
                        this.requireDynamic("event.subscribe", payload);
                        return this.subscribe(payload);
                    case "event.poll":
                        this.requireDynamic("event.subscribe", payload);
                        return this.poll(payload);
                    case "service.call":
                        this.requireDynamic("service.call", payload);
                        return await this.callService(payload);
                    case "ai.request.start":
                        return this.startAi(payload);
                    case "ai.request.poll":
                        return this.pollAi(payload);
                    case "ai.request.cancel":
                        return this.cancelAi(payload);
                    case "ai.request.result":
                        return this.resultAi(payload);
                    case "maps.physical.create.begin":
                        return this.beginPhysicalCreate(payload);
                    case "maps.physical.create.commit":
                        return this.commitPhysicalCreate(payload);
                    default:
                        throw failure("unknown-method", `unsupported plugin method: ${method}`);
                }
            })();
            if (requestId && ["entity.create", "entity.update", "entity.delete"].includes(method)) {
                this.committedRequests.set(requestId, { method, result: structuredClone(result) });
            }
            return result;
        }
        catch (error) {
            if (isPluginRpcError(error))
                throw error;
            throw failure("host-error", error instanceof Error ? error.message : String(error));
        }
    }
    bootstrap() {
        return {
            rpcVersion: 1,
            pluginId: this.manifest.id,
            sessionId: this.sessionId,
            projectId: this.projectId,
            version: this.manifest.version,
            hostApi: this.manifest.hostApi,
            grantedCapabilities: [...this.grants].sort(),
            optionalFeatures: [],
        };
    }
    require(capability) {
        if (!this.grants.has(capability))
            throw failure("capability-denied", `capability is not granted: ${capability}`);
    }
    beginPhysicalCreate(payload) {
        this.require("entity.write");
        this.require("asset.write:self");
        this.require("field.write:self");
        const value = payload;
        if (typeof value.name !== "string" || !value.name.trim() || typeof value.size !== "number" || value.size < 0)
            throw failure("invalid-payload", "maps.physical.create.begin requires name and non-negative size");
        if (!value.generation || typeof value.generation !== "object")
            throw failure("invalid-payload", "maps.physical.create.begin requires generation");
        const handle = `${this.manifest.id}:physical:${this.physicalTransfers.size + 1}`;
        this.physicalTransfers.set(handle, {
            name: value.name,
            size: value.size,
            generation: structuredClone(value.generation),
        });
        return { handle, url: `memory://${handle}/0`, maxChunkBytes: 1024 * 1024, expiresInMs: 60_000 };
    }
    commitPhysicalCreate(payload) {
        this.require("entity.write");
        this.require("asset.write:self");
        this.require("field.write:self");
        const value = payload;
        if (typeof value.handle !== "string" || typeof value.contentHash !== "string")
            throw failure("invalid-payload", "maps.physical.create.commit requires handle and contentHash");
        const transfer = this.physicalTransfers.get(value.handle);
        if (!transfer)
            throw failure("not-found", "physical upload handle does not exist");
        this.physicalTransfers.delete(value.handle);
        return { accepted: true, name: transfer.name, size: transfer.size, generation: transfer.generation };
    }
    startAi(payload) {
        const value = payload;
        const operation = value.operation;
        const capability = operation === "generate_structured" ? "ai.text.generate-structured" : "ai.text.generate";
        this.require(capability);
        if (!["generate_text", "generate_structured"].includes(String(operation)) ||
            typeof value.userInstruction !== "string" ||
            !value.userInstruction.trim()) {
            throw failure("invalid-payload", "ai.request.start requires a supported operation and instruction");
        }
        const requestId = `${this.manifest.id}:ai:${this.aiResults.size + 1}`;
        const output = operation === "generate_structured"
            ? structuredClone(value.immediateContext ?? {})
            : `${value.userInstruction}: ${String(value.immediateContext?.selection ?? "")}`;
        if (operation === "generate_structured")
            validateFakeAiOutput(value.outputContract, output);
        this.aiResults.set(requestId, output);
        this.aiCapabilities.set(requestId, capability);
        this.aiEvents.set(requestId, [
            { sequence: 0, requestId, phase: "started" },
            { sequence: 1, requestId, phase: "completed", output },
        ]);
        return { requestId };
    }
    pollAi(payload) {
        const requestId = payload.requestId;
        if (typeof requestId !== "string" || !this.aiEvents.has(requestId))
            throw failure("not-found", "AI request does not exist");
        this.requireAi(requestId);
        return structuredClone(this.aiEvents.get(requestId) ?? []);
    }
    cancelAi(payload) {
        const requestId = payload.requestId;
        if (typeof requestId !== "string" || !this.aiResults.has(requestId))
            throw failure("not-found", "AI request does not exist");
        this.requireAi(requestId);
        this.aiResults.delete(requestId);
        this.aiEvents.set(requestId, [{ sequence: 0, requestId, phase: "cancelled" }]);
    }
    resultAi(payload) {
        const requestId = payload.requestId;
        if (typeof requestId !== "string" || !this.aiResults.has(requestId))
            throw failure("not-found", "AI result does not exist");
        this.requireAi(requestId);
        return structuredClone(this.aiResults.get(requestId));
    }
    requireAi(requestId) {
        const capability = this.aiCapabilities.get(requestId);
        if (!capability)
            throw failure("not-found", "AI request does not exist");
        this.require(capability);
    }
    requireDynamic(prefix, payload) {
        const value = payload;
        const name = prefix.startsWith("event.") ? value.type : `${value.name}@${value.major}`;
        if (typeof name !== "string" ||
            (!this.grants.has(`${prefix}:${name}`) &&
                !this.grants.has(`${prefix}:<type>`) &&
                !this.grants.has(`${prefix}:<name>`))) {
            throw failure("capability-denied", `capability is not granted: ${prefix}:${String(name)}`);
        }
    }
    list(payload) {
        const type = payload.entityType;
        return [...this.entities.values()]
            .filter((entity) => typeof type !== "string" || entity.entityType === type)
            .map((entity) => structuredClone(entity));
    }
    create(payload) {
        const value = payload;
        if (typeof value.name !== "string" || !value.name.trim())
            throw failure("invalid-payload", "entity.create requires name");
        const entity = {
            id: `${this.manifest.id}:${this.nextEntity++}`,
            name: value.name,
            entityType: typeof value.type === "string" ? value.type : null,
            deleted: false,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            revision: this.revision(),
        };
        this.entities.set(entity.id, entity);
        return structuredClone(entity);
    }
    update(payload) {
        const value = payload;
        if (typeof value.id !== "string" || !this.entities.has(value.id))
            throw failure("not-found", "entity does not exist");
        const entity = this.entities.get(value.id);
        this.checkRevision(entity, value.expectedRevision);
        if (typeof value.name === "string")
            entity.name = value.name;
        if (typeof value.type === "string")
            entity.entityType = value.type;
        entity.updatedAt = new Date().toISOString();
        entity.revision = this.revision();
        return structuredClone(entity);
    }
    remove(payload) {
        const value = payload;
        const id = value.id;
        const entity = typeof id === "string" ? this.entities.get(id) : undefined;
        if (!entity)
            throw failure("not-found", "entity does not exist");
        this.checkRevision(entity, value.expectedRevision);
        this.entities.delete(id);
    }
    revision() {
        return `revision-${this.nextRevision++}`;
    }
    checkRevision(entity, expected) {
        if (typeof expected !== "string" || !expected)
            throw failure("revision-required", "expectedRevision is required");
        if (expected !== entity.revision)
            throw failure("revision-conflict", "entity revision does not match");
    }
    publish(payload) {
        const value = payload;
        if (typeof value.type !== "string")
            throw failure("invalid-payload", "event type is required");
        if (this.subscriptions.has(value.type))
            this.queues.set(value.type, [...(this.queues.get(value.type) ?? []), structuredClone(value.payload)]);
    }
    subscribe(payload) {
        const type = payload.type;
        if (typeof type !== "string")
            throw failure("invalid-payload", "event type is required");
        this.subscriptions.add(type);
        this.queues.set(type, this.queues.get(type) ?? []);
    }
    poll(payload) {
        const type = payload.type;
        if (typeof type !== "string")
            throw failure("invalid-payload", "event type is required");
        const queue = this.queues.get(type) ?? [];
        this.queues.set(type, []);
        return queue;
    }
    async callService(payload) {
        const value = payload;
        if (typeof value.name !== "string" || typeof value.major !== "number")
            throw failure("invalid-payload", "service name and major are required");
        const handler = this.services.get(`${value.name}@${value.major}`);
        if (!handler)
            throw failure("provider-unavailable", "service provider is unavailable", true);
        return handler(value.payload, {
            pluginId: this.manifest.id,
            projectId: this.projectId,
            deadlineMs: typeof value.deadlineMs === "number" ? value.deadlineMs : 5000,
        });
    }
}
function validateFakeAiOutput(schema, value) {
    if (!schema || typeof schema !== "object")
        throw failure("invalid-output", "structured output contract is required");
    const node = schema;
    switch (node.type) {
        case "object": {
            if (!value || typeof value !== "object" || Array.isArray(value))
                throw failure("invalid-output", "structured output must be an object");
            const properties = node.properties && typeof node.properties === "object" ? node.properties : {};
            for (const required of Array.isArray(node.required) ? node.required : [])
                if (typeof required === "string" && !(required in value))
                    throw failure("invalid-output", `missing structured field: ${required}`);
            if (node.additionalProperties === false)
                for (const key of Object.keys(value))
                    if (!(key in properties))
                        throw failure("invalid-output", `unknown structured field: ${key}`);
            for (const [key, child] of Object.entries(properties))
                if (key in value)
                    validateFakeAiOutput(child, value[key]);
            return;
        }
        case "string":
            if (typeof value !== "string" || (typeof node.maxLength === "number" && value.length > node.maxLength))
                throw failure("invalid-output", "structured string is invalid");
            return;
        case "boolean":
            if (typeof value !== "boolean")
                throw failure("invalid-output", "structured boolean is invalid");
            return;
        case "number":
        case "integer":
            if (typeof value !== "number")
                throw failure("invalid-output", "structured number is invalid");
            return;
        default:
            throw failure("invalid-output", "unsupported structured output type");
    }
}
function isPluginRpcError(value) {
    return typeof value === "object" && value !== null && typeof value.code === "string";
}
/** In-memory lifecycle host for end-to-end author-tool tests. */
export class FakePluginLifecycleHost {
    versions = new Map();
    selected = new Map();
    enabled = new Set();
    data = new Set();
    install(manifest) {
        assertValidPluginManifest(manifest);
        const versions = this.versions.get(manifest.id) ?? new Map();
        if (versions.has(manifest.version))
            throw new Error("plugin version is already installed");
        versions.set(manifest.version, structuredClone(manifest));
        this.versions.set(manifest.id, versions);
        this.data.add(manifest.id);
        if (!this.selected.has(manifest.id))
            this.selected.set(manifest.id, manifest.version);
    }
    enable(pluginId) {
        const versions = this.requireVersions(pluginId);
        if (!this.selected.has(pluginId))
            this.selected.set(pluginId, [...versions.keys()].sort(compareVersions).at(-1));
        this.enabled.add(pluginId);
    }
    disable(pluginId) {
        this.requireVersions(pluginId);
        this.enabled.delete(pluginId);
    }
    upgrade(pluginId, version) {
        const versions = this.requireVersions(pluginId);
        if (!versions.has(version))
            throw new Error("target plugin version is not installed");
        if (!this.enabled.has(pluginId))
            throw new Error("plugin must be enabled before upgrade");
        this.selected.set(pluginId, version);
    }
    rollback(pluginId, version) {
        const versions = this.requireVersions(pluginId);
        if (!versions.has(version))
            throw new Error("rollback target is not installed");
        if (!this.enabled.has(pluginId))
            throw new Error("plugin must be enabled before rollback");
        this.selected.set(pluginId, version);
    }
    uninstallCode(pluginId, version) {
        const versions = this.requireVersions(pluginId);
        if (this.selected.get(pluginId) === version && this.enabled.has(pluginId))
            throw new Error("cannot uninstall the selected enabled version");
        if (!versions.delete(version))
            throw new Error("plugin version is not installed");
        if (versions.size === 0) {
            this.versions.delete(pluginId);
            this.selected.delete(pluginId);
            this.enabled.delete(pluginId);
        }
    }
    deleteData(pluginId) {
        this.requireVersions(pluginId);
        if (this.enabled.has(pluginId))
            throw new Error("disable plugin before deleting project data");
        this.data.delete(pluginId);
    }
    snapshot(pluginId) {
        const versions = this.requireVersions(pluginId);
        return {
            pluginId,
            enabled: this.enabled.has(pluginId),
            selectedVersion: this.selected.get(pluginId) ?? null,
            installedVersions: [...versions.keys()].sort(compareVersions),
            dataPresent: this.data.has(pluginId),
        };
    }
    requireVersions(pluginId) {
        const versions = this.versions.get(pluginId);
        if (!versions)
            throw new Error("plugin is not installed");
        return versions;
    }
}
function compareVersions(left, right) {
    const a = left.split(".").map(Number);
    const b = right.split(".").map(Number);
    return a[0] - b[0] || a[1] - b[1] || a[2] - b[2];
}
export async function runConformance(host) {
    const client = host.client();
    const results = [];
    const check = async (name, operation, expectedFailure) => {
        try {
            await operation();
            results.push({
                name,
                passed: !expectedFailure,
                detail: expectedFailure ? "operation unexpectedly succeeded" : undefined,
            });
        }
        catch (error) {
            const code = error instanceof Error && "code" in error ? String(error.code) : "unknown";
            results.push({ name, passed: code === expectedFailure, detail: code });
        }
    };
    const bootstrap = await client.bootstrap();
    results.push({
        name: "host assigns plugin identity",
        passed: bootstrap.pluginId === host.manifest.id && bootstrap.sessionId === host.sessionId,
    });
    await check("entity read is granted", () => client.listEntities());
    await check("undeclared delete is denied", () => client.deleteEntity("missing"), "capability-denied");
    await check("undeclared event publish is denied", () => client.publishEvent("com.example.event", 1, {}), "capability-denied");
    await check("unknown RPC is rejected", () => client.call("host.filesystem", {}), "unknown-method");
    host.revoke();
    await check("revoked session is denied", () => client.bootstrap(), "session-revoked");
    return results;
}
