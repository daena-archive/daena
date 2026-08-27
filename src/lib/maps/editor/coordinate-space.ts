import type { MapBackgroundRef, MapCoordinateSpace, MapDescriptor } from "../../../../packages/plugin-sdk/src/maps.ts";
import type { VectorFeatureCollection } from "../native-vector/types.ts";

export type Extent4 = [number, number, number, number];
export type Position = [number, number];

export const WORLD_EXTENT: Extent4 = [-180, -90, 180, 90];

export const DEFAULT_WORLD_SPACE: MapCoordinateSpace = {
  kind: "world",
  extent: WORLD_EXTENT,
  origin: "bottom-left",
  units: { id: "world-unit", label: "World units", metresPerUnit: null },
  wrapX: false,
};

/** Explicit full-world space for physical rasters; not an authored image CRS. */
export const PHYSICAL_COORDINATE_SPACE: MapCoordinateSpace = {
  kind: "world",
  extent: WORLD_EXTENT,
  origin: "bottom-left",
  units: { id: "world-unit", label: "World units", metresPerUnit: null },
  wrapX: false,
};

export type OpenLayersMapDescriptor = Extract<MapDescriptor, { provider: { id: "daena-openlayers" } }>;

export function extentOf(space: MapCoordinateSpace): Extent4 {
  return [space.extent[0], space.extent[1], space.extent[2], space.extent[3]];
}

export function wrapXOf(space: MapCoordinateSpace): boolean {
  return space.kind === "image" ? false : space.wrapX;
}

export function isImageSpace(space: MapCoordinateSpace): boolean {
  return space.kind === "image";
}

export function coordinateSpaceKey(space: MapCoordinateSpace): string {
  return JSON.stringify(space);
}

function flipY(y: number, space: MapCoordinateSpace): number {
  const [, minY, , maxY] = extentOf(space);
  return minY + maxY - y;
}

/** Authored image coordinates are top-left / Y-down; OpenLayers is Y-up. */
export function authoredToView(position: readonly number[], space: MapCoordinateSpace): Position {
  const x = position[0];
  const y = position[1];
  if (space.kind === "image") return [x, flipY(y, space)];
  return [x, y];
}

export function viewToAuthored(position: readonly number[], space: MapCoordinateSpace): Position {
  return authoredToView(position, space);
}

export function authoredExtentToViewExtent(extent: readonly number[], space: MapCoordinateSpace): Extent4 {
  const a = authoredToView([extent[0], extent[1]], space);
  const b = authoredToView([extent[2], extent[3]], space);
  return [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.max(a[0], b[0]), Math.max(a[1], b[1])];
}

export function authoredToNormalized(x: number, y: number, space: MapCoordinateSpace): Position {
  const [minX, minY, maxX, maxY] = extentOf(space);
  const width = Math.max(maxX - minX, Number.EPSILON);
  const height = Math.max(maxY - minY, Number.EPSILON);
  const nx = (x - minX) / width;
  const ny = space.kind === "image" ? (y - minY) / height : (maxY - y) / height;
  return [clamp01(nx), clamp01(ny)];
}

export function normalizedToAuthored(nx: number, ny: number, space: MapCoordinateSpace): Position {
  const [minX, minY, maxX, maxY] = extentOf(space);
  const width = maxX - minX;
  const height = maxY - minY;
  const x = minX + clamp01(nx) * width;
  const y = space.kind === "image" ? minY + clamp01(ny) * height : maxY - clamp01(ny) * height;
  return [x, y];
}

function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(1, value));
}

export function mapPositions<T>(geometry: T, transform: (position: number[]) => number[]): T {
  const walk = (value: unknown, depth: number): unknown => {
    if (depth === 0) {
      const position = value as number[];
      const next = transform(position);
      return position.length > 2 ? [...next, ...position.slice(2)] : next;
    }
    return (value as unknown[]).map((item) => walk(item, depth - 1));
  };
  if (!geometry || typeof geometry !== "object" || !("type" in geometry) || !("coordinates" in geometry)) {
    return geometry;
  }
  const typed = geometry as { type: string; coordinates: unknown };
  const depth =
    typed.type === "Point"
      ? 0
      : typed.type === "MultiPoint" || typed.type === "LineString"
        ? 1
        : typed.type === "MultiLineString" || typed.type === "Polygon"
          ? 2
          : typed.type === "MultiPolygon"
            ? 3
            : -1;
  if (depth < 0) return geometry;
  return { ...typed, coordinates: walk(typed.coordinates, depth) } as T;
}

export function transformCollection(
  collection: VectorFeatureCollection,
  transform: (position: number[]) => number[],
): VectorFeatureCollection {
  return {
    type: "FeatureCollection",
    features: collection.features.map((feature) => ({
      ...feature,
      geometry: mapPositions(feature.geometry, transform),
    })),
  };
}

export function flipYPosition(position: readonly number[], space: MapCoordinateSpace): Position {
  return [position[0], flipY(position[1], space)];
}

export function flipYExtent(extent: readonly number[], space: MapCoordinateSpace): Extent4 {
  const a = flipYPosition([extent[0], extent[1]], space);
  const b = flipYPosition([extent[2], extent[3]], space);
  return [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.max(a[0], b[0]), Math.max(a[1], b[1])];
}

