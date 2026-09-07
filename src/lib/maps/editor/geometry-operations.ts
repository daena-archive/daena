import buffer from "@turf/buffer";
import difference from "@turf/difference";
import { featureCollection, lineString, multiPolygon, point, polygon } from "@turf/helpers";
import intersect from "@turf/intersect";
import lineSplit from "@turf/line-split";
import simplify from "@turf/simplify";
import union from "@turf/union";
import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps.ts";
import { VECTOR_MAX_FEATURE_POSITIONS } from "../../../../packages/plugin-sdk/src/maps.ts";
import { coordinateSpaceFromDescriptor } from "./coordinate-space.ts";
import { closeLineStringAsPolygon, geometryPositionCount } from "../native-vector/geometry.ts";
import { featureLayerId, layerAcceptsEdits, type VectorFeature } from "../native-vector/types.ts";
import {
  cancelledGeometryResult,
  canRunOperation,
  isLineGeometry,
  isPolygonGeometry,
  isReversibleLine,
  operationLabel,
  type GeometryOperationKind,
  type GeometryOpJobOptions,
  type GeometryOpParams,
  type GeometryOpProgress,
  type GeometryOpResult,
} from "./geometry-operation-kinds.ts";
import { findLayer, type MapDocument } from "./model.ts";

export {
  canRunOperation,
  cancelledGeometryResult,
  isLineGeometry,
  isPolygonGeometry,
  isReversibleLine,
  operationLabel,
  type GeometryOperationKind,
  type GeometryOpJobOptions,
  type GeometryOpParams,
  type GeometryOpProgress,
  type GeometryOpResult,
};

type Position = number[];
type GeoJsonProperties = Record<string, unknown> | null;
type Point = { type: "Point"; coordinates: Position };
type LineString = { type: "LineString"; coordinates: Position[] };
type Polygon = { type: "Polygon"; coordinates: Position[][] };
type MultiPolygon = { type: "MultiPolygon"; coordinates: Position[][][] };
type Feature<G> = { type: "Feature"; geometry: G; properties: GeoJsonProperties };

const MICRO_SCALE = 1_000_000;
const MERGE_TOLERANCE = 1 / MICRO_SCALE;
const INTERSECT_EPS = 1e-9;

type RingHit = {
  edgeIndex: number;
  tEdge: number;
  cutterSeg: number;
  tCutter: number;
  point: Position;
};

function roundCoord(value: number): number {
  return Math.round(value * MICRO_SCALE) / MICRO_SCALE;
}

function roundPosition(position: Position): Position {
  return [roundCoord(position[0]), roundCoord(position[1])];
}

function roundRing(ring: Position[]): Position[] {
  return ring.map((entry) => roundPosition(entry));
}

function closeRing(ring: Position[]): Position[] {
  if (ring.length < 3) return ring;
  const first = ring[0];
  const last = ring[ring.length - 1];
  if (first[0] === last[0] && first[1] === last[1]) return ring;
  return [...ring, [...first]];
}

function ringArea(ring: Position[]): number {
  let sum = 0;
  for (let index = 0; index < ring.length - 1; index += 1) {
    const current = ring[index];
    const next = ring[index + 1];
    sum += current[0] * next[1] - next[0] * current[1];
  }
  return sum;
}

function orientExterior(ring: Position[]): Position[] {
  const closed = closeRing(ring);
  if (closed.length < 4) return closed;
  return ringArea(closed) < 0 ? [...closed].reverse() : closed;
}

function turfPolygonToDaena(feature: Feature<Polygon | MultiPolygon>): VectorFeature["geometry"] | null {
  const geometry = feature.geometry;
  if (geometry.type === "Polygon") {
    const rings = geometry.coordinates.map((ring) => orientExterior(roundRing(ring)));
    if (rings.length === 0 || rings[0].length < 4) return null;
    return { type: "Polygon", coordinates: rings };
  }
  if (geometry.type === "MultiPolygon") {
    const polygons = geometry.coordinates.map((poly) => poly.map((ring) => orientExterior(roundRing(ring))));
    if (polygons.length === 0) return null;
    return { type: "MultiPolygon", coordinates: polygons };
  }
  return null;
}

