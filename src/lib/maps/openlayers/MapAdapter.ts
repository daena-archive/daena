import type Feature from "ol/Feature.js";
import type Geometry from "ol/geom/Geometry.js";
import Map from "ol/Map.js";
import View from "ol/View.js";
import { getCenter } from "ol/extent.js";
import { defaults as defaultInteractions } from "ol/interaction/defaults.js";
import "ol/ol.css";
import type { MapAnchor, MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps";
import {
  authoredToView,
  viewToAuthored,
} from "../editor/coordinate-space";
import {
  BASE_LAYER_ID,
  type VectorDrawMode,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "../native-vector/types";
import { createBackgroundRegistry, type RuntimeBackground } from "./background-registry";
import { collectionSignature, createFeatureCodec } from "./feature-codec";
import { anchorForFeature, featureAtPixel } from "./hit-testing";
import { createInteractionManager } from "./interaction-manager";
import { createLayerRegistry } from "./layer-registry";
import { bindMapLifecycle } from "./lifecycle";
import {
  maxZoomForCoordinateSpace,
  projectionFromCoordinateSpace,
  resolutionsForCoordinateSpace,
  viewExtentForCoordinateSpace,
} from "./projection";

export const RENDERER_UNAVAILABLE = "vector.renderer.unavailable";

const liveAdapters = new Set<MapAdapter>();

export function liveMapAdapterCount() {
  return liveAdapters.size;
}

/** @deprecated use liveMapAdapterCount */
export function liveNativeVectorEditorCount() {
  return liveMapAdapterCount();
}

export type MapAdapterView = { center: [number, number]; zoom: number; rotation: number };

export type MapAdapterCommandPayload =
  | { type: "replace-collection"; collection: VectorFeatureCollection; label?: string; coalesceKey?: string }
  | { type: "selection-ids"; ids: string[] }
  | { type: "set-view"; center: [number, number]; zoom: number; rotation: number };

export type MapAdapter = {
  setMode: (mode: VectorDrawMode) => void;
  switchLayer: (layerId: string) => void;
  syncDocument: (collection: VectorFeatureCollection, layers: readonly VectorLayerDefinition[]) => void;
  syncLayers: (layers: readonly VectorLayerDefinition[]) => void;
  syncBackgrounds: (backgrounds: readonly RuntimeBackground[]) => void;
  setBackground: (background: RuntimeBackground | null) => void;
  setBackgroundVisible: (visible: boolean) => void;
  applyView: (center: [number, number], zoom: number, rotation?: number) => void;
  setZoom: (zoom: number) => void;
  panBy: (dx: number, dy: number) => void;
  panCardinal: (x: number, y: number) => void;
  resetView: () => void;
  fitExtent: () => void;
  actualPixels: () => void;
  focusFeature: (featureId: string) => boolean;
  focusPoint: (authored: [number, number], zoom?: number) => void;
  flush: () => void;
  selectedFeatureIds: () => string[];
  selectedFeature: () => VectorFeature | null;
  resize: () => void;
  dispose: () => void;
};

export function createMapAdapter(
  container: HTMLElement,
  session: {
    draft: VectorFeatureCollection;
    layers: readonly VectorLayerDefinition[];
    activeLayerId: string | null;
    coordinateSpace: MapCoordinateSpace;
    setActiveLayerId: (id: string) => void;
    onCommand?: (payload: MapAdapterCommandPayload) => void;
    onDiagnostic?: (code: string, detail: string) => void;
    onSelect?: (feature: VectorFeature | null) => void;
    onSelectionChange?: (ids: string[]) => void;
    onDoubleClick?: (featureId: string) => void;
    pickArmed?: boolean;
    onMapPick?: (anchor: MapAnchor) => void;
    backgrounds?: readonly RuntimeBackground[];
    background?: RuntimeBackground | null;
    initialView?: MapAdapterView | null;
    onViewChange?: (view: MapAdapterView) => void;
    readOnly?: boolean;
  },
): MapAdapter | { error: typeof RENDERER_UNAVAILABLE; detail: string } {
  let disposed = false;
  let applyingView = false;
  let ignoreNextMoveEnd = true;
  let activeLayerId = session.activeLayerId;
  const readOnly = session.readOnly === true;
  const space = session.coordinateSpace;
  const projection = projectionFromCoordinateSpace(space);
  const codec = createFeatureCodec(space, projection);
  const extent = viewExtentForCoordinateSpace(space);
  const maxZoom = maxZoomForCoordinateSpace(space);

  const registry = createLayerRegistry(session.draft, session.layers, codec);
  const backgrounds = createBackgroundRegistry((detail) => {
    session.onDiagnostic?.(RENDERER_UNAVAILABLE, detail);
  });

  const initialCenter = session.initialView?.center ?? [(extent[0] + extent[2]) / 2, (extent[1] + extent[3]) / 2];
  const view = new View({
    projection,
    center: authoredToView(initialCenter, space),
    zoom: session.initialView?.zoom ?? 1,
    rotation: session.initialView?.rotation ?? 0,
    minZoom: 0,
    maxZoom,
    resolutions: resolutionsForCoordinateSpace(space, maxZoom + 1),
    extent,
    showFullExtent: true,
    constrainOnlyCenter: true,
  });

  let map: Map;
  try {
    map = new Map({
      target: container,
      layers: [backgrounds.group, registry.vectorLayer],
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

  const lifecycle = bindMapLifecycle(map, container);

  const emitSelection = () => {
    registry.selectedIds.clear();
    for (const feature of interactions.select.getFeatures().getArray()) {
      registry.selectedIds.add(String(feature.getId() ?? ""));
    }
    registry.refreshStyle();
    const ids = [...registry.selectedIds];
    session.onSelectionChange?.(ids);
    const selected = interactions.select.getFeatures().item(0);
    session.onSelect?.(selected ? codec.toVectorFeature(selected, activeLayerId ?? BASE_LAYER_ID) : null);
  };

  const commitSource = (label?: string, coalesceKey?: string) => {
    const collection = codec.collectionFromSource(registry.source, activeLayerId ?? BASE_LAYER_ID);
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
    codec,
    getActiveLayerId: () => activeLayerId,
    getPickArmed: () => Boolean(session.pickArmed),
    readOnly,
    onSourceCommitted: () => commitSource("Edit geometry"),
    onSelectionChange: emitSelection,
    onDiagnostic: session.onDiagnostic,
  });

  const currentBackgrounds = (): readonly RuntimeBackground[] => {
    if (session.backgrounds) return session.backgrounds;
    return session.background ? [session.background] : [];
  };

  const releaseViewGuard = () => {
    requestAnimationFrame(() => {
      applyingView = false;
    });
  };

  const fitContent = () => {
    applyingView = true;
    if (session.initialView && currentBackgrounds().length === 0 && session.draft.features.length === 0) {
      view.setCenter(authoredToView(session.initialView.center, space));
      view.setZoom(session.initialView.zoom);
      view.setRotation(session.initialView.rotation ?? 0);
      releaseViewGuard();
      return;
    }
    const rasters = currentBackgrounds();
    if (rasters.length > 0) {
      const xs = rasters.flatMap((item) => [item.extent[0], item.extent[2]]);
      const ys = rasters.flatMap((item) => [item.extent[1], item.extent[3]]);
      const authored: [number, number, number, number] = [
        Math.min(...xs),
        Math.min(...ys),
        Math.max(...xs),
        Math.max(...ys),
      ];
      const viewExtent = [
        ...authoredToView([authored[0], authored[1]], space),
        ...authoredToView([authored[2], authored[3]], space),
      ];
      const fitted: [number, number, number, number] = [
        Math.min(viewExtent[0], viewExtent[2]),
        Math.min(viewExtent[1], viewExtent[3]),
        Math.max(viewExtent[0], viewExtent[2]),
        Math.max(viewExtent[1], viewExtent[3]),
      ];
      view.fit(fitted, { padding: [28, 28, 28, 28], maxZoom, duration: 0 });
      releaseViewGuard();
      return;
    }
    const bounds = codec.collectionBounds(session.draft);
    if (bounds) view.fit(bounds, { padding: [28, 28, 28, 28], maxZoom: Math.min(8, maxZoom), duration: 0 });
    else {
      view.setCenter(authoredToView(initialCenter, space));
      view.setZoom(session.initialView?.zoom ?? 1);
    }
    releaseViewGuard();
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
        anchorForFeature(
          featureAtPixel(map, registry.vectorLayer, event.pixel),
          event.coordinate,
          activeLayerId ?? BASE_LAYER_ID,
          space,
          codec,
        ),
      );
    }
  });
  map.on("dblclick", (event) => {
    if (session.pickArmed || interactions.currentMode() !== "static") return;
    const feature = featureAtPixel(map, registry.vectorLayer, event.pixel);
    if (feature?.getId() !== undefined) session.onDoubleClick?.(String(feature.getId()));
  });
  map.on("moveend", () => {
    if (applyingView || ignoreNextMoveEnd) return;
    const center = view.getCenter();
    if (!center) return;
    const authored = viewToAuthored(center, space);
    session.onViewChange?.({
      center: authored,
      zoom: view.getZoom() ?? 0,
      rotation: view.getRotation() ?? 0,
    });
    session.onCommand?.({
      type: "set-view",
      center: authored,
      zoom: view.getZoom() ?? 0,
      rotation: view.getRotation() ?? 0,
    });
  });

  backgrounds.sync(currentBackgrounds(), space, projection);
  registry.syncSnap(session.draft);
  applyingView = true;
  if (!session.initialView) fitContent();
  else releaseViewGuard();
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      ignoreNextMoveEnd = false;
    });
  });

  let adapter: MapAdapter;
  const onKeyDown = (event: KeyboardEvent) => {
    if (readOnly) return;
    if (event.key !== "Delete" && event.key !== "Backspace") return;
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
    event.preventDefault();
    const ids = adapter.selectedFeatureIds();
    if (ids.length === 0) return;
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
      registry.syncSnap(codec.collectionFromSource(registry.source, activeLayerId ?? BASE_LAYER_ID));
      interactions.configureMode(interactions.currentMode());
    },
    syncBackgrounds(next) {
      backgrounds.sync(next, space, projection);
    },
    setBackground(background) {
      backgrounds.sync(background ? [background] : [], space, projection);
    },
    setBackgroundVisible(visible) {
      backgrounds.group.setVisible(visible);
    },
    applyView(center, zoom, rotation = 0) {
      applyingView = true;
      view.setCenter(authoredToView(center, space));
      view.setZoom(zoom);
      view.setRotation(rotation);
      releaseViewGuard();
    },
    setZoom: (zoom) => view.setZoom(zoom),
    panBy(dx, dy) {
      const center = view.getCenter() ?? authoredToView([(extent[0] + extent[2]) / 2, (extent[1] + extent[3]) / 2], space);
      const next = [center[0] + dx, center[1] + dy] as [number, number];
      view.setCenter(next);
    },
    panCardinal(x, y) {
      const [minX, minY, maxX, maxY] = extent;
      adapter.panBy(((maxX - minX) / 8) * x, ((maxY - minY) / 8) * y);
    },
    resetView: fitContent,
    fitExtent: fitContent,
    actualPixels() {
      applyingView = true;
      view.setResolution(1);
      releaseViewGuard();
    },
    focusFeature(featureId) {
      const feature = registry.source.getFeatureById(featureId) as Feature<Geometry> | null;
      const geometry = feature?.getGeometry();
      if (!feature || !geometry) return false;
      const featureExtent = geometry.getExtent();
      if (featureExtent[0] === featureExtent[2] && featureExtent[1] === featureExtent[3]) {
        view.setCenter(getCenter(featureExtent));
        view.setZoom(Math.min(6, maxZoom));
      } else view.fit(featureExtent, { padding: [48, 48, 48, 48], maxZoom: Math.min(8, maxZoom), duration: 0 });
      interactions.select.getFeatures().clear();
      interactions.select.getFeatures().push(feature);
      emitSelection();
      return true;
    },
    focusPoint(authored, zoom = 4) {
      view.setCenter(authoredToView(authored, space));
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
      return selected ? codec.toVectorFeature(selected, activeLayerId ?? BASE_LAYER_ID) : null;
    },
    resize: () => lifecycle.resize(),
    dispose() {
      if (disposed) return;
      disposed = true;
      container.removeEventListener("keydown", onKeyDown);
      interactions.dispose();
      backgrounds.dispose();
      lifecycle.dispose();
      liveAdapters.delete(adapter);
    },
  };

  liveAdapters.add(adapter);
  return adapter;
}

/** Compatibility alias while callers migrate. */
export const createNativeVectorEditor = createMapAdapter;
export type NativeVectorEditor = MapAdapter;
export type NativeVectorBackground = RuntimeBackground;
export type NativeVectorView = MapAdapterView;
