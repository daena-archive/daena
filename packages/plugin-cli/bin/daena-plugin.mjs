#!/usr/bin/env node

import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import * as sdk from "@daena-archive/plugin-sdk";
import { createZipArchive, readZipArchive } from "./zip.mjs";

const schemaPath = resolve(import.meta.dirname, "../../../schemas/plugin-manifest-v1.json");
if (!existsSync(schemaPath)) throw new Error(`generated manifest schema not found: ${schemaPath}`);
const ajv = new Ajv2020({ allErrors: true });
ajv.addFormat("uint32", (value) => Number.isInteger(value) && value >= 0 && value <= 0xffffffff);
const validateShape = ajv.compile(JSON.parse(readFileSync(schemaPath, "utf8")));

function shapeErrors(manifest) {
  if (validateShape(manifest)) return [];
  return (validateShape.errors ?? []).map((error) => `schema:${error.instancePath || "/"} ${error.message}`);
}

function usage() {
  console.error(`Usage:
daena-plugin validate <directory|archive>
daena-plugin package <directory> [--output file]
daena-plugin migration validate <directory>
daena-plugin init <directory> --id <plugin-id> [--name name]`);
}

function readManifest(directory) {
  const path = join(directory, "manifest.json");
  if (!existsSync(path)) throw new Error(`manifest.json is missing from ${directory}`);
  let manifest;
  try { manifest = JSON.parse(readFileSync(path, "utf8")); } catch (error) { throw new Error(`manifest.json is invalid JSON: ${error.message}`); }
  return { manifest, path };
}

function inspectPackageTree(directory) {
  const seen = new Set();
  const filesForArchive = [];
  let files = 0;
  let bytes = 0;
  function visit(current) {
    for (const name of readdirSync(current)) {
      const target = join(current, name);
      const relative = target.slice(directory.length + 1).replaceAll("\\", "/");
      const metadata = lstatSync(target);
      if (metadata.isSymbolicLink()) throw new Error(`links are not allowed in plugin packages: ${relative}`);
      const folded = relative.toLocaleLowerCase("en-US");
      if (seen.has(folded)) throw new Error(`case-colliding package path: ${relative}`);
      seen.add(folded);
      if (metadata.isDirectory()) visit(target);
      else if (metadata.isFile()) { files += 1; bytes += metadata.size; filesForArchive.push({ name: relative, path: target }); }
      else throw new Error(`unsupported package entry: ${relative}`);
    }
  }
  visit(directory);
  if (files > 4096) throw new Error("package contains too many files");
  if (bytes > 512 * 1024 * 1024) throw new Error("package is too large");
  return filesForArchive;
}

function validateDirectory(input) {
  const directory = resolve(input);
  if (!existsSync(directory) || !statSync(directory).isDirectory()) throw new Error(`package directory does not exist: ${directory}`);
  inspectPackageTree(directory);
  const { manifest } = readManifest(directory);
  const errors = [...shapeErrors(manifest), ...sdk.validatePluginManifest(manifest)];
  for (const entrypoint of [manifest.entrypoints?.ui, manifest.entrypoints?.wasm].filter(Boolean)) {
    const target = resolve(directory, entrypoint);
    if (!target.startsWith(`${directory}/`) || !existsSync(target) || !statSync(target).isFile()) errors.push(`missing entrypoint: ${entrypoint}`);
  }
  if (errors.length) throw new Error(errors.join("; "));
  return { directory, manifest };
}

function listArchive(archive) {
  const entries = readZipArchive(readFileSync(archive));
  const seen = new Set();
  const names = entries.map((entry) => entry.name);
  for (const name of names) {
    const path = name.endsWith("/") ? name.slice(0, -1) : name;
    const folded = path.toLocaleLowerCase("en-US");
    if (seen.has(folded)) throw new Error(`duplicate or case-colliding archive path: ${name}`);
    seen.add(folded);
    if (path.startsWith("/") || path.includes("\\") || path.split("/").some((part) => part === ".." || part === "" || part === ".")) throw new Error(`unsafe archive path: ${name}`);
  }
  return entries;
}

