import buffer from "@turf/buffer";
import difference from "@turf/difference";
import {
  featureCollection,
  lineString,
  multiPolygon,
  point,
  polygon,
  type Feature,
  type GeoJsonProperties,
  type LineString,
  type MultiPolygon,
  type Point,
  type Polygon,
  type Position,
} from "@turf/helpers";
import intersect from "@turf/intersect";
import lineSplit from "@turf/line-split";
import simplify from "@turf/simplify";
import union from "@turf/union";
import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps.ts";
import { VECTOR_MAX_FEATURE_POSITIONS } from "../../../../packages/plugin-sdk/src/maps.ts";
import { coordinateSpaceFromDescriptor } from "./coordinate-space.ts";
import { closeLineStringAsPolygon, geometryPositionCount } from "../native-vector/geometry.ts";
import {
  daenaProperties,
  featureLayerId,
  featureSemanticType,
  layerAcceptsEdits,
  type VectorFeature,
} from "../native-vector/types.ts";
import { findLayer, type MapDocument } from "./model.ts";

const MICRO_SCALE = 1_000_000;

export type GeometryOperationKind =
  | "union"
  | "difference"
  | "intersection"
  | "split"
  | "buffer"
  | "simplify";

export type GeometryOpParams = {
  bufferDistance?: number;
  simplifyTolerance?: number;
};

export type GeometryOpResult =
  | { ok: true; features: VectorFeature[]; removedIds: string[] }
  | { ok: false; code: string; detail: string };

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

function featureToTurf(feature: VectorFeature): Feature<Polygon | LineString | Point> | null {
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

function resultFeature(source: VectorFeature, geometry: VectorFeature["geometry"], id?: string): VectorFeature {
  return {
    type: "Feature",
    id: id ?? crypto.randomUUID(),
    properties: daenaProperties(
      featureLayerId(source),
      featureSemanticType(source),
      source.properties.daena.name,
    ),
    geometry,
  };
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

function isPolygonGeometry(geometry: VectorFeature["geometry"]): boolean {
  return geometry.type === "Polygon" || geometry.type === "MultiPolygon";
}

function isLineGeometry(geometry: VectorFeature["geometry"]): boolean {
  return geometry.type === "LineString";
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
  const error = validateGeometry(geometry);
  if (error) return { ok: false, code: "vector.geometry.invalid", detail: error };
  return {
    ok: true,
    features: [resultFeature(features[0], geometry)],
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
  const error = validateGeometry(geometry);
  if (error) return { ok: false, code: "vector.geometry.invalid", detail: error };
  return {
    ok: true,
    features: [resultFeature(features[0], geometry)],
    removedIds: features.map((feature) => feature.id),
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
    return { ok: false, code: "geometry.intersection.invalid", detail: "Intersection result could not be represented." };
  }
  const error = validateGeometry(geometry);
  if (error) return { ok: false, code: "vector.geometry.invalid", detail: error };
  return {
    ok: true,
    features: [resultFeature(features[0], geometry)],
    removedIds: features.map((feature) => feature.id),
  };
}

function runSplit(features: VectorFeature[]): GeometryOpResult {
  if (features.length !== 2) {
    return { ok: false, code: "geometry.split.count", detail: "Split requires one line and one cutter feature." };
  }
  const lineFeature = features.find((feature) => isLineGeometry(feature.geometry));
  const cutterFeature = features.find((feature) => feature !== lineFeature);
  if (!lineFeature || !cutterFeature) {
    return { ok: false, code: "geometry.split.type", detail: "Split requires a LineString and a cutter." };
  }
  const line = featureToTurf(lineFeature);
  if (!line || line.geometry.type !== "LineString") {
    return { ok: false, code: "geometry.split.line", detail: "Split line must be a LineString." };
  }
  let splitter: Feature<LineString | Polygon> | null = null;
  if (isLineGeometry(cutterFeature.geometry)) {
    splitter = featureToTurf(cutterFeature) as Feature<LineString>;
  } else if (isPolygonGeometry(cutterFeature.geometry)) {
    splitter = featureToTurf(cutterFeature) as Feature<Polygon | MultiPolygon>;
  }
  if (!splitter) {
    return { ok: false, code: "geometry.split.cutter", detail: "Cutter must be a line or polygon." };
  }
  const split = lineSplit(line, splitter);
  const parts = split.features
    .map((part) => turfLineToDaena(part as Feature<LineString>))
    .filter((geometry): geometry is VectorFeature["geometry"] => Boolean(geometry));
  if (parts.length < 2) {
    return { ok: false, code: "geometry.split.noop", detail: "Line was not split." };
  }
  for (const geometry of parts) {
    const error = validateGeometry(geometry);
    if (error) return { ok: false, code: "vector.geometry.invalid", detail: error };
  }
  return {
    ok: true,
    features: parts.map((geometry) => resultFeature(lineFeature, geometry)),
    removedIds: [lineFeature.id],
  };
}

/** Geographic maps buffer in geodesic metres; planar spaces buffer in authored coordinate units. */
function turfBufferDistance(space: MapCoordinateSpace, distance: number): { distance: number; units: "meters" | "degrees" } {
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
  const error = validateGeometry(geometry);
  if (error) return { ok: false, code: "vector.geometry.invalid", detail: error };
  return {
    ok: true,
    features: [resultFeature(source, geometry)],
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
    const error = validateGeometry(next);
    if (error) return { ok: false, code: "vector.geometry.invalid", detail: error };
    return { ok: true, features: [resultFeature(source, next)], removedIds: [source.id] };
  }
  if (geometry.type === "Polygon") {
    const turfFeature = polygon(geometry.coordinates as Position[][]);
    const simplified = simplify(turfFeature, { tolerance, highQuality: true }) as Feature<Polygon>;
    const next = turfPolygonToDaena(simplified);
    if (!next) return { ok: false, code: "geometry.simplify.invalid", detail: "Simplify produced invalid geometry." };
    const error = validateGeometry(next);
    if (error) return { ok: false, code: "vector.geometry.invalid", detail: error };
    return { ok: true, features: [resultFeature(source, next)], removedIds: [source.id] };
  }
  return { ok: false, code: "geometry.simplify.type", detail: "Simplify supports lines and polygons only." };
}

export function operationLabel(kind: GeometryOperationKind): string {
  switch (kind) {
    case "union":
      return "Union";
    case "difference":
      return "Difference";
    case "intersection":
      return "Intersection";
    case "split":
      return "Split line";
    case "buffer":
      return "Buffer";
    case "simplify":
      return "Simplify";
  }
}

export function canRunOperation(
  operation: GeometryOperationKind,
  features: readonly VectorFeature[],
): boolean {
  if (features.length === 0) return false;
  switch (operation) {
    case "union":
      return features.length >= 2 && features.every((feature) => isPolygonGeometry(feature.geometry));
    case "difference":
    case "intersection":
      return features.length >= 2 && features.every((feature) => isPolygonGeometry(feature.geometry));
    case "split":
      return (
        features.length === 2 &&
        features.some((feature) => isLineGeometry(feature.geometry)) &&
        features.some((feature) => isPolygonGeometry(feature.geometry) || isLineGeometry(feature.geometry))
      );
    case "buffer":
    case "simplify":
      return features.length === 1;
  }
}
