import { featureLayerId, type VectorFeature, type VectorFeatureCollection } from "../native-vector/types.ts";

export type SpatialExtent = [number, number, number, number];

export type SpatialRecord = {
  id: string;
  layerId: string;
  extent: SpatialExtent;
};

export type SpatialIndex = {
  clear: () => void;
  replaceAll: (records: readonly SpatialRecord[]) => void;
  upsert: (record: SpatialRecord) => void;
  remove: (id: string) => void;
  query: (extent: SpatialExtent) => SpatialRecord[];
  size: () => number;
};

const DEFAULT_CELL = 32;

function cellCoord(value: number, cellSize: number): number {
  return Math.floor(value / cellSize);
}

function cellKey(col: number, row: number): string {
  return `${col}:${row}`;
}

export function extentsOverlap(left: SpatialExtent, right: SpatialExtent): boolean {
  return left[0] <= right[2] && left[2] >= right[0] && left[1] <= right[3] && left[3] >= right[1];
}

export function pointInExtent(x: number, y: number, extent: SpatialExtent): boolean {
  return x >= extent[0] && x <= extent[2] && y >= extent[1] && y <= extent[3];
}

function visitPositions(value: unknown, visit: (x: number, y: number) => void): void {
  if (!Array.isArray(value) || value.length === 0) return;
  if (typeof value[0] === "number" && typeof value[1] === "number") {
    visit(value[0], value[1]);
    return;
  }
  for (const entry of value) visitPositions(entry, visit);
}

export function extentOfGeometry(geometry: VectorFeature["geometry"]): SpatialExtent | null {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  visitPositions(geometry.coordinates, (x, y) => {
    if (x < minX) minX = x;
    if (y < minY) minY = y;
    if (x > maxX) maxX = x;
    if (y > maxY) maxY = y;
  });
  if (!Number.isFinite(minX) || !Number.isFinite(minY) || !Number.isFinite(maxX) || !Number.isFinite(maxY)) {
    return null;
  }
  return [minX, minY, maxX, maxY];
}

function segmentIntersectsExtent(x0: number, y0: number, x1: number, y1: number, extent: SpatialExtent): boolean {
  if (pointInExtent(x0, y0, extent) || pointInExtent(x1, y1, extent)) return true;
  let t0 = 0;
  let t1 = 1;
  const dx = x1 - x0;
  const dy = y1 - y0;
  const clips: Array<[number, number]> = [
    [-dx, x0 - extent[0]],
    [dx, extent[2] - x0],
    [-dy, y0 - extent[1]],
    [dy, extent[3] - y0],
  ];
  for (const [p, q] of clips) {
    if (p === 0) {
      if (q < 0) return false;
      continue;
    }
    const r = q / p;
    if (p < 0) {
      if (r > t1) return false;
      if (r > t0) t0 = r;
    } else {
      if (r < t0) return false;
      if (r < t1) t1 = r;
    }
  }
  return t0 <= t1;
}

function lineIntersectsExtent(line: number[][], extent: SpatialExtent): boolean {
  for (let index = 0; index < line.length - 1; index += 1) {
    const start = line[index];
    const end = line[index + 1];
    if (segmentIntersectsExtent(start[0], start[1], end[0], end[1], extent)) return true;
  }
  return false;
}

function pointInRing(x: number, y: number, ring: number[][]): boolean {
  let inside = false;
  for (let index = 0, previous = ring.length - 1; index < ring.length; previous = index, index += 1) {
    const current = ring[index];
    const prior = ring[previous];
    const intersects =
      current[1] > y !== prior[1] > y &&
      x < ((prior[0] - current[0]) * (y - current[1])) / (prior[1] - current[1] + Number.EPSILON) + current[0];
    if (intersects) inside = !inside;
  }
  return inside;
}

function polygonIntersectsExtent(rings: number[][][], extent: SpatialExtent): boolean {
  const exterior = rings[0];
  if (!exterior || exterior.length < 2) return false;
  if (lineIntersectsExtent(exterior, extent)) return true;
  for (const hole of rings.slice(1)) {
    if (lineIntersectsExtent(hole, extent)) return true;
  }
  const corners: Array<[number, number]> = [
    [extent[0], extent[1]],
    [extent[2], extent[1]],
    [extent[2], extent[3]],
    [extent[0], extent[3]],
  ];
  for (const [x, y] of corners) {
    if (!pointInRing(x, y, exterior)) continue;
    const inHole = rings.slice(1).some((hole) => pointInRing(x, y, hole));
    if (!inHole) return true;
  }
  return pointInExtent(exterior[0][0], exterior[0][1], extent);
}

