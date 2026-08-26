export const VECTOR_PROVIDER = "daena-openlayers" as const;
export const BASE_LAYER_ID = "base" as const;
export const FREEHAND_RAW_POSITION_LIMIT = 8192;
export const FREEHAND_SIMPLIFIED_POSITION_LIMIT = 2048;
export const UNDO_STACK_SIZE = 50;

export type VectorKind = "land" | "lake" | "region" | "route" | "marker" | "custom";
export type VectorDrawMode =
  | "static"
  | "select"
  | "point"
  | "linestring"
  | "polygon"
  | "rectangle"
  | "freehand"
  | "measure-distance"
  | "measure-length"
  | "measure-area";

export type VectorFeatureProperties = {
  daena: {
    layerId: string;
    semanticType: VectorKind;
    name: string | null;
    style: Partial<MapStyleV2> | null;
    label: MapLabelV2 | null;
    custom: Record<string, string | number | boolean | null>;
  };
};

export type VectorFeature = {
  type: "Feature";
  id: string;
  properties: VectorFeatureProperties;
  geometry:
    | { type: "Point"; coordinates: number[] }
    | { type: "MultiPoint"; coordinates: number[][] }
    | { type: "LineString"; coordinates: number[][] }
    | { type: "MultiLineString"; coordinates: number[][][] }
    | { type: "Polygon"; coordinates: number[][][] }
    | { type: "MultiPolygon"; coordinates: number[][][][] };
};

export type VectorFeatureCollection = {
  type: "FeatureCollection";
  features: VectorFeature[];
};

export type VectorLayerStyle = MapStyleV2;

export type MapBlendMode = "normal" | "multiply" | "screen" | "overlay";

export type VectorLayerDefinition = {
  id: string;
  kind: "vector";
  name: string;
  order: number;
  defaultVisible: boolean;
  locked: boolean;
  opacity: number;
  blendMode: MapBlendMode;
  selector: Record<string, never>;
  style: VectorLayerStyle;
};

export type RasterLayerDefinition = {
  id: string;
  kind: "raster";
  name: string;
  order: number;
  defaultVisible: boolean;
  locked: boolean;
  opacity: number;
  blendMode: MapBlendMode;
  rasterAssetId: string;
  selector: Record<string, never>;
  style: Record<string, never>;
};

export type MapLayerDefinition = VectorLayerDefinition | RasterLayerDefinition;

export const DEFAULT_VECTOR_LAYER_STYLE: VectorLayerStyle = {
  fill: "#8f6fd1",
  fillOpacity: 0.35,
  stroke: "#5e4893",
  strokeOpacity: 1,
  strokeWidth: 1.5,
  strokeDash: [],
  pointRadius: 5,
  icon: null,
  iconSize: 20,
  label: {
    source: "name",
    text: null,
    size: 12,
    color: "#f7f0e5",
    haloColor: "#0d1b2a",
    haloWidth: 3,
    placement: "point",
    offset: [0, -14],
    rotation: 0,
    minZoom: null,
    maxZoom: null,
  },
};

export function featureLayerId(feature: { properties: VectorFeatureProperties }): string {
  return feature.properties.daena.layerId;
}

export function featureSemanticType(feature: { properties: VectorFeatureProperties }): VectorKind {
  return feature.properties.daena.semanticType;
}

export function featureName(feature: { properties: VectorFeatureProperties }): string | null {
  return feature.properties.daena.name;
}

export function isVectorLayer(layer: MapLayerDefinition): layer is VectorLayerDefinition {
  return layer.kind === "vector";
}

export function isRasterLayer(layer: MapLayerDefinition): layer is RasterLayerDefinition {
  return layer.kind === "raster";
}

export function vectorLayers(layers: readonly MapLayerDefinition[]): VectorLayerDefinition[] {
  return layers.filter(isVectorLayer);
}

export function layerAcceptsEdits(layer: MapLayerDefinition | undefined): layer is VectorLayerDefinition {
  return Boolean(
    layer && layer.kind === "vector" && layer.defaultVisible && !layer.locked && layer.id !== BASE_LAYER_ID,
  );
}

export function daenaProperties(
  layerId: string,
  semanticType: VectorKind,
  name: string | null = null,
): VectorFeatureProperties {
  return {
    daena: {
      layerId,
      semanticType,
      name,
      style: null,
      label: null,
      custom: {},
    },
  };
}
import type { MapLabelV2, MapStyleV2 } from "../../../../packages/plugin-sdk/src/maps";
