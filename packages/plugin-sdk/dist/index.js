export * from "./generated.js";
export class PluginRpcException extends Error {
    code;
    retryable;
    details;
    constructor(error) {
        super(error.message);
        this.name = "PluginRpcException";
        this.code = error.code;
        this.retryable = error.retryable;
        this.details = error.details;
    }
}
function qualified(name, version) {
    return `${name}@${version}`;
}
async function callTransport(transport, method, payload) {
    try {
        return await transport.call(method, payload);
    }
    catch (error) {
        if (isRpcError(error))
            throw new PluginRpcException(error);
        throw error;
    }
}
function isRpcError(value) {
    return typeof value === "object" && value !== null &&
        typeof value.code === "string" &&
        typeof value.message === "string" &&
        typeof value.retryable === "boolean";
}
function isRecord(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
function checkKeys(value, label, allowed, errors) {
    const known = new Set(allowed);
    for (const key of Object.keys(value))
        if (!known.has(key))
            errors.push(`unknown ${label} key: ${key}`);
}
/** Framework-neutral SDK boundary. The host owns identity and authorization. */
export function createPluginRpcClient(transport) {
    return {
        call: (method, payload) => callTransport(transport, method, payload),
        bootstrap: () => callTransport(transport, "plugin.bootstrap", {}),
        listEntities: (entityType) => callTransport(transport, "entity.list", entityType ? { entityType } : {}),
        createEntity: (entityType, fields, document) => callTransport(transport, "entity.create", { entityType, fields, document }),
        updateEntity: (id, fields, document) => callTransport(transport, "entity.update", { id, fields, document }),
        deleteEntity: (id) => callTransport(transport, "entity.delete", { id }),
        publishEvent: (name, version, payload) => callTransport(transport, "event.publish", { type: qualified(name, version), payload }),
        subscribeEvent: (name, version) => callTransport(transport, "event.subscribe", { type: qualified(name, version) }),
        pollEvents: (name, version) => callTransport(transport, "event.poll", { type: qualified(name, version) }),
        callService: (name, major, payload, deadlineMs = 5000) => callTransport(transport, "service.call", { name, major, payload, deadlineMs }),
    };
}
const knownCapabilities = new Set([
    "entity.read", "entity.write", "entity.delete", "document.read", "document.write",
    "field.read:self", "field.read:shared", "field.write:self", "relationship.read",
    "relationship.write", "asset.read:self", "asset.import", "search.query",
    "event.publish:<type>", "event.subscribe:<type>", "service.provide:<name>", "service.call:<name>",
]);
export function isPluginIdentifier(value) {
    return value.length > 0 && value.split(".").every((part) => /^[a-z0-9][a-z0-9_-]*$/.test(part));
}
export function isSemanticVersion(value) {
    return /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value);
}
export function isHostApiRange(value) {
    return value.trim().split(/\s+/).length > 0 && value.trim().split(/\s+/).every((part) => /^(?:\^|~|>=|<=|>|<|=)?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(part));
}
export function isPackagePath(value) {
    return value.length > 0 && !value.startsWith("/") && !value.includes("\\") && !value.split("/").some((part) => !part || part === ".." || part === ".");
}
export function validatePluginManifest(manifest) {
    const errors = [];
    const knownManifestKeys = new Set(["manifestVersion", "id", "name", "version", "publisher", "hostApi", "kind", "entrypoints", "capabilities", "dependencies", "namespaces", "schemas", "templates", "views", "commands", "services", "events", "migrations"]);
    const value = manifest;
    for (const key of Object.keys(value))
        if (!knownManifestKeys.has(key))
            errors.push(`unknown manifest key: ${key}`);
    for (const key of knownManifestKeys)
        if (!(key in value))
            errors.push(`missing manifest key: ${key}`);
    if (value.manifestVersion !== 1)
        errors.push("manifestVersion must be 1");
    if (typeof value.id !== "string" || !isPluginIdentifier(value.id))
        errors.push("id is invalid");
    if (typeof value.publisher !== "string" || !isPluginIdentifier(value.publisher))
        errors.push("publisher is invalid");
    if (typeof value.name !== "string" || !value.name.trim())
        errors.push("name is required");
    if (typeof value.version !== "string" || !isSemanticVersion(value.version))
        errors.push("version is invalid");
    if (typeof value.hostApi !== "string" || !isHostApiRange(value.hostApi))
        errors.push("hostApi is invalid");
    if (value.kind !== "declarative" && value.kind !== "sandboxed")
        errors.push("kind is invalid");
    const entrypoints = value.entrypoints;
    if (!entrypoints || typeof entrypoints !== "object" || Array.isArray(entrypoints))
        errors.push("entrypoints must be an object");
    else {
        for (const key of Object.keys(entrypoints))
            if (key !== "ui" && key !== "wasm")
                errors.push(`unknown entrypoint key: ${key}`);
        if (!("ui" in entrypoints) && !("wasm" in entrypoints))
            errors.push("an entrypoint is required");
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
    const views = value.views;
    const commands = value.commands;
    const services = value.services;
    const events = value.events;
    const migrations = value.migrations;
    if (!Array.isArray(capabilities))
        errors.push("capabilities must be an array");
    if (!dependencies || typeof dependencies !== "object" || Array.isArray(dependencies))
        errors.push("dependencies must be an object");
    if (!Array.isArray(namespaces))
        errors.push("namespaces must be an array");
    if (!Array.isArray(schemas))
        errors.push("schemas must be an array");
    if (!Array.isArray(templates))
        errors.push("templates must be an array");
    if (!Array.isArray(views))
        errors.push("views must be an array");
    if (!Array.isArray(commands))
        errors.push("commands must be an array");
    if (!services || typeof services !== "object" || Array.isArray(services))
        errors.push("services must be an object");
    if (!events || typeof events !== "object" || Array.isArray(events))
        errors.push("events must be an object");
    if (!Array.isArray(migrations))
        errors.push("migrations must be an array");
    if (Array.isArray(schemas))
        for (const schema of schemas) {
            if (!isRecord(schema)) {
                errors.push("schemas must contain objects");
                continue;
            }
            checkKeys(schema, "schema", ["namespace", "entityTypes", "fields"], errors);
            if (!Array.isArray(schema.entityTypes) || !Array.isArray(schema.fields))
                errors.push("schema entityTypes and fields must be arrays");
            else
                for (const field of schema.fields) {
                    if (!isRecord(field)) {
                        errors.push("schema fields must contain objects");
                        continue;
                    }
                    checkKeys(field, "field", ["key", "label", "type", "required", "options", "entityTypes", "relationshipType", "targetEntityTypes"], errors);
                }
        }
    if (Array.isArray(templates))
        for (const template of templates) {
            if (!isRecord(template)) {
                errors.push("templates must contain objects");
                continue;
            }
            checkKeys(template, "template", ["id", "name", "entityType", "description", "icon", "fields", "requiredFields", "document"], errors);
            if (!isRecord(template.fields))
                errors.push("template fields must be an object");
        }
    for (const [label, list] of [["views", views], ["commands", commands]])
        if (Array.isArray(list))
            for (const item of list) {
                if (!isRecord(item)) {
                    errors.push(`${label} must contain objects`);
                    continue;
                }
                checkKeys(item, label.slice(0, -1), ["id", "title"], errors);
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
            if (!Array.isArray(item.operations))
                errors.push("migration operations must be an array");
        }
    if (errors.length)
        return [...new Set(errors)];
    const entrypointRecord = entrypoints;
    const capabilityList = capabilities;
    const namespaceList = namespaces;
    const entrypointValues = [entrypointRecord.ui, entrypointRecord.wasm].filter((item) => typeof item === "string");
    for (const path of entrypointValues) {
        if (!isPackagePath(path))
            errors.push(`invalid package path: ${path}`);
    }
    for (const capability of capabilityList) {
        if (typeof capability !== "string") {
            errors.push("capabilities must contain strings");
            continue;
        }
        if (!knownCapabilities.has(capability) && !/^(event\.(publish|subscribe)|service\.(provide|call)):.+$/.test(capability))
            errors.push(`unknown capability: ${capability}`);
    }
    if (new Set(capabilityList).size !== capabilityList.length)
        errors.push("duplicate capability");
    if (new Set(namespaceList).size !== namespaceList.length)
        errors.push("duplicate namespace");
    const owned = new Set(namespaceList);
    for (const schema of schemas)
        if (!owned.has(schema.namespace))
            errors.push(`unowned schema namespace: ${schema.namespace}`);
    const entityTypes = new Set(schemas.flatMap((schema) => schema.entityTypes));
    const fields = new Map();
    for (const schema of schemas)
        for (const field of schema.fields) {
            if (fields.has(field.key))
                errors.push(`duplicate field key: ${field.key}`);
            fields.set(field.key, field);
            if (field.entityTypes?.some((type) => !entityTypes.has(type)))
                errors.push(`field ${field.key} uses an unknown entity type`);
            if (field.type === "relationship" && (!field.relationshipType || !field.targetEntityTypes?.length))
                errors.push(`relationship field ${field.key} is incomplete`);
            if (field.type !== "relationship" && (field.relationshipType || field.targetEntityTypes))
                errors.push(`non-relationship field ${field.key} has relationship metadata`);
        }
    const templateIds = new Set();
    for (const template of templates) {
        if (templateIds.has(template.id))
            errors.push(`duplicate template id: ${template.id}`);
        templateIds.add(template.id);
        if (!entityTypes.has(template.entityType))
            errors.push(`template ${template.id} uses an unknown entity type`);
        for (const key of Object.keys(template.fields)) {
            const field = fields.get(key);
            if (!field)
                errors.push(`template ${template.id} uses undeclared field: ${key}`);
            else if (field.entityTypes && !field.entityTypes.includes(template.entityType))
                errors.push(`template ${template.id} uses an inapplicable field: ${key}`);
        }
        for (const key of template.requiredFields ?? [])
            if (!fields.has(key))
                errors.push(`template ${template.id} requires undeclared field: ${key}`);
    }
    errors.push(...validateMigrationChain(migrations, namespaceList));
    return [...new Set(errors)];
}
export function assertValidPluginManifest(manifest) {
    const errors = validatePluginManifest(manifest);
    if (errors.length)
        throw new Error(`Invalid plugin manifest: ${errors.join("; ")}`);
}
/** Stable JSON representation for reproducible package digests and review. */
export function canonicalize(value) {
    if (Array.isArray(value))
        return `[${value.map(canonicalize).join(",")}]`;
    if (value && typeof value === "object") {
        return `{${Object.entries(value).filter(([, item]) => item !== undefined).sort(([a], [b]) => a.localeCompare(b)).map(([key, item]) => `${JSON.stringify(key)}:${canonicalize(item)}`).join(",")}}`;
    }
    return value === undefined ? "null" : JSON.stringify(value);
}
export function canonicalManifestJson(manifest) {
    assertValidPluginManifest(manifest);
    return `${canonicalize(manifest)}\n`;
}
export function migration(options) {
    const result = { id: options.id, from: options.from, to: options.to, recovery: options.recovery ?? "backup", operations: options.operations };
    if (result.from < 0 || result.to <= result.from || !result.id.trim())
        throw new Error("migration must have a non-empty ID and increasing versions");
    return result;
}
export function validateMigrationChain(migrations, namespaces = []) {
    const errors = [];
    let current = 0;
    const ids = new Set();
    for (const item of [...migrations].sort((a, b) => a.from - b.from)) {
        if (item.from !== current || item.to <= item.from || ids.has(item.id))
            errors.push("migration chain is invalid");
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
export function createMigrationOperation(kind, namespace, value = {}) {
    return { kind, namespace, ...value };
}
export function service(name, major) { return { name, major }; }
export function event(name, version) { return { name, version }; }
//# sourceMappingURL=index.js.map