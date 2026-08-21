import {
  TerraDraw,
  TerraDrawFreehandMode,
  TerraDrawLineStringMode,
  TerraDrawPointMode,
  TerraDrawPolygonMode,
  TerraDrawSelectMode,
  TerraDrawSessionUndoRedo,
  type GeoJSONStoreFeatures,
} from "terra-draw";
import { TerraDrawMapLibreGLAdapter } from "terra-draw-maplibre-gl-adapter";
import type { CanvasSourceSpecification, GeoJSONSource, Map as MapLibreMap, MapLayerMouseEvent } from "maplibre-gl";
import maplibregl from "maplibre-gl/dist/maplibre-gl-csp.js";
import workerUrl from "maplibre-gl/dist/maplibre-gl-csp-worker.js?url";
import "maplibre-gl/dist/maplibre-gl.css";
import {
  UNDO_STACK_SIZE,
  type VectorDrawMode,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "./types";
import {
  AUTHORED_SOURCE_ID,
  BASE_SOURCE_ID,
  IMAGE_LAYER_ID,
  IMAGE_SOURCE_ID,
  nativeBaseLayerVisibility,
  nativeVectorStyle,
  splitVectorSources,
  styleContainsRemoteUrl,
} from "./style";
import { drawModeForGeometry, kindForDrawMode, simplifyFreehandGeometry } from "./geometry";
import { imageOverlayCoordinates, normalizedToLonLat, type ImageOverlayCoordinates } from "./coordinates";

if (typeof maplibregl.setWorkerUrl === "function") maplibregl.setWorkerUrl(workerUrl);

export const RENDERER_UNAVAILABLE = "vector.renderer.unavailable";
export { workerUrl };

const liveEditors = new Set<NativeVectorEditor>();

export function liveNativeVectorEditorCount() {
  return liveEditors.size;
}

export type NativeVectorBackground = {
  url: string;
  width: number;
  height: number;
  canvas?: HTMLCanvasElement;
  coordinates?: ImageOverlayCoordinates;
};

export type NativeVectorView = {
  center: [number, number];
  zoom: number;
  bearing: number;
  pitch: number;
};

export type NativeVectorEditor = {
  workerUrl: string;
  objectUrls: string[];
  setMode: (mode: VectorDrawMode) => void;
  switchLayer: (layerId: string) => void;
  syncLayers: (layers: readonly VectorLayerDefinition[]) => void;
  setBackground: (background: NativeVectorBackground | null) => void;
  setBackgroundVisible: (visible: boolean) => void;
  applyView: (center: [number, number], zoom: number) => void;
  setZoom: (zoom: number) => void;
  panBy: (longitudeDegrees: number, latitudeDegrees: number) => void;
  resetView: () => void;
  focusFeature: (featureId: string) => boolean;
  flush: () => void;
  deleteSelection: () => void;
  updateSelectedName: (name: string | null) => void;
  undo: () => void;
  redo: () => void;
  resize: () => void;
  dispose: () => void;
};

function webgl2Available() {
  try {
    const canvas = document.createElement("canvas");
    const context = canvas.getContext("webgl2");
    context?.getExtension("WEBGL_lose_context")?.loseContext();
    return Boolean(context);
  } catch {
    return false;
  }
}

function asVectorFeature(feature: GeoJSONStoreFeatures, layerId: string): VectorFeature | null {
  if (
    feature.geometry.type !== "Point" &&
    feature.geometry.type !== "LineString" &&
    feature.geometry.type !== "Polygon"
  ) {
    return null;
  }
  const existingKind = feature.properties?.kind;
  const kind =
    existingKind === "land" ||
    existingKind === "lake" ||
    existingKind === "region" ||
    existingKind === "route" ||
    existingKind === "marker" ||
    existingKind === "custom"
      ? existingKind
      : kindForDrawMode(drawModeForGeometry(feature.geometry));
  return {
    type: "Feature",
    id: String(feature.id ?? crypto.randomUUID()),
    properties: {
      daenaLayerId: typeof feature.properties?.daenaLayerId === "string" ? feature.properties.daenaLayerId : layerId,
      kind,
      name: typeof feature.properties?.name === "string" ? feature.properties.name : null,
    },
    geometry: feature.geometry,
  };
}

function toStoreFeature(feature: VectorFeature): GeoJSONStoreFeatures | null {
  if (
    feature.geometry.type !== "Point" &&
    feature.geometry.type !== "LineString" &&
    feature.geometry.type !== "Polygon"
  ) {
    return null;
  }
  return {
    type: "Feature",
    id: feature.id,
    properties: {
      mode: drawModeForGeometry(feature.geometry),
      daenaLayerId: feature.properties.daenaLayerId,
      kind: feature.properties.kind,
      name: feature.properties.name,
    },
    geometry: feature.geometry,
  };
}

function collectionBounds(collection: VectorFeatureCollection): [[number, number], [number, number]] | null {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  const visit = (position: unknown) => {
    if (!Array.isArray(position)) return;
    if (typeof position[0] === "number" && typeof position[1] === "number") {
      minX = Math.min(minX, position[0]);
      minY = Math.min(minY, position[1]);
      maxX = Math.max(maxX, position[0]);
      maxY = Math.max(maxY, position[1]);
      return;
    }
    for (const item of position) visit(item);
  };
  for (const feature of collection.features) visit(feature.geometry.coordinates);
  if (!Number.isFinite(minX) || (minX === maxX && minY === maxY)) return null;
  return [
    [minX, minY],
    [maxX, maxY],
  ];
}

function featureBounds(feature: VectorFeature): [[number, number], [number, number]] {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  const visit = (position: unknown) => {
    if (!Array.isArray(position)) return;
    if (typeof position[0] === "number" && typeof position[1] === "number") {
      minX = Math.min(minX, position[0]);
      minY = Math.min(minY, position[1]);
      maxX = Math.max(maxX, position[0]);
      maxY = Math.max(maxY, position[1]);
      return;
    }
    for (const item of position) visit(item);
  };
  visit(feature.geometry.coordinates);
  if (!Number.isFinite(minX))
    return [
      [0, 0],
      [0, 0],
    ];
  return [
    [minX, minY],
    [maxX, maxY],
  ];
}

export function createNativeVectorEditor(
  container: HTMLElement,
  session: {
    draft: VectorFeatureCollection;
    layers: readonly VectorLayerDefinition[];
    activeLayerId: string | null;
    center: [number, number];
    zoom: number;
    setDraft: (next: VectorFeatureCollection) => void;
    setActiveLayerId: (id: string) => void;
    onDirty?: () => void;
    onDiagnostic?: (code: string, detail: string) => void;
    onSelect?: (feature: VectorFeature | null) => void;
    onDoubleClick?: (featureId: string) => void;
    background?: NativeVectorBackground | null;
    projection?: "mercator" | "globe";
    initialView?: NativeVectorView | null;
    onViewChange?: (view: NativeVectorView) => void;
  },
): NativeVectorEditor | { error: typeof RENDERER_UNAVAILABLE; detail: string } {
  if (!webgl2Available()) {
    return {
      error: RENDERER_UNAVAILABLE,
      detail: "WebGL2 is required for native vector maps and is not available.",
    };
  }

  const style = nativeVectorStyle(session.layers);
  if (session.projection === "globe") style.projection = { type: "globe" };
  if (styleContainsRemoteUrl(style)) {
    return { error: RENDERER_UNAVAILABLE, detail: "Native vector style must not request remote URLs." };
  }

  const globe = session.projection === "globe";
  const [longitude, latitude] = normalizedToLonLat(session.center[0], session.center[1]);
  let map: MapLibreMap;
  try {
    map = new maplibregl.Map({
      container,
      style,
      center: session.initialView?.center ?? (globe ? [0, 0] : [longitude, latitude]),
      zoom: session.initialView?.zoom ?? (globe ? 0 : session.zoom),
      bearing: session.initialView?.bearing ?? 0,
      pitch: session.initialView?.pitch ?? 0,
      renderWorldCopies: globe,
      attributionControl: false,
      minZoom: globe ? 0 : undefined,
      maxZoom: globe ? 8 : undefined,
      maxPitch: globe ? 85 : 0,
      pitchWithRotate: globe,
      fadeDuration: 0,
      canvasContextAttributes: globe ? { antialias: true } : undefined,
      transformRequest(url) {
        if (/^https?:\/\//i.test(url) && !url.startsWith(globalThis.location.origin)) {
          throw new Error("Native vector maps reject remote tile, glyph, sprite, and telemetry URLs");
        }
        return { url };
      },
    });
  } catch (cause) {
    return {
      error: RENDERER_UNAVAILABLE,
      detail: cause instanceof Error ? cause.message : "MapLibre failed to create a WebGL2 context.",
    };
  }

  let draw: TerraDraw | null = null;
  let disposed = false;
  let styleInitialized = false;
  const objectUrls: string[] = [];
  let hoveredId: string | number | null = null;
  let mapSelectedId: string | number | null = null;
  let terraSelectedId: string | number | null = null;
  let currentBackground = session.background ?? null;
  let backgroundVisible = true;
  let preservedLayerIds = new Set<string>();

  const styleNotLoaded = (error: unknown) =>
    /style is not done loading/i.test(error instanceof Error ? error.message : String(error));

  const whenStyleReady = (run: () => void) => {
    const attempt = () => {
      if (disposed) return;
      try {
        run();
      } catch (error) {
        if (!styleNotLoaded(error)) throw error;
        map.once("style.load", attempt);
        map.once("idle", attempt);
      }
    };
    if (map.isStyleLoaded()) attempt();
    else {
      map.once("style.load", attempt);
      map.once("idle", attempt);
    }
  };

  const clearFeatureState = (id: string | number | null, key: "hover" | "selected") => {
    if (id === null) return;
    try {
      map.removeFeatureState({ source: AUTHORED_SOURCE_ID, id }, key);
    } catch {
      // Source may already have been removed during teardown.
    }
  };

  const setMapSelection = (id: string | number | null) => {
    clearFeatureState(mapSelectedId, "selected");
    mapSelectedId = id;
    if (id !== null) {
      try {
        map.setFeatureState({ source: AUTHORED_SOURCE_ID, id }, { selected: true });
      } catch {
        // Feature may live only in Terra Draw for the active layer.
      }
    }
  };

  const emitSelect = (feature: VectorFeature | null) => {
    session.onSelect?.(feature);
  };

  const terraLayerId = () => {
    const layer = session.layers.find((item) => item.id === session.activeLayerId);
    if (!layer || layer.locked) return null;
    return layer.id;
  };

  const applySources = (activeLayerId: string | null) => {
    const editing = terraLayerId() === activeLayerId ? activeLayerId : null;
    const split = splitVectorSources(session.draft, editing);
    (map.getSource(BASE_SOURCE_ID) as GeoJSONSource | undefined)?.setData(split.base);
    (map.getSource(AUTHORED_SOURCE_ID) as GeoJSONSource | undefined)?.setData(split.authored);
  };

  const applyBackground = () => {
    const background = currentBackground;
    if (!background || disposed || map.getSource(IMAGE_SOURCE_ID)) return;
    const coordinates = background.coordinates ?? imageOverlayCoordinates(background.width, background.height);
    if (background.canvas) {
      const source: CanvasSourceSpecification = {
        type: "canvas",
        canvas: background.canvas,
        coordinates,
        animate: false,
      };
      map.addSource(IMAGE_SOURCE_ID, source);
    } else {
      map.addSource(IMAGE_SOURCE_ID, {
        type: "image",
        url: background.url,
        coordinates,
      });
    }
    if (map.getLayer("daena-base-fill")) {
      map.addLayer({ id: IMAGE_LAYER_ID, type: "raster", source: IMAGE_SOURCE_ID }, "daena-base-fill");
    } else {
      map.addLayer({ id: IMAGE_LAYER_ID, type: "raster", source: IMAGE_SOURCE_ID });
    }
    map.setLayoutProperty(IMAGE_LAYER_ID, "visibility", backgroundVisible ? "visible" : "none");
  };

  const replaceBackground = () => {
    if (map.getLayer(IMAGE_LAYER_ID)) map.removeLayer(IMAGE_LAYER_ID);
    if (map.getSource(IMAGE_SOURCE_ID)) map.removeSource(IMAGE_SOURCE_ID);
    applyBackground();
  };

  const wrapLon = (value: number) => ((((value + 180) % 360) + 360) % 360) - 180;
  const clampLat = (value: number) => Math.max(-85, Math.min(85, value));
  let lookAt = {
    lng: session.initialView?.center[0] ?? (globe ? 0 : longitude),
    lat: session.initialView?.center[1] ?? (globe ? 0 : latitude),
  };
  let applyingLookAt = false;

  const applyLookAt = (zoom = map.getZoom()) => {
    if (disposed) return;
    applyingLookAt = true;
    map.jumpTo({
      center: [lookAt.lng, lookAt.lat],
      zoom,
      bearing: map.getBearing(),
      pitch: map.getPitch(),
    });
    map.setCenter([lookAt.lng, lookAt.lat]);
    applyingLookAt = false;
    session.onViewChange?.({
      center: [lookAt.lng, lookAt.lat],
      zoom: map.getZoom(),
      bearing: map.getBearing(),
      pitch: map.getPitch(),
    });
  };

  const emitView = () => {
    if (disposed || applyingLookAt) return;
    const center = map.getCenter();
    lookAt = { lng: wrapLon(center.lng), lat: clampLat(center.lat) };
    session.onViewChange?.({
      center: [lookAt.lng, lookAt.lat],
      zoom: map.getZoom(),
      bearing: map.getBearing(),
      pitch: map.getPitch(),
    });
  };

  const fitContent = () => {
    if (disposed) return;
    if (session.initialView) {
      map.jumpTo({
        center: session.initialView.center,
        zoom: session.initialView.zoom,
        bearing: session.initialView.bearing,
        pitch: session.initialView.pitch,
      });
      return;
    }
    if (session.projection === "globe") {
      map.jumpTo({ center: [0, 0], zoom: 0, pitch: 0, bearing: 0 });
      return;
    }
    if (currentBackground) {
      const [northWest, northEast, southEast, southWest] =
        currentBackground.coordinates ?? imageOverlayCoordinates(currentBackground.width, currentBackground.height);
      map.fitBounds(
        [
          [
            Math.min(northWest[0], southWest[0], northEast[0], southEast[0]),
            Math.min(northWest[1], southWest[1], northEast[1], southEast[1]),
          ],
          [
            Math.max(northWest[0], southWest[0], northEast[0], southEast[0]),
            Math.max(northWest[1], southWest[1], northEast[1], southEast[1]),
          ],
        ],
        { padding: 28, duration: 0, maxZoom: 4 },
      );
      return;
    }
    const bounds = collectionBounds(session.draft);
    if (bounds) map.fitBounds(bounds, { padding: 28, duration: 0, maxZoom: 4 });
  };

  const mergeDrawIntoDraft = () => {
    const layerId = terraLayerId();
    if (!draw || !layerId) return;
    const kept = session.draft.features.filter(
      (feature) => feature.properties.daenaLayerId !== layerId || preservedLayerIds.has(feature.id),
    );
    const drawn: VectorFeature[] = [];
    for (const feature of draw.getSnapshot()) {
      const converted = asVectorFeature(feature, layerId);
      if (converted) drawn.push(converted);
    }
    session.setDraft({ type: "FeatureCollection", features: [...kept, ...drawn] });
    applySources(layerId);
  };

  const loadActiveLayer = () => {
    const layerId = terraLayerId();
    if (!draw || !layerId) return;
    preservedLayerIds = new Set<string>();
    const storeFeatures: GeoJSONStoreFeatures[] = [];
    for (const feature of session.draft.features) {
      if (feature.properties.daenaLayerId !== layerId) continue;
      const store = toStoreFeature(feature);
      if (store) storeFeatures.push(store);
      else preservedLayerIds.add(feature.id);
    }
    if (storeFeatures.length) draw.addFeatures(storeFeatures);
  };

  const onChange = (ids: (string | number)[], type: string) => {
    if (!draw) return;
    if (type === "create") {
      for (const id of ids) {
        const feature = draw.getSnapshotFeature(id);
        const count =
          feature?.geometry.type === "LineString"
            ? feature.geometry.coordinates.length
            : feature?.geometry.type === "Polygon"
              ? feature.geometry.coordinates.reduce((sum, ring) => sum + ring.length, 0)
              : 1;
        if (count > 8192) {
          draw.removeFeatures([id]);
          session.onDiagnostic?.(
            "vector.limit.exceeded",
            "Freehand drawing stopped because it exceeded 8,192 positions.",
          );
        }
      }
    }
    if (type === "create" || type === "update" || type === "delete") {
      mergeDrawIntoDraft();
      session.onDirty?.();
    }
  };

  const onSelect = (id: string | number) => {
    terraSelectedId = id;
    setMapSelection(null);
    const snapshot = draw?.getSnapshotFeature(id);
    const converted = snapshot ? asVectorFeature(snapshot, session.activeLayerId ?? "") : null;
    emitSelect(converted);
  };

  const onDeselect = () => {
    terraSelectedId = null;
    emitSelect(null);
  };

  const onFinish = (id: string | number, context: { mode?: string }) => {
    if (!draw) return;
    const snapshot = draw.getSnapshotFeature(id);
    if (!snapshot) return;
    const mode = context.mode ?? (typeof snapshot.properties?.mode === "string" ? snapshot.properties.mode : undefined);
    let geometry = snapshot.geometry as VectorFeature["geometry"];
    if (mode === "freehand") {
      const simplified = simplifyFreehandGeometry(geometry, map.getZoom());
      if ("error" in simplified) {
        draw.removeFeatures([id]);
        session.onDiagnostic?.(simplified.error, "Freehand geometry exceeded the editor budget or was invalid.");
        return;
      }
      geometry = simplified;
      if (geometry.type !== "Point" && geometry.type !== "LineString" && geometry.type !== "Polygon") return;
      draw.updateFeatureGeometry(id, geometry);
    }
    const layerId = terraLayerId();
    if (!layerId) return;
    const existingKind = snapshot.properties?.kind;
    draw.updateFeatureProperties(id, {
      daenaLayerId: layerId,
      kind:
        typeof existingKind === "string"
          ? existingKind
          : kindForDrawMode(mode === "freehand" ? "freehand" : drawModeForGeometry(geometry)),
      name: typeof snapshot.properties?.name === "string" ? snapshot.properties.name : null,
    });
    mergeDrawIntoDraft();
    session.onDirty?.();
  };

  const startDraw = () => {
    whenStyleReady(() => {
      if (disposed || draw) return;
      draw = new TerraDraw({
        adapter: new TerraDrawMapLibreGLAdapter({ map, coordinatePrecision: 6 }),
        idStrategy: {
          isValidId: (candidate) => typeof candidate === "string",
          getId: () => crypto.randomUUID(),
        },
        undoRedo: { sessionLevel: new TerraDrawSessionUndoRedo({ maxStackSize: UNDO_STACK_SIZE }) },
        modes: [
          new TerraDrawSelectMode({
            keyEvents: { deselect: "Escape", delete: "Delete", rotate: null, scale: null },
            flags: {
              point: { feature: { draggable: true } },
              linestring: {
                feature: { draggable: true, coordinates: { midpoints: true, draggable: true, deletable: true } },
              },
              polygon: {
                feature: { draggable: true, coordinates: { midpoints: true, draggable: true, deletable: true } },
              },
              freehand: {
                feature: { draggable: true, coordinates: { midpoints: true, draggable: true, deletable: true } },
              },
            },
          }),
          new TerraDrawPointMode(),
          new TerraDrawLineStringMode(),
          new TerraDrawPolygonMode(),
          new TerraDrawFreehandMode(),
        ],
      });
      applySources(session.activeLayerId);
      draw.start();
      loadActiveLayer();
      draw.setMode(terraLayerId() ? "select" : "static");
      draw.on("change", onChange);
      draw.on("finish", onFinish);
      draw.on("select", onSelect);
      draw.on("deselect", onDeselect);
    });
  };

  const onHover = (event: MapLayerMouseEvent) => {
    if (disposed) return;
    const layerIds = (map.getStyle()?.layers ?? [])
      .map((layer) => layer.id)
      .filter((id) => id.startsWith("daena-vector-"));
    const hit = layerIds.length ? map.queryRenderedFeatures(event.point, { layers: layerIds }) : [];
    const id = hit[0]?.id ?? null;
    if (hoveredId !== null && hoveredId !== id) clearFeatureState(hoveredId, "hover");
    hoveredId = id;
    if (id !== null) {
      try {
        map.setFeatureState({ source: AUTHORED_SOURCE_ID, id }, { hover: true });
      } catch {
        hoveredId = null;
      }
    }
  };

  const onMapClick = (event: MapLayerMouseEvent) => {
    if (disposed || terraLayerId()) return;
    const layerIds = (map.getStyle()?.layers ?? [])
      .map((layer) => layer.id)
      .filter((id) => id.startsWith("daena-vector-"));
    const hit = layerIds.length ? map.queryRenderedFeatures(event.point, { layers: layerIds }) : [];
    const id = hit[0]?.id;
    if (id === undefined) {
      setMapSelection(null);
      emitSelect(null);
      return;
    }
    setMapSelection(id);
    const feature = session.draft.features.find((item) => item.id === String(id)) ?? null;
    emitSelect(feature);
  };

  const onMapDoubleClick = (event: MapLayerMouseEvent) => {
    if (disposed || terraLayerId()) return;
    const layerIds = (map.getStyle()?.layers ?? [])
      .map((layer) => layer.id)
      .filter((id) => id.startsWith("daena-vector-"));
    const hit = layerIds.length ? map.queryRenderedFeatures(event.point, { layers: layerIds }) : [];
    if (hit[0]?.id !== undefined) session.onDoubleClick?.(String(hit[0].id));
  };

  const onStyleLoad = () => {
    if (disposed || styleInitialized) return;
    styleInitialized = true;
    map.resize();
    applyBackground();
    applySources(session.activeLayerId);
    fitContent();
    startDraw();
  };

  map.on("style.load", onStyleLoad);
  if (map.isStyleLoaded()) onStyleLoad();
  map.on("mousemove", onHover);
  map.on("click", onMapClick);
  map.on("dblclick", onMapDoubleClick);
  map.on("moveend", emitView);
  map.on("error", (event) => {
    const message = event.error?.message ?? "MapLibre renderer error";
    if (/webgl/i.test(message) || /context/i.test(message)) {
      session.onDiagnostic?.(RENDERER_UNAVAILABLE, message);
    }
  });

  const resizeObserver = new ResizeObserver(() => {
    if (disposed || container.clientWidth <= 0 || container.clientHeight <= 0) return;
    map.resize();
  });
  resizeObserver.observe(container);
  requestAnimationFrame(() => {
    if (!disposed) map.resize();
  });

  const editor: NativeVectorEditor = {
    workerUrl,
    objectUrls,
    setMode(mode) {
      if (!terraLayerId() && mode !== "static" && mode !== "select") {
        draw?.setMode("static");
        return;
      }
      draw?.setMode(mode === "static" ? "static" : mode);
    },
    switchLayer(layerId) {
      if (draw?.enabled) {
        mergeDrawIntoDraft();
        draw.off("change", onChange);
        draw.off("finish", onFinish);
        draw.off("select", onSelect);
        draw.off("deselect", onDeselect);
        draw.stop();
        draw = null;
      }
      terraSelectedId = null;
      setMapSelection(null);
      emitSelect(null);
      session.setActiveLayerId(layerId);
      applySources(layerId);
      startDraw();
    },
    syncLayers(layers) {
      whenStyleReady(() => {
        const generated = nativeVectorStyle(layers);
        if (styleContainsRemoteUrl(generated)) {
          session.onDiagnostic?.(RENDERER_UNAVAILABLE, "Native vector style must not request remote URLs.");
          return;
        }
        const specs = new Map(generated.layers.map((layer) => [layer.id, layer]));
        for (const layer of map.getStyle()?.layers ?? []) {
          if (!layer.id.startsWith("daena-vector-")) continue;
          const next = specs.get(layer.id);
          if (!next) {
            map.removeLayer(layer.id);
            continue;
          }
          const visibility =
            next.layout && "visibility" in next.layout && next.layout.visibility === "none" ? "none" : "visible";
          map.setLayoutProperty(layer.id, "visibility", visibility);
          for (const [key, value] of Object.entries(next.paint ?? {})) {
            if (!Object.is((layer as { paint?: Record<string, unknown> }).paint?.[key], value)) {
              map.setPaintProperty(layer.id, key, value);
            }
          }
          specs.delete(layer.id);
        }
        for (const layer of specs.values()) {
          if (!layer.id.startsWith("daena-vector-")) continue;
          if (map.getLayer("daena-hover-fill")) map.addLayer(layer, "daena-hover-fill");
          else map.addLayer(layer);
        }
        const vectorLayerIds = generated.layers
          .filter((layer) => layer.id.startsWith("daena-vector-"))
          .map((layer) => layer.id);
        for (let index = vectorLayerIds.length - 1; index >= 0; index -= 1) {
          const id = vectorLayerIds[index];
          if (!map.getLayer(id)) continue;
          const beforeId = index + 1 < vectorLayerIds.length ? vectorLayerIds[index + 1] : "daena-hover-fill";
          if (map.getLayer(beforeId)) map.moveLayer(id, beforeId);
          else map.moveLayer(id);
        }
        const baseVisibility = nativeBaseLayerVisibility(layers);
        if (map.getLayer("daena-base-fill")) {
          map.setLayoutProperty("daena-base-fill", "visibility", baseVisibility);
        }
        if (map.getLayer("daena-base-line")) {
          map.setLayoutProperty("daena-base-line", "visibility", baseVisibility);
        }
        const active = layers.find((layer) => layer.id === session.activeLayerId);
        const terraRendered = active ? active.defaultVisible : false;
        for (const id of ["td-point", "td-point-marker", "td-linestring", "td-polygon", "td-polygon-outline"]) {
          if (map.getLayer(id)) {
            map.setLayoutProperty(id, "visibility", terraRendered ? "visible" : "none");
          }
        }
        applySources(session.activeLayerId);
      });
    },
    setBackground(background) {
      currentBackground = background;
      whenStyleReady(replaceBackground);
    },
    setBackgroundVisible(visible) {
      backgroundVisible = visible;
      whenStyleReady(() => {
        if (map.getLayer(IMAGE_LAYER_ID)) {
          map.setLayoutProperty(IMAGE_LAYER_ID, "visibility", visible ? "visible" : "none");
        }
      });
    },
    applyView(center, zoom) {
      const [lon, lat] = normalizedToLonLat(center[0], center[1]);
      lookAt = { lng: lon, lat: lat };
      applyLookAt(zoom);
    },
    setZoom(zoom) {
      applyLookAt(zoom);
    },
    panBy(longitudeDegrees, latitudeDegrees) {
      lookAt = {
        lng: wrapLon(lookAt.lng + longitudeDegrees),
        lat: clampLat(lookAt.lat + latitudeDegrees),
      };
      applyLookAt();
    },
    resetView() {
      fitContent();
    },
    focusFeature(featureId) {
      const feature = session.draft.features.find((item) => item.id === featureId);
      if (!feature) return false;
      const bounds = featureBounds(feature);
      try {
        map.fitBounds(bounds, { padding: 48, duration: 0, maxZoom: 8 });
      } catch {
        map.jumpTo({ center: bounds[0], zoom: 4 });
      }
      setMapSelection(feature.id);
      emitSelect(feature);
      return true;
    },
    flush() {
      mergeDrawIntoDraft();
    },
    deleteSelection() {
      const id = terraSelectedId;
      if (!draw || id === null) return;
      draw.removeFeatures([id]);
      terraSelectedId = null;
      mergeDrawIntoDraft();
      session.onDirty?.();
      emitSelect(null);
    },
    updateSelectedName(name) {
      const id = terraSelectedId;
      if (!draw || id === null) return;
      draw.updateFeatureProperties(id, { name });
      mergeDrawIntoDraft();
      session.onDirty?.();
      const snapshot = draw.getSnapshotFeature(id);
      emitSelect(snapshot ? asVectorFeature(snapshot, session.activeLayerId ?? "") : null);
    },
    undo() {
      const before = JSON.stringify(session.draft);
      draw?.undo();
      mergeDrawIntoDraft();
      if (JSON.stringify(session.draft) !== before) session.onDirty?.();
    },
    resize() {
      map.resize();
    },
    redo() {
      const before = JSON.stringify(session.draft);
      draw?.redo();
      mergeDrawIntoDraft();
      if (JSON.stringify(session.draft) !== before) session.onDirty?.();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      resizeObserver.disconnect();
      map.off("style.load", onStyleLoad);
      map.off("mousemove", onHover);
      map.off("click", onMapClick);
      map.off("dblclick", onMapDoubleClick);
      map.off("moveend", emitView);
      clearFeatureState(hoveredId, "hover");
      clearFeatureState(mapSelectedId, "selected");
      try {
        draw?.off("change", onChange);
        draw?.off("finish", onFinish);
        draw?.off("select", onSelect);
        draw?.off("deselect", onDeselect);
        draw?.stop();
      } catch {
        // Terra Draw may already have been stopped during a layer switch.
      }
      draw = null;
      try {
        map.remove();
      } catch {
        // MapLibre may already have lost its WebGL context.
      }
      for (const url of objectUrls) URL.revokeObjectURL(url);
      objectUrls.length = 0;
      liveEditors.delete(editor);
    },
  };

  liveEditors.add(editor);
  return editor;
}
