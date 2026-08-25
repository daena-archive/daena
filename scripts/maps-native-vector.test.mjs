import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const require = createRequire(import.meta.url);

function fail(message) {
  throw new Error(`native vector maps check failed: ${message}`);
}

const pkg = JSON.parse(read("package.json"));
if (pkg.dependencies?.konva) fail("Konva must not remain after merging image import into native vector maps");
if (pkg.dependencies?.["d3-contour"]) fail("d3-contour must not remain after removing landmass generation");
for (const [name, major] of [
  ["maplibre-gl", "5"],
  ["terra-draw", "1"],
  ["terra-draw-maplibre-gl-adapter", "1"],
]) {
  const spec = pkg.dependencies?.[name];
  if (!spec) fail(`package.json missing ${name}`);
  if (!spec.includes(`^${major}.`) && !spec.startsWith(major + "."))
    fail(`${name} must stay on major ${major}, got ${spec}`);
}

const lock = read("deno.lock");
if (!lock.includes("npm:maplibre-gl@^5.24.0") || lock.includes("npm:maplibre-gl@^6")) {
  fail("deno.lock must pin MapLibre 5, not 6");
}

const tauri = JSON.parse(read("src-tauri/tauri.conf.json"));
const csp = tauri.app?.security?.csp ?? "";
if (!csp.includes("worker-src 'self'")) fail("tauri CSP must set worker-src 'self'");
if (!/img-src[^;]*blob:/.test(csp)) fail("tauri CSP must allow blob: images for local map previews");
if (csp.includes("worker-src") && csp.includes("blob:") && /worker-src[^;]*blob:/.test(csp)) {
  fail("MapLibre worker must not rely on blob: worker-src");
}

const requiredFiles = [
  "src/lib/maps/native-vector/NativeVectorMapEditor.svelte",
  "src/lib/maps/native-vector/NativeVectorImporter.svelte",
  "src/lib/maps/native-vector/runtime.ts",
  "src/lib/maps/native-vector/style.ts",
  "src/lib/maps/native-vector/source.ts",
  "docs/adr/0013-native-vector-maps.md",
];
for (const path of requiredFiles) {
  if (!existsSync(new URL(`../${path}`, import.meta.url))) fail(`missing ${path}`);
}
for (const removed of [
  "src/lib/maps/native-vector/generator.ts",
  "src/lib/maps/native-vector/generator.worker.js",
  "src/lib/maps/native-vector/NativeVectorGenerator.svelte",
  "src/lib/maps/native-vector/fixture.ts",
]) {
  if (existsSync(new URL(`../${removed}`, import.meta.url))) fail(`${removed} must be removed`);
}

const runtime = read("src/lib/maps/native-vector/runtime.ts");
for (const required of [
  "maplibre-gl/dist/maplibre-gl-csp.js",
  "maplibre-gl/dist/maplibre-gl-csp-worker.js?url",
  "setWorkerUrl",
  "renderWorldCopies: globe",
  "style.load",
  "TerraDrawPointMode",
  "TerraDrawLineStringMode",
  "TerraDrawPolygonMode",
  "TerraDrawFreehandMode",
  "TerraDrawSelectMode",
  ".stop()",
  "map.remove()",
  "revokeObjectURL",
  "vector.renderer.unavailable",
  "webgl2",
]) {
  if (!runtime.toLowerCase().includes(required.toLowerCase())) fail(`runtime.ts missing ${required}`);
}

