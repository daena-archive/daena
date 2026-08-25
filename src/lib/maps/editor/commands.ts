import { BASE_LAYER_ID, daenaProperties, featureLayerId, type VectorFeature, type VectorLayerDefinition, type VectorLayerStyle } from "../native-vector/types";
import {
  cloneCollection,
  cloneDocument,
  cloneLayers,
  findFeature,
  findLayer,
  nextLayerOrder,
  removeFeatures,
  replaceFeature,
  type MapDocument,
} from "./model";

export type MapCommandKind =
  | "CreateFeature"
  | "DeleteFeatures"
  | "ReplaceGeometry"
  | "DuplicateFeatures"
  | "MoveFeaturesToLayer"
  | "SetFeatureMetadata"
  | "CreateLayer"
  | "DuplicateLayer"
  | "DeleteLayer"
  | "RenameLayer"
  | "ReorderLayer"
  | "SetLayerVisibility"
  | "SetLayerLocked"
  | "SetLayerStyle";

export type MapCommand = {
  kind: MapCommandKind;
  label: string;
  coalesceKey?: string;
  apply: (document: MapDocument) => MapDocument;
  /** Inverse that restores prior document state for this command. */
  invert: (before: MapDocument) => MapCommand;
};

function withCollection(document: MapDocument, collection: MapDocument["collection"]): MapDocument {
  return { ...document, collection };
}

function withLayers(document: MapDocument, layers: VectorLayerDefinition[]): MapDocument {
  return { ...document, layers };
}

export function createFeatureCommand(feature: VectorFeature): MapCommand {
  return {
    kind: "CreateFeature",
    label: "Create feature",
    apply(document) {
      return withCollection(document, replaceFeature(document.collection, feature));
    },
    invert(before) {
      return deleteFeaturesCommand([feature.id], before.collection.features.filter((item) => item.id === feature.id));
    },
  };
}

