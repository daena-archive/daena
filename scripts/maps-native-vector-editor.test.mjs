import assert from "node:assert/strict";

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

const parsed = parseVectorDiagnostic("vector.limit.exceeded: $: source asset exceeds the configured limit");
assert.equal(parsed.code, "vector.limit.exceeded");
assert.equal(parsed.path, "$");

console.log("native vector editor state and coordinate checks passed");
