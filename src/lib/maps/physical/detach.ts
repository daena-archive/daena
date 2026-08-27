import {
  VECTOR_MAX_BYTES,
  VECTOR_MAX_FEATURES,
  VECTOR_MAX_FEATURE_POSITIONS,
  VECTOR_MAX_LAYERS,
  VECTOR_MAX_POSITIONS,
} from "../../../../packages/plugin-sdk/src/maps.ts";
import { nextLayerOrder } from "../editor/model.ts";
import { collectionBytes } from "../native-vector/source.ts";
import {
  featureLayerId,
  type MapLayerDefinition,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "../native-vector/types.ts";

export const PHYSICAL_DERIVED_LAYER_IDS = [
  "base",
  "ocean",
  "land",
  "shelves",
  "bathymetric-contours",
  "tectonic-plates",
  "tectonic-boundaries",
  "bathymetry",
  "volcanic-centers",
  "lakes",
  "rivers",
  "watersheds",
  "islands",
  "ice",
] as const;

export type PhysicalDerivedLayerId = (typeof PHYSICAL_DERIVED_LAYER_IDS)[number];
export type PhysicalDetachScope = "selected" | "layer";

export type PhysicalDetachPlan = {
  sourceLayerId: PhysicalDerivedLayerId;
  sourceLayerName: string;
  epochOffsetYears: number;
  scope: PhysicalDetachScope;
  sourceFeatureIds: string[];
  targetLayer: VectorLayerDefinition;
  copies: VectorFeature[];
};

export type PhysicalDetachError = {
  code: "physical.detach.invalid-layer" | "physical.detach.empty" | "vector.limit.exceeded";
  message: string;
};

export type BuildPhysicalDetachPlanInput = {
  collection: VectorFeatureCollection;
  document: { layers: readonly MapLayerDefinition[]; collection: VectorFeatureCollection };
  sourceLayer: MapLayerDefinition;
  epochOffsetYears: number;
  scope: PhysicalDetachScope;
  selectedIds?: readonly string[];
  newId?: () => string;
};

export function isPhysicalDerivedLayerId(id: string): id is PhysicalDerivedLayerId {
  return (PHYSICAL_DERIVED_LAYER_IDS as readonly string[]).includes(id);
}

export function physicalFeaturesForLayer(
  collection: VectorFeatureCollection,
  layerId: PhysicalDerivedLayerId,
): VectorFeature[] {
  return collection.features.filter((feature) => featureLayerId(feature) === layerId);
}

export function selectedPhysicalFeatures(
  collection: VectorFeatureCollection,
  layerId: PhysicalDerivedLayerId,
  selectedIds: readonly string[],
): VectorFeature[] {
  const selected = new Set(selectedIds);
  return physicalFeaturesForLayer(collection, layerId).filter((feature) => selected.has(feature.id));
}

export function physicalDetachLayerName(sourceLayerName: string, epochOffsetYears: number): string {
  if (epochOffsetYears === 0) return `${sourceLayerName} — detached at reference epoch`;
  return `${sourceLayerName} — detached at ${epochOffsetYears > 0 ? "+" : ""}${epochOffsetYears} years`;
}

function cloneFeature(feature: VectorFeature): VectorFeature {
  return JSON.parse(JSON.stringify(feature)) as VectorFeature;
}

function positionCount(feature: VectorFeature): number {
  let count = 0;
  const pending: unknown[] = [feature.geometry.coordinates];
  while (pending.length > 0) {
    const value = pending.pop();
    if (!Array.isArray(value)) continue;
    if (value.length >= 2 && typeof value[0] === "number" && typeof value[1] === "number") {
      count += 1;
      continue;
    }
    for (const item of value) pending.push(item);
  }
  return count;
}

function limit(message: string): PhysicalDetachError {
  return { code: "vector.limit.exceeded", message };
}

export function buildPhysicalDetachPlan(input: BuildPhysicalDetachPlanInput): PhysicalDetachPlan | PhysicalDetachError {
  const { sourceLayer } = input;
  if (!isPhysicalDerivedLayerId(sourceLayer.id) || sourceLayer.kind !== "vector" || !sourceLayer.locked) {
    return { code: "physical.detach.invalid-layer", message: "This physical layer cannot be detached for editing." };
  }

  const all = physicalFeaturesForLayer(input.collection, sourceLayer.id);
  const source =
    input.scope === "selected"
      ? selectedPhysicalFeatures(input.collection, sourceLayer.id, input.selectedIds ?? [])
      : all;
  if (source.length === 0) {
    return { code: "physical.detach.empty", message: "This physical layer has no generated features to detach." };
  }
  if (input.document.layers.length + 1 > VECTOR_MAX_LAYERS) {
    return limit(`Detaching would exceed the map layer limit of ${VECTOR_MAX_LAYERS}.`);
  }
  if (input.document.collection.features.length + source.length > VECTOR_MAX_FEATURES) {
    return limit(`Detaching would exceed the authored feature limit of ${VECTOR_MAX_FEATURES}.`);
  }
  if (source.some((feature) => positionCount(feature) > VECTOR_MAX_FEATURE_POSITIONS)) {
    return limit(`A detached feature exceeds the per-feature position limit of ${VECTOR_MAX_FEATURE_POSITIONS}.`);
  }

  const newId = input.newId ?? (() => crypto.randomUUID());
  const targetLayer: VectorLayerDefinition = {
    id: newId(),
    kind: "vector",
    name: physicalDetachLayerName(sourceLayer.name, input.epochOffsetYears),
    order: nextLayerOrder(input.document.layers),
    defaultVisible: true,
    locked: false,
    opacity: sourceLayer.opacity,
    blendMode: sourceLayer.blendMode,
    selector: {},
    style: JSON.parse(JSON.stringify(sourceLayer.style)),
  };
  const copies = [...source]
    .sort((left, right) => left.id.localeCompare(right.id))
    .map((feature) => {
      const copy = cloneFeature(feature);
      copy.id = newId();
      copy.properties.daena.layerId = targetLayer.id;
      copy.properties.daena.custom = {
        ...copy.properties.daena.custom,
        detachedFromProvider: "daena-physical",
        detachedFromLayerId: sourceLayer.id,
        detachedFromFeatureId: String(feature.id),
        detachedAtEpochYears: input.epochOffsetYears,
      };
      return copy;
    });
  const prospective: VectorFeatureCollection = {
    type: "FeatureCollection",
    features: [...input.document.collection.features, ...copies],
  };
  let positions = 0;
  for (const feature of prospective.features) positions += positionCount(feature);
  if (positions > VECTOR_MAX_POSITIONS) {
    return limit(`Detaching would exceed the authored position limit of ${VECTOR_MAX_POSITIONS}.`);
  }
  if (collectionBytes(prospective).byteLength > VECTOR_MAX_BYTES) {
    return limit(`Detaching would exceed the authored GeoJSON byte limit of ${VECTOR_MAX_BYTES}.`);
  }
  return {
    sourceLayerId: sourceLayer.id,
    sourceLayerName: sourceLayer.name,
    epochOffsetYears: input.epochOffsetYears,
    scope: input.scope,
    sourceFeatureIds: source.map((feature) => feature.id),
    targetLayer,
    copies,
  };
}
