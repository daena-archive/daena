import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  initialVectorEditorState,
  parseVectorDiagnostic,
  reduceVectorEditor,
} from "../src/lib/maps/native-vector/editor-state.ts";
import {
  physicalGridRowForRasterRow,
  physicalWorldOverlayCoordinates,
} from "../src/lib/maps/native-vector/coordinates.ts";
import { lonLatToEquirectangular } from "../src/lib/maps/physical/equirectangular.ts";

const source = readFileSync(new URL("../src/lib/maps/native-vector/source.ts", import.meta.url), "utf8");
const style = readFileSync(new URL("../src/lib/maps/native-vector/style.ts", import.meta.url), "utf8");
const runtime = readFileSync(new URL("../src/lib/maps/native-vector/runtime.ts", import.meta.url), "utf8");
const editor = readFileSync(
  new URL("../src/lib/maps/native-vector/NativeVectorMapEditor.svelte", import.meta.url),
  "utf8",
);
const importer = readFileSync(
  new URL("../src/lib/maps/native-vector/NativeVectorImporter.svelte", import.meta.url),
  "utf8",
);
const client = readFileSync(new URL("../src/lib/project/client.ts", import.meta.url), "utf8");

for (const required of [
  "parseVectorCollection",
  "parseVectorLayers",
  "featureCountForLayer",
  "isRevisionConflict",
  "collectionBytes",
  "sha256Hex",
]) {
  if (!source.includes(`function ${required}`)) {
    throw new Error(`source.ts missing ${required}`);
  }
}

