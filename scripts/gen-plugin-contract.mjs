// Regenerate `packages/plugin-sdk/src/generated.ts` from the four Rust-derived
// contract schemas. See docs/PLUGIN_PLATFORM_PLAN.md, "Contract
// reconciliation and generation record".
//
// Steps:
//   1. Run the `gen-contract` bin (Rust, `--features gen`) to (re)emit
//      schemas/plugin-manifest-v1.json, plugin-rpc-v1.json,
//      plugin-error-v1.json, and capability-registry-v1.json.
//   2. Convert the manifest `$defs` + RPC `$defs`/`x-methods` into the TS
//      type surface consumers import today, preserving names and the current
//      style (interfaces, `| null`, literal unions, `BrokerMethodPayloads`).
//
// Envelope types and SDK-only helpers (RpcRequest/RpcSuccess/RpcFailure,
// MutationOptions, Revisioned*, MigrationAuthoringOptions, LifecycleState) are
// not representable in the schemas and are kept as stable hand-authored
// snippets here.
//
// The drift guard (scripts/check-plugin-contract-drift.mjs) drives this with
// the `DAENA_SCHEMA_DIR` / `DAENA_GENERATED_TS` overrides: when a schema dir
// is supplied the Rust bin is skipped (the caller has already generated the
// schemas) and conversion runs against the supplied schemas, writing
// `generated.ts` to the supplied path.

import { spawnSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");

const suppliedSchemaDir = process.env.DAENA_SCHEMA_DIR;
const schemaDir = suppliedSchemaDir ?? "schemas";
const generatedTsPath = process.env.DAENA_GENERATED_TS ?? "packages/plugin-sdk/src/generated.ts";

const readJson = async (path) => JSON.parse(await readFile(resolve(root, schemaDir, path), "utf8"));

// ---------------------------------------------------------------------------
// Step 1 — run the Rust contract generator (skipped when schemas are supplied)
// ---------------------------------------------------------------------------

if (!suppliedSchemaDir) {
  const bin = spawnSync(
    "cargo",
    [
      "run",
      "--manifest-path",
      "crates/daena-plugin-api/Cargo.toml",
      "--features",
      "gen",
      "--bin",
      "gen-contract",
      "--locked",
      "--offline",
    ],
    { cwd: root, encoding: "utf8" },
  );
  if (bin.status !== 0) {
    process.stderr.write(bin.stderr ?? "");
    process.stderr.write(bin.stdout ?? "");
    process.exit(bin.status ?? 1);
  }
}

// ---------------------------------------------------------------------------
// Step 2 — schema -> TS conversion
// ---------------------------------------------------------------------------

// Refs that are string aliases; inlined so no extra exported names appear.
const INLINE_STRING_REFS = new Set(["identifier", "serviceName", "namespace", "semver", "packagePath"]);

const refName = (ref) => ref.replace(/^#\/\$defs\//, "");

const isIdentifier = (key) => /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key);
const quoteKey = (key) => (isIdentifier(key) ? key : JSON.stringify(key));

const literalToTs = (v) => (typeof v === "string" ? JSON.stringify(v) : String(v));

function arrayToTs(schema) {
  const inner = schema.items ? schemaToTs(schema.items) : "unknown";
  return inner.includes(" | ") ? `Array<${inner}>` : `${inner}[]`;
}

function objectToTs(schema) {
  const props = schema.properties ?? {};
  const required = new Set(schema.required ?? []);
  const members = [];
  for (const [key, sub] of Object.entries(props)) {
    members.push(`${quoteKey(key)}${required.has(key) ? "" : "?"}: ${schemaToTs(sub)}`);
  }
  if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
    const value = schemaToTs(schema.additionalProperties);
    if (Object.keys(props).length === 0) return `Record<string, ${value}>`;
    members.push(`[key: string]: ${value}`);
  }
  if (members.length === 0) return schema.type === "object" ? "Record<string, unknown>" : "unknown";
  return `{ ${members.join("; ")} }`;
}

const scalarToTs = (t) =>
  t === "string"
    ? "string"
    : t === "integer" || t === "number"
      ? "number"
      : t === "boolean"
        ? "boolean"
        : t === "null"
          ? "null"
          : "unknown";

