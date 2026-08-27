import assert from "node:assert/strict";
import Feature from "ol/Feature.js";
import Point from "ol/geom/Point.js";
import { CommandStack } from "../src/lib/maps/editor/command-stack.ts";
import { setFeaturesMetadataByIdCommand } from "../src/lib/maps/editor/commands.ts";
import { buildMapSearchIndex, searchMapFeatures } from "../src/lib/maps/editor/map-search.ts";
import { createFeatureCodec } from "../src/lib/maps/openlayers/feature-codec.ts";
import { createLayerRegistry } from "../src/lib/maps/openlayers/layer-registry.ts";
import { featureStyleCacheSize, nativeFeatureStyle } from "../src/lib/maps/openlayers/style-factory.ts";
import { projectionFromCoordinateSpace } from "../src/lib/maps/openlayers/projection.ts";
import { DEFAULT_VECTOR_LAYER_STYLE } from "../src/lib/maps/native-vector/types.ts";

const space = {
  kind: "world",
  extent: [0, 0, 100, 100],
  origin: "bottom-left",
  units: { id: "league", label: "Leagues", metresPerUnit: null },
  wrapX: false,
};
const layer = {
  id: "places",
  kind: "vector",
  name: "Places",
  order: 0,
  defaultVisible: true,
  locked: false,
  opacity: 1,
  blendMode: "normal",
  selector: {},
  style: structuredClone(DEFAULT_VECTOR_LAYER_STYLE),
};
const feature = {
  type: "Feature",
  id: "00000000-0000-4000-8000-000000000001",
  properties: {
    daena: {
      layerId: layer.id,
      semanticType: "marker",
      name: "Amber Harbor",
      style: { fill: "#ff8800", icon: "star", iconSize: 24 },
      label: {
        source: "explicit",
        text: "Port Amber",
        size: 14,
        color: "#ffffff",
        haloColor: "#000000",
        haloWidth: 2,
        placement: "point",
        offset: [2, -16],
        rotation: 0,
        minZoom: 2,
        maxZoom: 8,
      },
      custom: { trade: "amber", population: 1200 },
    },
  },
  geometry: { type: "Point", coordinates: [20, 30] },
};
const document = {
  descriptor: { schemaVersion: 2 },
  layers: [layer],
  collection: { type: "FeatureCollection", features: [feature] },
};

const codec = createFeatureCodec(space, projectionFromCoordinateSpace(space));
const registry = createLayerRegistry(
  document.collection,
  document.layers,
  codec,
  space,
  projectionFromCoordinateSpace(space),
);
assert.equal(registry.sourceFor(layer.id)?.getFeatures().length, 1, "initial map load populates its OpenLayers source");
registry.dispose();

const importedBase = {
  type: "FeatureCollection",
  features: [
    {
      ...feature,
      id: "00000000-0000-4000-8000-000000000002",
      properties: {
        daena: { ...feature.properties.daena, layerId: "base", semanticType: "land" },
      },
    },
  ],
};
const baseRegistry = createLayerRegistry(importedBase, [], codec, space, projectionFromCoordinateSpace(space));
assert.equal(
  baseRegistry.sourceFor("base")?.getFeatures().length,
  1,
  "an imported vector map renders its reserved base features without an authored layer row",
);
assert.equal(baseRegistry.collectionFromLayers().features.length, 1);
baseRegistry.dispose();

const roundTrip = codec.collectionFromSource(
  new (await import("ol/source/Vector.js")).default({ features: codec.readOlFeatures(document.collection) }),
  layer.id,
);
assert.deepEqual(roundTrip.features[0].properties.daena.style, feature.properties.daena.style);
assert.deepEqual(roundTrip.features[0].properties.daena.label, feature.properties.daena.label);
assert.deepEqual(roundTrip.features[0].properties.daena.custom, feature.properties.daena.custom);

const stack = new CommandStack(document);
stack.apply(
  setFeaturesMetadataByIdCommand(
    { [feature.id]: { style: { ...feature.properties.daena.style, stroke: "#123456" } } },
    { [feature.id]: { style: feature.properties.daena.style } },
    "Edit feature style",
  ),
);
assert.equal(stack.document.collection.features[0].properties.daena.style.stroke, "#123456");
stack.undo();
assert.deepEqual(stack.document.collection.features[0].properties.daena.style, feature.properties.daena.style);

const index = buildMapSearchIndex(document.collection, document.layers, new Map([[feature.id, "Guild of Amber"]]));
assert.equal(searchMapFeatures(index, "trade amber guild")[0].featureId, feature.id);
assert.equal(searchMapFeatures(index, "places marker")[0].name, "Amber Harbor");

const olFeature = new Feature({ geometry: new Point([20, 30]) });
olFeature.setId(feature.id);
olFeature.setProperties({
  daenaLayerId: layer.id,
  name: feature.properties.daena.name,
  daenaStyle: feature.properties.daena.style,
  daenaLabel: feature.properties.daena.label,
});
const visible = nativeFeatureStyle(olFeature, [layer], { hovered: false, selected: false, zoom: 4 });
const cached = nativeFeatureStyle(olFeature, [layer], { hovered: false, selected: false, zoom: 4 });
assert.equal(visible, cached, "identical Daena style values reuse one OpenLayers Style instance");
assert.equal(visible.getText().getText(), "Port Amber");
assert.equal(
  nativeFeatureStyle(olFeature, [layer], { hovered: false, selected: false, zoom: 4, labelsVisible: false }).getText(),
  null,
  "physical viewers can suppress generated feature labels without changing authored styles",
);
assert.equal(
  nativeFeatureStyle(olFeature, [layer], {
    hovered: false,
    selected: false,
    zoom: 4,
    labelsVisible: (id) => id !== layer.id,
  }).getText(),
  null,
  "label visibility can be controlled per layer without changing feature styles",
);
assert.equal(nativeFeatureStyle(olFeature, [layer], { hovered: false, selected: false, zoom: 1 }).getText(), null);
assert.ok(featureStyleCacheSize() >= 2);

console.log("map styling, metadata preservation, command, and local search checks passed");
