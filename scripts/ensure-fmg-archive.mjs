import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const archivePath = resolve(process.argv[2] ?? join(scriptRoot, "src-tauri/plugin-assets/maps/fmg-v1.119.zip"));
const sourceRoot = process.argv[3] ?? process.env.DAENA_FMG_SOURCE;
const metadataPath = join(scriptRoot, "docs/maps/fmg-v1.119-vendor.json");
const expectedHash = JSON.parse(readFileSync(metadataPath, "utf8")).archive.sha256;

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function fail(message) {
  throw new Error(`FMG archive check failed: ${message}`);
}

if (existsSync(archivePath) && sha256(archivePath) === expectedHash) {
  console.log(JSON.stringify({ archive: archivePath, sha256: expectedHash, generated: false }));
  process.exit(0);
}

if (!sourceRoot) {
  const actual = existsSync(archivePath) ? sha256(archivePath) : "missing";
  fail(`${actual} does not match ${expectedHash}; set DAENA_FMG_SOURCE to the pinned FMG checkout to regenerate it`);
}

const sourcePath = resolve(sourceRoot);
if (!existsSync(sourcePath)) fail(`FMG source directory does not exist: ${sourcePath}`);
const patchScript = join(scriptRoot, "scripts/patch-fmg-for-daena.mjs");
execFileSync(process.execPath, [patchScript, sourcePath, archivePath], { cwd: scriptRoot, stdio: "inherit" });

if (!existsSync(archivePath)) fail(`generation did not create ${archivePath}`);
const actual = sha256(archivePath);
if (actual !== expectedHash) fail(`generated archive hash is ${actual}, expected ${expectedHash}`);
console.log(JSON.stringify({ archive: archivePath, sha256: actual, generated: true, source: sourcePath }));
