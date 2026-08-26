import assert from "node:assert/strict";

import {
  VECTOR_MAX_BYTES,
  VECTOR_MAX_FEATURES,
  VECTOR_MAX_FEATURE_POSITIONS,
  VECTOR_MAX_LAYERS,
  VECTOR_MAX_POSITIONS,
} from "../packages/plugin-sdk/src/maps.ts";
import { CommandStack } from "../src/lib/maps/editor/command-stack.ts";
import { detachPhysicalFeaturesCommand } from "../src/lib/maps/editor/commands.ts";
import { createMapDocument } from "../src/lib/maps/editor/model.ts";
import {
  PHYSICAL_DERIVED_LAYER_IDS,
  buildPhysicalDetachPlan,
  isPhysicalDerivedLayerId,
  physicalDetachLayerName,
  physicalFeaturesForLayer,
  selectedPhysicalFeatures,
} from "../src/lib/maps/physical/detach.ts";

const physical = {
  id: "rivers", kind: "vector", name: "Rivers", order: 2, defaultVisible: true, locked: true,
  opacity: 0.8, blendMode: "multiply", selector: {},
  style: { fill: "#4f83a8", fillOpacity: 0.3, stroke: "#245577", strokeWidth: 1.5, pointRadius: 5 },
};
const feature = (id, layerId = "rivers") => ({
  type: "Feature", id,
  properties: { daena: { layerId, semanticType: "route", name: id, style: null, label: null, custom: { source: id } } },
  geometry: { type: "LineString", coordinates: [[0, 0], [1, 1]] },
});
const derived = { type: "FeatureCollection", features: [feature("derived-b"), feature("derived-a")] };
const authored = { type: "FeatureCollection", features: [] };
const document = createMapDocument({ descriptor: {}, layers: [physical], collection: authored });

// --- Supported layer ID contract -------------------------------------------------

assert.deepEqual(PHYSICAL_DERIVED_LAYER_IDS, [
  "base", "ocean", "land", "shelves", "bathymetric-contours", "tectonic-plates",
  "tectonic-boundaries", "bathymetry", "volcanic-centers", "lakes", "rivers",
  "watersheds", "islands", "ice",
]);
assert.equal(isPhysicalDerivedLayerId("earthquake-hazard"), false);
assert.equal(isPhysicalDerivedLayerId("volcanic-hazard"), false);
assert.equal(isPhysicalDerivedLayerId("rivers"), true);

// --- Scope resolution --------------------------------------------------------------

// Another layer's selected IDs have no effect on this layer's selection.
assert.deepEqual(selectedPhysicalFeatures(derived, "rivers", ["not-a-river-id"]), []);
const otherLayerFeature = feature("lake-a", "lakes");
const mixed = { type: "FeatureCollection", features: [...derived.features, otherLayerFeature] };
assert.deepEqual(selectedPhysicalFeatures(mixed, "rivers", ["lake-a"]).map((item) => item.id), []);

// Resolution formula used by the dialog: selected.length > 0 && selected.length < all.length => "selected".
function resolveScope(collection, layerId, selectedIds) {
  const all = physicalFeaturesForLayer(collection, layerId);
  const selected = selectedPhysicalFeatures(collection, layerId, selectedIds);
  return selected.length > 0 && selected.length < all.length ? "selected" : "layer";
}
assert.equal(resolveScope(derived, "rivers", []), "layer", "no selection resolves to whole layer");
assert.equal(
  resolveScope(derived, "rivers", ["derived-a", "derived-b"]),
  "layer",
  "a full-layer selection resolves to whole layer, not 'selected'",
);
assert.equal(resolveScope(derived, "rivers", ["derived-a"]), "selected", "a proper subset defaults to 'selected'");
assert.equal(resolveScope(derived, "rivers", ["lake-a"]), "layer", "a selection from another layer has no effect");

// --- Plan construction: IDs, provenance, style/opacity/blendMode copying -----------

const derivedSnapshotBefore = JSON.parse(JSON.stringify(derived));
const physicalSnapshotBefore = JSON.parse(JSON.stringify(physical));

const ids = ["target-layer", "copy-a", "copy-b"];
const plan = buildPhysicalDetachPlan({
  collection: derived, document, sourceLayer: physical, epochOffsetYears: -250,
  scope: "selected", selectedIds: ["derived-a"], newId: () => ids.shift() ?? "unexpected-id",
});
assert.equal("code" in plan, false);
assert.equal(plan.targetLayer.name, "Rivers — detached at -250 years");
assert.equal(plan.targetLayer.id, "target-layer");
assert.equal(plan.targetLayer.defaultVisible, true);
assert.equal(plan.targetLayer.locked, false);
assert.equal(plan.copies.length, 1);
assert.equal(plan.copies[0].id, "copy-a");
assert.equal(plan.copies[0].properties.daena.layerId, "target-layer");
assert.deepEqual(plan.copies[0].properties.daena.custom, {
  source: "derived-a", detachedFromProvider: "daena-physical", detachedFromLayerId: "rivers",
  detachedFromFeatureId: "derived-a", detachedAtEpochYears: -250,
});

