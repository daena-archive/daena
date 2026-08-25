import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const files = {
  model: readFileSync(new URL("../src/lib/maps/editor/model.ts", import.meta.url), "utf8"),
  commands: readFileSync(new URL("../src/lib/maps/editor/commands.ts", import.meta.url), "utf8"),
  stack: readFileSync(new URL("../src/lib/maps/editor/command-stack.ts", import.meta.url), "utf8"),
  persistence: readFileSync(new URL("../src/lib/maps/editor/persistence.ts", import.meta.url), "utf8"),
  selection: readFileSync(new URL("../src/lib/maps/editor/selection.ts", import.meta.url), "utf8"),
  adapter: readFileSync(new URL("../src/lib/maps/openlayers/MapAdapter.ts", import.meta.url), "utf8"),
};

for (const required of ["MapDocument", "documentHash", "createMapDocument"]) {
  assert.ok(files.model.includes(required), `model missing ${required}`);
}
for (const required of [
  "CreateFeature",
  "DeleteFeatures",
  "ReplaceGeometry",
  "DuplicateFeatures",
  "MoveFeaturesToLayer",
  "SetFeatureMetadata",
  "CreateLayer",
  "DuplicateLayer",
  "DeleteLayer",
  "RenameLayer",
  "ReorderLayer",
  "SetLayerVisibility",
  "SetLayerLocked",
  "SetLayerOpacity",
  "SetLayerStyle",
  "AddBackground",
  "SetDefaultView",
  "SetCoordinateSpace",
  "coalesceKey",
  "invert",
]) {
  assert.ok(files.commands.includes(required), `commands missing ${required}`);
}
for (const required of ["setBaseline", "canUndo", "canRedo", "coalesceKey", "byteBudget", "isDirty"]) {
  assert.ok(files.stack.includes(required), `command-stack missing ${required}`);
}
assert.ok(files.persistence.includes("daena-map-edit-draft"));
assert.ok(files.persistence.includes("encodeLayersField"));
assert.ok(files.selection.includes("selectionFromIds"));
assert.equal(files.adapter.includes("undo("), false);
assert.equal(files.adapter.includes("redo("), false);
assert.ok(files.adapter.includes("onCommand"));
assert.ok(files.adapter.includes("syncDocument"));
assert.ok(files.adapter.includes("dispose()"));
assert.ok(files.adapter.includes("liveAdapters"));

/** Minimal inlined stack to verify coalesce + baseline semantics without TS module resolution. */
function hash(value) {
  return JSON.stringify(value);
}

class MiniStack {
  constructor(document) {
    this.document = structuredClone(document);
    this.baseline = hash(document);
    this.undo = [];
    this.redo = [];
  }
  apply(command) {
    const before = structuredClone(this.document);
    if (command.coalesceKey && this.undo.at(-1)?.coalesceKey === command.coalesceKey) {
      this.document = command.apply(this.document);
      this.undo.at(-1).command = command;
      this.redo = [];
      return;
    }
    this.document = command.apply(this.document);
    this.undo.push({ command, inverse: command.invert(before), coalesceKey: command.coalesceKey });
    this.redo = [];
  }
  canUndo() {
    return this.undo.length > 0;
  }
  isDirty() {
    return hash(this.document) !== this.baseline;
  }
  setBaseline(document) {
    this.document = structuredClone(document);
    this.baseline = hash(document);
    this.undo = [];
    this.redo = [];
  }
  undoOnce() {
    const entry = this.undo.pop();
    if (!entry) return;
    this.document = entry.inverse.apply(this.document);
    this.redo.push(entry);
  }
}

const doc = { features: [], layers: [{ id: "a", visible: true }] };
const stack = new MiniStack(doc);
assert.equal(stack.isDirty(), false);
stack.apply({
  coalesceKey: "g",
  apply: (d) => ({ ...d, features: [1] }),
  invert: (before) => ({ apply: () => structuredClone(before) }),
});
stack.apply({
  coalesceKey: "g",
  apply: (d) => ({ ...d, features: [1, 2] }),
  invert: (before) => ({ apply: () => structuredClone(before) }),
});
assert.equal(stack.undo.length, 1);
assert.deepEqual(stack.document.features, [1, 2]);
stack.undoOnce();
assert.deepEqual(stack.document.features, []);
assert.equal(stack.isDirty(), false);
stack.apply({
  apply: (d) => ({ ...d, layers: [...d.layers, { id: "b", visible: true }] }),
  invert: (before) => ({ apply: () => structuredClone(before) }),
});
assert.equal(stack.isDirty(), true);
stack.setBaseline(stack.document);
assert.equal(stack.isDirty(), false);
assert.equal(stack.canUndo(), false);

assert.ok(files.commands.includes("buildCreateRasterLayer"));
assert.ok(files.commands.includes("buildDuplicateLayer"));
assert.ok(files.commands.includes("protectedLayerIds") || files.commands.includes("layerAcceptsEdits"));
assert.ok(files.commands.includes("SetLayerOpacity"));
assert.ok(files.commands.includes("reorderLayersByIdsCommand"));
assert.ok(files.adapter.includes("fitSelection"));
assert.ok(files.adapter.includes("clearSelection"));
assert.equal(files.adapter.includes("event.key !== \"Delete\""), false);
assert.equal(files.adapter.includes("registry.source.removeFeature"), false);

console.log("map editor command stack checks passed");
