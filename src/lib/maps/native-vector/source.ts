import {
  DEFAULT_VECTOR_LAYER_STYLE,
  daenaProperties,
  featureLayerId,
  isVectorLayer,
  type MapBlendMode,
  type MapLayerDefinition,
  type RasterLayerDefinition,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorFeatureProperties,
  type VectorKind,
  type VectorLayerDefinition,
  type VectorLayerStyle,
} from "./types.ts";
import type { MapLabelV2, MapStyleV2 } from "../../../../packages/plugin-sdk/src/maps.ts";

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
  return collection.features.filter((feature) => featureLayerId(feature) === layerId).length;
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
  if (type === "MultiPoint" && coordinateRingsValid(coordinates, 1)) {
    return { type, coordinates: coordinates as number[][] };
  }
  if (type === "LineString" && coordinateRingsValid(coordinates, 1)) {
    return { type, coordinates: coordinates as number[][] };
  }
  if (type === "MultiLineString" && coordinateRingsValid(coordinates, 2)) {
    return { type, coordinates: coordinates as number[][][] };
  }
  if (type === "Polygon" && coordinateRingsValid(coordinates, 2)) {
    return { type, coordinates: coordinates as number[][][] };
  }
  if (type === "MultiPolygon" && coordinateRingsValid(coordinates, 3)) {
    return { type, coordinates: coordinates as number[][][][] };
  }
  return null;
}

function asCustom(value: unknown): Record<string, string | number | boolean | null> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const out: Record<string, string | number | boolean | null> = {};
  for (const [key, entry] of Object.entries(value as Record<string, unknown>)) {
    if (entry === null || typeof entry === "string" || typeof entry === "number" || typeof entry === "boolean") {
      out[key] = entry;
    }
  }
  return out;
}

function asLabel(value: unknown): MapLabelV2 | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const label = value as Partial<MapLabelV2>;
  if (
    (label.source !== "name" && label.source !== "explicit") ||
    (label.placement !== "point" && label.placement !== "line" && label.placement !== "interior") ||
    !Array.isArray(label.offset) ||
    label.offset.length !== 2
  ) {
    return null;
  }
  return {
    source: label.source,
    text: typeof label.text === "string" ? label.text : null,
    size: Number(label.size),
    color: String(label.color ?? "#f7f0e5"),
    haloColor: String(label.haloColor ?? "#0d1b2a"),
    haloWidth: Number(label.haloWidth),
    placement: label.placement,
    offset: [Number(label.offset[0]), Number(label.offset[1])],
    rotation: Number(label.rotation),
    minZoom: typeof label.minZoom === "number" ? label.minZoom : null,
    maxZoom: typeof label.maxZoom === "number" ? label.maxZoom : null,
  };
}

function asPartialStyle(value: unknown): Partial<MapStyleV2> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const raw = value as Record<string, unknown>;
  const style: Partial<MapStyleV2> = {};
  if (typeof raw.fill === "string") style.fill = raw.fill;
  if (typeof raw.fillOpacity === "number") style.fillOpacity = raw.fillOpacity;
  if (typeof raw.stroke === "string") style.stroke = raw.stroke;
  if (typeof raw.strokeOpacity === "number") style.strokeOpacity = raw.strokeOpacity;
  if (typeof raw.strokeWidth === "number") style.strokeWidth = raw.strokeWidth;
  if (Array.isArray(raw.strokeDash) && raw.strokeDash.every((entry) => typeof entry === "number")) {
    style.strokeDash = raw.strokeDash as number[];
  }
  if (typeof raw.pointRadius === "number") style.pointRadius = raw.pointRadius;
  if (typeof raw.icon === "string" || raw.icon === null) style.icon = raw.icon;
  if (typeof raw.iconSize === "number") style.iconSize = raw.iconSize;
  const label = asLabel(raw.label);
  if (label) style.label = label;
  return style;
}