export function flipYCollection(
  collection: VectorFeatureCollection,
  space: MapCoordinateSpace,
): VectorFeatureCollection {
  return transformCollection(collection, (position) => flipYPosition(position, space));
}

export function flipYBackgrounds(
  backgrounds: readonly MapBackgroundRef[],
  space: MapCoordinateSpace,
): MapBackgroundRef[] {
  return backgrounds.map((background) => ({
    ...background,
    extent: flipYExtent(background.extent, space),
  }));
}

export function centerOfExtent(extent: readonly number[]): Position {
  return [(extent[0] + extent[2]) / 2, (extent[1] + extent[3]) / 2];
}

export function panDelta(space: MapCoordinateSpace, direction: { x: number; y: number }): Position {
  const [minX, minY, maxX, maxY] = extentOf(space);
  return [((maxX - minX) / 8) * direction.x, ((maxY - minY) / 8) * direction.y];
}

export function duplicateOffset(space: MapCoordinateSpace): Position {
  const [minX, minY, maxX, maxY] = extentOf(space);
  return [(maxX - minX) * 0.02, (maxY - minY) * -0.02];
}

export function isOpenLayersDescriptor(value: unknown): value is OpenLayersMapDescriptor {
  if (!value || typeof value !== "object") return false;
  const descriptor = value as { schemaVersion?: unknown; provider?: { id?: unknown }; coordinateSpace?: unknown };
  return (
    descriptor.schemaVersion === 1 &&
    descriptor.provider?.id === "daena-openlayers" &&
    descriptor.coordinateSpace !== undefined &&
    typeof descriptor.coordinateSpace === "object"
  );
}

export function coordinateSpaceFromDescriptor(descriptor: unknown): MapCoordinateSpace {
  if (!isOpenLayersDescriptor(descriptor)) return DEFAULT_WORLD_SPACE;
  return parseCoordinateSpace(descriptor.coordinateSpace) ?? DEFAULT_WORLD_SPACE;
}

export function parseCoordinateSpace(value: unknown): MapCoordinateSpace | null {
  if (!value || typeof value !== "object") return null;
  const space = value as {
    kind?: unknown;
    extent?: unknown;
    origin?: unknown;
    units?: unknown;
    wrapX?: unknown;
    projection?: unknown;
  };
  const extent = asExtent(space.extent);
  if (!extent) return null;
  if (space.kind === "image" && space.origin === "top-left" && space.units === "pixels") {
    return { kind: "image", extent, origin: "top-left", units: "pixels" };
  }
  if (space.kind === "world" && space.origin === "bottom-left" && space.units && typeof space.units === "object") {
    const units = space.units as { id?: unknown; label?: unknown; metresPerUnit?: unknown };
    if (typeof units.id !== "string" || typeof units.label !== "string") return null;
    return {
      kind: "world",
      extent,
      origin: "bottom-left",
      units: {
        id: units.id,
        label: units.label,
        metresPerUnit:
          typeof units.metresPerUnit === "number" && Number.isFinite(units.metresPerUnit) ? units.metresPerUnit : null,
      },
      wrapX: space.wrapX === true,
    };
  }
  if (space.kind === "geographic" && space.projection === "EPSG:4326") {
    return { kind: "geographic", projection: "EPSG:4326", extent, wrapX: space.wrapX === true };
  }
  return null;
}

export function backgroundsFromDescriptor(descriptor: unknown): MapBackgroundRef[] {
  if (!isOpenLayersDescriptor(descriptor) || !Array.isArray(descriptor.backgrounds)) return [];
  return descriptor.backgrounds.map((background) => ({
    ...background,
    extent: asExtent(background.extent) ?? extentOf(coordinateSpaceFromDescriptor(descriptor)),
  }));
}

export function defaultViewFromDescriptor(
  descriptor: unknown,
  space: MapCoordinateSpace,
): { center: Position; zoom: number; rotation: number } {
  const view =
    descriptor && typeof descriptor === "object"
      ? (descriptor as { defaultView?: { center?: unknown; zoom?: unknown; rotation?: unknown } }).defaultView
      : undefined;
  const extent = extentOf(space);
  const fallback = centerOfExtent(extent);
  const center = asPosition(view?.center) ?? fallback;
  return {
    center,
    zoom: typeof view?.zoom === "number" && Number.isFinite(view.zoom) ? view.zoom : 1,
    rotation: typeof view?.rotation === "number" && Number.isFinite(view.rotation) ? view.rotation : 0,
  };
}

export function asExtent(value: unknown): Extent4 | null {
  if (!Array.isArray(value) || value.length < 4) return null;
  const extent = value.slice(0, 4).map(Number) as Extent4;
  if (extent.some((entry) => !Number.isFinite(entry)) || extent[0] >= extent[2] || extent[1] >= extent[3]) return null;
  return extent;
}

function asPosition(value: unknown): Position | null {
  if (!Array.isArray(value) || value.length < 2) return null;
  const x = Number(value[0]);
  const y = Number(value[1]);
  if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
  return [x, y];
}

export function patchOpenLayersDescriptor(
  descriptor: unknown,
  patch: Partial<OpenLayersMapDescriptor> & Record<string, unknown>,
): unknown {
  if (!isOpenLayersDescriptor(descriptor)) return descriptor;
  return { ...descriptor, ...patch };
}
