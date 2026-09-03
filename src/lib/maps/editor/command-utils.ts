import { featureLayerId, layerAcceptsEdits, type MapLayerDefinition } from "../native-vector/types.ts";
import { isOpenLayersDescriptor, type OpenLayersMapDescriptor } from "./coordinate-space.ts";
import { type MapDocument } from "./model.ts";

export function withCollection(document: MapDocument, collection: MapDocument["collection"]): MapDocument {
  return { ...document, collection };
}

export function withLayers(document: MapDocument, layers: MapLayerDefinition[]): MapDocument {
  return { ...document, layers };
}

export function protectedLayerIds(layers: readonly MapLayerDefinition[]): Set<string> {
  return new Set(layers.filter((layer) => !layerAcceptsEdits(layer)).map((layer) => layer.id));
}

export function mergeProtectedCollection(
  current: MapDocument["collection"],
  next: MapDocument["collection"],
  protectedIds: ReadonlySet<string>,
): MapDocument["collection"] {
  const kept = current.features.filter((feature) => protectedIds.has(featureLayerId(feature)));
  const incoming = next.features.filter((feature) => !protectedIds.has(featureLayerId(feature)));
  return {
    type: "FeatureCollection",
    features: [...kept, ...incoming].sort((left, right) => left.id.localeCompare(right.id)),
  };
}

export function withDescriptor(document: MapDocument, descriptor: MapDocument["descriptor"]): MapDocument {
  return { ...document, descriptor };
}

export function openLayersDescriptor(document: MapDocument): OpenLayersMapDescriptor | null {
  return isOpenLayersDescriptor(document.descriptor) ? document.descriptor : null;
}
