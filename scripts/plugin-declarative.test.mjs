#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { cpSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { FakePluginHost } from "../packages/plugin-test-host/dist/index.js";
import { readZipArchive } from "../packages/plugin-cli/bin/zip.mjs";

const workspace = resolve(import.meta.dirname, "..");
const temporary = mkdtempSync(join(tmpdir(), "daena-plugin-declarative-"));
try {
  const fixture = join(temporary, "field-notes");
  cpSync(join(workspace, "examples/plugins/declarative"), fixture, { recursive: true });
  const archive = join(temporary, "field-notes.wbplugin");
  execFileSync("node", [join(workspace, "scripts/plugin-cli.mjs"), "package", fixture, "--output", archive], { stdio: "pipe" });
  const entries = readZipArchive(readFileSync(archive));
  const names = new Set(entries.map((entry) => entry.name));
  assert.equal(names.has("manifest.json"), true);
  assert.equal(names.has("dist/ui/index.html"), true);

  const manifest = JSON.parse(readFileSync(join(fixture, "manifest.json"), "utf8"));
  const host = new FakePluginHost({ manifest, grants: ["entity.read", "entity.write", "field.read:self", "field.write:self"] });
  host.activateDeclarative();
  const view = host.hostView("notes");
  assert.equal(view.title, "Field Notes");
  assert.equal(view.components?.some((component) => component.type === "field-form"), true);
  assert.deepEqual(host.invokeHostCommand("notes", "refresh"), { type: "refresh-view" });
  host.deactivateDeclarative();
  assert.throws(() => host.hostView("notes"), /not active/);
  console.log("packaged declarative plugin checks passed");
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
