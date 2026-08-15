import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const source = readFileSync(resolve(root, "src/lib/maps/physical/PhysicalMapEditor.svelte"), "utf8");
const native = readFileSync(resolve(root, "src/lib/maps/native-vector/NativeVectorMapEditor.svelte"), "utf8");
const client = readFileSync(resolve(root, "src/lib/project/client.ts"), "utf8");
const host = readFileSync(resolve(root, "src-tauri/src/lib.rs"), "utf8");
const moduleContext = readFileSync(resolve(root, "src/lib/modules/context.ts"), "utf8");
const timeline = readFileSync(resolve(root, "packages/modules/timeline/src/index.ts"), "utf8");
const mapsManifest = readFileSync(resolve(root, "packages/modules/maps/manifest.json"), "utf8");
const timelineManifest = readFileSync(resolve(root, "packages/modules/timeline/manifest.json"), "utf8");
const coreMaps = readFileSync(resolve(root, "crates/daena-core/src/maps.rs"), "utf8");

for (const required of [
  "onMount",
  "project.cancelPhysicalMap",
  "editor?.dispose()",
  'aria-label="Physical diagnostic layers"',
  'id: "tectonic-plates"',
  'id: "tectonic-boundaries"',
  'id: "bathymetry"',
  'id: "volcanic-centers"',
  'id: "earthquake-hazard"',
  'id: "volcanic-hazard"',
  "width: 384",
  "height: 192",
  "paintPhysicalSurface",
  "let seed = $state(nextPhysicalSeed())",
  'id: "ice"',
  "evolutionPreset",
  "Terrain age",
  "locked: true",
  "persistedCollection",
  "immutablePhysicalLayerIds",
  "authoredSourceAssetId",
  "physicalMapDerivedGeoJson",
]) {
  assert.ok(source.includes(required) || native.includes(required), `physical surface contract is missing ${required}`);
}
assert.ok(source.includes("PhysicalWorldView"), "generate preview must use PhysicalWorldView");
assert.ok(native.includes("PhysicalWorldView"), "saved physical maps must use PhysicalWorldView");
assert.ok(
  readFileSync(resolve(root, "src/lib/maps/physical/PhysicalWorldView.svelte"), "utf8").includes('projection: "globe"'),
  "physical world view must use MapLibre globe projection",
);
assert.ok(
  readFileSync(resolve(root, "src/lib/maps/physical/PhysicalWorldView.svelte"), "utf8").includes("setBackground"),
  "physical world view must update raster without remounting the globe",
);
assert.equal(source.includes("defaultVisible: true"), false, "physical diagnostic layers must start hidden");

for (const required of [
  "PHYSICAL_HISTORICAL_PROGRESS_EVENT",
  "epochPhase",
  "epochProgress",
  "PhysicalHistoricalProgress",
]) {
  assert.ok(native.includes(required), `native historical playback is missing ${required}`);
}
for (const required of ["PhysicalHistoricalProgress", "requestId", "physical-historical-progress"]) {
  assert.ok(client.includes(required), `physical client progress contract is missing ${required}`);
}
for (const required of ["derivationVersion", "relative-generated-v1"]) {
  assert.ok(client.includes(required), `physical client hazard contract is missing ${required}`);
}
for (const required of [
  "PhysicalEventMaterializationRequest",
  "physicalMaterializeEvents",
  "hazardSeed",
  "PhysicalEventMaterializationResult",
  "requestId",
]) {
  assert.ok(client.includes(required), `physical client event contract is missing ${required}`);
}
assert.ok(native.includes("eventRequestId"), "native event materialization must retain its request ID for retries");
for (const required of ["HistoricalProgressEvent", "physical-historical-progress", "with_reporter"]) {
  assert.ok(host.includes(required), `physical host progress contract is missing ${required}`);
}
for (const required of ["hazardDerivationVersion", "HAZARD_DERIVATION_VERSION", "relative-generated-v1"]) {
  assert.ok(host.includes(required), `physical host hazard contract is missing ${required}`);
}
for (const required of [
  "project_physical_materialize_events",
  "EVENT_MATERIALIZATION_VERSION",
  "PHYSICAL_EVENT_ON_MAP_RELATIONSHIP",
  "create_entries_with_request",
]) {
  assert.ok(host.includes(required), `physical host event contract is missing ${required}`);
}
for (const required of ["listShared", "field.read:shared", "__shared_only"]) {
  assert.ok(moduleContext.includes(required), `shared field bridge is missing ${required}`);
}
assert.match(host, /shared_only/);
assert.match(timeline, /listShared\(entity\.id, "maps"\)/);
assert.match(timeline, /catch \{/);
for (const required of [
  "physicalChronology",
  "physical-offset-years",
  "relativeOffsetLabel",
  "daena.maps/navigation",
]) {
  assert.ok(timeline.includes(required), `Timeline physical chronology adapter is missing ${required}`);
}
assert.match(mapsManifest, /"key": "physicalChronology"[\s\S]*"shared": true/);
assert.match(mapsManifest, /daena\.maps:physical-natural-event/);
assert.match(timelineManifest, /"field\.read:shared"/);
assert.match(coreMaps, /PHYSICAL_EVENT_CHRONOLOGY_KEY/);
assert.match(native, /relative generated rates; they are not real-world predictions/);
assert.match(source, /relative generated rates; they are not real-world predictions/);
assert.match(host, /PHYSICAL_PROVIDER/);

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
  "iceCells",
  "iceThicknessMm",
  "children",
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
  "iceCells",
  "iceThicknessMm",
  "children",
  "routingElevationMm",
  "fillDepthMm",
  "accumulationM3PerYear",
]) {
  assert.ok(host.includes(required), `physical host is missing ${required}`);
}

console.log("physical native surface contract check passed");
