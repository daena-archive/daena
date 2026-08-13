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
import type { GeoJSONSource, Map as MapLibreMap } from "maplibre-gl";
import maplibregl from "maplibre-gl/dist/maplibre-gl-csp.js";
import workerUrl from "maplibre-gl/dist/maplibre-gl-csp-worker.js?url";
import "maplibre-gl/dist/maplibre-gl.css";
import { BASE_LAYER_ID, type VectorDrawMode, type VectorFeature, type VectorFeatureCollection } from "./types";
import { PHASE0_VECTOR_LAYERS } from "./fixture";
import {
  AUTHORED_SOURCE_ID,
  BASE_SOURCE_ID,
  nativeVectorStyle,
  splitVectorSources,
  styleContainsRemoteUrl,
} from "./style";
import { drawModeForGeometry, kindForDrawMode, simplifyFreehandGeometry } from "./geometry";
import { normalizedToLonLat } from "./coordinates";

if (typeof maplibregl.setWorkerUrl === "function") maplibregl.setWorkerUrl(workerUrl);

export const RENDERER_UNAVAILABLE = "vector.renderer.unavailable";
export { workerUrl, PHASE0_VECTOR_LAYERS };

const liveEditors = new Set<NativeVectorEditor>();

export function liveNativeVectorEditorCount() {
  return liveEditors.size;
}

export type NativeVectorEditor = {
  workerUrl: string;
  objectUrls: string[];
  setMode: (mode: VectorDrawMode) => void;
  switchLayer: (layerId: string) => void;
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
    activeLayerId: string;
    setDraft: (next: VectorFeatureCollection) => void;
    setActiveLayerId: (id: string) => void;
    onDirty?: () => void;
    onDiagnostic?: (code: string, detail: string) => void;
  },
): NativeVectorEditor | { error: typeof RENDERER_UNAVAILABLE; detail: string } {
  if (!webgl2Available()) {
    return {
      error: RENDERER_UNAVAILABLE,
      detail: "WebGL2 is required for native vector maps and is not available.",
    };
  }

  const style = nativeVectorStyle(PHASE0_VECTOR_LAYERS);
  if (styleContainsRemoteUrl(style)) {
    return { error: RENDERER_UNAVAILABLE, detail: "Native vector style must not request remote URLs." };
  }

  const [longitude, latitude] = normalizedToLonLat(0.5, 0.5);
  let map: MapLibreMap;
  try {
    map = new maplibregl.Map({
      container,
      style,
      center: [longitude, latitude],
      zoom: 1.6,
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

  const applySources = (activeLayerId: string) => {
    const split = splitVectorSources(session.draft, activeLayerId);
    (map.getSource(BASE_SOURCE_ID) as GeoJSONSource | undefined)?.setData(split.base);
    (map.getSource(AUTHORED_SOURCE_ID) as GeoJSONSource | undefined)?.setData(split.authored);
  };

  const mergeDrawIntoDraft = () => {
    if (!draw) return;
    const kept = session.draft.features.filter((feature) => feature.properties.daenaLayerId !== session.activeLayerId);
    const drawn: VectorFeature[] = [];
    for (const feature of draw.getSnapshot()) {
      const converted = asVectorFeature(feature, session.activeLayerId);
      if (converted) drawn.push(converted);
    }
    session.setDraft({ type: "FeatureCollection", features: [...kept, ...drawn] });
    applySources(session.activeLayerId);
  };

  const loadActiveLayer = () => {
    if (!draw) return;
    const features = session.draft.features
      .filter(
        (feature) =>
          feature.properties.daenaLayerId === session.activeLayerId &&
          feature.properties.daenaLayerId !== BASE_LAYER_ID,
      )
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
    const existingKind = snapshot.properties?.kind;
    draw.updateFeatureProperties(id, {
      daenaLayerId: session.activeLayerId,
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
    if (disposed || draw) return;
    draw = new TerraDraw({
      adapter: new TerraDrawMapLibreGLAdapter({ map, coordinatePrecision: 6 }),
      idStrategy: {
        isValidId: (candidate) => typeof candidate === "string",
        getId: () => crypto.randomUUID(),
      },
      undoRedo: { sessionLevel: new TerraDrawSessionUndoRedo({ maxStackSize: 50 }) },
      modes: [
        new TerraDrawSelectMode({
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
    draw.setMode("select");
    draw.on("change", onChange);
    draw.on("finish", onFinish);
  };

  map.once("style.load", startDraw);
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
      draw?.setMode(mode === "static" ? "static" : mode);
    },
    switchLayer(layerId) {
      if (draw?.enabled) {
        mergeDrawIntoDraft();
        draw.off("change", onChange);
        draw.off("finish", onFinish);
        draw.stop();
        draw = null;
      }
      session.setActiveLayerId(layerId);
      applySources(layerId);
      startDraw();
    },
    undo() {
      draw?.undo();
      mergeDrawIntoDraft();
    },
    redo() {
      draw?.redo();
      mergeDrawIntoDraft();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      map.off("style.load", startDraw);
      try {
        draw?.off("change", onChange);
        draw?.off("finish", onFinish);
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
