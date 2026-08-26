import {
  runGeometryOperation,
  operationLabel,
  type GeometryOperationKind,
  type GeometryOpParams,
  type GeometryOpResult,
} from "./geometry-operations.ts";
import type { MapDocument } from "./model.ts";
import type { VectorFeature } from "../native-vector/types.ts";

export type GeometryPreview = {
  operation: GeometryOperationKind;
  inputFeatureIds: string[];
  previewFeatures: VectorFeature[];
  removedFeatureIds: string[];
  params: GeometryOpParams;
  label: string;
};

export function buildPreview(
  document: MapDocument,
  operation: GeometryOperationKind,
  selectedIds: readonly string[],
  params: GeometryOpParams = {},
): { preview: GeometryPreview | null; error: (GeometryOpResult & { ok: false }) | null } {
  const result = runGeometryOperation(document, operation, selectedIds, params);
  if (!result.ok) return { preview: null, error: result };
  return {
    preview: {
      operation,
      inputFeatureIds: [...selectedIds],
      previewFeatures: result.features,
      removedFeatureIds: result.removedIds,
      params,
      label: operationLabel(operation),
    },
    error: null,
  };
}

export function commitSelectionIds(preview: GeometryPreview): string[] {
  return preview.previewFeatures.map((feature) => feature.id);
}
