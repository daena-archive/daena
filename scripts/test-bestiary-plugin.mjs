#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { FakePluginHost, FakePluginLifecycleHost, runConformance } from "../packages/plugin-test-host/dist/index.js";

const manifestPath = "examples/plugins/bestiary/manifest.json";
const manifest = JSON.parse(await readFile(resolve(manifestPath), "utf8"));

const host = new FakePluginHost({ manifest });
const client = host.client();

const bootstrap = await client.bootstrap();
assert.equal(bootstrap.pluginId, "com.example.bestiary");
assert.equal(bootstrap.projectId, "test-project");
assert.equal(bootstrap.version, "1.0.0");
assert.deepEqual(bootstrap.grantedCapabilities, [...manifest.capabilities].sort());
console.log("bootstrap identity ok:", bootstrap.pluginId, "v" + bootstrap.version);

const created = await client.createEntity("creature", { habitat: "alpine", diet: "omnivore", dangerLevel: 3 });
assert.equal(created.entityType, "creature");
assert.equal(created.fields.diet, "omnivore");
const entries = await client.listEntities("creature");
assert.equal(entries.length, 1);
assert.equal(entries[0].id, created.id);
assert.deepEqual(entries[0].fields, created.fields);
console.log("entity create/list round-trip ok:", created.id);

const denied = new FakePluginHost({ manifest, grants: ["entity.read"] });
let deniedCode = null;
try {
  await denied.client().createEntity("creature", { diet: "herbivore" });
} catch (error) {
  deniedCode = error.code;
}
assert.equal(deniedCode, "capability-denied");
console.log("capability denial ok: entity.write not granted");

const conformance = await runConformance(host);
for (const result of conformance) {
  assert.equal(result.passed, true, `${result.name}: ${result.detail ?? "failed"}`);
}
console.log(`conformance ok (${conformance.length} checks)`);

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
console.log("lifecycle install/enable/upgrade/rollback/disable/uninstall/delete ok");

console.log(`bestiary plugin test passed (${conformance.length} conformance checks)`);
