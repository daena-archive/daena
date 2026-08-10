#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { validatePluginManifest } from "../packages/plugin-sdk/dist/index.js";

const root = resolve(import.meta.dirname, "..");
const fixtureDir = resolve(root, "schemas/fixtures/manifest");

const index = JSON.parse(await readFile(resolve(fixtureDir, "index.json"), "utf8"));
const cases = index.fixtures;
const readJson = async (file) => JSON.parse(await readFile(resolve(fixtureDir, file), "utf8"));

const tsOutcomes = [];
for (const { file } of cases) {
  const manifest = await readJson(file);
  const errors = validatePluginManifest(manifest);
  tsOutcomes.push({ file, outcome: errors.length ? "rejected" : "accepted" });
}

const rustStdout = execFileSync(
  "cargo",
  [
    "run",
    "--quiet",
    "--manifest-path",
    "crates/daena-plugin-api/Cargo.toml",
    "--locked",
    "--offline",
    "--bin",
    "validate-fixtures",
  ],
  { cwd: root, encoding: "utf8" },
);
const rustOutcomes = JSON.parse(rustStdout.trim());

assert.equal(rustOutcomes.length, cases.length, "Rust validator must exercise every indexed fixture");

for (let i = 0; i < cases.length; i += 1) {
  const expected = cases[i].expected;
  const ts = tsOutcomes[i];
  const rust = rustOutcomes[i];
  assert.equal(ts.file, cases[i].file, "TS outcome order must match the index");
  assert.equal(rust.file, cases[i].file, "Rust outcome order must match the index");
  assert.equal(
    rust.outcome,
    ts.outcome,
    `dual validator disagreement on ${cases[i].rule} (${cases[i].file}): Rust=${rust.outcome} TS=${ts.outcome}`,
  );
  assert.equal(
    ts.outcome,
    expected,
    `${cases[i].rule} (${cases[i].file}): TS disagrees with the fixture index (expected ${expected})`,
  );
}

console.log(`dual-validator conformance passed (Rust == TS on ${cases.length} manifest fixtures)`);
