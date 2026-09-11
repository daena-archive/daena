#!/usr/bin/env node

import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const SCHEMA_VERSION = 1;
const MAX_TRUST_BYTES = 256 * 1024;
const GITHUB_REPO = /^https:\/\/github\.com\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+\/?$/;

function fail(message) {
  throw new Error(message);
}

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    fail(`${path}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function assertPlainObject(value, path) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) fail(`${path} must be an object`);
}

function rejectUnknownKeys(value, allowed, path) {
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) fail(`${path} has unknown field ${key}`);
  }
}

function decodePublicKey(value, path) {
  const bytes = Buffer.from(value, "base64");
  if (!value || bytes.length !== 32 || bytes.toString("base64") !== value) {
    fail(`${path} must be a 32-byte Ed25519 public key`);
  }
  return value;
}

function pluginAllowed(publisher, pluginId) {
  return pluginId === publisher || pluginId.startsWith(`${publisher}.`);
}

function validatePublisher(directoryName, record, path) {
  assertPlainObject(record, path);
  rejectUnknownKeys(record, new Set(["schemaVersion", "id", "keys", "sourceRepo", "plugins"]), path);
  if (record.schemaVersion !== SCHEMA_VERSION) fail(`${path}: schemaVersion must be ${SCHEMA_VERSION}`);
  if (record.id !== directoryName) fail(`${path}: id must match directory name ${directoryName}`);
  if (!record.id) fail(`${path}: id is required`);
  if (!Array.isArray(record.keys) || record.keys.length === 0) fail(`${path}: keys must be a non-empty array`);
  for (const [index, key] of record.keys.entries()) {
    const keyPath = `${path}.keys[${index}]`;
    assertPlainObject(key, keyPath);
    rejectUnknownKeys(key, new Set(["keyId", "publicKey"]), keyPath);
    if (!key.keyId) fail(`${keyPath}: keyId is required`);
    decodePublicKey(key.publicKey, `${keyPath}.publicKey`);
  }
  if (typeof record.sourceRepo !== "string" || !GITHUB_REPO.test(record.sourceRepo.replace(/\.git$/, ""))) {
    fail(`${path}: sourceRepo must be an https GitHub repository URL`);
  }
  if (!Array.isArray(record.plugins)) fail(`${path}: plugins must be an array`);
  for (const pluginId of record.plugins) {
    if (typeof pluginId !== "string" || !pluginId) fail(`${path}: plugin ids must be non-empty strings`);
    if (!pluginAllowed(record.id, pluginId))
      fail(`${path}: plugin id ${pluginId} must be ${record.id} or ${record.id}.*`);
  }
}

function validateRevocations(record, path) {
  assertPlainObject(record, path);
  rejectUnknownKeys(record, new Set(["schemaVersion", "keys", "digests", "packages"]), path);
  if (record.schemaVersion !== SCHEMA_VERSION) fail(`${path}: schemaVersion must be ${SCHEMA_VERSION}`);
  const keys = record.keys ?? [];
  const digests = record.digests ?? [];
  const packages = record.packages ?? [];
  if (!Array.isArray(keys) || !Array.isArray(digests) || !Array.isArray(packages)) {
    fail(`${path}: keys, digests, and packages must be arrays`);
  }
  for (const [index, entry] of keys.entries()) {
    const keyPath = `${path}.keys[${index}]`;
    assertPlainObject(entry, keyPath);
    rejectUnknownKeys(entry, new Set(["publisher", "keyId", "publicKey"]), keyPath);
    if (!entry.publisher) fail(`${keyPath}: publisher is required`);
    if (Object.hasOwn(entry, "publicKey") && (!entry.publicKey || typeof entry.publicKey !== "string")) {
      fail(`${keyPath}: publicKey must be a 32-byte Ed25519 public key`);
    }
    if (entry.publicKey) decodePublicKey(entry.publicKey, `${keyPath}.publicKey`);
    if (!entry.publicKey && Object.hasOwn(entry, "keyId")) {
      fail(`${keyPath}: key revocation must include publicKey`);
    }
  }
  for (const digest of digests) {
    if (typeof digest !== "string" || !/^[0-9a-fA-F]{64}$/.test(digest)) {
      fail(`${path}: digest revocation must be a SHA-256 hex digest`);
    }
  }
  for (const [index, entry] of packages.entries()) {
    const packagePath = `${path}.packages[${index}]`;
    assertPlainObject(entry, packagePath);
    rejectUnknownKeys(entry, new Set(["pluginId", "version"]), packagePath);
    if (!entry.pluginId) fail(`${packagePath}: pluginId is required`);
  }
}

export function compileIndex(root) {
  const publishersRoot = join(root, "publishers");
  const revocationsPath = join(root, "revocations.json");
  if (!existsSync(publishersRoot) || !statSync(publishersRoot).isDirectory()) {
    fail("publishers/ directory is required");
  }
  const publishers = {};
  const seen = new Set();
  for (const name of readdirSync(publishersRoot)) {
    if (name.startsWith(".")) continue;
    const directory = join(publishersRoot, name);
    if (!statSync(directory).isDirectory()) continue;
    const recordPath = join(directory, "publisher.json");
    if (!existsSync(recordPath)) fail(`missing ${recordPath}`);
    if (seen.has(name)) fail(`colliding publisher directory: ${name}`);
    seen.add(name);
    const record = readJson(recordPath);
    validatePublisher(name, record, recordPath);
    publishers[record.id] = {
      keys: record.keys.map((key) => ({ keyId: key.keyId, publicKey: key.publicKey })),
    };
  }
  const revocations = existsSync(revocationsPath)
    ? readJson(revocationsPath)
    : { schemaVersion: SCHEMA_VERSION, keys: [], digests: [], packages: [] };
  validateRevocations(revocations, revocationsPath);
  const trust = {
    schemaVersion: SCHEMA_VERSION,
    publishers,
    revocations: {
      keys: revocations.keys ?? [],
      digests: revocations.digests ?? [],
      packages: revocations.packages ?? [],
    },
  };
  const catalog = { schemaVersion: SCHEMA_VERSION, plugins: [] };
  const trustBytes = Buffer.from(`${JSON.stringify(trust, null, 2)}\n`);
  if (trustBytes.length > MAX_TRUST_BYTES) fail("compiled trust.json exceeds 256 KiB");
  return { trust, catalog, trustBytes };
}

function main() {
  const args = process.argv.slice(2);
  const checkOnly = args.includes("--check");
  const root = resolve(args.find((arg) => !arg.startsWith("--")) ?? join(import.meta.dirname, ".."));
  const compiled = compileIndex(root);
  if (checkOnly) {
    console.log(JSON.stringify({ ok: true, publishers: Object.keys(compiled.trust.publishers).length }, null, 2));
    return;
  }
  const dist = join(root, "dist");
  mkdirSync(dist, { recursive: true });
  writeFileSync(join(dist, "trust.json"), compiled.trustBytes);
  writeFileSync(join(dist, "catalog.json"), `${JSON.stringify(compiled.catalog, null, 2)}\n`);
  console.log(
    JSON.stringify({ ok: true, trust: join(dist, "trust.json"), catalog: join(dist, "catalog.json") }, null, 2),
  );
}

const isMain = resolve(process.argv[1] ?? "") === resolve(join(import.meta.dirname, "compile.mjs"));
if (isMain) {
  try {
    main();
  } catch (error) {
    console.error(`plugin-registry error: ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  }
}