function turfLineToDaena(feature: Feature<LineString>): VectorFeature["geometry"] | null {
  const coords = feature.geometry.coordinates.map((entry) => roundPosition(entry));
  if (coords.length < 2) return null;
  return { type: "LineString", coordinates: coords };
}

function featureToTurf(feature: VectorFeature): Feature<Polygon | MultiPolygon | LineString | Point> | null {
  const { geometry } = feature;
  if (geometry.type === "Polygon") {
    return polygon(geometry.coordinates as Position[][]);
  }
  if (geometry.type === "MultiPolygon") {
    return multiPolygon(geometry.coordinates as Position[][][]);
  }
  if (geometry.type === "LineString") {
    return lineString(geometry.coordinates as Position[]);
  }
  if (geometry.type === "Point") {
    return point(geometry.coordinates as Position);
  }
  return null;
}

function validateGeometry(geometry: VectorFeature["geometry"]): string | null {
  const count = geometryPositionCount(geometry);
  if (count === 0) return "empty geometry";
  if (count > VECTOR_MAX_FEATURE_POSITIONS) return "geometry exceeds position budget";
  return null;
}

function cloneProperties(source: VectorFeature): VectorFeature["properties"] {
  return JSON.parse(JSON.stringify(source.properties)) as VectorFeature["properties"];
}

function resultFeature(source: VectorFeature, geometry: VectorFeature["geometry"], id: string): VectorFeature {
  return {
    type: "Feature",
    id,
    properties: cloneProperties(source),
    geometry,
  };
}

function validateResult(geometry: VectorFeature["geometry"]): GeometryOpResult | null {
  const error = validateGeometry(geometry);
  if (error) return { ok: false, code: "vector.geometry.invalid", detail: error };
  return null;
}

function validateSelection(
  document: MapDocument,
  ids: readonly string[],
): { ok: true; features: VectorFeature[] } | { ok: false; code: string; detail: string } {
  if (ids.length === 0) {
    return { ok: false, code: "geometry.selection.empty", detail: "Select one or more features." };
  }
  const features = ids
    .map((id) => document.collection.features.find((feature) => feature.id === id))
    .filter((feature): feature is VectorFeature => Boolean(feature));
  if (features.length !== ids.length) {
    return { ok: false, code: "geometry.selection.missing", detail: "One or more selected features were not found." };
  }
  const layerIds = new Set(features.map((feature) => featureLayerId(feature)));
  if (layerIds.size !== 1) {
    return { ok: false, code: "geometry.selection.layer", detail: "Selected features must belong to one layer." };
  }
  const layerId = [...layerIds][0];
  const layer = findLayer(document.layers, layerId);
  if (!layerAcceptsEdits(layer)) {
    return { ok: false, code: "geometry.layer.locked", detail: "The target layer is hidden or locked." };
  }
  return { ok: true, features };
}

function unionAll(features: Feature<Polygon | MultiPolygon>[]): Feature<Polygon | MultiPolygon> | null {
  if (features.length < 2) return null;
  return union(featureCollection(features)) as Feature<Polygon | MultiPolygon> | null;
}

export function runGeometryOperation(
  document: MapDocument,
  operation: GeometryOperationKind,
  selectedIds: readonly string[],
  params: GeometryOpParams = {},
): GeometryOpResult {
  const selection = validateSelection(document, selectedIds);
  if (!selection.ok) return selection;

  switch (operation) {
    case "union":
      return runUnion(selection.features);
    case "difference":
      return runDifference(selection.features);
    case "intersection":
      return runIntersection(selection.features);
    case "split":
      return runSplit(selection.features);
    case "buffer":
      return runBuffer(
        selection.features,
        params.bufferDistance ?? 0,
        coordinateSpaceFromDescriptor(document.descriptor),
      );
    case "simplify":
      return runSimplify(selection.features, params.simplifyTolerance ?? 0.01);
    case "reverse":
      return runReverse(selection.features);
    case "merge-lines":
      return runMergeLines(selection.features);
  }
}

