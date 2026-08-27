import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";

import { compile } from "svelte/compiler";

const root = resolve(import.meta.dirname, "..");
const require = createRequire(import.meta.url);
const pkg = JSON.parse(readFileSync(resolve(root, "package.json"), "utf8"));
const installed = require("ol/package.json");

assert.match(pkg.dependencies?.ol ?? "", /^\^10\./, "the application declares the supported OpenLayers major");
assert.match(installed.version, /^10\./, "the installed renderer matches the supported major");

const runtimeFiles = [
  "src/lib/maps/openlayers/MapAdapter.ts",
  "src/lib/maps/openlayers/background-registry.ts",
  "src/lib/maps/openlayers/interaction-manager.ts",
  "src/lib/maps/openlayers/layer-registry.ts",
  "src/lib/maps/openlayers/lifecycle.ts",
  "src/lib/maps/openlayers/projection.ts",
];
for (const path of runtimeFiles) assert.equal(existsSync(resolve(root, path)), true, `missing ${path}`);

const components = [
  "src/lib/maps/native-vector/NativeVectorImporter.svelte",
  "src/lib/maps/native-vector/NativeVectorMapEditor.svelte",
  "src/lib/maps/physical/PhysicalMapEditor.svelte",
  "src/lib/maps/physical/PhysicalWorldView.svelte",
  "src/lib/maps/physical/DetachPhysicalLayerDialog.svelte",
  "src/lib/maps/atlas/AtlasRenderPanel.svelte",
  "src/lib/maps/atlas/AtlasStudioView.svelte",
];
for (const path of components) {
  const source = readFileSync(resolve(root, path), "utf8");
  compile(source, { filename: resolve(root, path), css: "injected" });
  assert.doesNotMatch(source, /https?:\/\//i, `${path} must not embed a remote resource`);
}

const acceptedPhysicalEditor = readFileSync(
  resolve(root, "src/lib/maps/native-vector/NativeVectorMapEditor.svelte"),
  "utf8",
);
assert.doesNotMatch(acceptedPhysicalEditor, /PhysicalWorldView/, "accepted physical maps use the OpenLayers adapter");
assert.doesNotMatch(
  acceptedPhysicalEditor,
  /physicalLayerVisibility|withPhysicalVisibility/,
  "physical visibility is persisted in map layers",
);
assert.match(
  acceptedPhysicalEditor,
  /allowLockedBoxSelection: physicalMap/,
  "physical maps allow locked box selection only through OpenLayers",
);

for (const name of [
  "daena-atlas-relief.v1.json",
  "daena-atlas-antique.v1.json",
  "daena-atlas-political.v1.json",
  "daena-atlas-biome.v1.json",
  "daena-atlas-temperature.v1.json",
  "daena-atlas-precipitation.v1.json",
  "daena-atlas-bathymetry.v1.json",
  "daena-atlas-hydrology.v1.json",
]) {
  const style = readFileSync(resolve(root, "docs/maps/atlas/styles", name), "utf8");
  assert.doesNotMatch(style, /https?:\/\/|javascript|shader/i, `${name} must remain declarative and offline`);
  JSON.parse(style);
}

console.log(`OpenLayers boundaries passed (${components.length} Svelte components compiled)`);
