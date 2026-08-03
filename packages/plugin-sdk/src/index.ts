export * from "./generated";
import type { PluginManifest } from "./generated";

export interface PluginRpcTransport {
  call(method: string, payload: unknown): Promise<unknown>;
}

export interface PluginRpcClient {
  call<T>(method: string, payload: unknown): Promise<T>;
  publishEvent(name: string, version: number, payload: unknown): Promise<unknown>;
  subscribeEvent(name: string, version: number): Promise<void>;
  pollEvents<T = unknown>(name: string, version: number): Promise<T[]>;
  callService<T = unknown>(name: string, major: number, payload: unknown, deadlineMs?: number): Promise<T>;
}

/** Framework-neutral SDK boundary. The host owns identity and authorization. */
export function createPluginRpcClient(transport: PluginRpcTransport): PluginRpcClient {
  return {
    call: <T>(method: string, payload: unknown) => transport.call(method, payload) as Promise<T>,
    publishEvent: (name, version, payload) => transport.call("event.publish", { type: `${name}@${version}`, payload }),
    subscribeEvent: (name, version) => transport.call("event.subscribe", { type: `${name}@${version}` }).then(() => undefined),
    pollEvents: <T>(name: string, version: number) => transport.call("event.poll", { type: `${name}@${version}` }) as Promise<T[]>,
    callService: <T>(name: string, major: number, payload: unknown, deadlineMs = 5000) => transport.call("service.call", { name, major, payload, deadlineMs }) as Promise<T>,
  };
}

const knownCapabilities = new Set([
  "entity.read", "entity.write", "entity.delete", "document.read", "document.write",
  "field.read:self", "field.read:shared", "field.write:self", "relationship.read",
  "relationship.write", "asset.read:self", "asset.import", "search.query",
  "event.publish:<type>", "event.subscribe:<type>", "service.provide:<name>", "service.call:<name>",
]);

export function isPluginIdentifier(value: string): boolean {
  return value.length > 0 && value.split(".").every((part) => /^[a-z0-9][a-z0-9_-]*$/.test(part));
}

export function isSemanticVersion(value: string): boolean {
  return /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value);
}

export function isPackagePath(value: string): boolean {
  return value.length > 0 && !value.startsWith("/") && !value.includes("\\") && !value.split("/").some((part) => !part || part === "..");
}

export function validatePluginManifest(manifest: PluginManifest): string[] {
  const errors: string[] = [];
  if (manifest.manifestVersion !== 1) errors.push("manifestVersion must be 1");
  if (!isPluginIdentifier(manifest.id)) errors.push("id is invalid");
  if (!isPluginIdentifier(manifest.publisher)) errors.push("publisher is invalid");
  if (!manifest.name.trim()) errors.push("name is required");
  if (!isSemanticVersion(manifest.version)) errors.push("version is invalid");
  if (!manifest.entrypoints.ui && !manifest.entrypoints.wasm) errors.push("an entrypoint is required");
  for (const path of [manifest.entrypoints.ui, manifest.entrypoints.wasm].filter((value): value is string => Boolean(value))) {
    if (!isPackagePath(path)) errors.push(`invalid package path: ${path}`);
  }
  for (const capability of manifest.capabilities) {
    if (!knownCapabilities.has(capability) && !/^(event\.(publish|subscribe)|service\.(provide|call)):.+$/.test(capability)) errors.push(`unknown capability: ${capability}`);
  }
  if (new Set(manifest.namespaces).size !== manifest.namespaces.length) errors.push("duplicate namespace");
  const owned = new Set(manifest.namespaces);
  for (const schema of manifest.schemas) if (!owned.has(schema.namespace)) errors.push(`unowned schema namespace: ${schema.namespace}`);
  let current = 0;
  const migrationIds = new Set<string>();
  for (const migration of [...manifest.migrations].sort((a, b) => a.from - b.from)) {
    if (migration.from !== current || migration.to <= migration.from || migrationIds.has(migration.id)) errors.push("migration chain is invalid");
    migrationIds.add(migration.id);
    current = migration.to;
    for (const operation of migration.operations) if (!owned.has(operation.namespace)) errors.push(`unowned migration namespace: ${operation.namespace}`);
  }
  return errors;
}

export function assertValidPluginManifest(manifest: PluginManifest): asserts manifest is PluginManifest {
  const errors = validatePluginManifest(manifest);
  if (errors.length) throw new Error(`Invalid plugin manifest: ${errors.join("; ")}`);
}
