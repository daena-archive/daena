import assert from "node:assert/strict";

import { CommandStack } from "../src/lib/maps/editor/command-stack.ts";
import {
  captureReplaceCollection,
  createLayerCommand,
  renameLayerCommand,
  setLayerOpacityCommand,
  setLayerVisibilityCommand,
} from "../src/lib/maps/editor/commands.ts";
import { createMapDocument } from "../src/lib/maps/editor/model.ts";

const baseLayer = {
  id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  kind: "vector",
  name: "Places",
  order: 0,
  defaultVisible: true,
  locked: false,
  opacity: 1,
  blendMode: "normal",
  selector: {},
  style: { fill: "#8f6fd1", fillOpacity: 0.35, stroke: "#5e4893", strokeWidth: 1.5, pointRadius: 5 },
};

const document = createMapDocument({
  descriptor: {},
  layers: [baseLayer],
  collection: { type: "FeatureCollection", features: [] },
});
const stack = new CommandStack(document);
const snapshots = [];
stack.onChange((snapshot) => snapshots.push(snapshot));

assert.equal(stack.isDirty(), false);
assert.equal(stack.canUndo(), false);
assert.equal(stack.canRedo(), false);

stack.apply(setLayerOpacityCommand(baseLayer.id, 0.7, 1));
stack.apply(setLayerOpacityCommand(baseLayer.id, 0.4, 0.7));
assert.equal(stack.document.layers[0].opacity, 0.4);
assert.equal(stack.snapshot().undoLabel, "Layer opacity");
assert.equal(stack.undo()?.layers[0].opacity, 1, "coalesced edits undo to the value before the gesture");
assert.equal(stack.canUndo(), false, "coalesced edits occupy one undo entry");
assert.equal(stack.redo()?.layers[0].opacity, 0.4);

stack.apply(renameLayerCommand(baseLayer.id, "Places and regions", "Places"));
stack.apply(renameLayerCommand(baseLayer.id, "Regions", "Places and regions"));
assert.equal(stack.document.layers[0].name, "Regions");
assert.equal(stack.undo()?.layers[0].name, "Places");

stack.apply(setLayerVisibilityCommand(baseLayer.id, false, true));
assert.equal(stack.document.layers[0].defaultVisible, false);
assert.equal(stack.canRedo(), false, "a new command clears redo history");

const secondLayer = { ...baseLayer, id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb", name: "Routes", order: 1 };
stack.apply(createLayerCommand(secondLayer));
assert.equal(stack.document.layers.length, 2);
assert.equal(stack.undo()?.layers.length, 1);
assert.equal(stack.redo()?.layers.length, 2);

const committed = stack.document;
stack.setBaseline(committed);
assert.equal(stack.isDirty(), false);
assert.equal(stack.canUndo(), false);
assert.equal(stack.canRedo(), false);
assert.ok(snapshots.length >= 1, "subscribers receive command-stack snapshots");

committed.layers[0].name = "Mutated caller copy";
assert.equal(stack.document.layers[0].name, "Places", "the command stack owns a defensive document copy");

// --- Protected-layer merge on replace-collection --------------------------------
// This is the general mechanism a physical map's authored/derived state split relies
// on: `CommandStack.document.collection` never holds locked-layer features, so any
// OpenLayers "replace-collection" payload (which reflects the full rendered draft,
// including locked/physical features mixed in for rendering) must have those features
// stripped back out on apply, while any locked-layer features that *do* legitimately
// live in the document are protected from being dropped just because a resync payload
// omits them.

const lockedLayer = { ...baseLayer, id: "cccccccc-cccc-4ccc-8ccc-cccccccccccc", name: "Locked", order: 2, locked: true };
const lockedFeature = {
  type: "Feature", id: "locked-feature",
  properties: { daena: { layerId: lockedLayer.id, semanticType: "region", name: "Locked", style: null, label: null, custom: {} } },
  geometry: { type: "Point", coordinates: [1, 1] },
};
const editableFeature = {
  type: "Feature", id: "editable-feature",
  properties: { daena: { layerId: baseLayer.id, semanticType: "region", name: "Editable", style: null, label: null, custom: {} } },
  geometry: { type: "Point", coordinates: [0, 0] },
};
const protectedDocument = createMapDocument({
  descriptor: {},
  layers: [{ ...baseLayer, order: 0 }, lockedLayer],
  collection: { type: "FeatureCollection", features: [lockedFeature, editableFeature] },
});

// Simulate a rendered draft: the locked feature moved (which must never happen through
// this path) and a spurious extra locked-layer feature was mixed in, while the locked
// feature that legitimately lives in the document is entirely absent from the payload.
const movedLockedFeature = { ...lockedFeature, geometry: { type: "Point", coordinates: [9, 9] } };
const spuriousLockedFeature = {
  type: "Feature", id: "derived-only-feature",
  properties: { daena: { layerId: lockedLayer.id, semanticType: "region", name: "Derived", style: null, label: null, custom: {} } },
  geometry: { type: "Point", coordinates: [5, 5] },
};
const movedEditableFeature = { ...editableFeature, geometry: { type: "Point", coordinates: [2, 2] } };
const nextCollection = {
  type: "FeatureCollection",
  features: [movedLockedFeature, spuriousLockedFeature, movedEditableFeature],
};

const replaceCommand = captureReplaceCollection(protectedDocument, nextCollection, "Edit features");
assert.ok(replaceCommand, "a genuinely different collection produces a command");
const afterReplace = replaceCommand.apply(protectedDocument);
const afterIds = afterReplace.collection.features.map((feature) => feature.id).sort();
assert.deepEqual(afterIds, ["editable-feature", "locked-feature"], "locked-layer content is preserved verbatim and never sourced from the incoming payload");
assert.deepEqual(
  afterReplace.collection.features.find((feature) => feature.id === "locked-feature").geometry.coordinates,
  [1, 1],
  "the locked feature's original geometry is kept even though the payload tried to move it",
);
assert.deepEqual(
  afterReplace.collection.features.find((feature) => feature.id === "editable-feature").geometry.coordinates,
  [2, 2],
  "edits to unlocked-layer features are applied normally",
);

const restored = replaceCommand.invert(protectedDocument).apply(afterReplace);
assert.deepEqual(
  restored.collection.features.map((feature) => feature.id).sort(),
  ["editable-feature", "locked-feature"],
);
assert.deepEqual(
  restored.collection.features.find((feature) => feature.id === "editable-feature").geometry.coordinates,
  [0, 0],
  "undo restores the pre-replace geometry",
);

// A payload that is byte-identical to the current document produces no command at all.
assert.equal(captureReplaceCollection(protectedDocument, protectedDocument.collection), null);

console.log("map command stack behavior checks passed");
