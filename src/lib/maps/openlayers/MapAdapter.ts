import type Feature from "ol/Feature.js";
import type Geometry from "ol/geom/Geometry.js";
import Map from "ol/Map.js";
import View from "ol/View.js";
import { getCenter } from "ol/extent.js";
import { defaults as defaultInteractions } from "ol/interaction/defaults.js";
import "ol/ol.css";
import type { MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import { lonLatToNormalized, normalizedToLonLat } from "../native-vector/coordinates";
import {
  BASE_LAYER_ID,
  type VectorDrawMode,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "../native-vector/types";
import { createBackgroundRegistry, extentFromCoordinates, type MapBackground } from "./background-registry";
import { collectionFromSource, collectionSignature, toVectorFeature } from "./feature-codec";
import { anchorForFeature, featureAtPixel } from "./hit-testing";
import { createInteractionManager } from "./interaction-manager";
import { createLayerRegistry } from "./layer-registry";
import { WORLD_EXTENT, WORLD_RESOLUTIONS, worldProjection } from "./projection";
import { imageOverlayCoordinates } from "../native-vector/coordinates";
import { collectionBounds } from "./feature-codec";

export const RENDERER_UNAVAILABLE = "vector.renderer.unavailable";

const liveAdapters = new Set<MapAdapter>();

export function liveMapAdapterCount() {
  return liveAdapters.size;
}

/** @deprecated use liveMapAdapterCount */
export function liveNativeVectorEditorCount() {
  return liveMapAdapterCount();
}

export type MapAdapterView = { center: [number, number]; zoom: number };

export type MapAdapterCommandPayload =
  | { type: "replace-collection"; collection: VectorFeatureCollection; label?: string; coalesceKey?: string }
  | { type: "selection-ids"; ids: string[] };

export type MapAdapter = {
  setMode: (mode: VectorDrawMode) => void;
  switchLayer: (layerId: string) => void;
  syncDocument: (collection: VectorFeatureCollection, layers: readonly VectorLayerDefinition[]) => void;
  syncLayers: (layers: readonly VectorLayerDefinition[]) => void;
  setBackground: (background: MapBackground | null) => void;
  setBackgroundVisible: (visible: boolean) => void;
  applyView: (center: [number, number], zoom: number) => void;
  setZoom: (zoom: number) => void;
  panBy: (longitudeDegrees: number, latitudeDegrees: number) => void;
  resetView: () => void;
  focusFeature: (featureId: string) => boolean;
  focusPoint: (normalized: [number, number], zoom?: number) => void;
  flush: () => void;
  selectedFeatureIds: () => string[];
  selectedFeature: () => VectorFeature | null;
  resize: () => void;
  dispose: () => void;
};

function normalizedViewCenter(center: [number, number]): [number, number] {
  const [longitude, latitude] = normalizedToLonLat(center[0], center[1]);
  return [Math.max(-180, Math.min(180, longitude)), Math.max(-90, Math.min(90, latitude))];
}

export function createMapAdapter(
  container: HTMLElement,
  session: {
    draft: VectorFeatureCollection;
    layers: readonly VectorLayerDefinition[];
    activeLayerId: string | null;
    center: [number, number];
    zoom: number;
    setActiveLayerId: (id: string) => void;
    onCommand?: (payload: MapAdapterCommandPayload) => void;
    onDiagnostic?: (code: string, detail: string) => void;
    onSelect?: (feature: VectorFeature | null) => void;
    onSelectionChange?: (ids: string[]) => void;
    onDoubleClick?: (featureId: string) => void;
    pickArmed?: boolean;
    onMapPick?: (anchor: MapAnchor) => void;
    background?: MapBackground | null;
    initialView?: MapAdapterView | null;
    onViewChange?: (view: MapAdapterView) => void;
    /** When true, drawing/edit interactions are disabled (PhysicalWorldView). */
    readOnly?: boolean;
  },
): MapAdapter | { error: typeof RENDERER_UNAVAILABLE; detail: string } {
  let disposed = false;
  let activeLayerId = session.activeLayerId;
  const readOnly = session.readOnly === true;

  const registry = createLayerRegistry(session.draft, session.layers);
  const backgrounds = createBackgroundRegistry((detail) => {
    session.onDiagnostic?.(RENDERER_UNAVAILABLE, detail);
  });

  const view = new View({
    projection: worldProjection,
    center: session.initialView?.center ?? normalizedViewCenter(session.center),
    zoom: session.initialView?.zoom ?? session.zoom,
    minZoom: 0,
    maxZoom: 12,
    resolutions: WORLD_RESOLUTIONS,
    extent: WORLD_EXTENT,
    showFullExtent: true,
    constrainOnlyCenter: true,
  });

  let map: Map;
  try {
    map = new Map({
      target: container,
      layers: [backgrounds.layer, registry.vectorLayer],
      view,
      controls: [],
      interactions: defaultInteractions({ altShiftDragRotate: false, pinchRotate: false }),
    });
  } catch (cause) {
    return {
      error: RENDERER_UNAVAILABLE,
      detail: cause instanceof Error ? cause.message : "OpenLayers failed to create the map view.",
    };
  }

  const emitSelection = () => {
    registry.selectedIds.clear();
    for (const feature of interactions.select.getFeatures().getArray()) {
      registry.selectedIds.add(String(feature.getId() ?? ""));
    }
    registry.refreshStyle();
    const ids = [...registry.selectedIds];
    session.onSelectionChange?.(ids);
    const selected = interactions.select.getFeatures().item(0);
    session.onSelect?.(selected ? toVectorFeature(selected, activeLayerId ?? BASE_LAYER_ID) : null);
  };

  const commitSource = (label?: string, coalesceKey?: string) => {
    const collection = collectionFromSource(registry.source, activeLayerId ?? BASE_LAYER_ID);
    const nextSignature = collectionSignature(collection);
    if (nextSignature === registry.lastSignature) return;
    registry.lastSignature = nextSignature;
    registry.syncSnap(collection);
    session.onCommand?.({ type: "replace-collection", collection, label, coalesceKey });
  };

  const interactions = createInteractionManager({
    map,
    view,
    registry,
    getActiveLayerId: () => activeLayerId,
    getPickArmed: () => Boolean(session.pickArmed),
    readOnly,
    onSourceCommitted: () => commitSource("Edit geometry"),
    onSelectionChange: emitSelection,
    onDiagnostic: session.onDiagnostic,
  });

  const fitContent = () => {
    if (session.initialView) {
      view.setCenter(session.initialView.center);
      view.setZoom(session.initialView.zoom);
      return;
    }
    if (backgrounds.current) {
      const coordinates =
        backgrounds.current.coordinates ??
        imageOverlayCoordinates(backgrounds.current.width, backgrounds.current.height);
      view.fit(extentFromCoordinates(coordinates), { padding: [28, 28, 28, 28], maxZoom: 4, duration: 0 });
      return;
    }
    const extent = collectionBounds(session.draft);
    if (extent) view.fit(extent, { padding: [28, 28, 28, 28], maxZoom: 4, duration: 0 });
  };

  map.on("pointermove", (event) => {
    if (disposed || event.dragging || session.pickArmed) return;
    const next = featureAtPixel(map, registry.vectorLayer, event.pixel);
    const id = next ? String(next.getId() ?? "") : null;
    if (id !== registry.hoveredId) registry.setHovered(id);
  });
  map.on("singleclick", (event) => {
    if (session.pickArmed) {
      session.onMapPick?.(
        anchorForFeature(featureAtPixel(map, registry.vectorLayer, event.pixel), event.coordinate, activeLayerId ?? BASE_LAYER_ID),
      );
    }
  });
  map.on("dblclick", (event) => {
    if (session.pickArmed || interactions.currentMode() !== "static") return;
    const feature = featureAtPixel(map, registry.vectorLayer, event.pixel);
    if (feature?.getId() !== undefined) session.onDoubleClick?.(String(feature.getId()));
  });
  map.on("moveend", () => {
    const center = view.getCenter();
    if (center) session.onViewChange?.({ center: [center[0], center[1]], zoom: view.getZoom() ?? 0 });
  });

  backgrounds.setBackground(session.background ?? null);
  registry.syncSnap(session.draft);

  const resizeObserver = new ResizeObserver(() => {
    if (!disposed && container.clientWidth > 0 && container.clientHeight > 0) map.updateSize();
  });
  resizeObserver.observe(container);
  requestAnimationFrame(() => !disposed && map.updateSize());

  let adapter: MapAdapter;
  const onKeyDown = (event: KeyboardEvent) => {
    if (readOnly) return;
    if (event.key !== "Delete" && event.key !== "Backspace") return;
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
    event.preventDefault();
    const ids = adapter.selectedFeatureIds();
    if (ids.length === 0) return;
    // Host deletes via command stack; adapter only signals by removing and committing.
    for (const feature of [...interactions.select.getFeatures().getArray()]) {
      registry.source.removeFeature(feature);
    }
    interactions.select.getFeatures().clear();
    emitSelection();
    commitSource("Delete features");
  };
  container.addEventListener("keydown", onKeyDown);

  adapter = {
    setMode: interactions.configureMode,
    switchLayer(layerId) {
      interactions.select.getFeatures().clear();
      emitSelection();
      activeLayerId = layerId;
      interactions.setActiveLayerId(layerId);
      session.setActiveLayerId(layerId);
      interactions.configureMode("select");
    },
    syncDocument(collection, layers) {
      registry.syncLayers(layers);
      if (collectionSignature(collection) !== registry.lastSignature) {
        interactions.select.getFeatures().clear();
        registry.replaceCollection(collection);
        emitSelection();
      } else {
        registry.syncSnap(collection);
      }
      interactions.configureMode(interactions.currentMode());
    },
    syncLayers(layers) {
      registry.syncLayers(layers);
      registry.syncSnap(
        collectionFromSource(registry.source, activeLayerId ?? BASE_LAYER_ID),
      );
      interactions.configureMode(interactions.currentMode());
    },
    setBackground(background) {
      backgrounds.setBackground(background);
    },
    setBackgroundVisible(visible) {
      backgrounds.setVisible(visible);
    },
    applyView(center, zoom) {
      view.setCenter(normalizedViewCenter(center));
      view.setZoom(zoom);
    },
    setZoom: (zoom) => view.setZoom(zoom),
    panBy(longitudeDegrees, latitudeDegrees) {
      const center = view.getCenter() ?? [0, 0];
      view.setCenter([
        Math.max(-180, Math.min(180, center[0] + longitudeDegrees)),
        Math.max(-90, Math.min(90, center[1] + latitudeDegrees)),
      ]);
    },
    resetView: fitContent,
    focusFeature(featureId) {
      const feature = registry.source.getFeatureById(featureId) as Feature<Geometry> | null;
      const geometry = feature?.getGeometry();
      if (!feature || !geometry) return false;
      const extent = geometry.getExtent();
      if (extent[0] === extent[2] && extent[1] === extent[3]) {
        view.setCenter(getCenter(extent));
        view.setZoom(6);
      } else view.fit(extent, { padding: [48, 48, 48, 48], maxZoom: 8, duration: 0 });
      interactions.select.getFeatures().clear();
      interactions.select.getFeatures().push(feature);
      emitSelection();
      return true;
    },
    focusPoint(normalized, zoom = 4) {
      view.setCenter(normalizedViewCenter(normalized));
      view.setZoom(Math.max(2, zoom));
    },
    flush() {
      commitSource();
    },
    selectedFeatureIds() {
      return [...registry.selectedIds];
    },
    selectedFeature() {
      const selected = interactions.select.getFeatures().item(0);
      return selected ? toVectorFeature(selected, activeLayerId ?? BASE_LAYER_ID) : null;
    },
    resize: () => map.updateSize(),
    dispose() {
      if (disposed) return;
      disposed = true;
      resizeObserver.disconnect();
      container.removeEventListener("keydown", onKeyDown);
      interactions.dispose();
      map.setTarget(undefined);
      map.dispose();
      liveAdapters.delete(adapter);
    },
  };

  liveAdapters.add(adapter);
  return adapter;
}

/** Compatibility alias while callers migrate. */
export const createNativeVectorEditor = createMapAdapter;
export type NativeVectorEditor = MapAdapter;
export type NativeVectorBackground = MapBackground;
export type NativeVectorView = MapAdapterView;
