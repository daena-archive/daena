import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const source = readFileSync(resolve(root, "src/lib/maps/physical/PhysicalMapEditor.svelte"), "utf8");
const client = readFileSync(resolve(root, "src/lib/project/client.ts"), "utf8");
const host = readFileSync(resolve(root, "src-tauri/src/lib.rs"), "utf8");

for (const required of [
  "onMount",
  "project.cancelPhysicalMap",
  "editor?.dispose()",
  'aria-label="Physical diagnostic layers"',
  'id: "tectonic-plates"',
  'id: "tectonic-boundaries"',
  'id: "bathymetry"',
  'id: "volcanic-centers"',
  "evolutionPreset",
  "Terrain age",
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
for (const required of [
  "PhysicalClimateProducts",
  "physicalMapClimate",
  "physicalMapDerivedClimate",
  "PhysicalEvolutionProducts",
  "physicalMapEvolution",
  "physicalMapDerivedEvolution",
]) {
  assert.ok(client.includes(required), `physical client is missing ${required}`);
}
for (const required of [
  "project_physical_climate",
  "project_physical_derived_climate",
  "project_physical_evolution",
  "project_physical_derived_evolution",
  "temperatureCentiC",
  "runoffVolumeM3PerYear",
  "routingElevationMm",
  "accumulationM3PerYear",
]) {
  assert.ok(host.includes(required), `physical host is missing ${required}`);
}

console.log("physical native surface contract check passed");
