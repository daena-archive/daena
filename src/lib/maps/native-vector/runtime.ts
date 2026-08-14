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
import type { GeoJSONSource, Map as MapLibreMap, MapLayerMouseEvent } from "maplibre-gl";
import maplibregl from "maplibre-gl/dist/maplibre-gl-csp.js";
import workerUrl from "maplibre-gl/dist/maplibre-gl-csp-worker.js?url";
import "maplibre-gl/dist/maplibre-gl.css";
import { UNDO_STACK_SIZE, type VectorDrawMode, type VectorFeature, type VectorFeatureCollection, type VectorLayerDefinition } from "./types";
import {
  AUTHORED_SOURCE_ID,
  BASE_SOURCE_ID,
  IMAGE_LAYER_ID,
  IMAGE_SOURCE_ID,
  nativeVectorStyle,
  splitVectorSources,
  styleContainsRemoteUrl,
} from "./style";
import { drawModeForGeometry, kindForDrawMode, simplifyFreehandGeometry } from "./geometry";
import { imageOverlayCoordinates, normalizedToLonLat } from "./coordinates";

if (typeof maplibregl.setWorkerUrl === "function") maplibregl.setWorkerUrl(workerUrl);

export const RENDERER_UNAVAILABLE = "vector.renderer.unavailable";
export { workerUrl };

const liveEditors = new Set<NativeVectorEditor>();

export function liveNativeVectorEditorCount() {
  return liveEditors.size;
}

export type NativeVectorEditor = {
  workerUrl: string;
  objectUrls: string[];
  setMode: (mode: VectorDrawMode) => void;
  switchLayer: (layerId: string) => void;
  syncLayers: (layers: readonly VectorLayerDefinition[]) => void;
  applyView: (center: [number, number], zoom: number) => void;
  flush: () => void;
  deleteSelection: () => void;
  updateSelectedName: (name: string | null) => void;
  undo: () => void;
  redo: () => void;
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

function toStoreFeature(feature: VectorFeature): GeoJSONStoreFeatures {
  const geometry =
    feature.geometry.type === "MultiPolygon"
      ? { type: "Polygon" as const, coordinates: feature.geometry.coordinates[0] }
      : feature.geometry;
  return {
    type: "Feature",
    id: feature.id,
    properties: {
      mode: drawModeForGeometry(geometry),
      daenaLayerId: feature.properties.daenaLayerId,
      kind: feature.properties.kind,
      name: feature.properties.name,
    },
    geometry,
  };
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
    background?: { url: string; width: number; height: number } | null;
  },
): NativeVectorEditor | { error: typeof RENDERER_UNAVAILABLE; detail: string } {
  if (!webgl2Available()) {
    return {
      error: RENDERER_UNAVAILABLE,
      detail: "WebGL2 is required for native vector maps and is not available.",
    };
  }

  const style = nativeVectorStyle(session.layers);
  if (styleContainsRemoteUrl(style)) {
    return { error: RENDERER_UNAVAILABLE, detail: "Native vector style must not request remote URLs." };
  }

  const [longitude, latitude] = normalizedToLonLat(session.center[0], session.center[1]);
  let map: MapLibreMap;
  try {
    map = new maplibregl.Map({
      container,
      style,
      center: [longitude, latitude],
      zoom: session.zoom,
      attributionControl: false,
      maxPitch: 0,
      pitchWithRotate: false,
      fadeDuration: 0,
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
  const objectUrls: string[] = [];
  if (session.background?.url) objectUrls.push(session.background.url);
  let hoveredId: string | number | null = null;
  let mapSelectedId: string | number | null = null;
  let terraSelectedId: string | number | null = null;

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

  const applyBackground = () => {
    const background = session.background;
    if (!background || disposed) return;
    whenStyleReady(() => {
      if (disposed || !session.background) return;
      if (map.getLayer(IMAGE_LAYER_ID)) map.removeLayer(IMAGE_LAYER_ID);
      if (map.getSource(IMAGE_SOURCE_ID)) map.removeSource(IMAGE_SOURCE_ID);
      map.addSource(IMAGE_SOURCE_ID, {
        type: "image",
        url: background.url,
        coordinates: imageOverlayCoordinates(background.width, background.height),
      });
      if (map.getLayer("daena-base-fill")) {
        map.addLayer({ id: IMAGE_LAYER_ID, type: "raster", source: IMAGE_SOURCE_ID }, "daena-base-fill");
      } else {
        map.addLayer({ id: IMAGE_LAYER_ID, type: "raster", source: IMAGE_SOURCE_ID });
      }
    });
  };
  map.on("style.load", applyBackground);

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

  const mergeDrawIntoDraft = () => {
    const layerId = terraLayerId();
    if (!draw || !layerId) return;
    const kept = session.draft.features.filter((feature) => feature.properties.daenaLayerId !== layerId);
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
    const features = session.draft.features
      .filter((feature) => feature.properties.daenaLayerId === layerId)
      .map(toStoreFeature);
    if (features.length) draw.addFeatures(features);
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

  map.once("style.load", startDraw);
  map.on("mousemove", onHover);
  map.on("click", onMapClick);
  map.on("error", (event) => {
    const message = event.error?.message ?? "MapLibre renderer error";
    if (/webgl/i.test(message) || /context/i.test(message)) {
      session.onDiagnostic?.(RENDERER_UNAVAILABLE, message);
    }
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
        const existing = map.getStyle()?.layers ?? [];
        for (const layer of existing) {
          if (layer.id.startsWith("daena-vector-") && map.getLayer(layer.id)) map.removeLayer(layer.id);
        }
        for (const layer of generated.layers) {
          if (layer.id.startsWith("daena-vector-") && !map.getLayer(layer.id)) {
            if (map.getLayer("daena-hover-fill")) map.addLayer(layer, "daena-hover-fill");
            else map.addLayer(layer);
          }
        }
        applySources(session.activeLayerId);
      });
    },
    applyView(center, zoom) {
      const [lon, lat] = normalizedToLonLat(center[0], center[1]);
      map.jumpTo({ center: [lon, lat], zoom });
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
    redo() {
      const before = JSON.stringify(session.draft);
      draw?.redo();
      mergeDrawIntoDraft();
      if (JSON.stringify(session.draft) !== before) session.onDirty?.();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      map.off("style.load", startDraw);
      map.off("style.load", applyBackground);
      map.off("mousemove", onHover);
      map.off("click", onMapClick);
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
