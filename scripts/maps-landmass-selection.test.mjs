import assert from "node:assert/strict";
import {
  combineModeFromModifiers,
  combineSelection,
  displayLandmassGeometry,
  epochNotice,
  featureFromSelection,
  geometryContainsPoint,
  hoverCoversPoint,
  hoverIsRedundant,
  hoverPreviewFeature,
  invertSelection,
  leftoverGeometry,
  selectionFromProposal,
  selectionMinusOccupied,
} from "../src/lib/maps/physical/landmass-selection.ts";

function polygon(id, coords) {
  return {
    detector: "landmass",
    regionId: id,
    label: `Landmass ${id + 1}`,
    epochDependent: true,
    cellCount: 4,
    geometry: {
      type: "Polygon",
      coordinates: [coords],
    },
  };
}

const west = polygon(0, [
  [0, 0],
  [10, 0],
  [10, 10],
  [0, 10],
  [0, 0],
]);
const east = polygon(1, [
  [20, 0],
  [30, 0],
  [30, 10],
  [20, 10],
  [20, 0],
]);
const overlap = polygon(2, [
  [5, 5],
  [15, 5],
  [15, 15],
  [5, 15],
  [5, 5],
]);
const land = {
  detector: "land",
  regionId: 0,
  label: "All land",
  epochDependent: true,
  cellCount: 12,
  geometry: {
    type: "MultiPolygon",
    coordinates: [west.geometry.coordinates, east.geometry.coordinates],
  },
};

const replaced = combineSelection(null, west, 0, "replace");
assert.equal(replaced.ok, true);
assert.equal(replaced.selection.label, "Landmass 1");
assert.equal(replaced.selection.regionIds[0], 0);

const added = combineSelection(replaced.selection, east, 0, "add");
assert.equal(added.ok, true);
assert.equal(added.selection.geometry.type, "MultiPolygon");
assert.equal(added.selection.regionIds.length, 2);
assert.equal(added.selection.cellCount, 8);

const subtracted = combineSelection(added.selection, east, 0, "subtract");
assert.equal(subtracted.ok, true);
assert.equal(subtracted.selection.regionIds.includes(1), false);

const mixedEpoch = combineSelection(replaced.selection, east, -1000, "add");
assert.equal(mixedEpoch.ok, false);

const inverted = invertSelection(selectionFromProposal(west, 0), land);
assert.equal(inverted.ok, true);
assert.equal(inverted.selection.detector, "land");
assert.equal(inverted.selection.label, "Land except selection");

assert.equal(combineModeFromModifiers(false, false), "replace");
assert.equal(combineModeFromModifiers(true, false), "add");
assert.equal(combineModeFromModifiers(false, true), "subtract");
assert.match(epochNotice(0), /reference epoch/i);
assert.match(epochNotice(200), /will not rewrite/);

assert.equal(geometryContainsPoint(west.geometry, 5, 5), true);
assert.equal(geometryContainsPoint(west.geometry, 50, 50), false);
assert.equal(hoverCoversPoint(replaced.selection, 5, 5), true);
assert.equal(hoverIsRedundant(replaced.selection, added.selection), true);
assert.equal(hoverPreviewFeature(replaced.selection).id, "landmass-hover-preview");

const authored = featureFromSelection(replaced.selection, "layer-1");
assert.equal(authored.properties.daena.custom.epochDependent, false);
assert.equal(authored.properties.daena.custom.capturedOffsetYears, 0);
assert.equal(authored.properties.daena.custom.detector, "landmass");

const merged = combineSelection(replaced.selection, overlap, 0, "add");
assert.equal(merged.ok, true);
assert.ok(merged.selection.geometry.type === "Polygon" || merged.selection.geometry.type === "MultiPolygon");

const unclosed = polygon(3, [
  [40, 0],
  [50, 0],
  [50, 10],
  [40, 10],
]);
const closedFromOpen = combineSelection(null, unclosed, 0, "replace");
assert.equal(closedFromOpen.ok, true);
const addedUnclosed = combineSelection(closedFromOpen.selection, west, 0, "add");
assert.equal(addedUnclosed.ok, true);

const hole = leftoverGeometry(west.geometry, [
  {
    type: "Polygon",
    coordinates: [
      [
        [1, 1],
        [3, 1],
        [3, 3],
        [1, 3],
        [1, 1],
      ],
    ],
  },
]);
assert.ok(hole);
assert.equal(geometryContainsPoint(hole, 2, 2), false);
assert.equal(geometryContainsPoint(hole, 8, 8), true);

assert.equal(leftoverGeometry(west.geometry, [west.geometry]), null);
const disjoint = leftoverGeometry(west.geometry, [east.geometry]);
assert.ok(disjoint);
assert.equal(geometryContainsPoint(disjoint, 5, 5), true);

const covered = selectionMinusOccupied(replaced.selection, [west.geometry], false);
assert.equal(covered.ok, false);
const keepOverlap = selectionMinusOccupied(replaced.selection, [west.geometry], true);
assert.equal(keepOverlap.ok, true);
assert.equal(displayLandmassGeometry(replaced.selection, [west.geometry], false), null);
assert.ok(displayLandmassGeometry(replaced.selection, [west.geometry], true));
