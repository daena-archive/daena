import type { MapLayerDefinition, VectorFeature, VectorFeatureCollection } from "../native-vector/types.ts";
import type { OpenLayersMapDescriptor } from "./coordinate-space.ts";

/** Plain Daena map authoring state — no OpenLayers objects. */
export type MapDocument = {
  descriptor: OpenLayersMapDescriptor | Record<string, unknown>;
  layers: MapLayerDefinition[];
  collection: VectorFeatureCollection;
};

export function cloneDescriptor<T>(descriptor: T): T {
  return JSON.parse(JSON.stringify(descriptor)) as T;
}

export function cloneDocument(document: MapDocument): MapDocument {
  return JSON.parse(JSON.stringify(document)) as MapDocument;
}

export function cloneCollection(collection: VectorFeatureCollection): VectorFeatureCollection {
  return JSON.parse(JSON.stringify(collection)) as VectorFeatureCollection;
}

export function cloneLayers(layers: readonly MapLayerDefinition[]): MapLayerDefinition[] {
  return JSON.parse(JSON.stringify(layers)) as MapLayerDefinition[];
}

export function documentHash(document: MapDocument): string {
  return JSON.stringify({
    descriptor: document.descriptor,
    layers: document.layers,
    collection: document.collection,
  });
}

export function documentByteSize(document: MapDocument): number {
  return new TextEncoder().encode(documentHash(document)).byteLength;
}

export function createMapDocument(input: {
  descriptor: unknown;
  layers: readonly MapLayerDefinition[];
  collection: VectorFeatureCollection;
}): MapDocument {
  return {
    descriptor: (input.descriptor && typeof input.descriptor === "object"
      ? input.descriptor
      : {}) as MapDocument["descriptor"],
    layers: cloneLayers(input.layers),
    collection: cloneCollection(input.collection),
  };
}

export function replaceFeature(collection: VectorFeatureCollection, feature: VectorFeature): VectorFeatureCollection {
  const features = collection.features.map((item) => (item.id === feature.id ? feature : item));
  if (!features.some((item) => item.id === feature.id)) features.push(feature);
  return {
    type: "FeatureCollection",
    features: features.sort((left, right) => left.id.localeCompare(right.id)),
  };
}

export function removeFeatures(collection: VectorFeatureCollection, ids: ReadonlySet<string>): VectorFeatureCollection {
  return {
    type: "FeatureCollection",
    features: collection.features.filter((feature) => !ids.has(feature.id)),
  };
}

export function findFeature(collection: VectorFeatureCollection, id: string): VectorFeature | undefined {
  return collection.features.find((feature) => feature.id === id);
}

export function findLayer(layers: readonly MapLayerDefinition[], id: string): MapLayerDefinition | undefined {
  return layers.find((layer) => layer.id === id);
}

export function nextLayerOrder(layers: readonly MapLayerDefinition[]): number {
  if (layers.length === 0) return 0;
  return Math.max(...layers.map((layer) => layer.order)) + 1;
}