function runUnion(features: VectorFeature[]): GeometryOpResult {
  if (features.length < 2) {
    return { ok: false, code: "geometry.union.count", detail: "Union requires at least two polygon features." };
  }
  if (!features.every((feature) => isPolygonGeometry(feature.geometry))) {
    return { ok: false, code: "geometry.union.type", detail: "Union requires polygon features." };
  }
  const turfFeatures = features
    .map((feature) => featureToTurf(feature))
    .filter((feature): feature is Feature<Polygon | MultiPolygon> => Boolean(feature));
  const merged = unionAll(turfFeatures);
  if (!merged) {
    return { ok: false, code: "geometry.union.failed", detail: "Union produced no geometry." };
  }
  const geometry = turfPolygonToDaena(merged);
  if (!geometry) {
    return { ok: false, code: "geometry.union.invalid", detail: "Union result could not be represented." };
  }
  const invalid = validateResult(geometry);
  if (invalid) return invalid;
  return {
    ok: true,
    features: [resultFeature(features[0], geometry, features[0].id)],
    removedIds: features.map((feature) => feature.id),
  };
}

function runDifference(features: VectorFeature[]): GeometryOpResult {
  if (features.length < 2) {
    return { ok: false, code: "geometry.difference.count", detail: "Difference requires a base polygon and a cutter." };
  }
  if (!features.every((feature) => isPolygonGeometry(feature.geometry))) {
    return { ok: false, code: "geometry.difference.type", detail: "Difference requires polygon features." };
  }
  const turfFeatures = features
    .map((feature) => featureToTurf(feature))
    .filter((feature): feature is Feature<Polygon | MultiPolygon> => Boolean(feature));
  const result = difference(featureCollection(turfFeatures)) as Feature<Polygon | MultiPolygon> | null;
  if (!result) {
    return { ok: false, code: "geometry.difference.empty", detail: "Difference removed all geometry." };
  }
  const geometry = turfPolygonToDaena(result);
  if (!geometry) {
    return { ok: false, code: "geometry.difference.invalid", detail: "Difference result could not be represented." };
  }
  const invalid = validateResult(geometry);
  if (invalid) return invalid;
  return {
    ok: true,
    features: [resultFeature(features[0], geometry, features[0].id)],
    removedIds: [features[0].id],
  };
}

function runIntersection(features: VectorFeature[]): GeometryOpResult {
  if (features.length < 2) {
    return {
      ok: false,
      code: "geometry.intersection.count",
      detail: "Intersection requires at least two polygon features.",
    };
  }
  if (!features.every((feature) => isPolygonGeometry(feature.geometry))) {
    return { ok: false, code: "geometry.intersection.type", detail: "Intersection requires polygon features." };
  }
  const turfFeatures = features
    .map((feature) => featureToTurf(feature))
    .filter((feature): feature is Feature<Polygon | MultiPolygon> => Boolean(feature));
  const result = intersect(featureCollection(turfFeatures)) as Feature<Polygon | MultiPolygon> | null;
  if (!result) {
    return { ok: false, code: "geometry.intersection.empty", detail: "Intersection is empty." };
  }
  const geometry = turfPolygonToDaena(result);
  if (!geometry) {
    return {
      ok: false,
      code: "geometry.intersection.invalid",
      detail: "Intersection result could not be represented.",
    };
  }
  const invalid = validateResult(geometry);
  if (invalid) return invalid;
  return {
    ok: true,
    features: [resultFeature(features[0], geometry, crypto.randomUUID())],
    removedIds: [],
  };
}

function runSplit(features: VectorFeature[]): GeometryOpResult {
  if (features.length !== 2) {
    return {
      ok: false,
      code: "geometry.split.count",
      detail: "Split requires a target feature and a cutter.",
    };
  }
  const [target, cutter] = features;
  if (target.geometry.type === "Polygon" && isLineGeometry(cutter.geometry)) {
    return runSplitPolygon(target, cutter);
  }
  if (isLineGeometry(target.geometry)) {
    return runSplitLine(target, cutter);
  }
  return {
    ok: false,
    code: "geometry.split.type",
    detail: "Split a line with a cutter, or a polygon with a cutting line.",
  };
}