export function deleteFeaturesCommand(ids: string[], removed: VectorFeature[]): MapCommand {
  const idSet = new Set(ids);
  return {
    kind: "DeleteFeatures",
    label: ids.length === 1 ? "Delete feature" : `Delete ${ids.length} features`,
    apply(document) {
      return withCollection(document, removeFeatures(document.collection, idSet));
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
      return withCollection(document, cloneCollection(next));
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
      let next = document.collection;
      for (const feature of copies) next = replaceFeature(next, feature);
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
      const features = document.collection.features.map((feature) => {
        if (!ids.includes(feature.id)) return feature;
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
      if (!feature) return document;
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

export function createLayerCommand(layer: VectorLayerDefinition): MapCommand {
  return {
    kind: "CreateLayer",
    label: "Create layer",
    apply(document) {
      if (findLayer(document.layers, layer.id)) return document;
      return withLayers(document, [...document.layers, layer]);
    },
    invert() {
      return deleteLayerCommand(layer.id, layer, []);
    },
  };
}

export function duplicateLayerCommand(
  sourceLayerId: string,
  newLayer: VectorLayerDefinition,
  featureCopies: VectorFeature[],
): MapCommand {
  return {
    kind: "DuplicateLayer",
    label: "Duplicate layer",
    apply(document) {
      let next = withLayers(document, [...document.layers.filter((layer) => layer.id !== newLayer.id), newLayer]);
      let collection = next.collection;
      for (const feature of featureCopies) collection = replaceFeature(collection, feature);
      return withCollection(next, collection);
    },
    invert() {
      return {
        kind: "DeleteLayer",
        label: "Remove duplicated layer",
        apply(document) {
          return withCollection(
            withLayers(
              document,
              document.layers.filter((layer) => layer.id !== newLayer.id),
            ),
            removeFeatures(document.collection, new Set(featureCopies.map((feature) => feature.id))),
          );
        },
        invert() {
          return duplicateLayerCommand(sourceLayerId, newLayer, featureCopies);
        },
      };
    },
  };
}

export function deleteLayerCommand(
  layerId: string,
  removedLayer: VectorLayerDefinition,
  removedFeatures: VectorFeature[],
): MapCommand {
  return {
    kind: "DeleteLayer",
    label: "Delete layer",
    apply(document) {
      return withCollection(
        withLayers(
          document,
          document.layers.filter((layer) => layer.id !== layerId),
        ),
        removeFeatures(document.collection, new Set(removedFeatures.map((feature) => feature.id))),
      );
    },
    invert() {
      return {
        kind: "CreateLayer",
        label: "Restore layer",
        apply(document) {
          let next = withLayers(document, [...document.layers.filter((layer) => layer.id !== layerId), removedLayer]);
          let collection = next.collection;
          for (const feature of removedFeatures) collection = replaceFeature(collection, feature);
          return withCollection(next, collection);
        },
        invert(before) {
          return deleteLayerCommand(
            layerId,
            removedLayer,
            before.collection.features.filter((feature) => featureLayerId(feature) === layerId),
          );
        },
      };
    },
  };
}

export function renameLayerCommand(layerId: string, name: string, previousName: string): MapCommand {
  return {
    kind: "RenameLayer",
    label: "Rename layer",
    coalesceKey: `layer-name:${layerId}`,
    apply(document) {
      return withLayers(
        document,
        document.layers.map((layer) => (layer.id === layerId ? { ...layer, name } : layer)),
      );
    },
    invert() {
      return renameLayerCommand(layerId, previousName, name);
    },
  };
}

export function reorderLayerCommand(
  layerId: string,
  order: number,
  previousOrder: number,
  neighborId: string,
  neighborOrder: number,
  neighborPreviousOrder: number,
): MapCommand {
  return {
    kind: "ReorderLayer",
    label: "Reorder layer",
    apply(document) {
      return withLayers(
        document,
        document.layers.map((layer) => {
          if (layer.id === layerId) return { ...layer, order };
          if (layer.id === neighborId) return { ...layer, order: neighborOrder };
          return layer;
        }),
      );
    },
    invert() {
      return reorderLayerCommand(
        layerId,
        previousOrder,
        order,
        neighborId,
        neighborPreviousOrder,
        neighborOrder,
      );
    },
  };
}

export function setLayerVisibilityCommand(
  layerId: string,
  defaultVisible: boolean,
  previous: boolean,
): MapCommand {
  return {
    kind: "SetLayerVisibility",
    label: defaultVisible ? "Show layer" : "Hide layer",
    apply(document) {
      return withLayers(
        document,
        document.layers.map((layer) => (layer.id === layerId ? { ...layer, defaultVisible } : layer)),
      );
    },
    invert() {
      return setLayerVisibilityCommand(layerId, previous, defaultVisible);
    },
  };
}

export function setLayerLockedCommand(layerId: string, locked: boolean, previous: boolean): MapCommand {
  return {
    kind: "SetLayerLocked",
    label: locked ? "Lock layer" : "Unlock layer",
    apply(document) {
      return withLayers(
        document,
        document.layers.map((layer) => (layer.id === layerId ? { ...layer, locked } : layer)),
      );
    },
    invert() {
      return setLayerLockedCommand(layerId, previous, locked);
    },
  };
}

export function setLayerStyleCommand(
  layerId: string,
  style: VectorLayerStyle,
  previous: VectorLayerStyle,
): MapCommand {
  return {
    kind: "SetLayerStyle",
    label: "Edit layer style",
    coalesceKey: `layer-style:${layerId}`,
    apply(document) {
      return withLayers(
        document,
        document.layers.map((layer) => (layer.id === layerId ? { ...layer, style } : layer)),
      );
    },
    invert() {
      return setLayerStyleCommand(layerId, previous, style);
    },
  };
}

export function newVectorLayer(name: string, style?: VectorLayerStyle, order?: number): VectorLayerDefinition {
  return {
    id: crypto.randomUUID(),
    kind: "vector",
    name,
    order: order ?? 0,
    defaultVisible: true,
    locked: false,
    selector: {},
    style: style ?? {
      fill: "#8f6fd1",
      fillOpacity: 0.35,
      stroke: "#5e4893",
      strokeWidth: 1.5,
      pointRadius: 5,
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
      properties: daenaProperties(targetLayerId, feature.properties.daena.semanticType, feature.properties.daena.name),
    }));
}

export function buildCreateLayer(document: MapDocument, name: string): { command: MapCommand; layer: VectorLayerDefinition } {
  const layer = newVectorLayer(name, undefined, nextLayerOrder(document.layers));
  return { command: createLayerCommand(layer), layer };
}

export function buildDuplicateLayer(
  document: MapDocument,
  source: VectorLayerDefinition,
): { command: MapCommand; layer: VectorLayerDefinition } | null {
  if (source.id === BASE_LAYER_ID) return null;
  const layer = newVectorLayer(`${source.name} copy`, { ...source.style }, nextLayerOrder(document.layers));
  const copies = duplicateFeaturesOntoLayer(document, source.id, layer.id);
  return { command: duplicateLayerCommand(source.id, layer, copies), layer };
}

export function captureDeleteFeatures(document: MapDocument, ids: string[]): MapCommand | null {
  const removed = document.collection.features.filter((feature) => ids.includes(feature.id));
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
export function applyCommand(document: MapDocument, command: MapCommand): MapDocument {
  return command.apply(cloneDocument(document));
}

export function layersFieldValue(layers: readonly VectorLayerDefinition[]): {
  schemaVersion: 2;
  layers: VectorLayerDefinition[];
} {
  return {
    schemaVersion: 2,
    layers: cloneLayers(layers),
  };
}
