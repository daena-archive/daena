import assert from "node:assert/strict";
import {
  copyFeaturesForPaste,
  decodeFeatureClipboard,
  encodeFeatureClipboard,
  MAP_FEATURE_CLIPBOARD_KIND,
  selectableFeatureIds,
} from "../src/lib/maps/editor/clipboard.ts";
import { closestHitOnLine, isClosedPath, traceSubpath } from "../src/lib/maps/editor/trace.ts";

function feature(id, geometry, layerId = "lands") {
  return {
    type: "Feature",
    id,
    properties: {
      daena: {
        layerId,
        semanticType: "route",
        name: null,
        style: null,
        label: null,
        custom: {},
      },
    },
    geometry,
  };
}

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

const line = feature("line-1", {
  type: "LineString",
  coordinates: [
    [0, 0],
    [10, 0],
  ],
});

const encoded = encodeFeatureClipboard([line]);
const parsed = JSON.parse(encoded);
assert.equal(parsed.kind, MAP_FEATURE_CLIPBOARD_KIND);
assert.equal(decodeFeatureClipboard("not-json"), null);
assert.equal(decodeFeatureClipboard(JSON.stringify({ kind: "other", features: [line] })), null);
assert.equal(
  decodeFeatureClipboard(
    JSON.stringify({
      kind: MAP_FEATURE_CLIPBOARD_KIND,
      features: [{ type: "Feature", id: "x", geometry: line.geometry }],
    }),
  ),
  null,
);
const decoded = decodeFeatureClipboard(encoded);
assert.equal(decoded?.length, 1);
assert.equal(decoded[0].id, "line-1");

const pasted = copyFeaturesForPaste(decoded, 2, 3, "overlay");
assert.equal(pasted.length, 1);
assert.notEqual(pasted[0].id, "line-1");
assert.equal(pasted[0].properties.daena.layerId, "overlay");
assert.deepEqual(pasted[0].geometry.coordinates, [
  [2, 3],
  [12, 3],
]);

const hidden = { ...layer, id: "hidden", defaultVisible: false };
const locked = { ...layer, id: "locked", locked: true };
const features = [
  feature("a", line.geometry, "lands"),
  feature("b", line.geometry, "hidden"),
  feature("c", line.geometry, "locked"),
];
assert.deepEqual(selectableFeatureIds(features, [layer, hidden, locked], false), ["a"]);
assert.deepEqual(selectableFeatureIds(features, [layer, hidden, locked], true), ["a", "c"]);

const openPath = [
  [0, 0],
  [10, 0],
  [10, 10],
];
const from = closestHitOnLine(openPath, [2, 0]);
const to = closestHitOnLine(openPath, [10, 5]);
assert.ok(from);
assert.ok(to);
assert.equal(from.index, 0);
assert.equal(to.index, 1);
assert.deepEqual(traceSubpath(openPath, from, to, false), [from.point, [10, 0], to.point]);
assert.equal(isClosedPath(openPath), false);

const ring = [
  [0, 0],
  [10, 0],
  [10, 10],
  [0, 10],
  [0, 0],
];
assert.equal(isClosedPath(ring), true);
const start = closestHitOnLine(ring, [1, 0]);
const end = closestHitOnLine(ring, [9, 0]);
const short = traceSubpath(ring, start, end, true);
assert.equal(short.length, 2);
assert.deepEqual(short[0], start.point);
assert.deepEqual(short[1], end.point);

const midBottom = closestHitOnLine(ring, [5, 0]);
const midTop = closestHitOnLine(ring, [5, 10]);
const around = traceSubpath(ring, midBottom, midTop, true);
assert.deepEqual(around, [midBottom.point, [10, 0], [10, 10], midTop.point]);

const wrapStart = closestHitOnLine(ring, [0, 5]);
const wrapEnd = closestHitOnLine(ring, [5, 0]);
assert.deepEqual(traceSubpath(ring, wrapStart, wrapEnd, true), [wrapStart.point, [0, 0], wrapEnd.point]);
