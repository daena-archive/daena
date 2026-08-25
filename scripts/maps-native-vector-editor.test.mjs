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
const style = readFileSync(new URL("../src/lib/maps/native-vector/openlayers-style.ts", import.meta.url), "utf8");
const adapter = readFileSync(new URL("../src/lib/maps/openlayers/MapAdapter.ts", import.meta.url), "utf8");
const interactions = readFileSync(
  new URL("../src/lib/maps/openlayers/interaction-manager.ts", import.meta.url),
  "utf8",
);
const background = readFileSync(
  new URL("../src/lib/maps/openlayers/background-registry.ts", import.meta.url),
  "utf8",
);
const editor = readFileSync(
  new URL("../src/lib/maps/native-vector/NativeVectorMapEditor.svelte", import.meta.url),
  "utf8",
);
const importer = readFileSync(
  new URL("../src/lib/maps/native-vector/NativeVectorImporter.svelte", import.meta.url),
  "utf8",
);
const client = readFileSync(new URL("../src/lib/project/client.ts", import.meta.url), "utf8");
const commandStack = readFileSync(new URL("../src/lib/maps/editor/command-stack.ts", import.meta.url), "utf8");

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

assert.equal(style.includes("visibleUnlockedFeatures"), true);
assert.equal(style.includes("nativeFeatureStyle"), true);
assert.equal(style.includes("new Style"), true);
assert.equal(style.includes("new CircleStyle"), true);
assert.equal(style.includes("https://"), false);
assert.equal(editor.includes("JSON.parse(JSON.stringify(collection))"), true);
assert.equal(editor.includes("return structuredClone(collection)"), false);
assert.equal(background.includes("createBackgroundRegistry"), true);
assert.equal(adapter.includes("projectionFromCoordinateSpace"), true);
assert.equal(adapter.includes("imageOverlayCoordinates"), false);
const raster = readFileSync(new URL("../src/lib/maps/physical/raster.ts", import.meta.url), "utf8");
const worldView = readFileSync(new URL("../src/lib/maps/physical/PhysicalWorldView.svelte", import.meta.url), "utf8");
assert.equal(raster.includes("physicalGridRowForRasterRow"), true);
assert.equal(worldView.includes("createMapAdapter"), true);
assert.equal(worldView.includes("openlayers/MapAdapter"), true);
assert.equal(worldView.includes('projection: "globe"'), false);
assert.equal(worldView.includes("PHYSICAL_COORDINATE_SPACE"), true);
assert.deepEqual(physicalWorldOverlayCoordinates(), [
  [-180, 90],
  [180, 90],
  [180, -90],
  [-180, -90],
]);
assert.deepEqual(lonLatToEquirectangular(-180, 90), [0, 0]);
assert.deepEqual(lonLatToEquirectangular(180, -90), [360, 180]);
assert.deepEqual(lonLatToEquirectangular(0, 0), [180, 90]);
assert.equal(physicalGridRowForRasterRow(0, 256, 32), 31);
assert.equal(physicalGridRowForRasterRow(255, 256, 32), 0);
assert.equal(raster.includes("classifyPhysicalWater"), true);
assert.equal(raster.includes("MIN_VISIBLE_INLAND_WATER_CELLS"), true);
assert.equal(interactions.includes('from "ol/interaction/Draw.js"'), true);
assert.equal(interactions.includes('from "ol/interaction/Modify.js"'), true);
assert.equal(interactions.includes('from "ol/interaction/Snap.js"'), true);
assert.equal(interactions.includes('from "ol/interaction/Translate.js"'), true);
assert.equal(editor.includes("NativeVectorImporter"), true);
assert.equal(editor.includes('start?: "import" | "geojson"'), true);
assert.equal(editor.includes("focusLinkedLocation"), true);
assert.equal(editor.includes("normalizedToAuthored"), true);
assert.equal(editor.includes("Image as ImageIcon"), true);
assert.equal(editor.includes("new Image()"), true);
assert.equal(editor.includes("function publish"), true);
assert.equal(editor.includes("syncUiFromStack(true)"), true);
assert.equal(editor.includes("defaultViewFromDescriptor(commandStack.document.descriptor"), true);
assert.equal(editor.includes("pinsReady"), true);
assert.equal(adapter.includes("focusPoint"), true);
assert.equal(adapter.includes("onMapPick"), true);
assert.equal(adapter.includes("pickArmed"), true);
assert.equal(adapter.includes("lonLatToNormalized"), false);
assert.equal(adapter.includes("authoredToNormalized") || adapter.includes("viewToAuthored"), true);
assert.equal(interactions.includes("activeEditableLayer"), true);
assert.equal(adapter.includes("singleclick"), true);
assert.equal(worldView.includes("pickArmed"), true);
assert.equal(importer.includes("pickImageMapFile"), true);
assert.equal(importer.includes("pickVectorMapFile"), true);
assert.equal(importer.includes("importVectorMapFile"), true);
assert.equal(importer.includes('mode: "image" | "geojson"'), true);
assert.equal(client.includes("pickImageMapFile"), true);
assert.equal(client.includes("attachMapRasterAsset"), true);
assert.equal(editor.includes("Rasters"), true);
assert.equal(editor.includes("attachMapRasterAsset"), true);
assert.equal(editor.includes("actualPixels"), true);
assert.equal(editor.includes("measurementSummary"), true);
assert.equal(editor.includes("Add raster layer"), true);
assert.equal(editor.includes("featureCountForLayer"), true);
assert.equal(editor.includes(".layer-name"), true);
assert.equal(/\.layer-name\s*\{[^}]*width:\s*100%/.test(editor), true);
assert.equal(editor.includes("ondragstart"), true);
assert.equal(editor.includes("altKey"), true);
assert.equal(editor.includes("ArrowUp"), true);
assert.equal(editor.includes("ArrowDown"), true);
assert.equal(editor.includes("fitSelection"), true);
assert.equal(editor.includes("Alt-click a vertex"), true);
assert.equal(editor.includes("createRasterLayer"), false);
assert.equal(editor.includes("duplicateMapRasterAsset"), true);
assert.equal(editor.includes("updateMapLayer"), false);
assert.equal(client.includes("pickVectorMapFile"), true);
assert.equal(client.includes("importVectorMapFile"), true);
assert.equal(client.includes('"png", "jpg", "jpeg", "svg"'), true);
assert.equal(client.includes('"geojson", "json"'), true);
assert.equal(client.includes("acceptVectorMap"), false);
assert.equal(adapter.includes("fitSelection"), true);
assert.equal(adapter.includes('event.key !== "Delete"'), false);
assert.equal(adapter.includes("flush()"), true);
assert.equal(editor.includes("mapsRecoveryExport"), true);
assert.equal(editor.includes("CommandStack"), true);
assert.equal(editor.includes("deleteLayerCommand"), true);
assert.equal(editor.includes("createVectorLayer"), false);
assert.equal(editor.includes("deleteVectorLayer"), false);
assert.equal(editor.includes("Reload canonical source"), true);
assert.equal(editor.includes("Selected feature"), true);
assert.equal(editor.includes("reduceVectorEditor"), true);
assert.equal(editor.includes("onBack") || editor.includes("requestBack"), true);
assert.equal(editor.includes("Full screen"), true);
assert.equal(adapter.includes("applyView"), true);
assert.equal(adapter.includes("setZoom"), true);
assert.equal(adapter.includes("view.setCenter"), true);
assert.equal(adapter.includes("panCardinal"), true);
assert.equal(worldView.includes("onpan"), true);
assert.equal(worldView.includes("min={0}"), true);
assert.equal(worldView.includes("max={8}"), true);
assert.equal(editor.includes("onpan"), true);
assert.equal(editor.includes("min={0}"), true);
assert.equal(editor.includes("max={viewMaxZoom}"), true);
assert.equal(editor.includes("lonLatToNormalized"), false);
assert.equal(editor.includes("authoredToNormalized"), true);
assert.equal(adapter.includes("resetView"), true);
assert.equal(adapter.includes("initialView"), true);
assert.equal(adapter.includes("onViewChange"), true);
assert.equal(worldView.includes("setBackground"), true);
assert.equal(worldView.includes("MapViewControls"), true);
assert.equal(worldView.includes("initialView"), true);
assert.equal(editor.includes("applyHistoricalProducts(products);\n    mountEditor()"), false);
assert.equal(commandStack.includes("setBaseline"), true);
assert.equal(commandStack.includes("coalesceKey"), true);
assert.equal(adapter.includes("undo("), false);
assert.equal(adapter.includes("redo("), false);

let state = initialVectorEditorState();
state = reduceVectorEditor(state, { type: "loaded" });
state = reduceVectorEditor(state, { type: "geometry-changed" });
assert.equal(state.dirty, true);
assert.equal(state.status, "dirty");
state = reduceVectorEditor(state, { type: "document-changed" });
assert.equal(state.dirty, true);
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