function runSplitLine(lineFeature: VectorFeature, cutterFeature: VectorFeature): GeometryOpResult {
  const line = featureToTurf(lineFeature);
  if (!line || line.geometry.type !== "LineString") {
    return { ok: false, code: "geometry.split.line", detail: "Split line must be a LineString." };
  }
  let splitter: Feature<LineString | Polygon | MultiPolygon> | null = null;
  if (isLineGeometry(cutterFeature.geometry)) {
    splitter = featureToTurf(cutterFeature) as Feature<LineString>;
  } else if (isPolygonGeometry(cutterFeature.geometry)) {
    splitter = featureToTurf(cutterFeature) as Feature<Polygon | MultiPolygon>;
  }
  if (!splitter) {
    return { ok: false, code: "geometry.split.cutter", detail: "Cutter must be a line or polygon." };
  }
  const split = lineSplit(line as Feature<LineString>, splitter as Parameters<typeof lineSplit>[1]);
  const parts = split.features
    .map((part) => turfLineToDaena(part as Feature<LineString>))
    .filter((geometry): geometry is VectorFeature["geometry"] => Boolean(geometry));
  if (parts.length < 2) {
    return { ok: false, code: "geometry.split.noop", detail: "Line was not split." };
  }
  for (const geometry of parts) {
    const invalid = validateResult(geometry);
    if (invalid) return invalid;
  }
  return {
    ok: true,
    features: parts.map((geometry, index) =>
      resultFeature(lineFeature, geometry, index === 0 ? lineFeature.id : crypto.randomUUID()),
    ),
    removedIds: [lineFeature.id],
  };
}

function cross(ax: number, ay: number, bx: number, by: number): number {
  return ax * by - ay * bx;
}

function segmentIntersection(
  a0: Position,
  a1: Position,
  b0: Position,
  b1: Position,
): { point: Position; tA: number; tB: number } | null {
  const dax = a1[0] - a0[0];
  const day = a1[1] - a0[1];
  const dbx = b1[0] - b0[0];
  const dby = b1[1] - b0[1];
  const denom = cross(dax, day, dbx, dby);
  if (Math.abs(denom) < INTERSECT_EPS) return null;
  const tA = cross(b0[0] - a0[0], b0[1] - a0[1], dbx, dby) / denom;
  const tB = cross(b0[0] - a0[0], b0[1] - a0[1], dax, day) / denom;
  if (tA < -INTERSECT_EPS || tA > 1 + INTERSECT_EPS || tB < -INTERSECT_EPS || tB > 1 + INTERSECT_EPS) return null;
  return { point: [a0[0] + tA * dax, a0[1] + tA * day], tA, tB };
}

function positionsEqual(left: Position, right: Position, tolerance = MERGE_TOLERANCE): boolean {
  return Math.abs(left[0] - right[0]) <= tolerance && Math.abs(left[1] - right[1]) <= tolerance;
}

function cutterDist(hit: RingHit): number {
  return hit.cutterSeg + hit.tCutter;
}

function ringHits(ring: Position[], cutter: Position[]): RingHit[] {
  const hits: RingHit[] = [];
  const edgeCount = ring.length - 1;
  for (let edgeIndex = 0; edgeIndex < edgeCount; edgeIndex += 1) {
    const a0 = ring[edgeIndex];
    const a1 = ring[edgeIndex + 1];
    for (let cutterSeg = 0; cutterSeg < cutter.length - 1; cutterSeg += 1) {
      const hit = segmentIntersection(a0, a1, cutter[cutterSeg], cutter[cutterSeg + 1]);
      if (!hit) continue;
      if (hit.tA > 1 - INTERSECT_EPS) continue;
      hits.push({
        edgeIndex,
        tEdge: Math.max(0, Math.min(1, hit.tA)),
        cutterSeg,
        tCutter: Math.max(0, Math.min(1, hit.tB)),
        point: roundPosition(hit.point),
      });
    }
  }
  const unique: RingHit[] = [];
  for (const hit of hits) {
    if (!unique.some((existing) => positionsEqual(existing.point, hit.point))) unique.push(hit);
  }
  unique.sort((left, right) => left.edgeIndex - right.edgeIndex || left.tEdge - right.tEdge);
  return unique;
}

