import { readFile } from "node:fs/promises";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");

const docs = await read("docs/IMAGE_MAP_INTEGRATION.md");
const maps = await read("crates/daena-core/src/maps.rs");
const project = await read("crates/daena-core/src/project.rs");
const sdk = await read("packages/plugin-sdk/src/maps.ts");
const editor = await read("src/lib/maps/image-map/ImageMapEditor.svelte");
const engine = await read("src/lib/maps/image-map/engine.ts");
const client = await read("src/lib/project/client.ts");
const host = await read("src-tauri/src/lib.rs");

for (const required of [
  "IMAGE_MAX_PATH_POINTS: usize = 256",
  "IMAGE_MAX_AREA_RINGS: usize = 8",
  "IMAGE_MAX_SEMANTIC_LAYERS: usize = 32",
  "validate_semantic_style",
  "validate_semantic_selector",
  'rename = "semantic"',
]) {
  if (!maps.includes(required)) throw new Error(`maps.rs missing ${required}`);
}

for (const required of [
  "pub fn create_semantic_layer",
  "pub fn delete_semantic_layer",
  "pub fn query_map_locations",
  "ensure_map_location_projection_schema",
  "geometry TEXT",
  "write_location_projection",
]) {
  if (!project.includes(required)) throw new Error(`project.rs missing ${required}`);
}

for (const required of ["IMAGE_MAX_PATH_POINTS = 256", 'kind?: "semantic"']) {
  if (!sdk.includes(required)) throw new Error(`plugin-sdk maps.ts missing ${required}`);
}

for (const required of ["createSemanticLayer", "deleteSemanticLayer", "queryMapLocations", "anchor?: unknown"]) {
  if (!client.includes(required)) throw new Error(`project client missing ${required}`);
}

for (const required of [
  "project_create_semantic_layer",
  "project_delete_semantic_layer",
  "project_query_map_locations",
]) {
  if (!host.includes(required)) throw new Error(`host missing ${required}`);
}

for (const required of ['"path"', '"area"', "setFeatures", "finishDraft", "onFeatureChange"]) {
  if (!engine.includes(required)) throw new Error(`image map engine missing ${required}`);
}

for (const required of [
  "Add path overlay",
  "Add area overlay",
  "Semantic overlays",
  "persistFeature",
  "syncFeatures",
]) {
  if (!editor.includes(required)) throw new Error(`Image Map editor missing ${required}`);
}

for (const required of [
  "Semantic features are the existing location records",
  "query_map_locations",
  'kind: "semantic"',
]) {
  if (!docs.includes(required)) throw new Error(`IMAGE_MAP_INTEGRATION.md missing ${required}`);
}

if (editor.includes("rasterAssetId") && !editor.includes("deleteSemanticLayer")) {
  throw new Error("Semantic overlay delete must not use deleteRasterLayer");
}

console.log("maps phase-5 semantic features checks passed");
