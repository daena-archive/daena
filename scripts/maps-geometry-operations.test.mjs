import assert from "node:assert/strict";
import { applyGeometryOperationCommand, setSnapSettingsCommand } from "../src/lib/maps/editor/commands.ts";
import { createMapDocument, documentHash } from "../src/lib/maps/editor/model.ts";
import { canRunOperation, runGeometryOperation } from "../src/lib/maps/editor/geometry-operations.ts";
import { buildPreview, commitSelectionIds } from "../src/lib/maps/editor/geometry-preview.ts";

const layerId = "11111111-1111-4111-8111-111111111111";
const squareA = {
  type: "Feature",
  id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  properties: {
    daena: {
      layerId,
      semanticType: "region",
      name: "A",
      style: { fill: "#f00", fillOpacity: 0.5, stroke: "#000", strokeWidth: 1, pointRadius: 4 },
      label: null,
      custom: { region: "north" },
    },
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
  assert.equal(union.features[0].id, squareA.id);
  assert.equal(union.features[0].properties.daena.name, "A");
  assert.equal(union.features[0].properties.daena.style?.fill, "#f00");
  assert.equal(union.features[0].properties.daena.custom.region, "north");
  assert.deepEqual(union.removedIds, [squareA.id, squareB.id]);
  const again = runGeometryOperation(document, "union", [squareA.id, squareB.id]);
  assert.ok(again.ok);
  if (again.ok) {
    assert.equal(again.features[0].id, squareA.id);
    assert.deepEqual(again.features[0].geometry, union.features[0].geometry);
  }
}

const difference = runGeometryOperation(document, "difference", [squareA.id, squareB.id]);
assert.equal(difference.ok, true);
if (difference.ok) {
  assert.equal(difference.features[0].id, squareA.id);
  assert.deepEqual(difference.removedIds, [squareA.id]);
}

const intersection = runGeometryOperation(document, "intersection", [squareA.id, squareB.id]);
assert.equal(intersection.ok, true);
if (intersection.ok) {
  assert.equal(intersection.removedIds.length, 0);
  assert.notEqual(intersection.features[0].id, squareA.id);
  assert.notEqual(intersection.features[0].id, squareB.id);
}

const split = runGeometryOperation(document, "split", [line.id, squareA.id]);
assert.equal(split.ok, true);
if (split.ok) {
  assert.ok(split.features.length >= 2);
  assert.equal(split.features[0].id, line.id);
  assert.notEqual(split.features[1].id, line.id);
}

const cutter = {
  type: "Feature",
  id: "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee",
  properties: {
    daena: { layerId, semanticType: "route", name: "cut", style: null, label: null, custom: {} },
  },
  geometry: {
    type: "LineString",
    coordinates: [
      [5, -1],
      [5, 11],
    ],
  },
};
const polygonDoc = createMapDocument({
  ...document,
  collection: { type: "FeatureCollection", features: [squareA, cutter] },
});
function ringArea(ring) {
  let sum = 0;
  for (let index = 0; index < ring.length - 1; index += 1) {
    sum += ring[index][0] * ring[index + 1][1] - ring[index + 1][0] * ring[index][1];
  }
  return Math.abs(sum) / 2;
}

const polygonSplit = runGeometryOperation(polygonDoc, "split", [squareA.id, cutter.id]);
assert.equal(polygonSplit.ok, true);
if (polygonSplit.ok) {
  assert.equal(polygonSplit.features.length, 2);
  assert.equal(polygonSplit.features[0].id, squareA.id);
  assert.notEqual(polygonSplit.features[1].id, squareA.id);
  assert.equal(polygonSplit.features[0].geometry.type, "Polygon");
  assert.equal(polygonSplit.features[1].geometry.type, "Polygon");
  const left = ringArea(polygonSplit.features[0].geometry.coordinates[0]);
  const right = ringArea(polygonSplit.features[1].geometry.coordinates[0]);
  assert.ok(Math.abs(left - 50) < 1e-6, `expected half area 50, got ${left}`);
  assert.ok(Math.abs(right - 50) < 1e-6, `expected half area 50, got ${right}`);
}

assert.equal(canRunOperation("split", [squareA, cutter]), true);
assert.equal(canRunOperation("split", [cutter, squareA]), true);
assert.equal(canRunOperation("split", [squareA, squareB]), false);
assert.equal(canRunOperation("buffer", [line]), true);

const missCutter = {
  type: "Feature",
  id: "abababab-abab-4aba-8aba-abababababab",
  properties: {
    daena: { layerId, semanticType: "route", name: "miss", style: null, label: null, custom: {} },
  },
  geometry: {
    type: "LineString",
    coordinates: [
      [20, 20],
      [30, 30],
    ],
  },
};
const missDoc = createMapDocument({
  ...document,
  collection: { type: "FeatureCollection", features: [squareA, missCutter] },
});
const missed = runGeometryOperation(missDoc, "split", [squareA.id, missCutter.id]);
assert.equal(missed.ok, false);
if (!missed.ok) assert.match(missed.detail, /does not cross/);

const reversed = runGeometryOperation(document, "reverse", [line.id]);
assert.equal(reversed.ok, true);
if (reversed.ok) {
  assert.equal(reversed.features[0].id, line.id);
  assert.deepEqual(reversed.features[0].geometry.coordinates, [
    [20, 7.5],
    [0, 7.5],
  ]);
}

const lineA = {
  type: "Feature",
  id: "ffffffff-ffff-4fff-8fff-ffffffffffff",
  properties: {
    daena: { layerId, semanticType: "route", name: "LA", style: null, label: null, custom: {} },
  },
  geometry: {
    type: "LineString",
    coordinates: [
      [0, 0],
      [10, 0],
    ],
  },
};
const lineB = {
  type: "Feature",
  id: "99999999-9999-4999-8999-999999999999",
  properties: {
    daena: { layerId, semanticType: "route", name: "LB", style: null, label: null, custom: {} },
  },
  geometry: {
    type: "LineString",
    coordinates: [
      [10, 0],
      [20, 0],
    ],
  },
};
const mergeDoc = createMapDocument({
  ...document,
  collection: { type: "FeatureCollection", features: [lineA, lineB] },
});
const merged = runGeometryOperation(mergeDoc, "merge-lines", [lineA.id, lineB.id]);
assert.equal(merged.ok, true);
if (merged.ok) {
  assert.equal(merged.features[0].id, lineA.id);
  assert.deepEqual(merged.features[0].geometry.coordinates, [
    [0, 0],
    [10, 0],
    [20, 0],
  ]);
  assert.deepEqual(merged.removedIds, [lineA.id, lineB.id]);
}

const disconnectedDoc = createMapDocument({
  ...document,
  collection: { type: "FeatureCollection", features: [line, lineA] },
});
const disconnected = runGeometryOperation(disconnectedDoc, "merge-lines", [line.id, lineA.id]);
assert.equal(disconnected.ok, false);
if (!disconnected.ok) assert.match(disconnected.detail, /not connected/);

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
  assert.equal(imageBuffer.features[0].id, pointFeature.id);
  const ring = imageBuffer.features[0].geometry.coordinates[0];
  const xs = ring.map((position) => position[0]);
  const ys = ring.map((position) => position[1]);
  const width = Math.max(...xs) - Math.min(...xs);
  assert.ok(width > 3 && width < 5, `expected ~4px buffer width, got ${width}`);
}

console.log("geometry operations, preview, and snap commands passed");
