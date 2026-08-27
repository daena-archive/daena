import assert from "node:assert/strict";
import { applyGeometryOperationCommand, setSnapSettingsCommand } from "../src/lib/maps/editor/commands.ts";
import { createMapDocument, documentHash } from "../src/lib/maps/editor/model.ts";
import { runGeometryOperation } from "../src/lib/maps/editor/geometry-operations.ts";
import { buildPreview, commitSelectionIds } from "../src/lib/maps/editor/geometry-preview.ts";

const layerId = "11111111-1111-4111-8111-111111111111";
const squareA = {
  type: "Feature",
  id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  properties: {
    daena: { layerId, semanticType: "region", name: "A", style: null, label: null, custom: {} },
  },
  geometry: {
    type: "Polygon",
    coordinates: [
      [
        [0, 0],
        [10, 0],
        [10, 10],
        [0, 10],
        [0, 0],
      ],
    ],
  },
};
const squareB = {
  type: "Feature",
  id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
  properties: {
    daena: { layerId, semanticType: "region", name: "B", style: null, label: null, custom: {} },
  },
  geometry: {
    type: "Polygon",
    coordinates: [
      [
        [5, 5],
        [15, 5],
        [15, 15],
        [5, 15],
        [5, 5],
      ],
    ],
  },
};
const line = {
  type: "Feature",
  id: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
  properties: {
    daena: { layerId, semanticType: "route", name: "L", style: null, label: null, custom: {} },
  },
  geometry: {
    type: "LineString",
    coordinates: [
      [0, 7.5],
      [20, 7.5],
    ],
  },
};

const document = createMapDocument({
  descriptor: {
    schemaVersion: 1,
    provider: { id: "daena-openlayers", adapterVersion: 1, sourceFormat: "daena-geojson" },
    sourceAssetId: "22222222-2222-4222-8222-222222222222",
    previewAssetId: null,
    coordinateSpace: { kind: "image", extent: [0, 0, 100, 100], origin: "top-left", units: "pixels" },
    backgrounds: [],
    defaultView: { center: [50, 50], zoom: 1, rotation: 0 },
    settings: { snapEnabled: true, grid: null },
  },
  layers: [
    {
      id: layerId,
      kind: "vector",
      name: "Layer",
      order: 0,
      defaultVisible: true,
      locked: false,
      opacity: 1,
      blendMode: "normal",
      selector: {},
      style: { fill: "#000", fillOpacity: 0.3, stroke: "#000", strokeWidth: 1, pointRadius: 4 },
    },
  ],
  collection: { type: "FeatureCollection", features: [squareA, squareB, line] },
});

const union = runGeometryOperation(document, "union", [squareA.id, squareB.id]);
assert.equal(union.ok, true);
if (union.ok) {
  assert.equal(union.features.length, 1);
  assert.equal(union.removedIds.length, 2);
  const again = runGeometryOperation(document, "union", [squareA.id, squareB.id]);
  assert.ok(again.ok);
  if (again.ok) {
    assert.deepEqual(again.features[0].geometry, union.features[0].geometry);
  }
}

const intersection = runGeometryOperation(document, "intersection", [squareA.id, squareB.id]);
assert.equal(intersection.ok, true);

const split = runGeometryOperation(document, "split", [line.id, squareA.id]);
assert.equal(split.ok, true);
if (split.ok) assert.ok(split.features.length >= 2);

const preview = buildPreview(document, "union", [squareA.id, squareB.id]);
assert.ok(preview.preview);
const beforeHash = documentHash(document);
assert.equal(preview.preview && commitSelectionIds(preview.preview).length, 1);

class MiniStack {
  constructor(doc) {
    this.document = structuredClone(doc);
    this.undoStack = [];
  }
  apply(command) {
    const before = structuredClone(this.document);
    this.document = command.apply(this.document);
    this.undoStack.push({ command, inverse: command.invert(before) });
  }
  undo() {
    const entry = this.undoStack.pop();
    if (!entry) return;
    this.document = entry.inverse.apply(this.document);
  }
}

const stack = new MiniStack(document);
const built = buildPreview(stack.document, "union", [squareA.id, squareB.id]);
assert.ok(built.preview);
const removed = stack.document.collection.features.filter((feature) =>
  built.preview.removedFeatureIds.includes(feature.id),
);
stack.apply(applyGeometryOperationCommand(removed, built.preview.previewFeatures, built.preview.label));
assert.equal(stack.document.collection.features.length, 2);
stack.undo();
assert.equal(stack.document.collection.features.length, 3);
assert.equal(documentHash(stack.document), beforeHash);

const snapDoc = createMapDocument({
  descriptor: document.descriptor,
  layers: document.layers,
  collection: document.collection,
});
const snapStack = new MiniStack(snapDoc);
snapStack.apply(setSnapSettingsCommand(false, true));
assert.equal(snapStack.document.descriptor.settings.snapEnabled, false);
snapStack.undo();
assert.equal(snapStack.document.descriptor.settings.snapEnabled, true);

const lockedDoc = structuredClone(document);
const lockedStack = new MiniStack(lockedDoc);
const lockedBuilt = buildPreview(lockedStack.document, "union", [squareA.id, squareB.id]);
assert.ok(lockedBuilt.preview);
const lockedRemoved = lockedStack.document.collection.features.filter((feature) =>
  lockedBuilt.preview.removedFeatureIds.includes(feature.id),
);
lockedStack.apply(
  applyGeometryOperationCommand(lockedRemoved, lockedBuilt.preview.previewFeatures, lockedBuilt.preview.label),
);
lockedStack.document.layers = lockedStack.document.layers.map((layer) => ({ ...layer, locked: true }));
lockedStack.undo();
assert.equal(lockedStack.document.collection.features.length, 3);
assert.deepEqual(
  lockedStack.document.collection.features.map((feature) => feature.id).sort(),
  document.collection.features.map((feature) => feature.id).sort(),
);

const pointFeature = {
  type: "Feature",
  id: "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
  properties: {
    daena: { layerId, semanticType: "marker", name: null, style: null, label: null, custom: {} },
  },
  geometry: { type: "Point", coordinates: [10, 10] },
};
const imageDoc = createMapDocument({
  ...document,
  collection: { type: "FeatureCollection", features: [pointFeature] },
});
const imageBuffer = runGeometryOperation(imageDoc, "buffer", [pointFeature.id], { bufferDistance: 2 });
assert.equal(imageBuffer.ok, true);
if (imageBuffer.ok) {
  const ring = imageBuffer.features[0].geometry.coordinates[0];
  const xs = ring.map((position) => position[0]);
  const ys = ring.map((position) => position[1]);
  const width = Math.max(...xs) - Math.min(...xs);
  assert.ok(width > 3 && width < 5, `expected ~4px buffer width, got ${width}`);
}

console.log("geometry operations, preview, and snap commands passed");
