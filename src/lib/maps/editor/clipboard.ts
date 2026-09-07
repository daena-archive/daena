import { cloneCollection, offsetGeometry } from "../native-vector/feature-utils.ts";
import {
  featureLayerId,
  layerIsSelectable,
  type MapLayerDefinition,
  type VectorFeature,
} from "../native-vector/types.ts";

export const MAP_FEATURE_CLIPBOARD_KIND = "daena-map-features-v1";

export type MapFeatureClipboard = {
  kind: typeof MAP_FEATURE_CLIPBOARD_KIND;
  features: VectorFeature[];
};

export function encodeFeatureClipboard(features: readonly VectorFeature[]): string {
  return JSON.stringify({
    kind: MAP_FEATURE_CLIPBOARD_KIND,
    features: cloneCollection({ type: "FeatureCollection", features: [...features] }).features,
  } satisfies MapFeatureClipboard);
}

const GEOMETRY_TYPES = new Set(["Point", "MultiPoint", "LineString", "MultiLineString", "Polygon", "MultiPolygon"]);

export function decodeFeatureClipboard(text: string): VectorFeature[] | null {
  try {
    const parsed = JSON.parse(text) as Partial<MapFeatureClipboard>;
    if (parsed.kind !== MAP_FEATURE_CLIPBOARD_KIND || !Array.isArray(parsed.features)) return null;
    const features = parsed.features.filter((feature): feature is VectorFeature => {
      if (!feature || typeof feature !== "object") return false;
      if (feature.type !== "Feature" || typeof feature.id !== "string") return false;
      const geometry = feature.geometry as { type?: string } | null;
      if (!geometry || !GEOMETRY_TYPES.has(geometry.type ?? "")) return false;
      return Boolean(feature.properties?.daena && typeof feature.properties.daena === "object");
    });
    return features.length > 0 ? features : null;
  } catch {
    return null;
  }
}

export function copyFeaturesForPaste(
  features: readonly VectorFeature[],
  offsetX: number,
  offsetY: number,
  layerId: string,
): VectorFeature[] {
  return cloneCollection({ type: "FeatureCollection", features: [...features] }).features.map((feature) => ({
    ...feature,
    id: crypto.randomUUID(),
    properties: {
      daena: {
        ...feature.properties.daena,
        layerId,
      },
    },
    geometry: offsetGeometry(feature.geometry, offsetX, offsetY),
  }));
}

export function selectableFeatureIds(
  features: readonly VectorFeature[],
  layers: readonly MapLayerDefinition[],
  viewMode: boolean,
): string[] {
  return features
    .filter((feature) =>
      layerIsSelectable(
        layers.find((layer) => layer.id === featureLayerId(feature)),
        { viewMode },
      ),
    )
    .map((feature) => feature.id);
}