function validateArchive(input) {
  const archive = resolve(input);
  if (!existsSync(archive)) throw new Error(`archive does not exist: ${archive}`);
  const entries = listArchive(archive);
  const names = entries.map((entry) => entry.name);
  if (!names.includes("manifest.json")) throw new Error("archive must contain manifest.json at its root");
  let manifest;
  try { manifest = JSON.parse(entries.find((entry) => entry.name === "manifest.json").data.toString("utf8")); } catch (error) { throw new Error(`archive manifest is invalid: ${error.message}`); }
  const errors = [...shapeErrors(manifest), ...sdk.validatePluginManifest(manifest)];
  for (const entrypoint of [manifest.entrypoints?.ui, manifest.entrypoints?.wasm].filter(Boolean)) if (!names.includes(entrypoint)) errors.push(`missing entrypoint: ${entrypoint}`);
  if (errors.length) throw new Error(errors.join("; "));
  return { archive, manifest, files: names };
}

function parseFlag(args, flag, fallback) {
  const index = args.indexOf(flag);
  return index === -1 ? fallback : args[index + 1];
}

function packageDirectory(input, output) {
  const { directory, manifest } = validateDirectory(input);
  const target = resolve(output ?? `${manifest.id.replaceAll(".", "-")}-${manifest.version}.wbplugin`);
  if (target.startsWith(`${directory}/`)) throw new Error("output archive must not be inside the package directory");
  mkdirSync(dirname(target), { recursive: true });
  const temporary = `${target}.tmp-${process.pid}`;
  try {
    const files = inspectPackageTree(directory).map(({ name, path }) => ({ name, data: readFileSync(path) }));
    writeFileSync(temporary, createZipArchive(files));
    renameSync(temporary, target);
  } finally {
    if (existsSync(temporary)) rmSync(temporary);
  }
  return { target, manifest };
}

function initDirectory(directory, args) {
  const id = parseFlag(args, "--id");
  if (!id) throw new Error("init requires --id");
  if (!sdk.isPluginIdentifier(id)) throw new Error("--id must be a lowercase reverse-domain identifier");
  const name = parseFlag(args, "--name", id.split(".").at(-1));
  const target = resolve(directory);
  if (existsSync(target) && readdirSync(target).length) throw new Error("init target is not empty");
  mkdirSync(join(target, "dist", "ui"), { recursive: true });
  const manifest = {
    manifestVersion: 1, id, name, version: "0.1.0", publisher: id.split(".").slice(0, -1).join(".") || id,
    hostApi: ">=1.0.0 <2.0.0", kind: "sandboxed", entrypoints: { ui: "dist/ui/index.html" }, capabilities: ["entity.read"],
    dependencies: {}, namespaces: [], schemas: [], templates: [], views: [{ id: "main", title: name }], commands: [],
    services: { provides: [], consumes: [] }, events: { publishes: [], subscribes: [] }, migrations: [],
  };
  sdk.assertValidPluginManifest(manifest);
  writeFileSync(join(target, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  writeFileSync(join(target, "dist", "ui", "index.html"), `<!doctype html><html><body><main>${name}</main><script type="module" src="./index.js"></script></body></html>\n`);
  writeFileSync(join(target, "dist", "ui", "index.js"), `// Connect this bundle to the host-provided broker transport.\n`);
  return target;
}

try {
  const args = process.argv.slice(2);
  const command = args.shift();
  if (!command) { usage(); process.exitCode = 1; }
  else if (command === "validate") {
    const input = args[0];
    if (!input) throw new Error("validate requires a directory or .wbplugin archive");
    const result = input.endsWith(".wbplugin") ? validateArchive(input) : validateDirectory(input);
    console.log(JSON.stringify({ ok: true, id: result.manifest.id, version: result.manifest.version, files: result.files?.length }, null, 2));
  } else if (command === "package") {
    const result = packageDirectory(args[0], parseFlag(args, "--output"));
    console.log(JSON.stringify({ ok: true, archive: result.target, id: result.manifest.id, version: result.manifest.version }, null, 2));
  } else if (command === "migration" && args[0] === "validate") {
    const { manifest } = validateDirectory(args[1]);
    const errors = sdk.validateMigrationChain(manifest.migrations, manifest.namespaces);
    if (errors.length) throw new Error(errors.join("; "));
    console.log(JSON.stringify({ ok: true, migrations: manifest.migrations.length, dataVersion: manifest.migrations.at(-1)?.to ?? 0 }, null, 2));
  } else if (command === "init") {
    console.log(JSON.stringify({ ok: true, directory: initDirectory(args.shift(), args) }, null, 2));
  } else { usage(); process.exitCode = 1; }
} catch (error) {
  console.error(`plugin error: ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
}
