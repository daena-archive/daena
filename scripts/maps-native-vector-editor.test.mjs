import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  initialVectorEditorState,
  parseVectorDiagnostic,
  reduceVectorEditor,
} from "../src/lib/maps/native-vector/editor-state.ts";

const source = readFileSync(new URL("../src/lib/maps/native-vector/source.ts", import.meta.url), "utf8");
const style = readFileSync(new URL("../src/lib/maps/native-vector/style.ts", import.meta.url), "utf8");
const runtime = readFileSync(new URL("../src/lib/maps/native-vector/runtime.ts", import.meta.url), "utf8");
const editor = readFileSync(
  new URL("../src/lib/maps/native-vector/NativeVectorMapEditor.svelte", import.meta.url),
  "utf8",
);

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
assert.equal(runtime.includes("whenStyleReady"), true);
assert.equal(runtime.includes("style is not done loading"), true);
assert.equal(editor.includes("importImageMapFile") || editor.includes('start?: "generate" | "import"'), true);
assert.equal(runtime.includes('delete: "Delete"'), true);
assert.equal(runtime.includes("flush()"), true);
assert.equal(editor.includes("mapsRecoveryExport"), true);
assert.equal(editor.includes("deleteVectorLayer"), true);
assert.equal(editor.includes("Reload canonical source"), true);
assert.equal(editor.includes("Selected feature"), true);
assert.equal(editor.includes("reduceVectorEditor"), true);
assert.equal(editor.includes("Back to map details"), true);
assert.equal(editor.includes("Full screen"), true);
assert.equal(runtime.includes("resize()"), true);

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
