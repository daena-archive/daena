import type Feature from "ol/Feature.js";
import GeoJSON from "ol/format/GeoJSON.js";
import type Geometry from "ol/geom/Geometry.js";
import Draw from "ol/interaction/Draw.js";
import { createBox } from "ol/interaction/Draw.js";
import DragBox from "ol/interaction/DragBox.js";
import Modify from "ol/interaction/Modify.js";
import Select from "ol/interaction/Select.js";
import Snap from "ol/interaction/Snap.js";
import Translate from "ol/interaction/Translate.js";
import ImageLayer from "ol/layer/Image.js";
import VectorLayer from "ol/layer/Vector.js";
import Map from "ol/Map.js";
import Projection from "ol/proj/Projection.js";
import ImageStatic from "ol/source/ImageStatic.js";
import VectorSource from "ol/source/Vector.js";
import View from "ol/View.js";
import { getCenter } from "ol/extent.js";
import { defaults as defaultInteractions } from "ol/interaction/defaults.js";
import { platformModifierKeyOnly } from "ol/events/condition.js";
import "ol/ol.css";
import type { MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import { drawModeForGeometry, kindForDrawMode, simplifyFreehandGeometry } from "./geometry";
import {
  imageOverlayCoordinates,
  lonLatToNormalized,
  normalizedToLonLat,
  type ImageOverlayCoordinates,
} from "./coordinates";
import { nativeFeatureStyle, visibleUnlockedFeatures } from "./openlayers-style";
import {
  BASE_LAYER_ID,
  UNDO_STACK_SIZE,
  type VectorDrawMode,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorKind,
  type VectorLayerDefinition,
} from "./types";

export const RENDERER_UNAVAILABLE = "vector.renderer.unavailable";

const WORLD_EXTENT: [number, number, number, number] = [-180, -90, 180, 90];
const WORLD_RESOLUTIONS = Array.from({ length: 13 }, (_, zoom) => 360 / 256 / 2 ** zoom);
const projection = new Projection({
  code: "DAENA:WORLD",
  units: "degrees",
  extent: WORLD_EXTENT,
  worldExtent: WORLD_EXTENT,
});
const format = new GeoJSON({ dataProjection: projection, featureProjection: projection });
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

export type NativeVectorView = { center: [number, number]; zoom: number };

export type NativeVectorEditor = {
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
  focusPoint: (normalized: [number, number], zoom?: number) => void;
  flush: () => void;
  deleteSelection: () => void;
  duplicateSelection: () => void;
  duplicateLayerFeatures: (sourceLayerId: string, targetLayerId: string) => void;
  moveSelectionToLayer: (layerId: string) => void;
  updateSelectedName: (name: string | null) => void;
  undo: () => void;
  redo: () => void;
  resize: () => void;
  dispose: () => void;
};

function cloneCollection(collection: VectorFeatureCollection): VectorFeatureCollection {
  return JSON.parse(JSON.stringify(collection)) as VectorFeatureCollection;
}

function signature(collection: VectorFeatureCollection) {
  return JSON.stringify(collection);
}

function extentFromCoordinates(coordinates: ImageOverlayCoordinates): [number, number, number, number] {
  const xs = coordinates.map((coordinate) => coordinate[0]);
  const ys = coordinates.map((coordinate) => coordinate[1]);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

function featureKind(value: unknown, geometry: VectorFeature["geometry"]): VectorKind {
  if (
    value === "land" ||
    value === "lake" ||
    value === "region" ||
    value === "route" ||
    value === "marker" ||
    value === "custom"
  )
    return value;
  return kindForDrawMode(drawModeForGeometry(geometry));
}

function readFeatures(collection: VectorFeatureCollection): Feature<Geometry>[] {
  const features = format.readFeatures(collection as Parameters<GeoJSON["readFeatures"]>[0]) as Feature<Geometry>[];
  for (const feature of features) {
    const daena = feature.get("daena") as
      | { layerId?: unknown; semanticType?: unknown; name?: unknown }
      | undefined;
    if (daena && typeof daena === "object") {
      feature.setProperties({
        daenaLayerId: typeof daena.layerId === "string" ? daena.layerId : BASE_LAYER_ID,
        kind: typeof daena.semanticType === "string" ? daena.semanticType : "custom",
        name: typeof daena.name === "string" ? daena.name : null,
      });
      feature.unset("daena");
    }
  }
  return features;
}

function toVectorFeature(feature: Feature<Geometry>, fallbackLayerId: string): VectorFeature | null {
  const object = format.writeFeatureObject(feature) as {
    id?: string | number;
    properties?: Record<string, unknown> | null;
    geometry?: VectorFeature["geometry"] | null;
  };
  const geometry = object.geometry;
  if (
    !geometry ||
    (geometry.type !== "Point" &&
      geometry.type !== "MultiPoint" &&
      geometry.type !== "LineString" &&
      geometry.type !== "MultiLineString" &&
      geometry.type !== "Polygon" &&
      geometry.type !== "MultiPolygon")
  )
    return null;
  const properties = object.properties ?? {};
  const layerId =
    typeof properties.daenaLayerId === "string"
      ? properties.daenaLayerId
      : typeof (properties.daena as { layerId?: unknown } | undefined)?.layerId === "string"
        ? ((properties.daena as { layerId: string }).layerId)
        : fallbackLayerId;
  const kind = featureKind(
    properties.kind ?? (properties.daena as { semanticType?: unknown } | undefined)?.semanticType,
    geometry,
  );
  const name =
    typeof properties.name === "string"
      ? properties.name
      : typeof (properties.daena as { name?: unknown } | undefined)?.name === "string"
        ? ((properties.daena as { name: string }).name)
        : null;
  return {
    type: "Feature",
    id: String(feature.getId() ?? object.id ?? crypto.randomUUID()),
    properties: {
      daena: {
        layerId,
        semanticType: kind,
        name,
        style: null,
        label: null,
        custom: {},
      },
    },
    geometry,
  };
}

function collectionBounds(collection: VectorFeatureCollection): [number, number, number, number] | null {
  const features = readFeatures(collection);
  if (features.length === 0) return null;
  const extent = new VectorSource({ features }).getExtent();
  return extent && extent.every(Number.isFinite) ? (extent as [number, number, number, number]) : null;
}

function normalizedViewCenter(center: [number, number]): [number, number] {
  const [longitude, latitude] = normalizedToLonLat(center[0], center[1]);
  return [Math.max(-180, Math.min(180, longitude)), Math.max(-90, Math.min(90, latitude))];
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
    pickArmed?: boolean;
    onMapPick?: (anchor: MapAnchor) => void;
    background?: NativeVectorBackground | null;
    initialView?: NativeVectorView | null;
    onViewChange?: (view: NativeVectorView) => void;
  },
): NativeVectorEditor | { error: typeof RENDERER_UNAVAILABLE; detail: string } {
  let disposed = false;
  let currentLayers = [...session.layers];
  let currentMode: VectorDrawMode = "select";
  let currentBackground = session.background ?? null;
  let draw: Draw | null = null;
  let snap: Snap | null = null;
  let lastAppliedSignature = signature(session.draft);
  let hoveredId: string | null = null;

  const source = new VectorSource<Feature<Geometry>>({ features: readFeatures(session.draft), wrapX: false });
  const snapSource = new VectorSource<Feature<Geometry>>({ wrapX: false });
  const selectedIds = new Set<string>();
  const vectorLayer = new VectorLayer({
    source,
    updateWhileAnimating: true,
    updateWhileInteracting: true,
    style(feature) {
      const id = String(feature.getId() ?? "");
      return nativeFeatureStyle(feature, currentLayers, { hovered: id === hoveredId, selected: selectedIds.has(id) });
    },
  });
  const backgroundLayer = new ImageLayer({ visible: true });
  const view = new View({
    projection,
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
      layers: [backgroundLayer, vectorLayer],
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

  const select = new Select({
    layers: [vectorLayer],
    hitTolerance: 6,
    multi: true,
    condition: () => !session.pickArmed && (currentMode === "select" || currentMode === "static"),
    filter(feature) {
      const layerId = feature.get("daenaLayerId");
      const layer = currentLayers.find((candidate) => candidate.id === layerId);
      if (layerId === BASE_LAYER_ID) return currentMode === "static";
      if (!layer || !layer.defaultVisible) return false;
      return currentMode === "static" || (layer.id === session.activeLayerId && !layer.locked);
    },
  });
  const modify = new Modify({ features: select.getFeatures(), pixelTolerance: 10 });
  const translate = new Translate({ features: select.getFeatures(), hitTolerance: 6 });
  const dragBox = new DragBox({ condition: platformModifierKeyOnly });
  map.addInteraction(select);
  map.addInteraction(modify);
  map.addInteraction(translate);
  map.addInteraction(dragBox);

  const history: VectorFeatureCollection[] = [cloneCollection(session.draft)];
  let historyIndex = 0;

  const syncSnapSource = (collection = session.draft) => {
    snapSource.clear(true);
    snapSource.addFeatures(readFeatures(visibleUnlockedFeatures(collection, currentLayers)));
  };

  const replaceSource = (collection: VectorFeatureCollection) => {
    source.clear(true);
    source.addFeatures(readFeatures(collection));
    lastAppliedSignature = signature(collection);
    select.getFeatures().clear();
    selectedIds.clear();
    session.onSelect?.(null);
    syncSnapSource(collection);
    vectorLayer.changed();
  };

  const sourceCollection = (): VectorFeatureCollection => {
    const fallbackLayerId = session.activeLayerId ?? BASE_LAYER_ID;
    return {
      type: "FeatureCollection",
      features: source
        .getFeatures()
        .map((feature) => toVectorFeature(feature, fallbackLayerId))
        .filter((feature): feature is VectorFeature => feature !== null)
        .sort((left, right) => left.id.localeCompare(right.id)),
    };
  };

  const recordHistory = (collection: VectorFeatureCollection) => {
    history.splice(historyIndex + 1);
    history.push(cloneCollection(collection));
    if (history.length > UNDO_STACK_SIZE + 1) history.shift();
    historyIndex = history.length - 1;
  };

  const applyDraft = (collection: VectorFeatureCollection, dirty: boolean, record: boolean) => {
    const next = cloneCollection(collection);
    if (signature(next) === lastAppliedSignature) return;
    replaceSource(next);
    session.setDraft(next);
    if (record) recordHistory(next);
    if (dirty) session.onDirty?.();
  };
  const commitSource = () => {
    const next = sourceCollection();
    const nextSignature = signature(next);
    if (nextSignature === lastAppliedSignature) return;
    lastAppliedSignature = nextSignature;
    session.setDraft(cloneCollection(next));
    syncSnapSource(next);
    recordHistory(next);
    session.onDirty?.();
  };

  const emitSelection = () => {
    selectedIds.clear();
    for (const feature of select.getFeatures().getArray()) selectedIds.add(String(feature.getId() ?? ""));
    vectorLayer.changed();
    const selected = select.getFeatures().item(0);
    session.onSelect?.(selected ? toVectorFeature(selected, session.activeLayerId ?? BASE_LAYER_ID) : null);
  };
  select.on("select", emitSelection);
  dragBox.on("boxend", () => {
    const editable = activeEditableLayer();
    if (currentMode !== "select" || !editable) return;
    const extent = dragBox.getGeometry().getExtent();
    select.getFeatures().clear();
    source.forEachFeatureIntersectingExtent(extent, (feature) => {
      if (feature.get("daenaLayerId") === editable.id) select.getFeatures().push(feature);
    });
    emitSelection();
  });
  modify.on("modifyend", commitSource);
  translate.on("translateend", commitSource);

  const removeDrawingInteractions = () => {
    if (draw) map.removeInteraction(draw);
    if (snap) map.removeInteraction(snap);
    draw = null;
    snap = null;
  };
  const activeEditableLayer = () => {
    const layer = currentLayers.find((candidate) => candidate.id === session.activeLayerId);
    return layer && layer.defaultVisible && !layer.locked ? layer : null;
  };
  const configureMode = (mode: VectorDrawMode) => {
    removeDrawingInteractions();
    currentMode = mode;
    const editable = activeEditableLayer();
    const selecting = mode === "select" && Boolean(editable);
    select.setActive(mode === "static" || selecting);
    modify.setActive(selecting);
    translate.setActive(selecting);
    dragBox.setActive(selecting);
    if (!editable || mode === "static" || mode === "select") return;
    draw = new Draw({
      source,
      type:
        mode === "point" ? "Point" : mode === "linestring" ? "LineString" : mode === "rectangle" ? "Circle" : "Polygon",
      geometryFunction: mode === "rectangle" ? createBox() : undefined,
      freehand: mode === "freehand",
      trace: mode !== "freehand",
      traceSource: snapSource,
      stopClick: true,
    });
    draw.on("drawend", (event) => {
      const feature = event.feature as Feature<Geometry>;
      feature.setId(crypto.randomUUID());
      feature.setProperties({ daenaLayerId: editable.id, kind: kindForDrawMode(mode), name: null });
      if (mode === "freehand") {
        const converted = toVectorFeature(feature, editable.id);
        if (!converted) {
          queueMicrotask(() => source.removeFeature(feature));
          session.onDiagnostic?.("vector.geometry.invalid", "Freehand geometry could not be represented.");
          return;
        }
        const simplified = simplifyFreehandGeometry(converted.geometry, view.getZoom() ?? 0);
        if ("error" in simplified) {
          queueMicrotask(() => source.removeFeature(feature));
          session.onDiagnostic?.(simplified.error, "Freehand geometry exceeded the editor budget or was invalid.");
          return;
        }
        feature.setGeometry(format.readGeometry(simplified as Parameters<GeoJSON["readGeometry"]>[0]));
      }
      queueMicrotask(commitSource);
    });
    map.addInteraction(draw);
    snap = new Snap({ source: snapSource, edge: true, vertex: true, intersection: true });
    map.addInteraction(snap);
  };

  const updateBackground = () => {
    if (!currentBackground) {
      backgroundLayer.setSource(null);
      return;
    }
    const coordinates =
      currentBackground.coordinates ?? imageOverlayCoordinates(currentBackground.width, currentBackground.height);
    let url = currentBackground.url;
    if (currentBackground.canvas) {
      try {
        url = currentBackground.canvas.toDataURL("image/png");
      } catch (cause) {
        session.onDiagnostic?.(
          RENDERER_UNAVAILABLE,
          cause instanceof Error ? cause.message : "OpenLayers could not read the raster canvas.",
        );
        return;
      }
    }
    backgroundLayer.setSource(
      new ImageStatic({ url, projection, imageExtent: extentFromCoordinates(coordinates), interpolate: true }),
    );
  };

  const fitContent = () => {
    if (session.initialView) {
      view.setCenter(session.initialView.center);
      view.setZoom(session.initialView.zoom);
      return;
    }
    if (currentBackground) {
      const coordinates =
        currentBackground.coordinates ?? imageOverlayCoordinates(currentBackground.width, currentBackground.height);
      view.fit(extentFromCoordinates(coordinates), { padding: [28, 28, 28, 28], maxZoom: 4, duration: 0 });
      return;
    }
    const extent = collectionBounds(session.draft);
    if (extent) view.fit(extent, { padding: [28, 28, 28, 28], maxZoom: 4, duration: 0 });
  };

  const featureAtPixel = (pixel: number[]) =>
    map.forEachFeatureAtPixel(
      pixel,
      (feature, layer) => (layer === vectorLayer ? (feature as Feature<Geometry>) : undefined),
      { hitTolerance: 6, layerFilter: (layer) => layer === vectorLayer },
    );
  const anchorFor = (feature: Feature<Geometry> | undefined, coordinate: number[]): MapAnchor => {
    const converted = feature ? toVectorFeature(feature, session.activeLayerId ?? BASE_LAYER_ID) : null;
    if (converted) {
      const positions = converted.geometry.coordinates.flat(Infinity) as number[];
      return {
        kind: "provider-feature",
        provider: "daena-openlayers",
        featureKind: "geojson-feature",
        featureId: converted.id,
        fallbackPoint:
          positions.length >= 2
            ? lonLatToNormalized(positions[0], positions[1])
            : lonLatToNormalized(coordinate[0], coordinate[1]),
      };
    }
    const normalized = lonLatToNormalized(coordinate[0], coordinate[1]);
    return { kind: "point", point: [Math.max(0, Math.min(1, normalized[0])), Math.max(0, Math.min(1, normalized[1]))] };
  };
  map.on("pointermove", (event) => {
    if (disposed || event.dragging || session.pickArmed) return;
    const next = featureAtPixel(event.pixel);
    const id = next ? String(next.getId() ?? "") : null;
    if (id !== hoveredId) {
      hoveredId = id;
      vectorLayer.changed();
    }
  });
  map.on("singleclick", (event) => {
    if (session.pickArmed) session.onMapPick?.(anchorFor(featureAtPixel(event.pixel), event.coordinate));
  });
  map.on("dblclick", (event) => {
    if (session.pickArmed || currentMode !== "static") return;
    const feature = featureAtPixel(event.pixel);
    if (feature?.getId() !== undefined) session.onDoubleClick?.(String(feature.getId()));
  });
  map.on("moveend", () => {
    const center = view.getCenter();
    if (center) session.onViewChange?.({ center: [center[0], center[1]], zoom: view.getZoom() ?? 0 });
  });

  updateBackground();
  syncSnapSource();
  const resizeObserver = new ResizeObserver(() => {
    if (!disposed && container.clientWidth > 0 && container.clientHeight > 0) map.updateSize();
  });
  resizeObserver.observe(container);
  requestAnimationFrame(() => !disposed && map.updateSize());

  let editor: NativeVectorEditor;
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Delete" && event.key !== "Backspace") return;
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
    event.preventDefault();
    editor.deleteSelection();
  };
  container.addEventListener("keydown", onKeyDown);

  editor = {
    setMode: configureMode,
    switchLayer(layerId) {
      select.getFeatures().clear();
      emitSelection();
      session.setActiveLayerId(layerId);
      configureMode("select");
    },
    syncLayers(layers) {
      currentLayers = [...layers];
      if (signature(session.draft) !== lastAppliedSignature) replaceSource(session.draft);
      else syncSnapSource();
      vectorLayer.changed();
      configureMode(currentMode);
    },
    setBackground(background) {
      currentBackground = background;
      updateBackground();
    },
    setBackgroundVisible(visible) {
      backgroundLayer.setVisible(visible);
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
      const feature = source.getFeatureById(featureId);
      const geometry = feature?.getGeometry();
      if (!feature || !geometry) return false;
      const extent = geometry.getExtent();
      if (extent[0] === extent[2] && extent[1] === extent[3]) {
        view.setCenter(getCenter(extent));
        view.setZoom(6);
      } else view.fit(extent, { padding: [48, 48, 48, 48], maxZoom: 8, duration: 0 });
      select.getFeatures().clear();
      select.getFeatures().push(feature);
      emitSelection();
      return true;
    },
    focusPoint(normalized, zoom = 4) {
      view.setCenter(normalizedViewCenter(normalized));
      view.setZoom(Math.max(2, zoom));
    },
    flush() {
      const collection = sourceCollection();
      if (signature(collection) !== lastAppliedSignature) applyDraft(collection, true, true);
    },
    deleteSelection() {
      const selected = [...select.getFeatures().getArray()];
      if (selected.length === 0) return;
      for (const feature of selected) source.removeFeature(feature);
      select.getFeatures().clear();
      emitSelection();
      commitSource();
    },
    duplicateSelection() {
      const selected = [...select.getFeatures().getArray()];
      if (selected.length === 0) return;
      const offset = (view.getResolution() ?? WORLD_RESOLUTIONS[0]) * 12;
      const copies: Feature<Geometry>[] = [];
      for (const feature of selected) {
        const copy = feature.clone() as Feature<Geometry>;
        copy.setId(crypto.randomUUID());
        copy.getGeometry()?.translate(offset, -offset);
        source.addFeature(copy);
        copies.push(copy);
      }
      select.getFeatures().clear();
      select.getFeatures().extend(copies);
      commitSource();
      emitSelection();
    },
    duplicateLayerFeatures(sourceLayerId, targetLayerId) {
      const target = currentLayers.find((layer) => layer.id === targetLayerId);
      if (!target || target.locked || target.id === BASE_LAYER_ID) return;
      const copies = source
        .getFeatures()
        .filter((feature) => feature.get("daenaLayerId") === sourceLayerId)
        .map((feature) => {
          const copy = feature.clone() as Feature<Geometry>;
          copy.setId(crypto.randomUUID());
          copy.set("daenaLayerId", targetLayerId);
          return copy;
        });
      if (copies.length === 0) return;
      source.addFeatures(copies);
      commitSource();
    },
    moveSelectionToLayer(layerId) {
      const target = currentLayers.find((layer) => layer.id === layerId);
      if (!target || !target.defaultVisible || target.locked || target.id === BASE_LAYER_ID) return;
      const selected = [...select.getFeatures().getArray()];
      if (selected.length === 0) return;
      for (const feature of selected) feature.set("daenaLayerId", layerId);
      session.setActiveLayerId(layerId);
      commitSource();
      emitSelection();
      vectorLayer.changed();
      configureMode("select");
    },
    updateSelectedName(name) {
      const selected = select.getFeatures().item(0);
      if (!selected) return;
      selected.set("name", name);
      commitSource();
      emitSelection();
    },
    undo() {
      if (historyIndex <= 0) return;
      historyIndex -= 1;
      replaceSource(history[historyIndex]);
      session.setDraft(cloneCollection(history[historyIndex]));
      session.onDirty?.();
    },
    redo() {
      if (historyIndex >= history.length - 1) return;
      historyIndex += 1;
      replaceSource(history[historyIndex]);
      session.setDraft(cloneCollection(history[historyIndex]));
      session.onDirty?.();
    },
    resize: () => map.updateSize(),
    dispose() {
      if (disposed) return;
      disposed = true;
      resizeObserver.disconnect();
      container.removeEventListener("keydown", onKeyDown);
      removeDrawingInteractions();
      select.getFeatures().clear();
      map.setTarget(undefined);
      map.dispose();
      liveEditors.delete(editor);
    },
  };
  configureMode("select");
  liveEditors.add(editor);
  return editor;
}
