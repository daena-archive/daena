import assert from "node:assert/strict";

import {
  authoredToNormalized,
  authoredToView,
  coordinateSpaceFromDescriptor,
  DEFAULT_WORLD_SPACE,
  flipYCollection,
  flipYExtent,
  normalizedToAuthored,
  viewToAuthored,
} from "../src/lib/maps/editor/coordinate-space.ts";
import {
  addBackgroundCommand,
  buildCreateLayer,
  buildCreateRasterLayer,
  buildDuplicateLayer,
  calibrateImageToWorld,
  captureDeleteFeatures,
  createFeatureCommand,
  layersFieldValue,
  listedBackgrounds,
  reorderLayersByIdsCommand,
  setBackgroundOpacityCommand,
  setDefaultViewCommand,
  setLayerLockedCommand,
  setLayerOpacityCommand,
  setLayerVisibilityCommand,
} from "../src/lib/maps/editor/commands.ts";
import { CommandStack } from "../src/lib/maps/editor/command-stack.ts";
import { createMapDocument } from "../src/lib/maps/editor/model.ts";
import {
  measurementSummary,
  pathLength,
  polygonArea,
  unitsForCoordinateSpace,
} from "../src/lib/maps/editor/measurement.ts";
import { parseVectorLayers } from "../src/lib/maps/native-vector/source.ts";

const imageSpace = {
  kind: "image",
  extent: [0, 0, 800, 400],
  origin: "top-left",
  units: "pixels",
};

assert.deepEqual(authoredToView([0, 0], imageSpace), [0, 400]);
assert.deepEqual(authoredToView([800, 400], imageSpace), [800, 0]);
assert.deepEqual(viewToAuthored(authoredToView([120, 40], imageSpace), imageSpace), [120, 40]);
assert.deepEqual(authoredToNormalized(0, 0, imageSpace), [0, 0]);
assert.deepEqual(authoredToNormalized(800, 400, imageSpace), [1, 1]);
assert.deepEqual(normalizedToAuthored(0, 0, imageSpace), [0, 0]);
assert.deepEqual(normalizedToAuthored(1, 1, imageSpace), [800, 400]);
assert.deepEqual(authoredToNormalized(0, 90, DEFAULT_WORLD_SPACE), [0.5, 0]);
assert.deepEqual(normalizedToAuthored(0.5, 0, DEFAULT_WORLD_SPACE), [0, 90]);
assert.deepEqual(authoredToView([10, 20], DEFAULT_WORLD_SPACE), [10, 20]);

const descriptor = {
  schemaVersion: 1,
  provider: { id: "daena-openlayers", adapterVersion: 1, sourceFormat: "daena-geojson" },
  sourceAssetId: "11111111-1111-4111-8111-111111111111",
  previewAssetId: "22222222-2222-4222-8222-222222222222",
  coordinateSpace: imageSpace,
  backgrounds: [
    {
      id: "33333333-3333-4333-8333-333333333333",
      assetId: "22222222-2222-4222-8222-222222222222",
      name: "Base image",
      visible: true,
      locked: true,
      opacity: 1,
      order: 0,
      extent: [0, 0, 800, 400],
    },
  ],
  defaultView: { center: [400, 200], zoom: 1, rotation: 0 },
  settings: { snapEnabled: true, grid: null },
};

assert.equal(coordinateSpaceFromDescriptor(descriptor).kind, "image");
assert.equal(unitsForCoordinateSpace(imageSpace).length, "px");
assert.match(measurementSummary(imageSpace), /pixels/);
assert.equal(
  pathLength(
    [
      [0, 0],
      [10, 0],
    ],
    imageSpace,
  ),
  10,
);
assert.equal(
  polygonArea(
    [
      [0, 0],
      [10, 0],
      [10, 10],
      [0, 10],
      [0, 0],
    ],
    imageSpace,
  ),
  100,
);

const geographic = {
  kind: "geographic",
  projection: "EPSG:4326",
  extent: [-180, -90, 180, 90],
  wrapX: true,
};
assert.equal(unitsForCoordinateSpace(geographic).length, "m");
assert.ok(
  pathLength(
    [
      [0, 0],
      [0, 1],
    ],
    geographic,
  ) > 100000,
);

const world = {
  kind: "world",
  extent: [-180, -90, 180, 90],
  origin: "bottom-left",
  units: { id: "world-unit", label: "World units", metresPerUnit: null },
  wrapX: false,
};
assert.match(measurementSummary(world), /uncalibrated/);

const document = createMapDocument({
  descriptor,
  layers: [],
  collection: {
    type: "FeatureCollection",
    features: [
      {
        id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        type: "Feature",
        properties: {
          daena: { layerId: "base", semanticType: "marker", name: null, style: null, label: null, custom: {} },
        },
        geometry: { type: "Point", coordinates: [0, 0] },
      },
    ],
  },
});

const stack = new CommandStack(document);
stack.apply(
  addBackgroundCommand({
    id: "44444444-4444-4444-8444-444444444444",
    assetId: "55555555-5555-4555-8555-555555555555",
    name: "Overlay",
    visible: true,
    locked: false,
    opacity: 1,
    order: 1,
    extent: [0, 0, 800, 400],
  }),
);
assert.equal(listedBackgrounds(stack.document).length, 2);
stack.apply(setBackgroundOpacityCommand("44444444-4444-4444-8444-444444444444", 0.4, 1));
assert.equal(
  listedBackgrounds(stack.document).find((item) => item.id === "44444444-4444-4444-8444-444444444444")?.opacity,
  0.4,
);
stack.undo();
assert.equal(
  listedBackgrounds(stack.document).find((item) => item.id === "44444444-4444-4444-8444-444444444444")?.opacity,
  1,
);

