import Collection from "ol/Collection.js";
import type Feature from "ol/Feature.js";
import type Geometry from "ol/geom/Geometry.js";
import ImageLayer from "ol/layer/Image.js";
import LayerGroup from "ol/layer/Group.js";
import type BaseLayer from "ol/layer/Base.js";
import VectorLayer from "ol/layer/Vector.js";
import ImageStatic from "ol/source/ImageStatic.js";
import VectorSource from "ol/source/Vector.js";
import type Projection from "ol/proj/Projection.js";
import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps.ts";
import { authoredExtentToViewExtent, extentOf } from "../editor/coordinate-space.ts";
import {
  BASE_LAYER_ID,
  DEFAULT_VECTOR_LAYER_STYLE,
  featureLayerId,
  isRasterLayer,
  isVectorLayer,
  type MapLayerDefinition,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "../native-vector/types.ts";
import { collectionSignature, type FeatureCodec } from "./feature-codec.ts";
import { nativeFeatureStyle, snapTargetFeatures } from "./style-factory.ts";

export type RasterLayerSource = {
  url: string;
  canvas?: HTMLCanvasElement;
};

export type LayerRegistry = {
  group: LayerGroup;
  snapSource: VectorSource;
  selectedIds: Set<string>;
  hoveredId: string | null;
  layers: MapLayerDefinition[];
  lastSignature: string;
  sourceFor: (layerId: string) => VectorSource | null;
  vectorOlLayers: () => VectorLayer[];
  layerById: (id: string) => MapLayerDefinition | undefined;
  isSelectableVectorLayer: (layer: BaseLayer) => boolean;
  getFeatureById: (id: string) => Feature<Geometry> | null;
  forEachVectorFeature: (callback: (feature: Feature<Geometry>) => void) => void;
  setHovered: (id: string | null) => void;
  sync: (
    layers: readonly MapLayerDefinition[],
    collection: VectorFeatureCollection,
    rasters?: ReadonlyMap<string, RasterLayerSource>,
  ) => void;
  syncLayers: (layers: readonly MapLayerDefinition[]) => void;
  replaceCollection: (collection: VectorFeatureCollection) => void;
  collectionFromLayers: () => VectorFeatureCollection;
  syncSnap: (collection: VectorFeatureCollection, snapTargetLayerIds?: ReadonlySet<string>) => void;
  setSnapTargetLayerIds: (ids: ReadonlySet<string>) => void;
  refreshStyle: () => void;
  dispose: () => void;
};

const IMPLICIT_BASE_LAYER: VectorLayerDefinition = {
  id: BASE_LAYER_ID,
  kind: "vector",
  name: "Base",
  order: Number.MIN_SAFE_INTEGER,
  defaultVisible: true,
  locked: true,
  opacity: 1,
  blendMode: "normal",
  selector: {},
  style: DEFAULT_VECTOR_LAYER_STYLE,
};

function rasterUrl(source: RasterLayerSource | undefined): string | null {
  if (!source) return null;
  if (source.canvas) {
    try {
      return source.canvas.toDataURL("image/png");
    } catch {
      return source.url || null;
    }
  }
  return source.url || null;
}

export function createLayerRegistry(
  collection: VectorFeatureCollection,
  layers: readonly MapLayerDefinition[],
  codec: FeatureCodec,
  space: MapCoordinateSpace,
  projection: Projection,
  options: { labelsVisible?: boolean } = {},
): LayerRegistry {
  const selectedIds = new Set<string>();
  let hoveredId: string | null = null;
  let currentLayers: MapLayerDefinition[] = [...layers];
  // The initial collection has not been copied into any OpenLayers source yet.
  // Starting with its signature would make the first sync look like a no-op
  // and leave every runtime layer empty.
  let lastSignature = "";
  let currentRasters = new Map<string, RasterLayerSource>();
  let snapTargetLayerIds = new Set<string>();
  const group = new LayerGroup({ layers: [] });
  const vectorEntries = new Map<string, { layer: VectorLayer; source: VectorSource }>();
  const rasterEntries = new Map<string, ImageLayer<any>>();
  const snapSource = new VectorSource({ wrapX: false });

  const runtimeVectorLayers = () => {
    const authored = currentLayers.filter(isVectorLayer);
    return authored.some((layer) => layer.id === BASE_LAYER_ID) ? authored : [IMPLICIT_BASE_LAYER, ...authored];
  };

  const authoredWidth = Math.max(extentOf(space)[2] - extentOf(space)[0], Number.EPSILON);
  const styleFor = (feature: Feature<Geometry>, resolution?: number) => {
    const id = String(feature.getId() ?? "");
    return nativeFeatureStyle(feature, runtimeVectorLayers(), {
      hovered: id === hoveredId,
      selected: selectedIds.has(id),
      zoom: resolution && resolution > 0 ? Math.max(0, Math.log2(authoredWidth / 256 / resolution)) : undefined,
      labelsVisible: options.labelsVisible,
    });
  };

  const ensureVector = (layer: VectorLayerDefinition) => {
    let entry = vectorEntries.get(layer.id);
    if (!entry) {
      const source = new VectorSource({ wrapX: false });
      const olLayer = new VectorLayer({
        source,
        updateWhileAnimating: true,
        updateWhileInteracting: true,
        style(feature, resolution) {
          return styleFor(feature as Feature<Geometry>, resolution);
        },
      });
      olLayer.set("daenaLayerId", layer.id);
      entry = { layer: olLayer, source };
      vectorEntries.set(layer.id, entry);
    }
    entry.layer.setVisible(layer.defaultVisible);
    entry.layer.setOpacity(Math.max(0, Math.min(1, layer.opacity)));
    entry.layer.set("locked", layer.locked);
    return entry;
  };

  const ensureRaster = (layer: MapLayerDefinition, rasters: ReadonlyMap<string, RasterLayerSource>) => {
    if (!isRasterLayer(layer)) return null;
    const url = rasterUrl(rasters.get(layer.rasterAssetId));
    let olLayer = rasterEntries.get(layer.id);
    if (!olLayer) {
      olLayer = new ImageLayer({ visible: layer.defaultVisible });
      olLayer.set("daenaLayerId", layer.id);
      rasterEntries.set(layer.id, olLayer);
    }
    olLayer.setVisible(layer.defaultVisible);
    olLayer.setOpacity(Math.max(0, Math.min(1, layer.opacity)));
    olLayer.set("locked", layer.locked);
    if (url) {
      olLayer.setSource(
        new ImageStatic({
          url,
          projection,
          imageExtent: authoredExtentToViewExtent(extentOf(space), space),
          interpolate: true,
        }),
      );
    }
    return olLayer;
  };

  const featuresForLayer = (next: VectorFeatureCollection, layerId: string) =>
    codec.readOlFeatures({
      type: "FeatureCollection",
      features: next.features.filter((feature) => featureLayerId(feature) === layerId),
    });

  const applyCollection = (next: VectorFeatureCollection) => {
    for (const layer of runtimeVectorLayers()) {
      const entry = ensureVector(layer);
      entry.source.clear(true);
      entry.source.addFeatures(featuresForLayer(next, layer.id));
    }
    lastSignature = collectionSignature(next);
    snapSource.clear(true);
    snapSource.addFeatures(
      codec.readOlFeatures(snapTargetFeatures(next, currentLayers.filter(isVectorLayer), snapTargetLayerIds)),
    );
  };

  const orderedOlLayers = (rasters: ReadonlyMap<string, RasterLayerSource>): BaseLayer[] => {
    const ordered = [
      ...(currentLayers.some((layer) => layer.id === BASE_LAYER_ID) ? [] : [IMPLICIT_BASE_LAYER]),
      ...currentLayers,
    ].sort((left, right) => left.order - right.order || left.id.localeCompare(right.id));
    const keepVector = new Set(ordered.filter(isVectorLayer).map((layer) => layer.id));
    const keepRaster = new Set(ordered.filter(isRasterLayer).map((layer) => layer.id));
    for (const [id, entry] of vectorEntries) {
      if (!keepVector.has(id)) {
        entry.source.clear(true);
        vectorEntries.delete(id);
      }
    }
    for (const [id, layer] of rasterEntries) {
      if (!keepRaster.has(id)) {
        layer.setSource(null);
        rasterEntries.delete(id);
      }
    }
    const rendered: BaseLayer[] = [];
    ordered.forEach((layer, index) => {
      if (isVectorLayer(layer)) {
        const entry = ensureVector(layer);
        entry.layer.setZIndex(index);
        rendered.push(entry.layer);
        return;
      }
      const raster = ensureRaster(layer, rasters);
      if (!raster) return;
      raster.setZIndex(index);
      rendered.push(raster);
    });
    return rendered;
  };

  const registry: LayerRegistry = {
    group,
    snapSource,
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
    sourceFor(layerId) {
      return vectorEntries.get(layerId)?.source ?? null;
    },
    vectorOlLayers() {
      return runtimeVectorLayers()
        .filter((layer) => layer.defaultVisible)
        .flatMap((layer) => {
          const entry = vectorEntries.get(layer.id);
          return entry ? [entry.layer] : [];
        });
    },
    layerById(id) {
      return currentLayers.find((layer) => layer.id === id);
    },
    isSelectableVectorLayer(layer) {
      return runtimeVectorLayers().some(
        (daena) => daena.defaultVisible && vectorEntries.get(daena.id)?.layer === layer,
      );
    },
    getFeatureById(id) {
      for (const entry of vectorEntries.values()) {
        const feature = entry.source.getFeatureById(id) as Feature<Geometry> | null;
        if (feature) return feature;
      }
      return null;
    },
    forEachVectorFeature(callback) {
      for (const entry of vectorEntries.values()) {
        for (const feature of entry.source.getFeatures()) callback(feature as Feature<Geometry>);
      }
    },
    setHovered(id) {
      hoveredId = id;
      registry.refreshStyle();
    },
    sync(nextLayers, nextCollection, rasters = currentRasters) {
      currentLayers = [...nextLayers];
      registry.layers = currentLayers;
      currentRasters = new Map(rasters);
      group.setLayers(new Collection(orderedOlLayers(currentRasters)));
      if (collectionSignature(nextCollection) !== lastSignature) {
        selectedIds.clear();
        applyCollection(nextCollection);
      } else {
        registry.syncSnap(nextCollection);
      }
      registry.refreshStyle();
    },
    syncLayers(nextLayers) {
      currentLayers = [...nextLayers];
      registry.layers = currentLayers;
      group.setLayers(new Collection(orderedOlLayers(currentRasters)));
      registry.refreshStyle();
    },
    replaceCollection(next) {
      selectedIds.clear();
      applyCollection(next);
      registry.refreshStyle();
    },
    collectionFromLayers() {
      return codec.collectionFromSources(
        runtimeVectorLayers().map((layer) => vectorEntries.get(layer.id)?.source ?? new VectorSource()),
      );
    },
    syncSnap(next, targetIds = snapTargetLayerIds) {
      snapSource.clear(true);
      snapSource.addFeatures(
        codec.readOlFeatures(snapTargetFeatures(next, currentLayers.filter(isVectorLayer), targetIds)),
      );
    },
    setSnapTargetLayerIds(ids) {
      snapTargetLayerIds = new Set(ids);
      registry.syncSnap(
        codec.collectionFromSources(
          runtimeVectorLayers().map((layer) => vectorEntries.get(layer.id)?.source ?? new VectorSource()),
        ),
      );
    },
    refreshStyle() {
      for (const entry of vectorEntries.values()) entry.layer.changed();
    },
    dispose() {
      for (const entry of vectorEntries.values()) entry.source.clear(true);
      vectorEntries.clear();
      for (const layer of rasterEntries.values()) layer.setSource(null);
      rasterEntries.clear();
      snapSource.clear(true);
      group.setLayers(new Collection([]));
    },
  };

  registry.sync(layers, collection);
  return registry;
}