function ringChain(ring: Position[], from: RingHit, to: RingHit): Position[] {
  const points: Position[] = [from.point];
  const edgeCount = ring.length - 1;
  if (from.edgeIndex === to.edgeIndex && from.tEdge <= to.tEdge) {
    if (!positionsEqual(from.point, to.point)) points.push(to.point);
    return points;
  }
  let edge = from.edgeIndex + 1;
  for (let step = 0; step <= edgeCount; step += 1) {
    const vertex = ring[edge % edgeCount];
    if (!positionsEqual(points[points.length - 1], vertex)) points.push(roundPosition(vertex));
    if (edge % edgeCount === to.edgeIndex) break;
    edge += 1;
  }
  if (!positionsEqual(points[points.length - 1], to.point)) points.push(to.point);
  return points;
}

function cutterSubpath(cutter: Position[], from: RingHit, to: RingHit): Position[] {
  const start = cutterDist(from) <= cutterDist(to) ? from : to;
  const end = start === from ? to : from;
  const points: Position[] = [start.point];
  for (let index = start.cutterSeg + 1; index <= end.cutterSeg; index += 1) {
    if (index > cutterDist(start) + INTERSECT_EPS && index < cutterDist(end) - INTERSECT_EPS) {
      const vertex = roundPosition(cutter[index]);
      if (!positionsEqual(points[points.length - 1], vertex)) points.push(vertex);
    }
  }
  if (!positionsEqual(points[points.length - 1], end.point)) points.push(end.point);
  return start === from ? points : [...points].reverse();
}

function polygonArea(geometry: VectorFeature["geometry"]): number {
  if (geometry.type !== "Polygon" || geometry.coordinates.length === 0) return 0;
  return Math.abs(ringArea(closeRing(geometry.coordinates[0])));
}

function polygonFromChains(alongRing: Position[], alongCutter: Position[]): VectorFeature["geometry"] | null {
  const ring = [...alongRing];
  for (const point of alongCutter.slice(1)) {
    if (!positionsEqual(ring[ring.length - 1], point)) ring.push(point);
  }
  const closed = orientExterior(roundRing(ring));
  if (closed.length < 4) return null;
  return { type: "Polygon", coordinates: [closed] };
}

function runSplitPolygon(polygonFeature: VectorFeature, cutterFeature: VectorFeature): GeometryOpResult {
  const geometry = polygonFeature.geometry;
  if (geometry.type !== "Polygon") {
    return { ok: false, code: "geometry.split.polygon", detail: "Cannot split a multi-polygon in one step." };
  }
  if (geometry.coordinates.length > 1) {
    return { ok: false, code: "geometry.split.holes", detail: "Cannot split a polygon that contains holes." };
  }
  const ring = closeRing(geometry.coordinates[0].map((entry) => roundPosition(entry)));
  const cutter = (cutterFeature.geometry as LineString).coordinates.map((entry) => roundPosition(entry));
  if (cutter.length < 2) {
    return { ok: false, code: "geometry.split.cutter", detail: "Cutting line must have at least two coordinates." };
  }
  const hits = ringHits(ring, cutter);
  if (hits.length !== 2) {
    return {
      ok: false,
      code: "geometry.split.cross",
      detail: "Cannot split this polygon because the cut does not cross the boundary.",
    };
  }
  const [hitA, hitB] = hits;
  if (positionsEqual(hitA.point, hitB.point)) {
    return {
      ok: false,
      code: "geometry.split.cross",
      detail: "Cannot split this polygon because the cut does not cross the boundary.",
    };
  }
  const first = polygonFromChains(ringChain(ring, hitA, hitB), cutterSubpath(cutter, hitB, hitA));
  const second = polygonFromChains(ringChain(ring, hitB, hitA), cutterSubpath(cutter, hitA, hitB));
  if (!first || !second) {
    return { ok: false, code: "geometry.split.invalid", detail: "Split produced an invalid ring." };
  }
  const originalArea = Math.abs(ringArea(ring));
  const partArea = polygonArea(first) + polygonArea(second);
  if (partArea <= INTERSECT_EPS || Math.abs(partArea - originalArea) > Math.max(1e-6, originalArea * 1e-6)) {
    return {
      ok: false,
      code: "geometry.split.cross",
      detail: "Cannot split this polygon because the cut does not cross the boundary.",
    };
  }
  for (const part of [first, second]) {
    const invalid = validateResult(part);
    if (invalid) return invalid;
  }
  return {
    ok: true,
    features: [
      resultFeature(polygonFeature, first, polygonFeature.id),
      resultFeature(polygonFeature, second, crypto.randomUUID()),
    ],
    removedIds: [polygonFeature.id],
  };
}