const beforeCalibrate = stack.document.collection.features[0].geometry.coordinates.slice();
const calibrate = calibrateImageToWorld(stack.document, 2);
assert.ok(calibrate);
stack.apply(calibrate);
assert.equal(stack.document.descriptor.coordinateSpace.kind, "world");
assert.deepEqual(stack.document.collection.features[0].geometry.coordinates, [0, 400]);
assert.deepEqual(flipYExtent([0, 0, 800, 400], imageSpace), [0, 0, 800, 400]);
stack.undo();
assert.equal(stack.document.descriptor.coordinateSpace.kind, "image");
assert.deepEqual(stack.document.collection.features[0].geometry.coordinates, beforeCalibrate);

stack.apply(
  setDefaultViewCommand({ center: [10, 20], zoom: 3, rotation: 0 }, { center: [400, 200], zoom: 1, rotation: 0 }),
);
assert.deepEqual(stack.document.descriptor.defaultView.center, [10, 20]);
assert.equal(stack.isDirty(), true);

const flipped = flipYCollection(document.collection, imageSpace);
assert.deepEqual(flipped.features[0].geometry.coordinates, [0, 400]);

const vectorLayer = {
  id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
  kind: "vector",
  name: "Countries",
  order: 0,
  defaultVisible: true,
  locked: false,
  opacity: 1,
  blendMode: "normal",
  selector: {},
  style: { fill: "#8f6fd1", fillOpacity: 0.35, stroke: "#5e4893", strokeWidth: 1.5, pointRadius: 5 },
};
const rasterParsed = parseVectorLayers({
  layers: [
    vectorLayer,
    {
      id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
      name: "Overlay",
      order: 1,
      defaultVisible: true,
      style: {},
      selector: {},
      kind: "raster",
      rasterAssetId: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
      opacity: 0.8,
      locked: false,
      blendMode: "normal",
    },
  ],
});
assert.equal(rasterParsed.length, 2);
assert.equal(rasterParsed[1].kind, "raster");
assert.equal(layersFieldValue(rasterParsed).layers[1].kind, "raster");

const layerDoc = createMapDocument({
  descriptor,
  layers: rasterParsed,
  collection: { type: "FeatureCollection", features: [] },
});
const created = buildCreateRasterLayer(layerDoc, "Hillshade", "dddddddd-dddd-4ddd-8ddd-dddddddddddd");
assert.equal(created.layer.kind, "raster");
const layerStack = new CommandStack(layerDoc);
layerStack.apply(created.command);
assert.equal(
  layerStack.document.layers.some((layer) => layer.id === created.layer.id),
  true,
);
layerStack.apply(setLayerOpacityCommand(created.layer.id, 0.25, 1));
assert.equal(layerStack.document.layers.find((layer) => layer.id === created.layer.id)?.opacity, 0.25);
layerStack.undo();
assert.equal(layerStack.document.layers.find((layer) => layer.id === created.layer.id)?.opacity, 1);

const feature = {
  type: "Feature",
  id: "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee",
  properties: {
    daena: {
      layerId: vectorLayer.id,
      semanticType: "region",
      name: "West",
      style: null,
      label: null,
      custom: {},
    },
  },
  geometry: { type: "Point", coordinates: [1, 2] },
};
layerStack.apply(createFeatureCommand(feature));
layerStack.apply(setLayerLockedCommand(vectorLayer.id, true, false));
const blocked = captureDeleteFeatures(layerStack.document, [feature.id]);
assert.equal(blocked, null);
layerStack.apply(createFeatureCommand({ ...feature, id: "ffffffff-ffff-4fff-8fff-ffffffffffff" }));
assert.equal(layerStack.document.collection.features.length, 1);

const builtVector = buildCreateLayer(layerStack.document, "Cities");
assert.equal(builtVector.layer.kind, "vector");
assert.equal(builtVector.layer.opacity, 1);

const rasterSource = layerStack.document.layers.find((layer) => layer.id === created.layer.id);
assert.equal(rasterSource?.kind, "raster");
const duplicatedAssetId = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
const duplicatedRaster = buildDuplicateLayer(layerStack.document, rasterSource, duplicatedAssetId);
assert.ok(duplicatedRaster);
assert.equal(duplicatedRaster.layer.kind, "raster");
assert.notEqual(duplicatedRaster.layer.rasterAssetId, rasterSource.rasterAssetId);
assert.equal(duplicatedRaster.layer.rasterAssetId, duplicatedAssetId);
layerStack.apply(duplicatedRaster.command);
assert.equal(
  new Set(layerStack.document.layers.filter((layer) => layer.kind === "raster").map((layer) => layer.rasterAssetId))
    .size,
  layerStack.document.layers.filter((layer) => layer.kind === "raster").length,
);

const previousIds = layerStack.document.layers.map((layer) => layer.id);
const reversedIds = [...previousIds].reverse();
layerStack.apply(reorderLayersByIdsCommand(reversedIds, previousIds));
assert.deepEqual(
  [...layerStack.document.layers].sort((left, right) => left.order - right.order).map((layer) => layer.id),
  reversedIds,
);
layerStack.undo();
assert.deepEqual(
  [...layerStack.document.layers].sort((left, right) => left.order - right.order).map((layer) => layer.id),
  previousIds,
);

layerStack.apply(setLayerLockedCommand(vectorLayer.id, false, true));
layerStack.apply(setLayerVisibilityCommand(vectorLayer.id, false, true));
assert.equal(captureDeleteFeatures(layerStack.document, [feature.id]), null);
layerStack.apply(createFeatureCommand({ ...feature, id: "99999999-9999-4999-8999-999999999999" }));
assert.equal(layerStack.document.collection.features.length, 1);

console.log("coordinate-space, measurement units, and raster commands passed");
