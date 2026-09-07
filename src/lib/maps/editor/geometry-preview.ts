import { runGeometryOperation, runGeometryOperationAsync } from "./geometry-operations.ts";
import {
  cancelledGeometryResult,
  commitSelectionIds,
  operationLabel,
  type GeometryOperationKind,
  type GeometryOpJobOptions,
  type GeometryOpParams,
  type GeometryOpResult,
  type GeometryPreview,
} from "./geometry-operation-kinds.ts";
import type { MapDocument } from "./model.ts";

export { commitSelectionIds, type GeometryPreview };

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

export async function buildPreviewAsync(
  document: MapDocument,
  operation: GeometryOperationKind,
  selectedIds: readonly string[],
  params: GeometryOpParams = {},
  options: GeometryOpJobOptions = {},
): Promise<{ preview: GeometryPreview | null; error: (GeometryOpResult & { ok: false }) | null }> {
  const result = await runGeometryOperationAsync(document, operation, selectedIds, params, options);
  if (!result.ok) return { preview: null, error: result };
  if (options.signal?.aborted) return { preview: null, error: cancelledGeometryResult() };
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
