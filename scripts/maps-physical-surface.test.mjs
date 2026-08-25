import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const source = readFileSync(resolve(root, "src/lib/maps/physical/PhysicalMapEditor.svelte"), "utf8");
const native = readFileSync(resolve(root, "src/lib/maps/native-vector/NativeVectorMapEditor.svelte"), "utf8");
const worldView = readFileSync(resolve(root, "src/lib/maps/physical/PhysicalWorldView.svelte"), "utf8");
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
  'id: "tectonic-boundaries"',
  'id: "ocean"',
  'id: "lakes"',
  'id: "rivers"',
  'id: "islands"',
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
for (const required of [
  "tectonic-plates",
  "tectonic-boundaries",
  "bathymetry",
  "volcanic-centers",
  "earthquake-hazard",
  "volcanic-hazard",
]) {
  assert.ok(native.includes(`"${required}"`), `saved physical maps must retain ${required}`);
}
assert.ok(source.includes("PhysicalWorldView"), "generate preview must use PhysicalWorldView");
assert.ok(native.includes("PhysicalWorldView"), "saved physical maps must use PhysicalWorldView");
assert.ok(
  readFileSync(resolve(root, "src/lib/maps/physical/PhysicalWorldView.svelte"), "utf8").includes("openlayers-runtime"),
  "physical world view must use the OpenLayers runtime",
);
assert.ok(
  readFileSync(resolve(root, "src/lib/maps/physical/PhysicalWorldView.svelte"), "utf8").includes("setBackground"),
  "physical world view must update raster without remounting the globe",
);
assert.match(source, /id: BASE_LAYER_ID[\s\S]*defaultVisible: false/, "physical base must start hidden");
assert.match(source, /overlayLayers\(\)/, "generator chips must omit the hidden base layer");
assert.equal(source.includes("Exposed land"), false, "generator must not expose the land overlay");
assert.match(source, /id: "ocean"[\s\S]*defaultVisible: true/, "ocean overlay must start enabled");
assert.match(source, /id: "ice"[\s\S]*defaultVisible: true/, "ice overlay must start enabled");
assert.match(
  source,
  /id: "ocean"[\s\S]*id: "ice"[\s\S]*id: "tectonic-boundaries"/,
  "ice overlay chip must follow ocean",
);
assert.match(source, /This preview locks the world’s physical shape/);
assert.match(source, /high-resolution render/);
assert.match(source, /aria-label="About this preview"/);
assert.match(source, /physical-map-help/);
assert.equal(source.includes("Hillshade"), false, "hillshade must not be a generator toggle");
assert.equal(source.includes("One world, one preview"), false, "generator copy must not use the old slogan");
assert.match(source, /Generate a globe, then accept it/);
assert.match(source, /physical-map-stage/, "generation progress must overlay the globe");
assert.equal(source.includes("physical-map-progress"), false, "generation progress must not sit above overlay chips");

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
assert.ok(native.includes("physicalLayerVisibility"), "physical overlay visibility must stay local");
assert.ok(native.includes("EPOCH_STEP"), "world epoch slider must use year strides");
assert.ok(native.includes("historyCollapsed"), "natural history must start collapsed");
assert.ok(native.includes("layersCollapsed"), "vector layers must be collapsible");
assert.match(native, /Atlas Studio/);
assert.match(native, /Open Physical Map/);
assert.match(native, /ATLAS STUDIO/);
assert.match(native, /editor-body.studio|class:studio=\{studioOpen\}/);
assert.match(native, /studioOpen = studioSupported/);
assert.match(native, /aria-label="Close"/);
assert.match(native, /Export atlas/);
assert.match(native, /PHYSICAL WORLD/);
assert.equal(native.includes("NATIVE VECTOR MAP"), false);
assert.match(native, /years before epoch/);
assert.match(native, /years after epoch/);
assert.match(native, /toLocaleString\("en-US"\)/);
assert.match(native, /sidebar-resizer/);
assert.match(native, /map-busy/);
assert.match(native, /MapViewControls/);
assert.match(worldView, /onpan/);
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
assert.match(native, /relative generated rates; they are not real-world predictions/);
assert.ok(
  readFileSync(resolve(root, "crates/daena-physical-spike/src/contours.rs"), "utf8").includes(
    "CONTOUR_DERIVATION_VERSION",
  ),
  "physical contours must version interpolated geometry",
);
assert.ok(
  readFileSync(resolve(root, "crates/daena-physical-spike/src/history.rs"), "utf8").includes(
    "HISTORICAL_DERIVATION_VERSION: u16 = 2",
  ),
  "historical forcing must bump the derivation version for the cosine model",
);
assert.ok(
  readFileSync(resolve(root, "crates/daena-physical-spike/src/hazards.rs"), "utf8").includes(
    "VOLCANIC_SOURCE_DERIVATION_VERSION",
  ),
  "volcanic origin/rate must be a versioned derivation from v2 centers",
);
for (const required of ["annualRateNano", "sampledCenterId", "volcanicSourceDerivationVersion"]) {
  assert.ok(client.includes(required), `physical client event/hazard contract is missing ${required}`);
}
for (const required of [
  "components",
  "sensitivityPpm",
  "iceMidpointCentiC",
  "iceTransitionWidthCentiC",
  "laggedTemperatureOffsetCentiC",
  "landIceEquilibriumM3",
]) {
  assert.ok(client.includes(required), `physical client historical contract is missing ${required}`);
}
assert.ok(host.includes("history-v{}"), "historical cache key must include the derivation version");
assert.ok(
  readFileSync(resolve(root, "crates/daena-physical-spike/src/hydrology.rs"), "utf8").includes("ThermalExpansion"),
  "hydrology must accept coupled thermal expansion",
);

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