const style = read("src/lib/maps/native-vector/style.ts");
if (/https?:\/\//i.test(style)) fail("style.ts must not embed remote URLs");
if (!style.includes("daena-base") || !style.includes("daena-authored"))
  fail("style.ts must define base and authored sources");
if (style.includes("glyphs") || style.includes("sprite") || style.includes("tiles")) {
  fail("offline style must omit glyphs, sprites, and tiles");
}
if (!style.includes('if (!base) return "visible"')) {
  fail("base land must stay visible when maps:layers omits a base row (imports)");
}

const importer = read("src/lib/maps/native-vector/NativeVectorImporter.svelte");
for (const required of [
  "pickImageMapFile",
  "pickVectorMapFile",
  "importImageMapFile",
  "importVectorMapFile",
  "Import vector map",
  "Import image",
]) {
  if (!importer.includes(required)) fail(`NativeVectorImporter missing ${required}`);
}
if (importer.includes("acceptVectorMap") || importer.includes("generateCandidates")) {
  fail("importer must not use landmass generation");
}

const editor = read("src/lib/maps/native-vector/NativeVectorMapEditor.svelte");
for (const required of [
  "switchLayer",
  "Freehand",
  "Select",
  "editor?.dispose()",
  "NativeVectorImporter",
  "replaceVectorSource",
  "createVectorLayer",
  "deleteVectorLayer",
  "Reload canonical source",
  "Export draft",
  "Keep editing",
  "Add layer",
  "Selected feature",
  "reduceVectorEditor",
  "Full screen",
]) {
  if (!editor.includes(required)) fail(`NativeVectorMapEditor missing ${required}`);
}
if (editor.includes("NativeVectorGenerator") || editor.includes("PHASE0_VECTOR_LAYERS") || editor.includes("phase0Fixture")) {
  fail("editor must not reference the removed generator or Phase 0 fixture");
}
if (runtime.includes("PHASE0_VECTOR_LAYERS") || runtime.includes("./fixture")) {
  fail("runtime must style from persisted vector layers, not the Phase 0 fixture");
}
if (!runtime.includes("UNDO_STACK_SIZE") || !runtime.includes("syncLayers") || !runtime.includes("deleteSelection")) {
  fail("runtime.ts missing layer sync, undo budget, or selection delete");
}

const client = read("src/lib/project/client.ts");
for (const required of [
  "replaceVectorSource",
  "createVectorLayer",
  "deleteVectorLayer",
  "mapsRecoveryExport",
  "importVectorMapFile",
  "importImageMapFile",
]) {
  if (!client.includes(required)) fail(`project client missing ${required}`);
}
if (client.includes("acceptVectorMap")) fail("shell client must not expose landmass acceptVectorMap");

const tauriLib = read("src-tauri/src/lib.rs");
for (const required of [
  "project_replace_vector_source",
  "project_create_vector_layer",
  "project_delete_vector_layer",
  "maps_recovery_export",
  "project_import_vector_map_file",
  "project_import_image_map_file",
]) {
  if (!tauriLib.includes(required)) fail(`src-tauri/src/lib.rs missing ${required}`);
}

const host = read("src/routes/+page.svelte");
if (!host.includes("NativeVectorMapEditor") || !host.includes('createMap("vector")')) {
  fail("host surface must dispatch the native vector editor");
}
if (!host.includes('createMap("image")') || !host.includes("Import vector map") || !host.includes("Import image")) {
  fail("host must expose Import vector map and Import image create actions");
}
if (!host.includes('provider === "image" ? "import" : "geojson"')) {
  fail("createMap must set mapsVectorStart for image import vs GeoJSON import");
}

const vite = read("vite.config.js");
if (vite.includes("d3-contour")) fail("vite must not prebundle removed d3-contour");
if (!vite.includes("export default ${code}") || !vite.includes(".geojson")) {
  fail("vite must wrap .geojson imports as JSON modules");
}

const mapsCore = read("crates/daena-core/src/maps.rs");
if (!mapsCore.includes("daena-vector") || !mapsCore.includes("VECTOR_PROVIDER")) {
  fail("maps.rs must register the daena-vector provider");
}
const vectorCore = read("crates/daena-core/src/maps/vector.rs");
if (!vectorCore.includes("canonicalize_imported_base")) {
  fail("vector.rs must canonicalize imported GeoJSON base land");
}

const maplibrePkg = require("maplibre-gl/package.json");
if (!String(maplibrePkg.version).startsWith("5.")) fail(`resolved maplibre-gl must be 5.x, got ${maplibrePkg.version}`);
if (!existsSync(require.resolve("maplibre-gl/dist/maplibre-gl-csp.js"))) fail("MapLibre CSP bundle is missing");
if (!existsSync(require.resolve("maplibre-gl/dist/maplibre-gl-csp-worker.js"))) fail("MapLibre CSP worker is missing");

const adr = read("docs/adr/0013-native-vector-maps.md");
for (const required of ["One GeoJSON", "longitude", "trusted host", "worker-src"]) {
  if (!adr.toLowerCase().includes(required.toLowerCase())) fail(`ADR 0013 missing ${required}`);
}

console.log("native vector dependency, CSP, importer, and host-surface checks passed");