function schemaToTs(schema) {
  if (schema === true || schema === undefined || schema === null || typeof schema !== "object") {
    return "unknown";
  }
  if (schema.$ref) {
    const name = refName(schema.$ref);
    return INLINE_STRING_REFS.has(name) ? "string" : name;
  }
  if (schema.const !== undefined) return literalToTs(schema.const);
  if (Array.isArray(schema.enum)) return schema.enum.map(literalToTs).join(" | ");
  if (Array.isArray(schema.oneOf)) return schema.oneOf.map(schemaToTs).join(" | ");
  if (Array.isArray(schema.anyOf)) return schema.anyOf.map(schemaToTs).join(" | ");

  const t = schema.type;
  if (Array.isArray(t)) {
    const parts = t.map((t2) =>
      t2 === "array" ? arrayToTs(schema) : t2 === "object" ? objectToTs(schema) : scalarToTs(t2),
    );
    return parts.join(" | ");
  }
  if (t === "array") return arrayToTs(schema);
  if (t === "object") return objectToTs(schema);
  if (t === "string") return "string";
  if (t === "integer" || t === "number") return "number";
  if (t === "boolean") return "boolean";
  if (t === "null") return "null";
  return "unknown";
}

function interfaceBody(schema) {
  return objectToTs(schema);
}

function defToDeclaration(name, schema) {
  const t = schema.type;
  if (schema.enum || schema.oneOf || schema.anyOf || (Array.isArray(t) && t.includes("null"))) {
    return `export type ${name} = ${schemaToTs(schema)};`;
  }
  if (t === "object" || t === undefined) {
    const body = interfaceBody(schema);
    return body.startsWith("{") ? `export interface ${name} ${body}` : `export type ${name} = ${body};`;
  }
  return `export type ${name} = ${schemaToTs(schema)};`;
}

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

const manifest = await readJson("plugin-manifest-v1.json");
const manifestDefs = manifest.$defs ?? {};

// FieldType is the current SDK name for the FieldDefinition.type enum.
const fieldTypeUnion = schemaToTs(manifestDefs.FieldDefinition?.properties?.type);

const MANIFEST_DEF_ORDER = [
  "Entrypoints",
  "Dependency",
  "OneOfVariant",
  "MetadataFieldDefinition",
  "TimelineFieldRole",
  "TimelineFieldLayer",
  "TimelineFieldContribution",
  "IconRef",
  "EntityTypeDefinition",
  "FieldDefinition",
  "SchemaContribution",
  "EntityTemplate",
  "MigrationOperation",
  "Migration",
  "ViewComponent",
  "ViewRenderer",
  "View",
  "CommandAction",
  "CommandExposure",
  "CommandValueType",
  "CommandProperty",
  "CommandSchema",
  "Command",
  "PluginStability",
  "Service",
  "Event",
  "Services",
  "Events",
];

// Names that are not emitted as standalone declarations.
const SKIP_DEFS = new Set([
  "PluginManifest", // emitted from the curated root object below
  ...INLINE_STRING_REFS,
]);

const manifestLines = [];
manifestLines.push(`export type PluginKind = ${schemaToTs(manifestDefs.PluginKind)};`);
manifestLines.push(`export type FieldType = ${fieldTypeUnion};`);
const catalogIconVariant = manifestDefs.IconRef.oneOf.find(
  (variant) => variant.properties?.kind?.enum?.[0] === "catalog",
);
const catalogIconIds = catalogIconVariant?.properties?.id?.enum;
if (!Array.isArray(catalogIconIds) || catalogIconIds.length === 0)
  throw new Error("manifest schema IconRef catalog variant must declare icon IDs");
manifestLines.push(`export const CATALOG_ICON_IDS = ${JSON.stringify(catalogIconIds)} as const;`);

for (const name of MANIFEST_DEF_ORDER) {
  if (SKIP_DEFS.has(name)) continue;
  const schema = manifestDefs[name];
  if (!schema) throw new Error(`manifest schema missing $defs.${name}`);
  if (name === "FieldDefinition") {
    // FieldDefinition.type carries the named `FieldType` union.
    const schema = structuredClone(manifestDefs.FieldDefinition);
    schema.properties.type = { $ref: "#/$defs/FieldType" };
    manifestLines.push(`export interface FieldDefinition ${interfaceBody(schema)}`);
  } else {
    manifestLines.push(defToDeclaration(name, schema));
  }
}

// PluginManifest comes from the curated root object, not the plain derive.
manifestLines.push(`export interface PluginManifest ${interfaceBody(manifest)}`);

// ---------------------------------------------------------------------------
// RPC types
// ---------------------------------------------------------------------------

const rpc = await readJson("plugin-rpc-v1.json");
const rpcDefs = rpc.$defs ?? {};
const xMethods = rpc["x-methods"] ?? {};

const rpcLines = [];

