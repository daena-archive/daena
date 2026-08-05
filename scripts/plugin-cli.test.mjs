#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createZipArchive } from "../packages/plugin-cli/bin/zip.mjs";

const workspace = resolve(import.meta.dirname, "..");
const cli = join(workspace, "scripts/plugin-cli.mjs");
const temporary = mkdtempSync(join(tmpdir(), "daena-plugin-cli-"));
try {
  const fixture = join(temporary, "fixture");
  cpSync(join(workspace, "examples/plugins/declarative"), fixture, { recursive: true });
  const init = join(temporary, "initialized");
  execFileSync("node", [cli, "init", init, "--id", "com.example.cli-fixture"], { encoding: "utf8" });
  const initialized = JSON.parse(readFileSync(join(init, "manifest.json"), "utf8"));
  assert.equal(initialized.id, "com.example.cli-fixture");
  execFileSync("node", [cli, "validate", fixture], { stdio: "pipe" });
  const archive = join(temporary, "fixture.wbplugin");
  execFileSync("node", [cli, "package", fixture, "--output", archive], { stdio: "pipe" });
  const validation = execFileSync("node", [cli, "validate", archive], { encoding: "utf8" });
  assert.equal(JSON.parse(validation).ok, true);
  const migrations = execFileSync("node", [cli, "migration", "validate", fixture], { encoding: "utf8" });
  assert.equal(JSON.parse(migrations).dataVersion, 1);
  for (const example of ["declarative", "ui", "wasm-service"]) {
    execFileSync("node", [cli, "validate", join(workspace, "examples/plugins", example)], { stdio: "pipe" });
  }
  const unsafeArchive = join(temporary, "unsafe.wbplugin");
  writeFileSync(unsafeArchive, createZipArchive([
    { name: "manifest.json", data: "{}" },
    { name: "MANIFEST.json", data: "{}" },
  ]));
  assert.throws(() => execFileSync("node", [cli, "validate", unsafeArchive], { stdio: "pipe" }), /duplicate|case-colliding/i);
  console.log("plugin CLI checks passed");
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
