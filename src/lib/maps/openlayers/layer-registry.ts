import VectorLayer from "ol/layer/Vector.js";
import VectorSource from "ol/source/Vector.js";
import type { VectorFeatureCollection, VectorLayerDefinition } from "../native-vector/types";
import { collectionSignature, type FeatureCodec } from "./feature-codec";
import { nativeFeatureStyle, visibleUnlockedFeatures } from "./style-factory";

/** Single-source registry (Slice 4 will expand to one OL layer per Daena layer). */
export type LayerRegistry = {
  source: VectorSource;
  snapSource: VectorSource;
  vectorLayer: VectorLayer;
  selectedIds: Set<string>;
  hoveredId: string | null;
  layers: VectorLayerDefinition[];
  lastSignature: string;
  setHovered: (id: string | null) => void;
  syncLayers: (layers: readonly VectorLayerDefinition[]) => void;
  replaceCollection: (collection: VectorFeatureCollection) => void;
  syncSnap: (collection: VectorFeatureCollection) => void;
  refreshStyle: () => void;
};

export function createLayerRegistry(
  collection: VectorFeatureCollection,
  layers: readonly VectorLayerDefinition[],
  codec: FeatureCodec,
): LayerRegistry {
  const selectedIds = new Set<string>();
  let hoveredId: string | null = null;
  let currentLayers = [...layers];
  let lastSignature = collectionSignature(collection);
  const source = new VectorSource({ features: codec.readOlFeatures(collection), wrapX: false });
  const snapSource = new VectorSource({ wrapX: false });
  const vectorLayer = new VectorLayer({
    source,
    updateWhileAnimating: true,
    updateWhileInteracting: true,
    style(feature) {
      const id = String(feature.getId() ?? "");
      return nativeFeatureStyle(feature, currentLayers, {
        hovered: id === hoveredId,
        selected: selectedIds.has(id),
      });
    },
  });

  const registry: LayerRegistry = {
    source,
    snapSource,
    vectorLayer,
    selectedIds,
    get hoveredId() {
      return hoveredId;
    },
    set hoveredId(_value: string | null) {
      hoveredId = _value;
    },
    layers: currentLayers,
    get lastSignature() {
      return lastSignature;
    },
    set lastSignature(value: string) {
      lastSignature = value;
    },
    setHovered(id) {
      hoveredId = id;
      vectorLayer.changed();
    },
    syncLayers(nextLayers) {
      currentLayers = [...nextLayers];
      registry.layers = currentLayers;
      vectorLayer.changed();
    },
    replaceCollection(next) {
      source.clear(true);
      source.addFeatures(codec.readOlFeatures(next));
      lastSignature = collectionSignature(next);
      selectedIds.clear();
      registry.syncSnap(next);
      vectorLayer.changed();
    },
    syncSnap(next) {
      snapSource.clear(true);
      snapSource.addFeatures(codec.readOlFeatures(visibleUnlockedFeatures(next, currentLayers)));
    },
    refreshStyle() {
      vectorLayer.changed();
    },
  };
  return registry;
}
