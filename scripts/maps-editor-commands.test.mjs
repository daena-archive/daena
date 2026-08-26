import assert from "node:assert/strict";

import { CommandStack } from "../src/lib/maps/editor/command-stack.ts";
import {
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

console.log("map command stack behavior checks passed");
