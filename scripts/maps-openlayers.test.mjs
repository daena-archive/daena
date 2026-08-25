import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const require = createRequire(import.meta.url);

function fail(message) {
  throw new Error(`OpenLayers maps check failed: ${message}`);
}

const pkg = JSON.parse(read("package.json"));
if (!String(pkg.dependencies?.ol ?? "").startsWith("^10.")) fail("package.json must pin OpenLayers major 10");
for (const removed of ["maplibre-gl", "terra-draw", "terra-draw-maplibre-gl-adapter"]) {
  if (pkg.dependencies?.[removed]) fail(`${removed} must be removed`);
}

const lock = read("deno.lock");
if (!lock.includes("npm:ol@^10.10.0")) fail("deno.lock must resolve OpenLayers 10.10");
for (const removed of ["maplibre-gl", "terra-draw"]) {
  if (lock.includes(removed)) fail(`deno.lock still contains ${removed}`);
}

for (const path of [
  "src/lib/maps/native-vector/NativeVectorMapEditor.svelte",
  "src/lib/maps/native-vector/NativeVectorImporter.svelte",
  "src/lib/maps/openlayers/projection.ts",
  "src/lib/maps/openlayers/background-registry.ts",
  "src/lib/maps/openlayers/lifecycle.ts",
  "src/lib/maps/openlayers/interaction-manager.ts",
  "src/lib/maps/editor/command-stack.ts",
  "src/lib/maps/native-vector/openlayers-style.ts",
  "src/lib/maps/atlas/AtlasStudioView.svelte",
]) {
  if (!existsSync(new URL(`../${path}`, import.meta.url))) fail(`missing ${path}`);
}
for (const path of [
  "src/lib/maps/native-vector/runtime.ts",
  "src/lib/maps/native-vector/style.ts",
  "src/lib/maps/native-vector/maplibre-csp.d.ts",
  "src/lib/maps/native-vector/openlayers-runtime.ts",
]) {
  if (existsSync(new URL(`../${path}`, import.meta.url))) fail(`obsolete renderer file remains: ${path}`);
}

const adapter = read("src/lib/maps/openlayers/MapAdapter.ts");
for (const required of [
  'from "ol/Map.js"',
  'from "ol/View.js"',
  "createMapAdapter",
  "bindMapLifecycle",
  "vector.renderer.unavailable",
  "onCommand",
  "syncDocument",
  "projectionFromCoordinateSpace",
]) {
  if (!adapter.includes(required)) fail(`MapAdapter missing ${required}`);
}
for (const forbidden of ["maplibre", "terra-draw", "webgl2", "workerUrl", "UNDO_STACK_SIZE", "imageOverlayCoordinates"]) {
  if (adapter.toLowerCase().includes(forbidden.toLowerCase())) fail(`MapAdapter still contains ${forbidden}`);
}

const registry = read("src/lib/maps/openlayers/layer-registry.ts");
if (registry.includes("Single-source registry")) fail("layer-registry must not remain a single-source registry");
if (!registry.includes("sourceFor") || !registry.includes("vectorOlLayers") || !registry.includes("ImageStatic")) {
  fail("layer-registry must own one OL layer per Daena layer including rasters");
}
if (!registry.includes("featureLayerId") || !registry.includes("featuresForLayer") || !registry.includes("collectionFromSources")) {
  fail("layer-registry must partition GeoJSON by daena.layerId and merge sources on commit");
}
const interactions = read("src/lib/maps/openlayers/interaction-manager.ts");
for (const required of [
  'from "ol/interaction/Draw.js"',
  'from "ol/interaction/Modify.js"',
  'from "ol/interaction/Select.js"',
  'from "ol/interaction/Snap.js"',
  'from "ol/interaction/Translate.js"',
  "traceSource",
  "intersection: true",
]) {
  if (!interactions.includes(required)) fail(`interaction-manager missing ${required}`);
}
if (interactions.includes("layer.id === activeLayerId && !layer.locked")) {
  fail("Select must not be limited to the active layer");
}
if (!interactions.includes("deleteCondition") || !interactions.includes("altKeyOnly")) {
  fail("Modify must expose Alt-click vertex deletion");
}
if (!interactions.includes("pruneSelection") || !interactions.includes("filter(feature)")) {
  fail("hidden and locked features must be pruned from selection and excluded from Modify");
}
if (!adapter.includes("fitSelection") || adapter.includes("event.key !== \"Delete\"")) {
  fail("MapAdapter must fit selection and must not mutate features on Delete");
}

const projection = read("src/lib/maps/openlayers/projection.ts");
if (!projection.includes("projectionFromCoordinateSpace")) fail("projection missing projectionFromCoordinateSpace");
if (!projection.includes('kind === "image"') && !projection.includes("space.kind === \"image\"")) {
  fail("projection factory must handle image coordinate spaces");
}

const background = read("src/lib/maps/openlayers/background-registry.ts");
if (!background.includes('from "ol/source/ImageStatic.js"')) fail("background-registry missing ImageStatic");
if (!background.includes("sync(") && !background.includes("sync(backgrounds")) {
  fail("background-registry must sync multiple rasters");
}

const style = read("src/lib/maps/native-vector/openlayers-style.ts");
for (const required of ["nativeFeatureStyle", "visibleUnlockedFeatures", "new Style", "new Fill", "new Stroke"]) {
  if (!style.includes(required)) fail(`style boundary missing ${required}`);
}
if (/https?:\/\//i.test(style)) fail("style boundary must not embed remote resources");

const editor = read("src/lib/maps/native-vector/NativeVectorMapEditor.svelte");
if (!editor.includes("createMapAdapter") || !editor.includes('renderer: "openlayers"')) {
  fail("native editor is not wired to OpenLayers MapAdapter");
}
if (!editor.includes("CommandStack") || !editor.includes("dispatchCommand")) {
  fail("native editor must use the command stack");
}
if (editor.includes("createVectorLayer") || editor.includes("deleteVectorLayer")) {
  fail("layer mutations must go through the command stack, not immediate RPCs");
}
const physical = read("src/lib/maps/physical/PhysicalWorldView.svelte");
if (!physical.includes("createMapAdapter") || physical.includes('projection: "globe"')) {
  fail("physical world view is not on the OpenLayers MapAdapter");
}
if (!physical.includes("PHYSICAL_COORDINATE_SPACE") && !physical.includes("coordinateSpace")) {
  fail("physical world view must pass an explicit coordinate space");
}
const atlas = read("src/lib/maps/atlas/AtlasStudioView.svelte");
for (const required of ['from "ol/Map.js"', 'from "ol/source/XYZ.js"', "configureTileSource", "tileUrlAllowed"]) {
  if (!atlas.includes(required)) fail(`Atlas Studio missing ${required}`);
}
const lifecycle = read("src/lib/maps/openlayers/lifecycle.ts");
if (!lifecycle.includes("map.dispose()") || !lifecycle.includes("ResizeObserver")) {
  fail("openlayers lifecycle helper must dispose maps and observe size");
}
if (!atlas.includes("bindMapLifecycle")) fail("Atlas Studio must reuse bindMapLifecycle");

const resolved = require("ol/package.json");
if (!String(resolved.version).startsWith("10.")) fail(`resolved OpenLayers must be 10.x, got ${resolved.version}`);

console.log("OpenLayers dependency, renderer boundary, and hard-cut cleanup checks passed");