// The target layer inherits the source layer's presentation exactly, via a deep clone.
assert.equal(plan.targetLayer.opacity, physical.opacity);
assert.equal(plan.targetLayer.blendMode, physical.blendMode);
assert.deepEqual(plan.targetLayer.style, physical.style);
assert.notEqual(plan.targetLayer.style, physical.style, "style must be deep-cloned, not shared by reference");

// Zero and positive epoch offsets use the exact generated names.
assert.equal(physicalDetachLayerName("Rivers", 0), "Rivers — detached at reference epoch");
assert.equal(physicalDetachLayerName("Rivers", 40), "Rivers — detached at +40 years");
assert.equal(physicalDetachLayerName("Rivers", -40), "Rivers — detached at -40 years");

// Building the plan must not mutate the source collection, source layer, or source features.
assert.deepEqual(derived, derivedSnapshotBefore, "buildPhysicalDetachPlan must not mutate its input collection");
assert.deepEqual(physical, physicalSnapshotBefore, "buildPhysicalDetachPlan must not mutate its source layer");

// --- Whole-layer scope, feature order -----------------------------------------------

const layerScopeIds = ["target-layer-2", "copy-1", "copy-2"];
const layerScopePlan = buildPhysicalDetachPlan({
  collection: derived, document, sourceLayer: physical, epochOffsetYears: 0,
  scope: "layer", newId: () => layerScopeIds.shift() ?? "unexpected-id",
});
assert.equal("code" in layerScopePlan, false);
assert.equal(layerScopePlan.copies.length, 2);
// Source features are sorted by existing string ID ("derived-a" < "derived-b") before new IDs are assigned.
assert.deepEqual(
  layerScopePlan.copies.map((item) => item.properties.daena.custom.detachedFromFeatureId),
  ["derived-a", "derived-b"],
);
assert.deepEqual(layerScopePlan.copies.map((item) => item.id), ["copy-1", "copy-2"]);

// --- Preflight budgets ---------------------------------------------------------------

function vectorLayer(id) {
  return { ...physical, id, name: id, locked: false };
}

// 1. Layer count limit.
const fullLayers = [physical, ...Array.from({ length: VECTOR_MAX_LAYERS - 1 }, (_, i) => vectorLayer(`filler-${i}`))];
const layersLimitDocument = createMapDocument({ descriptor: {}, layers: fullLayers, collection: authored });
const layersLimitPlan = buildPhysicalDetachPlan({
  collection: derived, document: layersLimitDocument, sourceLayer: physical, epochOffsetYears: 0, scope: "layer",
});
assert.equal(layersLimitPlan.code, "vector.limit.exceeded");
assert.equal(layersLimitPlan.message, `Detaching would exceed the map layer limit of ${VECTOR_MAX_LAYERS}.`);

// 2. Authored feature count limit (existing authored features + prospective copies).
const bulkDerived = {
  type: "FeatureCollection",
  features: Array.from({ length: VECTOR_MAX_FEATURES + 1 }, (_, i) => feature(`bulk-${i}`)),
};
const featuresLimitPlan = buildPhysicalDetachPlan({
  collection: bulkDerived, document, sourceLayer: physical, epochOffsetYears: 0, scope: "layer",
});
assert.equal(featuresLimitPlan.code, "vector.limit.exceeded");
assert.equal(featuresLimitPlan.message, `Detaching would exceed the authored feature limit of ${VECTOR_MAX_FEATURES}.`);

// 3. Per-copied-feature position limit.
const oversizedFeature = feature("oversized");
oversizedFeature.geometry = {
  type: "LineString",
  coordinates: Array.from({ length: VECTOR_MAX_FEATURE_POSITIONS + 1 }, (_, i) => [i, i]),
};
const oversizedDerived = { type: "FeatureCollection", features: [oversizedFeature] };
const featurePositionsPlan = buildPhysicalDetachPlan({
  collection: oversizedDerived, document, sourceLayer: physical, epochOffsetYears: 0, scope: "layer",
});
assert.equal(featurePositionsPlan.code, "vector.limit.exceeded");
assert.equal(
  featurePositionsPlan.message,
  `A detached feature exceeds the per-feature position limit of ${VECTOR_MAX_FEATURE_POSITIONS}.`,
);

// 4. Total authored position limit (an existing authored feature pushes the prospective total over).
const hugeAuthoredFeature = feature("authored-huge", "existing-vector-layer");
hugeAuthoredFeature.geometry = {
  type: "LineString",
  coordinates: Array.from({ length: VECTOR_MAX_POSITIONS + 1 }, (_, i) => [i, i]),
};
const totalPositionsDocument = createMapDocument({
  descriptor: {}, layers: [physical],
  collection: { type: "FeatureCollection", features: [hugeAuthoredFeature] },
});
const totalPositionsPlan = buildPhysicalDetachPlan({
  collection: derived, document: totalPositionsDocument, sourceLayer: physical, epochOffsetYears: 0,
  scope: "selected", selectedIds: ["derived-a"],
});
assert.equal(totalPositionsPlan.code, "vector.limit.exceeded");
assert.equal(totalPositionsPlan.message, `Detaching would exceed the authored position limit of ${VECTOR_MAX_POSITIONS}.`);

