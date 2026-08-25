import type { MapBackgroundRef, MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps";
import { BASE_LAYER_ID, daenaProperties, featureLayerId, type VectorFeature, type VectorLayerDefinition, type VectorLayerStyle } from "../native-vector/types.ts";
import {
  backgroundsFromDescriptor,
  flipYBackgrounds,
  flipYCollection,
  isOpenLayersDescriptor,
  patchOpenLayersDescriptor,
  type OpenLayersMapDescriptor,
} from "./coordinate-space.ts";
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
} from "./model.ts";

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
  | "SetLayerStyle"
  | "AddBackground"
  | "ReplaceBackground"
  | "RemoveBackground"
  | "ReorderBackground"
  | "SetBackgroundOpacity"
  | "SetBackgroundVisibility"
  | "SetDefaultView"
  | "SetCoordinateSpace";

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

function withDescriptor(document: MapDocument, descriptor: MapDocument["descriptor"]): MapDocument {
  return { ...document, descriptor };
}

function openLayersDescriptor(document: MapDocument): OpenLayersMapDescriptor | null {
  return isOpenLayersDescriptor(document.descriptor) ? document.descriptor : null;
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

export function addBackgroundCommand(background: MapBackgroundRef): MapCommand {
  return {
    kind: "AddBackground",
    label: "Add raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      if (descriptor.backgrounds.some((item) => item.id === background.id)) return document;
      return withDescriptor(document, patchOpenLayersDescriptor(descriptor, {
        backgrounds: [...descriptor.backgrounds, background],
      }) as OpenLayersMapDescriptor);
    },
    invert() {
      return removeBackgroundCommand(background.id, background);
    },
  };
}

export function removeBackgroundCommand(id: string, removed: MapBackgroundRef): MapCommand {
  return {
    kind: "RemoveBackground",
    label: "Remove raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(document, patchOpenLayersDescriptor(descriptor, {
        backgrounds: descriptor.backgrounds.filter((item) => item.id !== id),
      }) as OpenLayersMapDescriptor);
    },
    invert() {
      return addBackgroundCommand(removed);
    },
  };
}

export function replaceBackgroundCommand(id: string, next: MapBackgroundRef, previous: MapBackgroundRef): MapCommand {
  return {
    kind: "ReplaceBackground",
    label: "Replace raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(document, patchOpenLayersDescriptor(descriptor, {
        backgrounds: descriptor.backgrounds.map((item) => (item.id === id ? next : item)),
      }) as OpenLayersMapDescriptor);
    },
    invert() {
      return replaceBackgroundCommand(id, previous, next);
    },
  };
}

export function reorderBackgroundCommand(
  id: string,
  order: number,
  previousOrder: number,
  neighborId: string,
  neighborOrder: number,
  neighborPreviousOrder: number,
): MapCommand {
  return {
    kind: "ReorderBackground",
    label: "Reorder raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(document, patchOpenLayersDescriptor(descriptor, {
        backgrounds: descriptor.backgrounds.map((item) => {
          if (item.id === id) return { ...item, order };
          if (item.id === neighborId) return { ...item, order: neighborOrder };
          return item;
        }),
      }) as OpenLayersMapDescriptor);
    },
    invert() {
      return reorderBackgroundCommand(id, previousOrder, order, neighborId, neighborPreviousOrder, neighborOrder);
    },
  };
}

export function setBackgroundOpacityCommand(id: string, opacity: number, previous: number): MapCommand {
  return {
    kind: "SetBackgroundOpacity",
    label: "Raster opacity",
    coalesceKey: `background-opacity:${id}`,
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(document, patchOpenLayersDescriptor(descriptor, {
        backgrounds: descriptor.backgrounds.map((item) => (item.id === id ? { ...item, opacity } : item)),
      }) as OpenLayersMapDescriptor);
    },
    invert() {
      return setBackgroundOpacityCommand(id, previous, opacity);
    },
  };
}

