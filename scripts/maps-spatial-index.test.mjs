import assert from "node:assert/strict";
import {
  createSpatialIndex,
  extentOfGeometry,
  extentsOverlap,
  geometryIntersectsExtent,
  recordsFromCollection,
} from "../src/lib/maps/editor/spatial-index.ts";
import {
  layerAcceptsEdits,
  layerIsSelectable,
  layerIsSnapTarget,
  layerIsVisible,
} from "../src/lib/maps/native-vector/types.ts";

const layer = {
  id: "lands",
  kind: "vector",
  name: "Lands",
  order: 0,
  defaultVisible: true,
  locked: false,
  opacity: 1,
  blendMode: "normal",
  selector: {},
  style: {},
};

assert.equal(layerIsVisible(layer), true);
assert.equal(layerIsSelectable(layer), true);
assert.equal(layerAcceptsEdits(layer), true);
assert.equal(layerIsSnapTarget(layer), true);
assert.equal(layerIsSelectable({ ...layer, locked: true }), false);
assert.equal(layerIsSelectable({ ...layer, locked: true }, { viewMode: true }), true);
assert.equal(layerAcceptsEdits({ ...layer, locked: true }), false);
assert.equal(layerIsSnapTarget({ ...layer, locked: true }), false);
assert.equal(layerIsSnapTarget({ ...layer, locked: true }, new Set(["lands"])), true);
assert.equal(layerIsVisible({ ...layer, defaultVisible: false }), false);
assert.equal(layerIsSelectable({ ...layer, defaultVisible: false }), false);

const diagonal = {
  type: "LineString",
  coordinates: [
    [0, 10],
    [10, 0],
  ],
};
const box = [8, 8, 12, 12];
assert.equal(extentsOverlap(extentOfGeometry(diagonal), box), true);
assert.equal(geometryIntersectsExtent(diagonal, box), false, "box select must not use AABB overlap");
assert.equal(
  geometryIntersectsExtent(
    {
      type: "LineString",
      coordinates: [
        [0, 0],
        [10, 10],
      ],
    },
    [4, 4, 6, 6],
  ),
  true,
);
assert.equal(
  geometryIntersectsExtent(
    {
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
    [2, 2, 3, 3],
  ),
  true,
);

const collection = {
  type: "FeatureCollection",
  features: [
    {
      type: "Feature",
      id: "line",
      properties: {
        daena: { layerId: "lands", semanticType: "route", name: null, style: null, label: null, custom: {} },
      },
      geometry: diagonal,
    },
    {
      type: "Feature",
      id: "poly",
      properties: {
        daena: { layerId: "lands", semanticType: "region", name: null, style: null, label: null, custom: {} },
      },
      geometry: {
        type: "Polygon",
        coordinates: [
          [
            [0, 0],
            [4, 0],
            [4, 4],
            [0, 4],
            [0, 0],
          ],
        ],
      },
    },
  ],
};
const index = createSpatialIndex(4);
index.replaceAll(recordsFromCollection(collection));
assert.equal(index.size(), 2);
assert.deepEqual(
  index
    .query([1, 1, 2, 2])
    .filter((record) =>
      geometryIntersectsExtent(collection.features.find((feature) => feature.id === record.id).geometry, [1, 1, 2, 2]),
    )
    .map((record) => record.id),
  ["poly"],
);
assert.equal(geometryIntersectsExtent(collection.features[0].geometry, [8, 8, 12, 12]), false);
assert.deepEqual(
  index.query([8, 8, 12, 12]).map((record) => record.id),
  ["line"],
);
assert.deepEqual(
  index.query([12, 12, 8, 8]).map((record) => record.id),
  ["line"],
);
index.remove("line");
assert.equal(index.size(), 1);
assert.equal(index.query([8, 8, 12, 12]).length, 0);

console.log("spatial index and layer flags passed");
