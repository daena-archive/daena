#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { FakePluginHost } from "../packages/plugin-test-host/dist/index.js";

const workspace = resolve(import.meta.dirname, "..");
const example = join(workspace, "examples/plugins/timeline-consumer");
const manifest = JSON.parse(readFileSync(join(example, "manifest.json"), "utf8"));

assert.equal(manifest.id, "com.example.timeline-consumer");
assert.equal(manifest.dependencies["daena.timeline"]?.required, false);
assert.deepEqual(manifest.services.consumes, [{ name: "daena.timeline.resolve-date", major: 1 }]);
assert.equal(manifest.capabilities.includes("service.call:daena.timeline.resolve-date@1"), true);

execFileSync("node", [join(workspace, "scripts/plugin-cli.mjs"), "validate", example], { stdio: "pipe" });

const host = new FakePluginHost({
  manifest,
  grants: ["entity.read", "service.call:daena.timeline.resolve-date@1"],
});
host.registerService("daena.timeline.resolve-date", 1, (payload) => ({
  date: payload && typeof payload === "object" ? payload.date : null,
}));
const resolved = await host.client().callService("daena.timeline.resolve-date", 1, { date: "0042-03-15" }, 100);
assert.equal(resolved.date, "0042-03-15");

const missing = new FakePluginHost({
  manifest,
  grants: ["entity.read", "service.call:daena.timeline.resolve-date@1"],
});
await assert.rejects(
  () => missing.client().callService("daena.timeline.resolve-date", 1, { date: "0042-03-15" }, 100),
  /provider-unavailable|not available|unavailable/i,
);

console.log("timeline consumer plugin checks passed");
