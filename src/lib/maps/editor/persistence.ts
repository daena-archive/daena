import { collectionBytes, isReservedPhysicalLayerId, sha256Hex } from "../native-vector/source.ts";
import type { MapLayerDefinition, VectorFeatureCollection, VectorLayerStyle } from "../native-vector/types.ts";
import { layersFieldValue } from "./commands.ts";
import type { MapDocument } from "./model.ts";

const PHYSICAL_LAYER_STYLE_KEYS = ["fill", "fillOpacity", "stroke", "strokeWidth", "pointRadius"] as const;

function canonicalizeLayerForSave(layer: MapLayerDefinition): MapLayerDefinition {
  if (!isReservedPhysicalLayerId(layer.id) || layer.kind !== "vector") return layer;
  const style = {} as VectorLayerStyle;
  for (const key of PHYSICAL_LAYER_STYLE_KEYS) {
    if (layer.style[key] !== undefined) (style as Record<string, unknown>)[key] = layer.style[key];
  }
  const { overlayFamily: _overlayFamily, ...rest } = layer;
  return { ...rest, style };
}

export type MapEditDraftPackage = {
  schemaVersion: 1;
  kind: "daena-map-edit-draft";
  mapEntityId: string;
  descriptor: unknown;
  layers: { schemaVersion: 1; layers: MapDocument["layers"] };
  geojson: string;
  linkMutations: unknown[];
};

export function encodeLayersField(document: MapDocument) {
  return layersFieldValue(document.layers.map(canonicalizeLayerForSave));
}

export function encodeGeoJsonBytes(collection: VectorFeatureCollection): Uint8Array {
  return collectionBytes(collection);
}

export async function contentHashForCollection(collection: VectorFeatureCollection): Promise<string> {
  return sha256Hex(collectionBytes(collection));
}

export function buildRecoveryPackage(
  mapEntityId: string,
  document: MapDocument,
  linkMutations: unknown[] = [],
): MapEditDraftPackage {
  return {
    schemaVersion: 1,
    kind: "daena-map-edit-draft",
    mapEntityId,
    descriptor: document.descriptor,
    layers: encodeLayersField(document),
    geojson: new TextDecoder().decode(encodeGeoJsonBytes(document.collection)),
    linkMutations,
  };
}

export function recoveryPackageBytes(packageValue: MapEditDraftPackage): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(packageValue));
}
