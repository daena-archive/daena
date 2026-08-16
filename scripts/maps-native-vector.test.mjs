import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const require = createRequire(import.meta.url);

function fail(message) {
  throw new Error(`native vector maps check failed: ${message}`);
}

const pkg = JSON.parse(read("package.json"));
if (pkg.dependencies?.konva) fail("Konva must not remain after merging image import into native vector maps");
for (const [name, major] of [
  ["maplibre-gl", "5"],
  ["terra-draw", "1"],
  ["terra-draw-maplibre-gl-adapter", "1"],
  ["d3-contour", "4"],
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
  "src/lib/maps/native-vector/runtime.ts",
  "src/lib/maps/native-vector/style.ts",
  "docs/maps/native-vector-fixtures/phase0-land.geojson",
  "docs/maps/native-vector-licenses.md",
  "docs/maps/native-vector-licenses/maplibre-gl-LICENSE.txt",
  "docs/maps/native-vector-licenses/terra-draw-LICENSE.txt",
  "docs/maps/native-vector-licenses/terra-draw-maplibre-gl-adapter-LICENSE.txt",
  "docs/maps/native-vector-licenses/d3-contour-LICENSE.txt",
  "docs/adr/0013-native-vector-maps.md",
  "src/lib/maps/native-vector/generator.ts",
  "src/lib/maps/native-vector/generator.worker.js",
  "src/lib/maps/native-vector/NativeVectorGenerator.svelte",
  "src/lib/maps/native-vector/source.ts",
  "docs/maps/native-vector-fixtures/phase2-generator.json",
  "docs/maps/phase-0-native-vector-spike.md",
];
for (const path of requiredFiles) {
  if (!existsSync(new URL(`../${path}`, import.meta.url))) fail(`missing ${path}`);
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

const generator = read("src/lib/maps/native-vector/generator.ts");
for (const forbidden of ["Math.random", "fetch(", "invoke(", "localStorage", "indexedDB"]) {
  if (generator.includes(forbidden)) fail(`generator.ts must not use ${forbidden}`);
}
for (const required of [
  "mix32",
  "next(",
  "contours()",
  'viewBox="0 0 340 150"',
  "daena-landmass",
  "GENERATOR_VERSION = 1",
  "CONTINENT_LOBES",
  "NOISE_STRENGTH",
]) {
  if (!generator.includes(required)) fail(`generator.ts missing ${required}`);
}
const worker = read("src/lib/maps/native-vector/generator.worker.js");
if (!worker.includes("generateCandidates") || worker.includes("invoke(")) {
  fail("generator worker must stay offline and mutation-free");
}
const dialog = read("src/lib/maps/native-vector/NativeVectorGenerator.svelte");
for (const required of [
  "Copy",
  "Paste",
  "Regenerate",
  "Back to map details",
  "Full screen",
  "Accept candidate",
  "radiogroup",
]) {
  if (!dialog.includes(required)) fail(`NativeVectorGenerator missing ${required}`);
}
if (!dialog.includes("acceptVectorMap") || !dialog.includes("generationProvenance")) {
  fail("generator dialog must accept through the Phase 1 vector create path");
}

const editor = read("src/lib/maps/native-vector/NativeVectorMapEditor.svelte");
for (const required of [
  "switchLayer",
  "Freehand",
  "Select",
  "editor?.dispose()",
  "NativeVectorGenerator",
  "replaceVectorSource",
  "createVectorLayer",
  "deleteVectorLayer",
  "Reload canonical source",
  "Export draft",
  "Keep editing",
  "Add layer",
  "Selected feature",
  "reduceVectorEditor",
  "Close",
  "Full screen",
]) {
  if (!editor.includes(required)) fail(`NativeVectorMapEditor missing ${required}`);
}
if (editor.includes("PHASE0_VECTOR_LAYERS") || editor.includes("phase0Fixture")) {
  fail("Phase 3 editor must load the canonical source, not the Phase 0 fixture");
}
if (runtime.includes("PHASE0_VECTOR_LAYERS") || runtime.includes("./fixture")) {
  fail("runtime must style from persisted vector layers, not the Phase 0 fixture");
}
if (!runtime.includes("UNDO_STACK_SIZE") || !runtime.includes("syncLayers") || !runtime.includes("deleteSelection")) {
  fail("runtime.ts missing layer sync, undo budget, or selection delete");
}

const client = read("src/lib/project/client.ts");
for (const required of ["replaceVectorSource", "createVectorLayer", "deleteVectorLayer", "mapsRecoveryExport"]) {
  if (!client.includes(required)) fail(`project client missing ${required}`);
}

const tauriLib = read("src-tauri/src/lib.rs");
for (const required of [
  "project_replace_vector_source",
  "project_create_vector_layer",
  "project_delete_vector_layer",
  "maps_recovery_export",
]) {
  if (!tauriLib.includes(required)) fail(`src-tauri/src/lib.rs missing ${required}`);
}

const host = read("src/routes/+page.svelte");
if (!host.includes("NativeVectorMapEditor") || !host.includes('createMap("vector")')) {
  fail("host surface must dispatch the native vector editor");
}

const vite = read("vite.config.js");
if (!vite.includes("export default ${code}") || !vite.includes(".geojson")) {
  fail("vite must wrap .geojson imports as JSON modules");
}

const mapsCore = read("crates/daena-core/src/maps.rs");
if (!mapsCore.includes("daena-vector") || !mapsCore.includes("VECTOR_PROVIDER")) {
  fail("Phase 1 must register the daena-vector provider in maps.rs");
}

const fixture = JSON.parse(read("docs/maps/native-vector-fixtures/phase0-land.geojson"));
if (fixture.type !== "FeatureCollection" || fixture.features.length < 3) fail("phase0 fixture is incomplete");
if (JSON.stringify(fixture).search(/https?:\/\//i) >= 0) fail("fixture must not contain remote URLs");

const maplibrePkg = require("maplibre-gl/package.json");
if (!String(maplibrePkg.version).startsWith("5.")) fail(`resolved maplibre-gl must be 5.x, got ${maplibrePkg.version}`);
if (!existsSync(require.resolve("maplibre-gl/dist/maplibre-gl-csp.js"))) fail("MapLibre CSP bundle is missing");
if (!existsSync(require.resolve("maplibre-gl/dist/maplibre-gl-csp-worker.js"))) fail("MapLibre CSP worker is missing");

const adr = read("docs/adr/0013-native-vector-maps.md");
for (const required of ["One GeoJSON", "longitude", "trusted host", "worker-src"]) {
  if (!adr.toLowerCase().includes(required.toLowerCase())) fail(`ADR 0013 missing ${required}`);
}

console.log("native vector dependency, CSP, generator, fixture, and host-surface checks passed");