function turfBufferDistance(
  space: MapCoordinateSpace,
  distance: number,
): { distance: number; units: "meters" | "degrees" } {
  if (space.kind === "geographic") return { distance, units: "meters" };
  return { distance, units: "degrees" };
}

function runBuffer(features: VectorFeature[], distance: number, space: MapCoordinateSpace): GeometryOpResult {
  if (features.length !== 1) {
    return { ok: false, code: "geometry.buffer.count", detail: "Buffer one feature at a time." };
  }
  if (!Number.isFinite(distance) || distance <= 0) {
    return { ok: false, code: "geometry.buffer.distance", detail: "Buffer distance must be a positive number." };
  }
  const source = features[0];
  const turfFeature = featureToTurf(source);
  if (!turfFeature) {
    return { ok: false, code: "geometry.buffer.type", detail: "Buffer requires a point, line, or polygon." };
  }
  const turfDistance = turfBufferDistance(space, distance);
  const buffered = buffer(turfFeature, turfDistance.distance, { units: turfDistance.units });
  const geometry = turfPolygonToDaena(buffered as Feature<Polygon | MultiPolygon>);
  if (!geometry) {
    return { ok: false, code: "geometry.buffer.invalid", detail: "Buffer result could not be represented." };
  }
  const invalid = validateResult(geometry);
  if (invalid) return invalid;
  return {
    ok: true,
    features: [resultFeature(source, geometry, source.id)],
    removedIds: [source.id],
  };
}

function runSimplify(features: VectorFeature[], tolerance: number): GeometryOpResult {
  if (features.length !== 1) {
    return { ok: false, code: "geometry.simplify.count", detail: "Simplify one feature at a time." };
  }
  if (!Number.isFinite(tolerance) || tolerance <= 0) {
    return { ok: false, code: "geometry.simplify.tolerance", detail: "Simplify tolerance must be positive." };
  }
  const source = features[0];
  const { geometry } = source;
  if (geometry.type === "LineString") {
    const ring = closeLineStringAsPolygon(geometry.coordinates);
    if (!ring) {
      return { ok: false, code: "geometry.simplify.line", detail: "LineString is too short to simplify." };
    }
    const simplified = simplify(lineString(geometry.coordinates as Position[]), {
      tolerance,
      highQuality: true,
    }) as Feature<LineString>;
    const next = turfLineToDaena(simplified);
    if (!next) return { ok: false, code: "geometry.simplify.invalid", detail: "Simplify produced invalid geometry." };
    const invalid = validateResult(next);
    if (invalid) return invalid;
    return { ok: true, features: [resultFeature(source, next, source.id)], removedIds: [source.id] };
  }
  if (geometry.type === "Polygon") {
    const turfFeature = polygon(geometry.coordinates as Position[][]);
    const simplified = simplify(turfFeature, { tolerance, highQuality: true }) as Feature<Polygon>;
    const next = turfPolygonToDaena(simplified);
    if (!next) return { ok: false, code: "geometry.simplify.invalid", detail: "Simplify produced invalid geometry." };
    const invalid = validateResult(next);
    if (invalid) return invalid;
    return { ok: true, features: [resultFeature(source, next, source.id)], removedIds: [source.id] };
  }
  return { ok: false, code: "geometry.simplify.type", detail: "Simplify supports lines and polygons only." };
}

function runReverse(features: VectorFeature[]): GeometryOpResult {
  if (features.length !== 1) {
    return { ok: false, code: "geometry.reverse.count", detail: "Reverse one line at a time." };
  }
  const source = features[0];
  const { geometry } = source;
  if (geometry.type === "LineString") {
    const next: VectorFeature["geometry"] = {
      type: "LineString",
      coordinates: [...geometry.coordinates].reverse().map((entry) => roundPosition(entry)),
    };
    const invalid = validateResult(next);
    if (invalid) return invalid;
    return { ok: true, features: [resultFeature(source, next, source.id)], removedIds: [source.id] };
  }
  if (geometry.type === "MultiLineString") {
    const next: VectorFeature["geometry"] = {
      type: "MultiLineString",
      coordinates: [...geometry.coordinates]
        .reverse()
        .map((line) => [...line].reverse().map((entry) => roundPosition(entry))),
    };
    const invalid = validateResult(next);
    if (invalid) return invalid;
    return { ok: true, features: [resultFeature(source, next, source.id)], removedIds: [source.id] };
  }
  return { ok: false, code: "geometry.reverse.type", detail: "Reverse requires a line." };
}

