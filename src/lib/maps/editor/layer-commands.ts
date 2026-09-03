import {
  BASE_LAYER_ID,
  featureLayerId,
  isRasterLayer,
  isVectorLayer,
  type MapLayerDefinition,
  type RasterLayerDefinition,
  type VectorFeature,
  type VectorLayerDefinition,
  type VectorLayerStyle,
} from "../native-vector/types.ts";
import { cloneLayers, findLayer, nextLayerOrder, removeFeatures, replaceFeature, type MapDocument } from "./model.ts";
import type { MapCommand } from "./commands.ts";
import { withCollection, withLayers } from "./command-utils.ts";
import { duplicateFeaturesOntoLayer } from "./feature-commands.ts";

export function createLayerCommand(layer: MapLayerDefinition): MapCommand {
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
  newLayer: MapLayerDefinition,
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
  removedLayer: MapLayerDefinition,
  removedFeatures: VectorFeature[],
): MapCommand {
  return {
    kind: "DeleteLayer",
    label: "Delete layer",
    apply(document) {
      const existing = findLayer(document.layers, layerId);
      if (!existing || existing.locked || existing.id === BASE_LAYER_ID) return document;
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
      return reorderLayerCommand(layerId, previousOrder, order, neighborId, neighborPreviousOrder, neighborOrder);
    },
  };
}

export function reorderLayersByIdsCommand(orderedIds: readonly string[], previousIds: readonly string[]): MapCommand {
  return {
    kind: "ReorderLayer",
    label: "Reorder layer",
    apply(document) {
      const index = new Map(orderedIds.map((id, order) => [id, order]));
      return withLayers(
        document,
        document.layers.map((layer) => {
          const order = index.get(layer.id);
          return order === undefined ? layer : { ...layer, order };
        }),
      );
    },
    invert() {
      return reorderLayersByIdsCommand(previousIds, orderedIds);
    },
  };
}

export function setLayerVisibilityCommand(layerId: string, defaultVisible: boolean, previous: boolean): MapCommand {
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

export function detachPhysicalFeaturesCommand(input: {
  sourceLayerId: string;
  sourceLayerName: string;
  sourceWasVisible: boolean;
  targetLayer: VectorLayerDefinition;
  copies: VectorFeature[];
}): MapCommand {
  const copyIds = new Set(input.copies.map((feature) => feature.id));
  const label = `Detach ${input.sourceLayerName} for editing`;
  return {
    kind: "DetachPhysicalFeatures",
    label,
    apply(document) {
      const source = findLayer(document.layers, input.sourceLayerId);
      if (!source || source.kind !== "vector" || !source.locked || findLayer(document.layers, input.targetLayer.id)) {
        return document;
      }
      const layers = document.layers.map((layer) =>
        layer.id === input.sourceLayerId ? { ...layer, defaultVisible: false } : layer,
      );
      return withCollection(
        withLayers(document, [...layers, JSON.parse(JSON.stringify(input.targetLayer)) as VectorLayerDefinition]),
        {
          type: "FeatureCollection",
          features: [
            ...document.collection.features,
            ...input.copies.map((feature) => JSON.parse(JSON.stringify(feature))),
          ],
        },
      );
    },
    invert() {
      return {
        kind: "DetachPhysicalFeatures",
        label: `Undo ${label.toLocaleLowerCase()}`,
        apply(document) {
          return withCollection(
            withLayers(
              document,
              document.layers
                .filter((layer) => layer.id !== input.targetLayer.id)
                .map((layer) =>
                  layer.id === input.sourceLayerId ? { ...layer, defaultVisible: input.sourceWasVisible } : layer,
                ),
            ),
            {
              type: "FeatureCollection",
              features: document.collection.features.filter((feature) => !copyIds.has(feature.id)),
            },
          );
        },
        invert() {
          return detachPhysicalFeaturesCommand(input);
        },
      };
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

export function setLayerStyleCommand(layerId: string, style: VectorLayerStyle, previous: VectorLayerStyle): MapCommand {
  return {
    kind: "SetLayerStyle",
    label: "Edit layer style",
    coalesceKey: `layer-style:${layerId}`,
    apply(document) {
      return withLayers(
        document,
        document.layers.map((layer) => (layer.id === layerId && isVectorLayer(layer) ? { ...layer, style } : layer)),
      );
    },
    invert() {
      return setLayerStyleCommand(layerId, previous, style);
    },
  };
}

export function setLayerOpacityCommand(layerId: string, opacity: number, previous: number): MapCommand {
  const next = Math.min(1, Math.max(0, opacity));
  return {
    kind: "SetLayerOpacity",
    label: "Layer opacity",
    coalesceKey: `layer-opacity:${layerId}`,
    apply(document) {
      return withLayers(
        document,
        document.layers.map((layer) => (layer.id === layerId ? { ...layer, opacity: next } : layer)),
      );
    },
    invert() {
      return setLayerOpacityCommand(layerId, previous, next);
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
    opacity: 1,
    blendMode: "normal",
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

export function newRasterLayer(name: string, rasterAssetId: string, order?: number): RasterLayerDefinition {
  return {
    id: crypto.randomUUID(),
    kind: "raster",
    name,
    order: order ?? 0,
    defaultVisible: true,
    locked: false,
    opacity: 1,
    blendMode: "normal",
    rasterAssetId,
    selector: {},
    style: {},
  };
}

export function buildCreateLayer(
  document: MapDocument,
  name: string,
): { command: MapCommand; layer: VectorLayerDefinition } {
  const layer = newVectorLayer(name, undefined, nextLayerOrder(document.layers));
  return { command: createLayerCommand(layer), layer };
}

export function buildCreateRasterLayer(
  document: MapDocument,
  name: string,
  rasterAssetId: string,
): { command: MapCommand; layer: RasterLayerDefinition } {
  const layer = newRasterLayer(name, rasterAssetId, nextLayerOrder(document.layers));
  return { command: createLayerCommand(layer), layer };
}

export function buildDuplicateLayer(
  document: MapDocument,
  source: MapLayerDefinition,
  rasterAssetId?: string,
): { command: MapCommand; layer: MapLayerDefinition } | null {
  if (source.id === BASE_LAYER_ID) return null;
  if (isRasterLayer(source)) {
    if (!rasterAssetId) return null;
    const layer = newRasterLayer(`${source.name} copy`, rasterAssetId, nextLayerOrder(document.layers));
    return { command: duplicateLayerCommand(source.id, layer, []), layer };
  }
  if (!isVectorLayer(source)) return null;
  const layer = newVectorLayer(`${source.name} copy`, { ...source.style }, nextLayerOrder(document.layers));
  const copies = duplicateFeaturesOntoLayer(document, source.id, layer.id);
  return { command: duplicateLayerCommand(source.id, layer, copies), layer };
}

export function layersFieldValue(layers: readonly MapLayerDefinition[]): {
  schemaVersion: 1;
  layers: MapLayerDefinition[];
} {
  return {
    schemaVersion: 1,
    layers: cloneLayers(layers),
  };
}
