import {
  DEFAULT_VECTOR_LAYER_STYLE,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorKind,
  type VectorLayerDefinition,
  type VectorLayerStyle,
} from "./types";

const KINDS: readonly VectorKind[] = ["land", "lake", "region", "route", "marker", "custom"];

export function emptyCollection(): VectorFeatureCollection {
  return { type: "FeatureCollection", features: [] };
}

export function collectionBytes(collection: VectorFeatureCollection): Uint8Array {
  return new TextEncoder().encode(JSON.stringify({ type: "FeatureCollection", features: collection.features }));
}

export async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const copy = bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength ? bytes : bytes.slice();
  const hash = await crypto.subtle.digest("SHA-256", copy.buffer as ArrayBuffer);
  return `sha256:${Array.from(new Uint8Array(hash), (value) => value.toString(16).padStart(2, "0")).join("")}`;
}

export function isRevisionConflict(message: string) {
  return message.toLowerCase().includes("revision conflict");
}

export function featureCountForLayer(collection: VectorFeatureCollection, layerId: string) {
  return collection.features.filter((feature) => feature.properties.daenaLayerId === layerId).length;
}

function asKind(value: unknown): VectorKind {
  return typeof value === "string" && KINDS.includes(value as VectorKind) ? (value as VectorKind) : "custom";
}

function asPosition(value: unknown): value is number[] {
  return (
    Array.isArray(value) &&
    value.length >= 2 &&
    value.every((entry) => typeof entry === "number" && Number.isFinite(entry))
  );
}

function coordinateRingsValid(coordinates: unknown, depth: number): boolean {
  if (!Array.isArray(coordinates)) return false;
  if (depth === 0) return asPosition(coordinates);
  if (coordinates.length === 0) return false;
  return coordinates.every((item) => coordinateRingsValid(item, depth - 1));
}

function asGeometry(value: unknown): VectorFeature["geometry"] | null {
  if (!value || typeof value !== "object" || !("type" in value) || !("coordinates" in value)) return null;
  const type = (value as { type: unknown }).type;
  const coordinates = (value as { coordinates: unknown }).coordinates;
  if (type === "Point" && coordinateRingsValid(coordinates, 0)) return { type, coordinates: coordinates as number[] };
  if (type === "LineString" && coordinateRingsValid(coordinates, 1)) {
    return { type, coordinates: coordinates as number[][] };
  }
  if (type === "Polygon" && coordinateRingsValid(coordinates, 2)) {
    return { type, coordinates: coordinates as number[][][] };
  }
  if (type === "MultiPolygon" && coordinateRingsValid(coordinates, 3)) {
    return { type, coordinates: coordinates as number[][][][] };
  }
  return null;
}

export interface VectorCollectionParseOptions {
  /** When true, malformed features are skipped instead of failing the whole
   *  collection. Used for derived physical overlays whose degenerate cells can
   *  emit empty rings/polygons; canonical authored sources stay strict so data
   *  loss is surfaced rather than silently dropped. Skipped features (if any)
   *  are reported through `onSkipped`. */
  lenient?: boolean;
  onSkipped?: (path: string, detail: string) => void;
}

export function parseVectorCollection(
  bytes: number[] | Uint8Array,
  options: VectorCollectionParseOptions = {},
): VectorFeatureCollection {
  const { lenient = false, onSkipped } = options;
  const text = new TextDecoder().decode(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes));
  const parsed = JSON.parse(text) as { type?: string; features?: unknown[] };
  if (parsed?.type !== "FeatureCollection" || !Array.isArray(parsed.features)) {
    throw new Error("vector.source.invalid: $: source is not a FeatureCollection");
  }
  const features: VectorFeature[] = [];
  for (let index = 0; index < parsed.features.length; index++) {
    const path = `features/${index}`;
    const item = parsed.features[index];
    if (!item || typeof item !== "object") {
      if (lenient) {
        onSkipped?.(path, "feature is not an object");
        continue;
      }
      throw new Error(`vector.geometry.invalid: ${path}: feature is not an object`);
    }
    const feature = item as { id?: unknown; properties?: Record<string, unknown>; geometry?: unknown };
    if (typeof feature.id !== "string" && typeof feature.id !== "number") {
      if (lenient) {
        onSkipped?.(path, "feature id is required");
        continue;
      }
      throw new Error(`vector.geometry.invalid: ${path}/id: feature id is required`);
    }
    const geometry = asGeometry(feature.geometry);
    if (!geometry) {
      if (lenient) {
        onSkipped?.(path, "unsupported or malformed geometry");
        continue;
      }
      throw new Error(
        `vector.geometry.invalid: ${path}/geometry: unsupported or malformed geometry (expected Point, LineString, Polygon, or MultiPolygon with finite coordinates)`,
      );
    }
    const properties = feature.properties ?? {};
    features.push({
      type: "Feature",
      id: String(feature.id),
      properties: {
        daenaLayerId: typeof properties.daenaLayerId === "string" ? properties.daenaLayerId : "base",
        kind: asKind(properties.kind),
        name: typeof properties.name === "string" ? properties.name : null,
      },
      geometry,
    });
  }
  return { type: "FeatureCollection", features };
}

function asStyle(value: unknown): VectorLayerStyle {
  const style = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  return {
    fill: typeof style.fill === "string" ? style.fill : DEFAULT_VECTOR_LAYER_STYLE.fill,
    fillOpacity: typeof style.fillOpacity === "number" ? style.fillOpacity : DEFAULT_VECTOR_LAYER_STYLE.fillOpacity,
    stroke: typeof style.stroke === "string" ? style.stroke : DEFAULT_VECTOR_LAYER_STYLE.stroke,
    strokeWidth: typeof style.strokeWidth === "number" ? style.strokeWidth : DEFAULT_VECTOR_LAYER_STYLE.strokeWidth,
    pointRadius: typeof style.pointRadius === "number" ? style.pointRadius : DEFAULT_VECTOR_LAYER_STYLE.pointRadius,
  };
}

export function parseVectorLayers(value: unknown): VectorLayerDefinition[] {
  const layers = Array.isArray((value as { layers?: unknown[] } | undefined)?.layers)
    ? ((value as { layers: Array<Record<string, unknown>> }).layers ?? [])
    : [];
  const parsed: VectorLayerDefinition[] = [];
  for (const layer of layers) {
    if (layer.kind !== "vector" || typeof layer.id !== "string") continue;
    parsed.push({
      id: layer.id,
      kind: "vector",
      name: String(layer.name ?? "Layer"),
      order: Number(layer.order ?? 0),
      defaultVisible: layer.defaultVisible !== false,
      locked: layer.locked === true,
      selector: {},
      style: asStyle(layer.style),
    });
  }
  return parsed;
}

export function layerFromField(
  layers: { layers?: Array<Record<string, unknown>> } | undefined,
  layerId: string,
): VectorLayerDefinition | null {
  const found = (layers?.layers ?? []).find((layer) => layer.id === layerId);
  if (!found || found.kind !== "vector" || typeof found.id !== "string") return null;
  return parseVectorLayers({ layers: [found] })[0] ?? null;
}