// Envelope shapes are stable and hand-assembled from the `request`/`response`
// `$defs` (their `allOf`/`if`/`then` structure is not representable as a TS
// object type). `method` stays `string` because the SDK builds requests from
// a `string` method variable.
const reqProps = rpcDefs.request?.properties ?? {};
const rpcVersionTs = schemaToTs(reqProps.rpcVersion);
const idTs = schemaToTs(reqProps.requestId);
const sessionTs = schemaToTs(reqProps.sessionId);
rpcLines.push(
  `export interface RpcRequest { rpcVersion: ${rpcVersionTs}; sessionId: ${sessionTs}; requestId: ${idTs}; method: string; payload: unknown }`,
);
rpcLines.push(defToDeclaration("RpcError", rpcDefs.error));
rpcLines.push(`export type PluginRpcError = RpcError;`);
rpcLines.push(
  `export interface RpcSuccess { rpcVersion: ${rpcVersionTs}; requestId: ${idTs}; ok: true; result: unknown }`,
);
rpcLines.push(
  `export interface RpcFailure { rpcVersion: ${rpcVersionTs}; requestId: ${idTs}; ok: false; error: RpcError }`,
);
rpcLines.push(`export type RpcResponse = RpcSuccess | RpcFailure;`);

rpcLines.push(defToDeclaration("PluginBootstrap", rpcDefs.PluginBootstrap));
rpcLines.push(defToDeclaration("EntityRecord", rpcDefs.EntityRecord));
rpcLines.push(defToDeclaration("EntityTypeCountRecord", rpcDefs.EntityTypeCountRecord));
rpcLines.push(defToDeclaration("EntityPageRecord", rpcDefs.EntityPageRecord));

// SDK-only helpers not derivable from the schemas.
rpcLines.push(`export interface MutationOptions { expectedRevision?: string; requestId?: string }`);
rpcLines.push(`export interface RevisionedEntityPayload { id: string; expectedRevision: string }`);
rpcLines.push(
  `export interface RevisionedDocumentPayload { entityId: string; body: string; format?: string; expectedRevision: string }`,
);
rpcLines.push(
  `export interface RevisionedFieldPayload { entityId: string; namespace: string; key: string; value: unknown; expectedRevision: string }`,
);
rpcLines.push(`export interface RevisionedRelationshipPayload { id: string; expectedRevision: string }`);
rpcLines.push(
  `export interface RevisionedAssetPayload { entityId: string; namespace: string; filename: string; contentHash: string; size: number; mimeType: string; path: string; expectedRevision: string }`,
);

for (const name of ["EntityCreateDocument", "EntityCreateField", "EntityCreateRelationship"]) {
  rpcLines.push(defToDeclaration(name, rpcDefs[name]));
}
rpcLines.push(defToDeclaration("AiRetrievalMode", rpcDefs.AiRetrievalMode));
rpcLines.push(defToDeclaration("AiRetrievalPolicyPayload", rpcDefs.AiRetrievalPolicyPayload));

// Per-method payload interfaces, in x-methods order (deduplicated — some
// methods share a payload type).
const payloadDefNames = [...new Set(Object.values(xMethods).map((c) => c.payload))];
for (const name of payloadDefNames) {
  if (!rpcDefs[name]) throw new Error(`rpc schema missing $defs.${name}`);
  rpcLines.push(defToDeclaration(name, rpcDefs[name]));
}

// BrokerMethodPayloads references the named payload interfaces.
const payloadEntries = Object.entries(xMethods)
  .map(([method, contract]) => `  ${JSON.stringify(method)}: ${contract.payload};`)
  .join("\n");
rpcLines.push(`export interface BrokerMethodPayloads {\n${payloadEntries}\n}`);
rpcLines.push(`export type BrokerMethod = keyof BrokerMethodPayloads;`);

rpcLines.push(
  `export interface MigrationAuthoringOptions {\n  recovery?: Migration["recovery"];\n  description?: string;\n}`,
);
rpcLines.push(
  `export type LifecycleState = "discovered" | "validated" | "installed" | "resolved" | "activating" | "active" | "deactivating" | "failed" | "quarantined" | "incompatible" | "uninstalling" | "removed";`,
);

// ---------------------------------------------------------------------------
// Write
// ---------------------------------------------------------------------------

const output = [
  "/* eslint-disable */",
  "/**",
  " * GENERATED CONTRACT TYPES.",
  " * Source: crates/daena-plugin-api (Rust types -> schemas/*.json via gen-contract).",
  " * Generated by scripts/gen-plugin-contract.mjs; run `npm run gen:plugin-contract`.",
  " * Do not edit this file by hand.",
  " */",
  "",
  ...manifestLines,
  "",
  ...rpcLines,
  "",
].join("\n");

await writeFile(resolve(root, generatedTsPath), output);
console.log(`wrote ${generatedTsPath}`);
