import type { VectorFeature } from "../native-vector/types.ts";

export type GeometryOperationKind =
  "union" | "difference" | "intersection" | "split" | "buffer" | "simplify" | "reverse" | "merge-lines";

export type GeometryOpParams = {
  bufferDistance?: number;
  simplifyTolerance?: number;
};

export type GeometryOpResult =
  { ok: true; features: VectorFeature[]; removedIds: string[] } | { ok: false; code: string; detail: string };

export type GeometryOpProgress = {
  completed: number;
  total: number;
  label: string;
};

export type GeometryOpJobOptions = {
  signal?: AbortSignal;
  onProgress?: (progress: GeometryOpProgress) => void;
};

export type GeometryPreview = {
  operation: GeometryOperationKind;
  inputFeatureIds: string[];
  previewFeatures: VectorFeature[];
  removedFeatureIds: string[];
  params: GeometryOpParams;
  label: string;
};

export const HEAVY_GEOMETRY_OPERATIONS = new Set<GeometryOperationKind>([
  "union",
  "difference",
  "intersection",
  "simplify",
]);

export function cancelledGeometryResult(): GeometryOpResult & { ok: false } {
  return { ok: false, code: "geometry.cancelled", detail: "Operation cancelled." };
}

export function isPolygonGeometry(geometry: VectorFeature["geometry"]): boolean {
  return geometry.type === "Polygon" || geometry.type === "MultiPolygon";
}

export function isLineGeometry(geometry: VectorFeature["geometry"]): boolean {
  return geometry.type === "LineString";
}

export function isReversibleLine(geometry: VectorFeature["geometry"]): boolean {
  return geometry.type === "LineString" || geometry.type === "MultiLineString";
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
      return "Split";
    case "buffer":
      return "Buffer";
    case "simplify":
      return "Simplify";
    case "reverse":
      return "Reverse line";
    case "merge-lines":
      return "Merge lines";
  }
}

export function canRunOperation(operation: GeometryOperationKind, features: readonly VectorFeature[]): boolean {
  if (features.length === 0) return false;
  switch (operation) {
    case "union":
    case "difference":
    case "intersection":
      return features.length >= 2 && features.every((feature) => isPolygonGeometry(feature.geometry));
    case "split": {
      if (features.length !== 2) return false;
      const [target, cutter] = features;
      if (target.geometry.type === "Polygon" && isLineGeometry(cutter.geometry)) return true;
      return isLineGeometry(target.geometry) && (isLineGeometry(cutter.geometry) || isPolygonGeometry(cutter.geometry));
    }
    case "buffer":
      return (
        features.length === 1 &&
        (features[0].geometry.type === "Point" ||
          features[0].geometry.type === "LineString" ||
          isPolygonGeometry(features[0].geometry))
      );
    case "simplify":
      return features.length === 1 && (isLineGeometry(features[0].geometry) || features[0].geometry.type === "Polygon");
    case "reverse":
      return features.length === 1 && isReversibleLine(features[0].geometry);
    case "merge-lines":
      return features.length >= 2 && features.every((feature) => isLineGeometry(feature.geometry));
  }
}

export function commitSelectionIds(preview: GeometryPreview): string[] {
  return preview.previewFeatures.map((feature) => feature.id);
}
