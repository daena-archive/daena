import { collectionBytes, sha256Hex } from "../native-vector/source";
import type { VectorFeatureCollection } from "../native-vector/types";
import { layersFieldValue } from "./commands";
import type { MapDocument } from "./model";

export type MapEditDraftPackage = {
  schemaVersion: 1;
  kind: "daena-map-edit-draft";
  mapEntityId: string;
  descriptor: unknown;
  layers: { schemaVersion: 2; layers: MapDocument["layers"] };
  geojson: string;
  linkMutations: unknown[];
};

export function encodeLayersField(document: MapDocument) {
  return layersFieldValue(document.layers);
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