function asDaenaProperties(value: unknown, fallbackLayerId: string, lenient: boolean): VectorFeatureProperties | null {
  if (!value || typeof value !== "object") return null;
  const properties = value as Record<string, unknown>;
  const daena = properties.daena;
  if (daena && typeof daena === "object" && !Array.isArray(daena)) {
    const nested = daena as Record<string, unknown>;
    return {
      daena: {
        layerId: typeof nested.layerId === "string" ? nested.layerId : fallbackLayerId,
        semanticType: asKind(nested.semanticType),
        name: typeof nested.name === "string" ? nested.name : null,
        style: asPartialStyle(nested.style),
        label: asLabel(nested.label),
        custom: asCustom(nested.custom),
      },
    };
  }
  if (!lenient) return null;
  if (typeof properties.daenaLayerId === "string" || typeof properties.kind === "string") {
    return daenaProperties(
      typeof properties.daenaLayerId === "string" ? properties.daenaLayerId : fallbackLayerId,
      asKind(properties.kind),
      typeof properties.name === "string" ? properties.name : null,
    );
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
        `vector.geometry.invalid: ${path}/geometry: unsupported or malformed geometry (expected Point, MultiPoint, LineString, MultiLineString, Polygon, or MultiPolygon with finite coordinates)`,
      );
    }
    const properties = asDaenaProperties(feature.properties, "base", lenient);
    if (!properties) {
      if (lenient) {
        onSkipped?.(path, "missing properties.daena");
        continue;
      }
      throw new Error(
        `vector.source.unsupported-version: ${path}/properties: flat feature properties are unsupported; use properties.daena`,
      );
    }
    features.push({
      type: "Feature",
      id: String(feature.id),
      properties,
      geometry,
    });
  }
  return { type: "FeatureCollection", features };
}

function asStyle(value: unknown): VectorLayerStyle {
  const style = asPartialStyle(value) ?? {};
  return {
    ...DEFAULT_VECTOR_LAYER_STYLE,
    ...style,
    strokeDash: style.strokeDash ?? DEFAULT_VECTOR_LAYER_STYLE.strokeDash,
    label: style.label ?? DEFAULT_VECTOR_LAYER_STYLE.label,
  };
}

function asBlendMode(value: unknown): MapBlendMode {
  return value === "multiply" || value === "screen" || value === "overlay" ? value : "normal";
}

function asOpacity(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 1;
}

function parseVectorLayer(layer: Record<string, unknown>): VectorLayerDefinition | null {
  if (typeof layer.id !== "string") return null;
  return {
    id: layer.id,
    kind: "vector",
    name: String(layer.name ?? "Layer"),
    order: Number(layer.order ?? 0),
    defaultVisible: layer.defaultVisible !== false,
    locked: layer.locked === true,
    opacity: asOpacity(layer.opacity),
    blendMode: asBlendMode(layer.blendMode),
    selector: {},
    style: asStyle(layer.style),
  };
}

function parseRasterLayer(layer: Record<string, unknown>): RasterLayerDefinition | null {
  if (typeof layer.id !== "string" || typeof layer.rasterAssetId !== "string") return null;
  return {
    id: layer.id,
    kind: "raster",
    name: String(layer.name ?? "Raster"),
    order: Number(layer.order ?? 0),
    defaultVisible: layer.defaultVisible !== false,
    locked: layer.locked === true,
    opacity: asOpacity(layer.opacity),
    blendMode: asBlendMode(layer.blendMode),
    rasterAssetId: layer.rasterAssetId,
    selector: {},
    style: {},
  };
}

export function parseVectorLayers(value: unknown): MapLayerDefinition[] {
  const layers = Array.isArray((value as { layers?: unknown[] } | undefined)?.layers)
    ? ((value as { layers: Array<Record<string, unknown>> }).layers ?? [])
    : [];
  const parsed: MapLayerDefinition[] = [];
  for (const layer of layers) {
    if (layer.kind === "raster") {
      const raster = parseRasterLayer(layer);
      if (raster) parsed.push(raster);
      continue;
    }
    if (layer.kind !== "vector") continue;
    const vector = parseVectorLayer(layer);
    if (vector) parsed.push(vector);
  }
  return parsed;
}

export function layerFromField(
  layers: { layers?: Array<Record<string, unknown>> } | undefined,
  layerId: string,
): VectorLayerDefinition | null {
  const found = parseVectorLayers(layers).find((layer) => layer.id === layerId);
  return found && isVectorLayer(found) ? found : null;
}

export { daenaProperties };