function lineEndpoints(coordinates: Position[]): { start: Position; end: Position } {
  return { start: coordinates[0], end: coordinates[coordinates.length - 1] };
}

function concatLines(left: Position[], right: Position[]): Position[] {
  const next = left.map((entry) => roundPosition(entry));
  const skip = positionsEqual(next[next.length - 1], right[0]) ? 1 : 0;
  for (const point of right.slice(skip)) next.push(roundPosition(point));
  return next;
}

function tryJoin(first: Position[], second: Position[]): Position[] | null {
  const a = lineEndpoints(first);
  const b = lineEndpoints(second);
  if (positionsEqual(a.end, b.start)) return concatLines(first, second);
  if (positionsEqual(a.end, b.end)) return concatLines(first, [...second].reverse());
  if (positionsEqual(a.start, b.end)) return concatLines(second, first);
  if (positionsEqual(a.start, b.start)) return concatLines([...second].reverse(), first);
  return null;
}

function runMergeLines(features: VectorFeature[]): GeometryOpResult {
  if (features.length < 2) {
    return { ok: false, code: "geometry.merge.count", detail: "Merge requires at least two lines." };
  }
  if (!features.every((feature) => isLineGeometry(feature.geometry))) {
    return { ok: false, code: "geometry.merge.type", detail: "Merge requires LineString features." };
  }
  let coordinates = (features[0].geometry as LineString).coordinates.map((entry) => roundPosition(entry));
  const consumed = new Set<number>([0]);
  while (consumed.size < features.length) {
    let joined = false;
    for (let index = 1; index < features.length; index += 1) {
      if (consumed.has(index)) continue;
      const candidate = (features[index].geometry as LineString).coordinates;
      const next = tryJoin(coordinates, candidate);
      if (!next || next.length < 2) continue;
      coordinates = next;
      consumed.add(index);
      joined = true;
      break;
    }
    if (!joined) {
      return {
        ok: false,
        code: "geometry.merge.disconnected",
        detail: "Cannot merge these lines because their endpoints are not connected.",
      };
    }
  }
  const geometry: VectorFeature["geometry"] = { type: "LineString", coordinates };
  const invalid = validateResult(geometry);
  if (invalid) return invalid;
  return {
    ok: true,
    features: [resultFeature(features[0], geometry, features[0].id)],
    removedIds: features.map((feature) => feature.id),
  };
}

async function yieldForCancel(signal?: AbortSignal): Promise<GeometryOpResult | null> {
  if (signal?.aborted) return cancelledGeometryResult();
  await new Promise<void>((resolve) => setTimeout(resolve, 0));
  if (signal?.aborted) return cancelledGeometryResult();
  return null;
}

function combinePolygons(
  kind: "union" | "difference" | "intersection",
  left: Feature<Polygon | MultiPolygon>,
  right: Feature<Polygon | MultiPolygon>,
): Feature<Polygon | MultiPolygon> | null {
  const collection = featureCollection([left, right]);
  if (kind === "union") return union(collection) as Feature<Polygon | MultiPolygon> | null;
  if (kind === "difference") return difference(collection) as Feature<Polygon | MultiPolygon> | null;
  return intersect(collection) as Feature<Polygon | MultiPolygon> | null;
}

