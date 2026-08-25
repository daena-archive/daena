export const VECTOR_PROVIDER = "daena-openlayers" as const;
export const BASE_LAYER_ID = "base" as const;
export const FREEHAND_RAW_POSITION_LIMIT = 8192;
export const FREEHAND_SIMPLIFIED_POSITION_LIMIT = 2048;
export const UNDO_STACK_SIZE = 50;

export type VectorKind = "land" | "lake" | "region" | "route" | "marker" | "custom";
export type VectorDrawMode = "static" | "select" | "point" | "linestring" | "polygon" | "rectangle" | "freehand";

export type VectorFeatureProperties = {
  daena: {
    layerId: string;
    semanticType: VectorKind;
    name: string | null;
    style: Record<string, unknown> | null;
    label: Record<string, unknown> | null;
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

export type VectorLayerStyle = {
  fill: string;
  fillOpacity: number;
  stroke: string;
  strokeWidth: number;
  pointRadius: number;
};

export type VectorLayerDefinition = {
  id: string;
  kind: "vector";
  name: string;
  order: number;
  defaultVisible: boolean;
  locked: boolean;
  selector: Record<string, never>;
  style: VectorLayerStyle;
};

export const DEFAULT_VECTOR_LAYER_STYLE: VectorLayerStyle = {
  fill: "#8f6fd1",
  fillOpacity: 0.35,
  stroke: "#5e4893",
  strokeWidth: 1.5,
  pointRadius: 5,
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
