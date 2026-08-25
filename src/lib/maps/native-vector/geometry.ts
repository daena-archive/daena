import {
  FREEHAND_RAW_POSITION_LIMIT,
  FREEHAND_SIMPLIFIED_POSITION_LIMIT,
  type VectorFeature,
  type VectorKind,
} from "./types";

export function geometryPositionCount(geometry: VectorFeature["geometry"]): number {
  switch (geometry.type) {
    case "Point":
      return 1;
    case "LineString":
      return geometry.coordinates.length;
    case "Polygon":
      return geometry.coordinates.reduce((sum, ring) => sum + ring.length, 0);
    case "MultiPolygon":
      return geometry.coordinates.reduce(
        (sum, polygon) => sum + polygon.reduce((inner, ring) => inner + ring.length, 0),
        0,
      );
  }
}

export function distinctPositions(positions: number[][]): number[][] {
  const unique: number[][] = [];
  for (const position of positions) {
    const previous = unique[unique.length - 1];
    if (!previous || previous[0] !== position[0] || previous[1] !== position[1]) unique.push(position);
  }
  return unique;
}

export function closeLineStringAsPolygon(coordinates: number[][]): number[][] | null {
  const unique = distinctPositions(coordinates);
  if (unique.length < 3) return null;
  const first = unique[0];
  const last = unique[unique.length - 1];
  if (first[0] !== last[0] || first[1] !== last[1]) unique.push([...first]);
  return unique.length >= 4 ? unique : null;
}

function perpendicularDistanceSquared(point: number[], start: number[], end: number[]): number {
  const dx = end[0] - start[0];
  const dy = end[1] - start[1];
  if (dx === 0 && dy === 0) {
    const ox = point[0] - start[0];
    const oy = point[1] - start[1];
    return ox * ox + oy * oy;
  }
  const t = Math.max(0, Math.min(1, ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / (dx * dx + dy * dy)));
  const px = point[0] - (start[0] + t * dx);
  const py = point[1] - (start[1] + t * dy);
  return px * px + py * py;
}

export function simplifyRing(ring: number[][], maxPositions: number, tolerance: number): number[][] {
  const closed = closeLineStringAsPolygon(ring);
  if (!closed) return ring;
  const inner = closed.slice(0, -1);
  while (inner.length > 3) {
    const overBudget = inner.length + 1 > maxPositions;
    let removeAt = 1;
    let best = Infinity;
    for (let index = 1; index < inner.length - 1; index += 1) {
      const distance = perpendicularDistanceSquared(inner[index], inner[index - 1], inner[index + 1]);
      if (distance < best || (distance === best && index < removeAt)) {
        best = distance;
        removeAt = index;
      }
    }
    if (!overBudget && best >= tolerance) break;
    inner.splice(removeAt, 1);
  }
  return [...inner, [...inner[0]]];
}

export function simplifyFreehandGeometry(
  geometry: VectorFeature["geometry"],
  zoom: number,
): VectorFeature["geometry"] | { error: "vector.limit.exceeded" } | { error: "vector.geometry.invalid" } {
  if (geometryPositionCount(geometry) > FREEHAND_RAW_POSITION_LIMIT) return { error: "vector.limit.exceeded" };
  const tolerance = Math.pow(0.5, Math.max(0, zoom)) * 0.05;
  if (geometry.type === "LineString") {
    const ring = closeLineStringAsPolygon(geometry.coordinates);
    if (!ring) return { error: "vector.geometry.invalid" };
    return {
      type: "Polygon",
      coordinates: [simplifyRing(ring, FREEHAND_SIMPLIFIED_POSITION_LIMIT, tolerance)],
    };
  }
  if (geometry.type === "Polygon") {
    return {
      type: "Polygon",
      coordinates: geometry.coordinates.map((ring) =>
        simplifyRing(ring, FREEHAND_SIMPLIFIED_POSITION_LIMIT, tolerance),
      ),
    };
  }
  return { error: "vector.geometry.invalid" };
}

export function kindForDrawMode(mode: "point" | "linestring" | "polygon" | "rectangle" | "freehand"): VectorKind {
  if (mode === "point") return "marker";
  if (mode === "linestring") return "route";
  return "region";
}

export function drawModeForGeometry(geometry: VectorFeature["geometry"]): "point" | "linestring" | "polygon" {
  if (geometry.type === "Point" || geometry.type === "MultiPoint") return "point";
  if (geometry.type === "LineString" || geometry.type === "MultiLineString") return "linestring";
  return "polygon";
}
