#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createZipArchive, readZipArchive } from "../packages/plugin-cli/bin/zip.mjs";

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
  const archive = join(temporary, "fixture.daenaplugin");
  execFileSync("node", [cli, "package", fixture, "--output", archive], { stdio: "pipe" });
  const validation = execFileSync("node", [cli, "validate", archive], { encoding: "utf8" });
  assert.equal(JSON.parse(validation).ok, true);
  const legacyArchive = join(temporary, "legacy.wbplugin");
  writeFileSync(legacyArchive, readFileSync(archive));
  assert.throws(
    () => execFileSync("node", [cli, "validate", legacyArchive], { stdio: "pipe" }),
    /\.wbplugin is not accepted/,
  );
  const migrations = execFileSync("node", [cli, "migration", "validate", fixture], { encoding: "utf8" });
  assert.equal(JSON.parse(migrations).dataVersion, 1);
  for (const example of ["declarative", "ui", "wasm-service", "theme", "timeline-consumer"]) {
    execFileSync("node", [cli, "validate", join(workspace, "examples/plugins", example)], { stdio: "pipe" });
  }
  const unsafeArchive = join(temporary, "unsafe.daenaplugin");
  writeFileSync(
    unsafeArchive,
    createZipArchive([
      { name: "manifest.json", data: "{}" },
      { name: "MANIFEST.json", data: "{}" },
    ]),
  );
  assert.throws(
    () => execFileSync("node", [cli, "validate", unsafeArchive], { stdio: "pipe" }),
    /duplicate|case-colliding/i,
  );
  const shapeOnly = join(temporary, "shape-only");
  cpSync(join(workspace, "examples/plugins/declarative"), shapeOnly, { recursive: true });
  const shapeManifest = JSON.parse(readFileSync(join(shapeOnly, "manifest.json"), "utf8"));
  shapeManifest.schemas[0].fields[0].options = "not-an-array";
  shapeManifest.templates[0].fields = {};
  writeFileSync(join(shapeOnly, "manifest.json"), `${JSON.stringify(shapeManifest, null, 2)}\n`);
  assert.throws(
    () => execFileSync("node", [cli, "validate", shapeOnly], { stdio: "pipe" }),
    /schema:\/schemas\/0\/fields\/0\/options/,
  );
  const ecosystemNames = join(temporary, "ecosystem-names");
  cpSync(join(workspace, "examples/plugins/declarative"), ecosystemNames, { recursive: true });
  const ecosystemManifest = JSON.parse(readFileSync(join(ecosystemNames, "manifest.json"), "utf8"));
  ecosystemManifest.id = "com.example.my_plugin";
  ecosystemManifest.services = { provides: [], consumes: [{ name: "daena.maps/navigation", major: 1 }] };
  writeFileSync(join(ecosystemNames, "manifest.json"), `${JSON.stringify(ecosystemManifest, null, 2)}\n`);
  execFileSync("node", [cli, "validate", ecosystemNames], { stdio: "pipe" });
  const keyPath = join(temporary, "signing.json");
  const generated = JSON.parse(
    execFileSync("node", [cli, "keygen", "--output", keyPath, "--key-id", "2026-09"], { encoding: "utf8" }),
  );
  assert.equal(generated.ok, true);
  assert.equal(generated.keyId, "2026-09");
  const signedArchive = join(temporary, "signed.daenaplugin");
  execFileSync("node", [cli, "package", fixture, "--output", signedArchive], { stdio: "pipe" });
  const signed = JSON.parse(execFileSync("node", [cli, "sign", signedArchive, "--key", keyPath], { encoding: "utf8" }));
  assert.equal(signed.ok, true);
  assert.equal(signed.digest.length, 64);
  const entries = readZipArchive(readFileSync(signedArchive));
  const signature = JSON.parse(entries.find((entry) => entry.name === "signature.json").data.toString("utf8"));
  assert.equal(signature.algorithm, "ed25519");
  assert.equal(signature.publicKey, generated.publicKey);
  assert.equal(signature.keyId, "2026-09");
  console.log("plugin CLI checks passed");
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
