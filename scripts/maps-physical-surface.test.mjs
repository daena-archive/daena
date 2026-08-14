import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const source = readFileSync(resolve(root, "src/lib/maps/physical/PhysicalMapEditor.svelte"), "utf8");

for (const required of [
  "onMount",
  "project.cancelPhysicalMap",
  "editor?.dispose()",
  'aria-label="Physical diagnostic layers"',
  'id: "tectonic-plates"',
  'id: "tectonic-boundaries"',
  'id: "bathymetry"',
  'id: "volcanic-centers"',
  "locked: true",
]) {
  assert.ok(source.includes(required), `physical surface contract is missing ${required}`);
}

assert.match(
  source,
  /status\.state === "running" \|\| status\.state === "cancelling"/,
  "surface teardown must cancel an active generation",
);
assert.equal(/https?:\/\//.test(source), false, "physical surface must not introduce remote resources");

console.log("physical native surface contract check passed");
