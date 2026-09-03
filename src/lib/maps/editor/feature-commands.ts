import type { MapLabelV2, MapStyleV2 } from "../../../../packages/plugin-sdk/src/maps.ts";
import { featureLayerId, layerAcceptsEdits, type VectorFeature } from "../native-vector/types.ts";
import { cloneCollection, findFeature, findLayer, removeFeatures, replaceFeature, type MapDocument } from "./model.ts";
import type { MapCommand } from "./commands.ts";
import { mergeProtectedCollection, protectedLayerIds, withCollection } from "./command-utils.ts";

export function createFeatureCommand(feature: VectorFeature): MapCommand {
  return {
    kind: "CreateFeature",
    label: "Create feature",
    apply(document) {
      if (!layerAcceptsEdits(findLayer(document.layers, featureLayerId(feature)))) return document;
      return withCollection(document, replaceFeature(document.collection, feature));
    },
    invert(before) {
      return deleteFeaturesCommand(
        [feature.id],
        before.collection.features.filter((item) => item.id === feature.id),
      );
    },
  };
}

export function deleteFeaturesCommand(ids: string[], removed: VectorFeature[]): MapCommand {
  const idSet = new Set(ids);
  return {
    kind: "DeleteFeatures",
    label: ids.length === 1 ? "Delete feature" : `Delete ${ids.length} features`,
    apply(document) {
      const blocked = protectedLayerIds(document.layers);
      const removable = new Set(
        document.collection.features
          .filter((feature) => idSet.has(feature.id) && !blocked.has(featureLayerId(feature)))
          .map((feature) => feature.id),
      );
      return withCollection(document, removeFeatures(document.collection, removable));
    },
    invert() {
      return {
        kind: "CreateFeature",
        label: "Restore features",
        apply(document) {
          let next = document.collection;
          for (const feature of removed) next = replaceFeature(next, feature);
          return withCollection(document, next);
        },
        invert(before) {
          return deleteFeaturesCommand(
            removed.map((feature) => feature.id),
            before.collection.features.filter((feature) => idSet.has(feature.id)),
          );
        },
      };
    },
  };
}

export function replaceGeometryCommand(
  featureId: string,
  nextFeature: VectorFeature,
  previous: VectorFeature,
): MapCommand {
  return {
    kind: "ReplaceGeometry",
    label: "Edit geometry",
    coalesceKey: `replace-geometry:${featureId}`,
    apply(document) {
      if (!layerAcceptsEdits(findLayer(document.layers, featureLayerId(nextFeature)))) return document;
      return withCollection(document, replaceFeature(document.collection, nextFeature));
    },
    invert() {
      return replaceGeometryCommand(featureId, previous, nextFeature);
    },
  };
}

/** Replace an entire collection snapshot (used when many features change in one gesture). */

export function replaceCollectionCommand(
  next: MapDocument["collection"],
  previous: MapDocument["collection"],
  label = "Edit features",
  coalesceKey?: string,
): MapCommand {
  return {
    kind: "ReplaceGeometry",
    label,
    coalesceKey,
    apply(document) {
      return withCollection(
        document,
        mergeProtectedCollection(document.collection, cloneCollection(next), protectedLayerIds(document.layers)),
      );
    },
    invert() {
      return replaceCollectionCommand(previous, next, label, coalesceKey);
    },
  };
}

export function duplicateFeaturesCommand(copies: VectorFeature[]): MapCommand {
  return {
    kind: "DuplicateFeatures",
    label: copies.length === 1 ? "Duplicate feature" : `Duplicate ${copies.length} features`,
    apply(document) {
      const blocked = protectedLayerIds(document.layers);
      let next = document.collection;
      for (const feature of copies) {
        if (blocked.has(featureLayerId(feature))) continue;
        next = replaceFeature(next, feature);
      }
      return withCollection(document, next);
    },
    invert() {
      return deleteFeaturesCommand(
        copies.map((feature) => feature.id),
        copies,
      );
    },
  };
}

export function moveFeaturesToLayerCommand(
  ids: string[],
  targetLayerId: string,
  previousLayerIds: Record<string, string>,
): MapCommand {
  return {
    kind: "MoveFeaturesToLayer",
    label: "Move features to layer",
    apply(document) {
      const target = findLayer(document.layers, targetLayerId);
      if (!layerAcceptsEdits(target)) return document;
      const blocked = protectedLayerIds(document.layers);
      const features = document.collection.features.map((feature) => {
        if (!ids.includes(feature.id) || blocked.has(featureLayerId(feature))) return feature;
        return {
          ...feature,
          properties: {
            daena: {
              ...feature.properties.daena,
              layerId: targetLayerId,
            },
          },
        };
      });
      return withCollection(document, {
        type: "FeatureCollection",
        features: features.sort((left, right) => left.id.localeCompare(right.id)),
      });
    },
    invert() {
      return {
        kind: "MoveFeaturesToLayer",
        label: "Restore feature layers",
        apply(document) {
          const features = document.collection.features.map((feature) => {
            const prior = previousLayerIds[feature.id];
            if (!prior) return feature;
            return {
              ...feature,
              properties: {
                daena: {
                  ...feature.properties.daena,
                  layerId: prior,
                },
              },
            };
          });
          return withCollection(document, {
            type: "FeatureCollection",
            features: features.sort((left, right) => left.id.localeCompare(right.id)),
          });
        },
        invert(before) {
          return moveFeaturesToLayerCommand(ids, targetLayerId, previousLayerIds);
        },
      };
    },
  };
}

