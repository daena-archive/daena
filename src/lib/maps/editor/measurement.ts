import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps.ts";
import { extentOf } from "./coordinate-space.ts";

const EARTH_RADIUS_METRES = 6_371_000;

export type MeasurementUnits = {
  id: string;
  label: string;
  length: string;
  area: string;
  metresPerUnit: number | null;
};

export function unitsForCoordinateSpace(space: MapCoordinateSpace): MeasurementUnits {
  if (space.kind === "image") {
    return { id: "pixels", label: "pixels", length: "px", area: "px²", metresPerUnit: null };
  }
  if (space.kind === "world") {
    const metres = space.units.metresPerUnit;
    return {
      id: space.units.id,
      label: space.units.label,
      length: metres === 1 ? "m" : space.units.id,
      area: metres === 1 ? "m²" : `${space.units.id}²`,
      metresPerUnit: metres,
    };
  }
  return { id: "geodesic", label: "metres (geodesic)", length: "m", area: "m²", metresPerUnit: 1 };
}

export function formatMeasurement(value: number, unit: string, digits = 2): string {
  if (!Number.isFinite(value)) return `— ${unit}`;
  const abs = Math.abs(value);
  const precision = abs >= 100 ? 1 : abs >= 10 ? digits : abs >= 1 ? digits : 3;
  return `${value.toFixed(precision)} ${unit}`;
}

function toRadians(degrees: number): number {
  return (degrees * Math.PI) / 180;
}

export function geodesicDistanceMetres(lon1: number, lat1: number, lon2: number, lat2: number): number {
  const phi1 = toRadians(lat1);
  const phi2 = toRadians(lat2);
  const dPhi = toRadians(lat2 - lat1);
  const dLambda = toRadians(lon2 - lon1);
  const a = Math.sin(dPhi / 2) ** 2 + Math.cos(phi1) * Math.cos(phi2) * Math.sin(dLambda / 2) ** 2;
  return 2 * EARTH_RADIUS_METRES * Math.atan2(Math.sqrt(a), Math.sqrt(Math.max(0, 1 - a)));
}

function planarDistance(ax: number, ay: number, bx: number, by: number): number {
  return Math.hypot(bx - ax, by - ay);
}

export function pathLength(points: ReadonlyArray<readonly number[]>, space: MapCoordinateSpace): number {
  let total = 0;
  for (let index = 1; index < points.length; index += 1) {
    const prev = points[index - 1];
    const next = points[index];
    total +=
      space.kind === "geographic"
        ? geodesicDistanceMetres(prev[0], prev[1], next[0], next[1])
        : planarDistance(prev[0], prev[1], next[0], next[1]);
  }
  return total;
}

export function polygonArea(ring: ReadonlyArray<readonly number[]>, space: MapCoordinateSpace): number {
  if (ring.length < 4) return 0;
  if (space.kind === "geographic") return Math.abs(sphericalPolygonAreaMetres2(ring));
  let sum = 0;
  for (let index = 0; index < ring.length - 1; index += 1) {
    const current = ring[index];
    const next = ring[index + 1];
    sum += current[0] * next[1] - next[0] * current[1];
  }
  return Math.abs(sum) / 2;
}

function sphericalPolygonAreaMetres2(ring: ReadonlyArray<readonly number[]>): number {
  if (ring.length < 4) return 0;
  let total = 0;
  for (let index = 0; index < ring.length - 1; index += 1) {
    const current = ring[index];
    const next = ring[index + 1];
    total += toRadians(next[0] - current[0]) * (2 + Math.sin(toRadians(current[1])) + Math.sin(toRadians(next[1])));
  }
  return (total * EARTH_RADIUS_METRES * EARTH_RADIUS_METRES) / 2;
}

export function pointDistance(a: readonly number[], b: readonly number[], space: MapCoordinateSpace): number {
  return space.kind === "geographic"
    ? geodesicDistanceMetres(a[0], a[1], b[0], b[1])
    : planarDistance(a[0], a[1], b[0], b[1]);
}

export function measurementSummary(space: MapCoordinateSpace): string {
  const units = unitsForCoordinateSpace(space);
  if (space.kind === "image") return `Units: ${units.label}`;
  if (space.kind === "world") {
    if (units.metresPerUnit == null) return `Units: ${units.label} (uncalibrated)`;
    return `Units: ${units.label} (${units.metresPerUnit} m / unit)`;
  }
  return "Units: geodesic metres";
}

export function extentSize(space: MapCoordinateSpace): { width: number; height: number } {
  const [minX, minY, maxX, maxY] = extentOf(space);
  return { width: maxX - minX, height: maxY - minY };
}

export function measureFeature(
  feature: { geometry: { type: string; coordinates: unknown } },
  space: MapCoordinateSpace,
): { length: number | null; area: number | null } {
  const { geometry } = feature;
  if (geometry.type === "LineString") {
    return { length: pathLength(geometry.coordinates as readonly number[][], space), area: null };
  }
  if (geometry.type === "MultiLineString") {
    const lines = geometry.coordinates as readonly number[][][];
    const total = lines.reduce((sum, line) => sum + pathLength(line, space), 0);
    return { length: total, area: null };
  }
  if (geometry.type === "Polygon") {
    const rings = geometry.coordinates as readonly number[][][];
    return { length: pathLength(rings[0] ?? [], space), area: polygonArea(rings[0] ?? [], space) };
  }
  if (geometry.type === "MultiPolygon") {
    const polygons = geometry.coordinates as readonly number[][][][];
    const area = polygons.reduce((sum, polygon) => sum + polygonArea(polygon[0] ?? [], space), 0);
    const length = polygons.reduce((sum, polygon) => sum + pathLength(polygon[0] ?? [], space), 0);
    return { length, area };
  }
  return { length: null, area: null };
}