assert.equal(style.includes("daenaLayerId !== activeLayerId"), true);
assert.equal(style.includes("daena-hover-fill"), true);
assert.equal(style.includes('filter: ["==", ["feature-state"'), false);
assert.equal(style.includes('"fill-opacity": ["case", ["boolean", ["feature-state"'), true);
assert.equal(style.includes("IMAGE_SOURCE_ID"), true);
assert.equal(style.includes("blob:"), true);
assert.equal(editor.includes("JSON.parse(JSON.stringify(collection))"), true);
assert.equal(editor.includes("return structuredClone(collection)"), false);
assert.equal(runtime.includes('type: "canvas"'), true);
assert.equal(runtime.includes("fitBounds"), true);
assert.equal(runtime.includes("imageOverlayCoordinates"), true);
const raster = readFileSync(new URL("../src/lib/maps/physical/raster.ts", import.meta.url), "utf8");
const worldView = readFileSync(new URL("../src/lib/maps/physical/PhysicalWorldView.svelte", import.meta.url), "utf8");
assert.equal(raster.includes("physicalGridRowForRasterRow"), true);
assert.equal(worldView.includes("createNativeVectorEditor"), true);
assert.equal(worldView.includes('projection: "globe"'), true);
assert.equal(worldView.includes("physicalWorldOverlayCoordinates"), true);
assert.deepEqual(physicalWorldOverlayCoordinates(), [
  [-180, 85.05112878],
  [180, 85.05112878],
  [180, -85.05112878],
  [-180, -85.05112878],
]);
assert.deepEqual(lonLatToEquirectangular(-180, 90), [0, 0]);
assert.deepEqual(lonLatToEquirectangular(180, -90), [360, 180]);
assert.deepEqual(lonLatToEquirectangular(0, 0), [180, 90]);
assert.equal(physicalGridRowForRasterRow(0, 256, 32), 31);
assert.equal(physicalGridRowForRasterRow(255, 256, 32), 0);
assert.equal(raster.includes("classifyPhysicalWater"), true);
assert.equal(raster.includes("MIN_VISIBLE_INLAND_WATER_CELLS"), true);
assert.equal(runtime.includes("whenStyleReady"), true);
assert.equal(runtime.includes("style is not done loading"), true);
assert.equal(editor.includes("NativeVectorImporter"), true);
assert.equal(editor.includes('start?: "import" | "geojson"'), true);
assert.equal(editor.includes("focusLinkedLocation"), true);
assert.equal(editor.includes("pinsReady"), true);
assert.equal(runtime.includes("focusPoint"), true);
assert.equal(runtime.includes("onMapPick"), true);
assert.equal(runtime.includes("pickArmed"), true);
assert.equal(runtime.includes("lonLatToNormalized"), true);
assert.equal(runtime.includes("active-layer gate"), true);
assert.equal(runtime.includes("onCanvasPointerUp"), true);
assert.equal(worldView.includes("pickArmed"), true);
assert.equal(importer.includes("pickImageMapFile"), true);
assert.equal(importer.includes("pickVectorMapFile"), true);
assert.equal(importer.includes("importVectorMapFile"), true);
assert.equal(importer.includes('mode: "image" | "geojson"'), true);
assert.equal(client.includes("pickImageMapFile"), true);
assert.equal(client.includes("pickVectorMapFile"), true);
assert.equal(client.includes("importVectorMapFile"), true);
assert.equal(client.includes('"png", "jpg", "jpeg", "svg"'), true);
assert.equal(client.includes('"geojson", "json"'), true);
assert.equal(client.includes("acceptVectorMap"), false);
assert.equal(runtime.includes('delete: "Delete"'), true);
assert.equal(runtime.includes("flush()"), true);
assert.equal(editor.includes("mapsRecoveryExport"), true);
assert.equal(editor.includes("deleteVectorLayer"), true);
assert.equal(editor.includes("Reload canonical source"), true);
assert.equal(editor.includes("Selected feature"), true);
assert.equal(editor.includes("reduceVectorEditor"), true);
assert.equal(editor.includes("onBack") || editor.includes("requestBack"), true);
assert.equal(editor.includes("Full screen"), true);
assert.equal(runtime.includes("applyView"), true);
assert.equal(runtime.includes("setZoom"), true);
assert.equal(runtime.includes("applyLookAt"), true);
assert.equal(runtime.includes("panBy"), true);
assert.equal(worldView.includes("onpan"), true);
assert.equal(worldView.includes("min={0}"), true);
assert.equal(worldView.includes("max={8}"), true);
assert.equal(editor.includes("onpan"), true);
assert.equal(editor.includes("min={0}"), true);
assert.equal(editor.includes("max={8}"), true);
assert.equal(editor.includes("lonLatToNormalized"), true);
assert.equal(runtime.includes("resetView"), true);
assert.equal(runtime.includes("initialView"), true);
assert.equal(runtime.includes("onViewChange"), true);
assert.equal(worldView.includes("setBackground"), true);
assert.equal(worldView.includes("MapViewControls"), true);
assert.equal(worldView.includes("initialView"), true);
assert.equal(editor.includes("applyHistoricalProducts(products);\n    mountEditor()"), false);

let state = initialVectorEditorState();
state = reduceVectorEditor(state, { type: "loaded" });
state = reduceVectorEditor(state, { type: "geometry-changed" });
assert.equal(state.dirty, true);
assert.equal(state.status, "dirty");
state = reduceVectorEditor(state, { type: "save-started" });
assert.equal(state.status, "saving");
state = reduceVectorEditor(state, { type: "save-conflict", message: "asset revision conflict: expected a, current b" });
assert.equal(state.conflict, true);
assert.equal(state.dirty, true);
assert.equal(state.diagnosticCode, "asset.revision-conflict");
state = reduceVectorEditor(state, { type: "keep-editing" });
assert.equal(state.conflict, false);
assert.equal(state.dirty, true);
state = reduceVectorEditor(state, { type: "reload" });
assert.equal(state.dirty, false);
state = reduceVectorEditor(state, { type: "geometry-changed" });
state = reduceVectorEditor(state, {
  type: "save-failed",
  message: "vector.geometry.invalid: features/0/geometry: ring is self-intersecting",
});
assert.equal(state.status, "error");
assert.equal(state.diagnosticCode, "vector.geometry.invalid");
assert.match(state.diagnostic, /features\/0\/geometry/);

const parsed = parseVectorDiagnostic("vector.limit.exceeded: $: source asset exceeds 16 MiB");
assert.equal(parsed.code, "vector.limit.exceeded");
assert.equal(parsed.path, "$");

console.log("native vector Phase 3 source and layer helpers passed");