export function setFeatureMetadataCommand(
  featureId: string,
  name: string | null,
  previousName: string | null,
): MapCommand {
  return {
    kind: "SetFeatureMetadata",
    label: "Rename feature",
    coalesceKey: `feature-name:${featureId}`,
    apply(document) {
      const feature = findFeature(document.collection, featureId);
      if (!feature || !layerAcceptsEdits(findLayer(document.layers, featureLayerId(feature)))) return document;
      return withCollection(
        document,
        replaceFeature(document.collection, {
          ...feature,
          properties: {
            daena: {
              ...feature.properties.daena,
              name,
            },
          },
        }),
      );
    },
    invert() {
      return setFeatureMetadataCommand(featureId, previousName, name);
    },
  };
}

export type FeatureMetadataPatch = {
  name?: string | null;
  semanticType?: string;
  style?: Partial<MapStyleV2> | null;
  label?: MapLabelV2 | null;
  custom?: Record<string, string | number | boolean | null>;
};

/** Apply one inspector edit to one or more features as a single undoable command. */

export function setFeaturesMetadataCommand(
  featureIds: readonly string[],
  patch: FeatureMetadataPatch,
  previous: Readonly<Record<string, FeatureMetadataPatch>>,
  label = featureIds.length === 1 ? "Edit feature" : `Edit ${featureIds.length} features`,
  coalesceKey?: string,
): MapCommand {
  const ids = new Set(featureIds);
  return {
    kind: "SetFeatureMetadata",
    label,
    coalesceKey,
    apply(document) {
      const features = document.collection.features.map((feature) => {
        if (!ids.has(feature.id) || !layerAcceptsEdits(findLayer(document.layers, featureLayerId(feature)))) {
          return feature;
        }
        return {
          ...feature,
          properties: {
            daena: {
              ...feature.properties.daena,
              ...patch,
            },
          },
        } as VectorFeature;
      });
      return withCollection(document, {
        type: "FeatureCollection",
        features: features.sort((left, right) => left.id.localeCompare(right.id)),
      });
    },
    invert() {
      return {
        kind: "SetFeatureMetadata",
        label: `Undo ${label.toLocaleLowerCase()}`,
        apply(document) {
          const features = document.collection.features.map((feature) => {
            const prior = previous[feature.id];
            if (!prior) return feature;
            return {
              ...feature,
              properties: {
                daena: {
                  ...feature.properties.daena,
                  ...prior,
                },
              },
            } as VectorFeature;
          });
          return withCollection(document, {
            type: "FeatureCollection",
            features: features.sort((left, right) => left.id.localeCompare(right.id)),
          });
        },
        invert() {
          return setFeaturesMetadataCommand(featureIds, patch, previous, label, coalesceKey);
        },
      };
    },
  };
}

export function setFeaturesMetadataByIdCommand(
  next: Readonly<Record<string, FeatureMetadataPatch>>,
  previous: Readonly<Record<string, FeatureMetadataPatch>>,
  label: string,
  coalesceKey?: string,
): MapCommand {
  const ids = Object.keys(next);
  return {
    kind: "SetFeatureMetadata",
    label,
    coalesceKey,
    apply(document) {
      const features = document.collection.features.map((feature) => {
        const patch = next[feature.id];
        if (!patch || !layerAcceptsEdits(findLayer(document.layers, featureLayerId(feature)))) return feature;
        return {
          ...feature,
          properties: { daena: { ...feature.properties.daena, ...patch } },
        } as VectorFeature;
      });
      return withCollection(document, { type: "FeatureCollection", features });
    },
    invert() {
      return setFeaturesMetadataByIdCommand(previous, next, `Undo ${label.toLocaleLowerCase()}`, coalesceKey);
    },
  };
}

export function duplicateFeaturesOntoLayer(
  document: MapDocument,
  sourceLayerId: string,
  targetLayerId: string,
): VectorFeature[] {
  return document.collection.features
    .filter((feature) => featureLayerId(feature) === sourceLayerId)
    .map((feature) => ({
      ...cloneCollection({ type: "FeatureCollection", features: [feature] }).features[0],
      id: crypto.randomUUID(),
      properties: {
        daena: {
          ...feature.properties.daena,
          layerId: targetLayerId,
          style: feature.properties.daena.style ? { ...feature.properties.daena.style } : null,
          label: feature.properties.daena.label ? { ...feature.properties.daena.label } : null,
          custom: { ...feature.properties.daena.custom },
        },
      },
    }));
}

export function captureDeleteFeatures(document: MapDocument, ids: string[]): MapCommand | null {
  const removed = document.collection.features.filter(
    (feature) => ids.includes(feature.id) && layerAcceptsEdits(findLayer(document.layers, featureLayerId(feature))),
  );
  if (removed.length === 0) return null;
  return deleteFeaturesCommand(
    removed.map((feature) => feature.id),
    removed,
  );
}

export function captureReplaceCollection(
  before: MapDocument,
  afterCollection: MapDocument["collection"],
  label?: string,
  coalesceKey?: string,
): MapCommand | null {
  if (JSON.stringify(before.collection) === JSON.stringify(afterCollection)) return null;
  return replaceCollectionCommand(afterCollection, before.collection, label, coalesceKey);
}

/** Apply a command without mutating the input document. */