export function setBackgroundVisibilityCommand(id: string, visible: boolean, previous: boolean): MapCommand {
  return {
    kind: "SetBackgroundVisibility",
    label: visible ? "Show raster" : "Hide raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(document, patchOpenLayersDescriptor(descriptor, {
        backgrounds: descriptor.backgrounds.map((item) => (item.id === id ? { ...item, visible } : item)),
      }) as OpenLayersMapDescriptor);
    },
    invert() {
      return setBackgroundVisibilityCommand(id, previous, visible);
    },
  };
}

export function setDefaultViewCommand(
  next: { center: [number, number]; zoom: number; rotation: number },
  previous: { center: [number, number]; zoom: number; rotation: number },
): MapCommand {
  const stored = {
    center: next.center,
    zoom: Math.max(next.zoom, 1e-6),
    rotation: next.rotation,
  };
  return {
    kind: "SetDefaultView",
    label: "Set view",
    coalesceKey: "default-view",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(document, patchOpenLayersDescriptor(descriptor, {
        defaultView: stored,
      }) as OpenLayersMapDescriptor);
    },
    invert() {
      return setDefaultViewCommand(previous, next);
    },
  };
}

export function setCoordinateSpaceCommand(
  next: MapCoordinateSpace,
  previous: MapCoordinateSpace,
  nextCollection: MapDocument["collection"] | null,
  previousCollection: MapDocument["collection"] | null,
  nextBackgrounds: readonly MapBackgroundRef[] | null,
  previousBackgrounds: readonly MapBackgroundRef[] | null,
): MapCommand {
  return {
    kind: "SetCoordinateSpace",
    label: "Calibrate map",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      let patched = withDescriptor(document, patchOpenLayersDescriptor(descriptor, {
        coordinateSpace: next,
        backgrounds: nextBackgrounds ?? descriptor.backgrounds,
      }) as OpenLayersMapDescriptor);
      if (nextCollection) patched = withCollection(patched, nextCollection);
      return patched;
    },
    invert() {
      return setCoordinateSpaceCommand(
        previous,
        next,
        previousCollection,
        nextCollection,
        previousBackgrounds,
        nextBackgrounds,
      );
    },
  };
}

export function calibrateImageToWorld(
  document: MapDocument,
  metresPerUnit: number | null,
  label = "metres",
): MapCommand | null {
  const descriptor = openLayersDescriptor(document);
  if (!descriptor || descriptor.coordinateSpace.kind !== "image") return null;
  const previous = descriptor.coordinateSpace;
  const next: MapCoordinateSpace = {
    kind: "world",
    extent: previous.extent,
    origin: "bottom-left",
    units: { id: "metre", label, metresPerUnit },
    wrapX: false,
  };
  return setCoordinateSpaceCommand(
    next,
    previous,
    flipYCollection(document.collection, previous),
    document.collection,
    flipYBackgrounds(descriptor.backgrounds, previous),
    descriptor.backgrounds,
  );
}

export function calibrateWorldUnits(
  document: MapDocument,
  metresPerUnit: number | null,
  label?: string,
): MapCommand | null {
  const descriptor = openLayersDescriptor(document);
  if (!descriptor || descriptor.coordinateSpace.kind !== "world") return null;
  const previous = descriptor.coordinateSpace;
  const next: MapCoordinateSpace = {
    ...previous,
    units: {
      id: previous.units.id,
      label: label ?? previous.units.label,
      metresPerUnit,
    },
  };
  return setCoordinateSpaceCommand(next, previous, null, null, null, null);
}

export function listedBackgrounds(document: MapDocument): MapBackgroundRef[] {
  return [...backgroundsFromDescriptor(document.descriptor)].sort(
    (left, right) => right.order - left.order || left.id.localeCompare(right.id),
  );
}

export function nextBackgroundOrder(backgrounds: readonly MapBackgroundRef[]): number {
  if (backgrounds.length === 0) return 0;
  return Math.max(...backgrounds.map((item) => item.order)) + 1;
}