function finishPolygonOp(
  kind: "union" | "difference" | "intersection",
  source: VectorFeature,
  merged: Feature<Polygon | MultiPolygon> | null,
  removedIds: string[],
): GeometryOpResult {
  if (!merged) {
    if (kind === "union") return { ok: false, code: "geometry.union.failed", detail: "Union produced no geometry." };
    if (kind === "difference") {
      return { ok: false, code: "geometry.difference.empty", detail: "Difference removed all geometry." };
    }
    return { ok: false, code: "geometry.intersection.empty", detail: "Intersection is empty." };
  }
  const geometry = turfPolygonToDaena(merged);
  if (!geometry) {
    return {
      ok: false,
      code: `geometry.${kind}.invalid`,
      detail: `${operationLabel(kind)} result could not be represented.`,
    };
  }
  const invalid = validateResult(geometry);
  if (invalid) return invalid;
  const id = kind === "intersection" ? crypto.randomUUID() : source.id;
  return { ok: true, features: [resultFeature(source, geometry, id)], removedIds };
}

async function runBooleanAsync(
  kind: "union" | "difference" | "intersection",
  features: VectorFeature[],
  options: GeometryOpJobOptions,
): Promise<GeometryOpResult> {
  const turfFeatures = features
    .map((feature) => featureToTurf(feature))
    .filter((feature): feature is Feature<Polygon | MultiPolygon> => Boolean(feature));
  if (turfFeatures.length !== features.length || turfFeatures.length < 2) {
    return { ok: false, code: `geometry.${kind}.type`, detail: `${operationLabel(kind)} requires polygon features.` };
  }
  const total = turfFeatures.length - 1;
  let acc: Feature<Polygon | MultiPolygon> | null = turfFeatures[0];
  for (let index = 1; index < turfFeatures.length; index += 1) {
    options.onProgress?.({ completed: index - 1, total, label: operationLabel(kind) });
    const cancelled = await yieldForCancel(options.signal);
    if (cancelled) return cancelled;
    if (!acc) break;
    acc = combinePolygons(kind, acc, turfFeatures[index]);
  }
  if (options.signal?.aborted) return cancelledGeometryResult();
  options.onProgress?.({ completed: total, total, label: operationLabel(kind) });
  const removedIds =
    kind === "difference" ? [features[0].id] : kind === "intersection" ? [] : features.map((feature) => feature.id);
  return finishPolygonOp(kind, features[0], acc, removedIds);
}

export async function runGeometryOperationAsync(
  document: MapDocument,
  operation: GeometryOperationKind,
  selectedIds: readonly string[],
  params: GeometryOpParams = {},
  options: GeometryOpJobOptions = {},
): Promise<GeometryOpResult> {
  const cancelled = await yieldForCancel(options.signal);
  if (cancelled) return cancelled;
  if (operation === "union" || operation === "difference" || operation === "intersection") {
    const selection = validateSelection(document, selectedIds);
    if (!selection.ok) return selection;
    if (operation === "union") {
      if (selection.features.length < 2) {
        return { ok: false, code: "geometry.union.count", detail: "Union requires at least two polygon features." };
      }
      if (!selection.features.every((feature) => isPolygonGeometry(feature.geometry))) {
        return { ok: false, code: "geometry.union.type", detail: "Union requires polygon features." };
      }
    } else if (operation === "difference") {
      if (selection.features.length < 2) {
        return {
          ok: false,
          code: "geometry.difference.count",
          detail: "Difference requires a base polygon and a cutter.",
        };
      }
      if (!selection.features.every((feature) => isPolygonGeometry(feature.geometry))) {
        return { ok: false, code: "geometry.difference.type", detail: "Difference requires polygon features." };
      }
    } else if (selection.features.length < 2) {
      return {
        ok: false,
        code: "geometry.intersection.count",
        detail: "Intersection requires at least two polygon features.",
      };
    } else if (!selection.features.every((feature) => isPolygonGeometry(feature.geometry))) {
      return { ok: false, code: "geometry.intersection.type", detail: "Intersection requires polygon features." };
    }
    return runBooleanAsync(operation, selection.features, options);
  }
  options.onProgress?.({ completed: 0, total: 1, label: operationLabel(operation) });
  const cancelledBefore = await yieldForCancel(options.signal);
  if (cancelledBefore) return cancelledBefore;
  const result = runGeometryOperation(document, operation, selectedIds, params);
  if (!options.signal?.aborted) {
    options.onProgress?.({ completed: 1, total: 1, label: operationLabel(operation) });
  }
  return options.signal?.aborted ? cancelledGeometryResult() : result;
}