// 5. Encoded byte limit (a large property value inflates the prospective collection past the byte budget).
const hugeBytesFeature = feature("authored-bytes", "existing-vector-layer");
hugeBytesFeature.properties.daena.custom = { blob: "a".repeat(VECTOR_MAX_BYTES + 1024) };
const bytesDocument = createMapDocument({
  descriptor: {}, layers: [physical],
  collection: { type: "FeatureCollection", features: [hugeBytesFeature] },
});
const bytesPlan = buildPhysicalDetachPlan({
  collection: derived, document: bytesDocument, sourceLayer: physical, epochOffsetYears: 0,
  scope: "selected", selectedIds: ["derived-a"],
});
assert.equal(bytesPlan.code, "vector.limit.exceeded");
assert.equal(bytesPlan.message, `Detaching would exceed the authored GeoJSON byte limit of ${VECTOR_MAX_BYTES}.`);

// --- Invalid-layer and empty-scope errors -------------------------------------------

const unlockedPhysical = { ...physical, locked: false };
const unlockedDocument = createMapDocument({ descriptor: {}, layers: [unlockedPhysical], collection: authored });
const unlockedPlan = buildPhysicalDetachPlan({
  collection: derived, document: unlockedDocument, sourceLayer: unlockedPhysical, epochOffsetYears: 0, scope: "layer",
});
assert.equal(unlockedPlan.code, "physical.detach.invalid-layer");

const authoredOnlyLayer = { ...physical, id: "not-physical", name: "Custom" };
const authoredOnlyDocument = createMapDocument({ descriptor: {}, layers: [authoredOnlyLayer], collection: authored });
const authoredOnlyPlan = buildPhysicalDetachPlan({
  collection: derived, document: authoredOnlyDocument, sourceLayer: authoredOnlyLayer, epochOffsetYears: 0, scope: "layer",
});
assert.equal(
  authoredOnlyPlan.code,
  "physical.detach.invalid-layer",
  "imported/authored layers are never treated as physical detach sources",
);

const emptyPlan = buildPhysicalDetachPlan({
  collection: { type: "FeatureCollection", features: [] }, document, sourceLayer: physical, epochOffsetYears: 0, scope: "layer",
});
assert.equal(emptyPlan.code, "physical.detach.empty");

// --- Atomic command: apply, undo, redo ----------------------------------------------

const documentSnapshotBefore = JSON.parse(JSON.stringify(document));
const stack = new CommandStack(document);
stack.apply(detachPhysicalFeaturesCommand({
  sourceLayerId: "rivers", sourceLayerName: "Rivers", sourceWasVisible: true,
  targetLayer: plan.targetLayer, copies: plan.copies,
}));
assert.deepEqual(document, documentSnapshotBefore, "the command must not mutate the document object passed to it");
assert.equal(stack.document.layers.find((layer) => layer.id === "rivers").defaultVisible, false);
assert.equal(stack.document.layers.some((layer) => layer.id === "target-layer"), true);
assert.equal(stack.document.collection.features.length, 1);
assert.equal(stack.document.collection.features[0].id, "copy-a");

stack.undo();
assert.equal(stack.document.layers.length, 1, "undo removes only the detached layer");
assert.equal(stack.document.layers[0].defaultVisible, true, "undo restores physical visibility");
assert.equal(stack.document.collection.features.length, 0, "undo removes copied features");

stack.redo();
assert.equal(stack.document.layers.some((layer) => layer.id === "target-layer"), true, "redo restores the same target layer");
assert.equal(stack.document.collection.features[0].id, "copy-a", "redo preserves generated IDs");

// Undo again from a source layer that was already hidden must restore hidden, not visible.
const hiddenPhysical = { ...physical, defaultVisible: false };
const hiddenDocument = createMapDocument({ descriptor: {}, layers: [hiddenPhysical], collection: authored });
const hiddenStack = new CommandStack(hiddenDocument);
hiddenStack.apply(detachPhysicalFeaturesCommand({
  sourceLayerId: "rivers", sourceLayerName: "Rivers", sourceWasVisible: false,
  targetLayer: layerScopePlan.targetLayer, copies: layerScopePlan.copies,
}));
assert.equal(hiddenStack.document.layers.find((layer) => layer.id === "rivers").defaultVisible, false);
hiddenStack.undo();
assert.equal(
  hiddenStack.document.layers.find((layer) => layer.id === "rivers").defaultVisible,
  false,
  "undo restores the exact prior visibility, not an assumed default",
);

console.log("physical detach plan and atomic command checks passed");
