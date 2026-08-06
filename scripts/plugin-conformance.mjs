#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { FakePluginHost, FakePluginLifecycleHost, runConformance } from "../packages/plugin-test-host/dist/index.js";

const fixturePaths = [
  "examples/plugins/declarative/manifest.json",
  "examples/plugins/ui/manifest.json",
  "examples/plugins/wasm-service/manifest.json",
  "packages/modules/lore/manifest.json",
  "packages/modules/timeline/manifest.json",
  "packages/modules/writing/manifest.json",
];
let brokerChecks = 0;
let manifest;
for (const fixturePath of fixturePaths) {
  manifest = JSON.parse(await readFile(resolve(fixturePath), "utf8"));
  const host = new FakePluginHost({ manifest, grants: ["entity.read"] });
  const results = await runConformance(host);
  for (const result of results) assert.equal(result.passed, true, `${fixturePath}: ${result.name}: ${result.detail ?? "failed"}`);
  brokerChecks += results.length;
}

const lifecycle = new FakePluginLifecycleHost();
lifecycle.install(manifest);
lifecycle.enable(manifest.id);
const upgraded = structuredClone(manifest);
upgraded.version = "1.1.0";
lifecycle.install(upgraded);
lifecycle.upgrade(manifest.id, upgraded.version);
assert.equal(lifecycle.snapshot(manifest.id).selectedVersion, "1.1.0");
lifecycle.rollback(manifest.id, manifest.version);
lifecycle.disable(manifest.id);
lifecycle.uninstallCode(manifest.id, upgraded.version);
assert.deepEqual(lifecycle.snapshot(manifest.id), {
  pluginId: manifest.id,
  enabled: false,
  selectedVersion: manifest.version,
  installedVersions: [manifest.version],
  dataPresent: true,
});
lifecycle.deleteData(manifest.id);
assert.equal(lifecycle.snapshot(manifest.id).dataPresent, false);

const revisionHost = new FakePluginHost({
  manifest,
  grants: ["entity.read", "entity.write", "entity.delete"],
});
const revisionClient = revisionHost.client();
const created = await revisionClient.createEntity("Revisioned note", "note", { requestId: "revisioned-create" });
const replayed = await revisionClient.createEntity("This must not duplicate", "note", { requestId: "revisioned-create" });
assert.equal(replayed.id, created.id);
const updated = await revisionClient.updateEntity(created.id, "Updated note", "note", {
  expectedRevision: created.revision,
});
assert.notEqual(updated.revision, created.revision);
await assert.rejects(
  revisionClient.updateEntity(created.id, "Stale note", "note", {
    expectedRevision: created.revision,
  }),
  (error) => error?.code === "revision-conflict",
);
await revisionClient.deleteEntity(created.id, { expectedRevision: updated.revision });
console.log(`plugin conformance passed (${brokerChecks} broker checks across ${fixturePaths.length} fixtures + lifecycle install/enable/upgrade/rollback/uninstall checks)`);