export function geometryIntersectsExtent(geometry: VectorFeature["geometry"], extent: SpatialExtent): boolean {
  const bounds = extentOfGeometry(geometry);
  if (!bounds || !extentsOverlap(bounds, extent)) return false;
  switch (geometry.type) {
    case "Point":
      return pointInExtent(geometry.coordinates[0], geometry.coordinates[1], extent);
    case "MultiPoint":
      return geometry.coordinates.some((point) => pointInExtent(point[0], point[1], extent));
    case "LineString":
      return lineIntersectsExtent(geometry.coordinates, extent);
    case "MultiLineString":
      return geometry.coordinates.some((line) => lineIntersectsExtent(line, extent));
    case "Polygon":
      return polygonIntersectsExtent(geometry.coordinates, extent);
    case "MultiPolygon":
      return geometry.coordinates.some((polygon) => polygonIntersectsExtent(polygon, extent));
    default:
      return true;
  }
}

export function recordsFromCollection(collection: VectorFeatureCollection): SpatialRecord[] {
  const records: SpatialRecord[] = [];
  for (const feature of collection.features) {
    const extent = extentOfGeometry(feature.geometry);
    if (!extent) continue;
    records.push({ id: feature.id, layerId: featureLayerId(feature), extent });
  }
  return records;
}

export function createSpatialIndex(cellSize = DEFAULT_CELL): SpatialIndex {
  const size = cellSize > 0 ? cellSize : DEFAULT_CELL;
  const records = new Map<string, SpatialRecord>();
  const cells = new Map<string, Set<string>>();

  const unindex = (record: SpatialRecord) => {
    const minCol = cellCoord(record.extent[0], size);
    const maxCol = cellCoord(record.extent[2], size);
    const minRow = cellCoord(record.extent[1], size);
    const maxRow = cellCoord(record.extent[3], size);
    for (let col = minCol; col <= maxCol; col += 1) {
      for (let row = minRow; row <= maxRow; row += 1) {
        const key = cellKey(col, row);
        const bucket = cells.get(key);
        if (!bucket) continue;
        bucket.delete(record.id);
        if (bucket.size === 0) cells.delete(key);
      }
    }
  };

  const index = (record: SpatialRecord) => {
    const minCol = cellCoord(record.extent[0], size);
    const maxCol = cellCoord(record.extent[2], size);
    const minRow = cellCoord(record.extent[1], size);
    const maxRow = cellCoord(record.extent[3], size);
    for (let col = minCol; col <= maxCol; col += 1) {
      for (let row = minRow; row <= maxRow; row += 1) {
        const key = cellKey(col, row);
        const bucket = cells.get(key) ?? new Set<string>();
        bucket.add(record.id);
        cells.set(key, bucket);
      }
    }
  };

  return {
    clear() {
      records.clear();
      cells.clear();
    },
    replaceAll(next) {
      records.clear();
      cells.clear();
      for (const record of next) {
        records.set(record.id, record);
        index(record);
      }
    },
    upsert(record) {
      const previous = records.get(record.id);
      if (previous) unindex(previous);
      records.set(record.id, record);
      index(record);
    },
    remove(id) {
      const previous = records.get(id);
      if (!previous) return;
      unindex(previous);
      records.delete(id);
    },
    query(extent) {
      const minX = Math.min(extent[0], extent[2]);
      const minY = Math.min(extent[1], extent[3]);
      const maxX = Math.max(extent[0], extent[2]);
      const maxY = Math.max(extent[1], extent[3]);
      const minCol = cellCoord(minX, size);
      const maxCol = cellCoord(maxX, size);
      const minRow = cellCoord(minY, size);
      const maxRow = cellCoord(maxY, size);
      const seen = new Set<string>();
      const matches: SpatialRecord[] = [];
      for (let col = minCol; col <= maxCol; col += 1) {
        for (let row = minRow; row <= maxRow; row += 1) {
          const bucket = cells.get(cellKey(col, row));
          if (!bucket) continue;
          for (const id of bucket) {
            if (seen.has(id)) continue;
            seen.add(id);
            const record = records.get(id);
            if (record && extentsOverlap(record.extent, [minX, minY, maxX, maxY])) matches.push(record);
          }
        }
      }
      return matches;
    },
    size() {
      return records.size;
    },
  };
}
