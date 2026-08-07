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
  const shapeOnly = join(temporary, "shape-only");
  cpSync(join(workspace, "examples/plugins/declarative"), shapeOnly, { recursive: true });
  const shapeManifest = JSON.parse(readFileSync(join(shapeOnly, "manifest.json"), "utf8"));
  shapeManifest.schemas[0].fields[0].options = "not-an-array";
  shapeManifest.templates[0].fields = {};
  writeFileSync(join(shapeOnly, "manifest.json"), `${JSON.stringify(shapeManifest, null, 2)}\n`);
  assert.throws(() => execFileSync("node", [cli, "validate", shapeOnly], { stdio: "pipe" }), /schema:\/schemas\/0\/fields\/0\/options/);
  const ecosystemNames = join(temporary, "ecosystem-names");
  cpSync(join(workspace, "examples/plugins/declarative"), ecosystemNames, { recursive: true });
  const ecosystemManifest = JSON.parse(readFileSync(join(ecosystemNames, "manifest.json"), "utf8"));
  ecosystemManifest.id = "com.example.my_plugin";
  ecosystemManifest.services = { provides: [], consumes: [{ name: "daena.maps/navigation", major: 1 }] };
  writeFileSync(join(ecosystemNames, "manifest.json"), `${JSON.stringify(ecosystemManifest, null, 2)}\n`);
  execFileSync("node", [cli, "validate", ecosystemNames], { stdio: "pipe" });
  console.log("plugin CLI checks passed");
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
