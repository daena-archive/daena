import Feature from "ol/Feature.js";
import Point from "ol/geom/Point.js";
import type Geometry from "ol/geom/Geometry.js";
import Draw from "ol/interaction/Draw.js";
import { createBox } from "ol/interaction/Draw.js";
import DragBox from "ol/interaction/DragBox.js";
import Modify from "ol/interaction/Modify.js";
import Select from "ol/interaction/Select.js";
import Snap from "ol/interaction/Snap.js";
import Translate from "ol/interaction/Translate.js";
import VectorLayer from "ol/layer/Vector.js";
import VectorSource from "ol/source/Vector.js";
import type Map from "ol/Map.js";
import type View from "ol/View.js";
import CircleStyle from "ol/style/Circle.js";
import Fill from "ol/style/Fill.js";
import Stroke from "ol/style/Stroke.js";
import Style from "ol/style/Style.js";
import { altKeyOnly, platformModifierKeyOnly } from "ol/events/condition.js";
import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps";
import {
  formatMeasurement,
  pathLength,
  pointDistance,
  polygonArea,
  unitsForCoordinateSpace,
} from "../editor/measurement";
import { viewToAuthored } from "../editor/coordinate-space";
import { kindForDrawMode, simplifyFreehandGeometry } from "../native-vector/geometry";
import {
  BASE_LAYER_ID,
  layerAcceptsEdits,
  type VectorDrawMode,
  type VectorLayerDefinition,
} from "../native-vector/types";
import type { FeatureCodec } from "./feature-codec";
import type { LayerRegistry } from "./layer-registry";

export type SnapOptions = {
  enabled: boolean;
  vertex: boolean;
  edge: boolean;
  intersection: boolean;
};

export type MeasureReadout = {
  label: string;
  point?: [number, number];
};

export type InteractionManager = {
  select: Select;
  configureMode: (mode: VectorDrawMode) => void;
  setActiveLayerId: (layerId: string | null) => void;
  setSnapOptions: (options: SnapOptions) => void;
  clearMeasure: () => void;
  clearSelection: () => void;
  dispose: () => void;
  currentMode: () => VectorDrawMode;
};

const DEFAULT_SNAP: SnapOptions = { enabled: true, vertex: true, edge: true, intersection: true };

function isMeasureMode(mode: VectorDrawMode): boolean {
  return mode === "measure-distance" || mode === "measure-length" || mode === "measure-area";
}

function isDrawMode(mode: VectorDrawMode): mode is "point" | "linestring" | "polygon" | "rectangle" | "freehand" {
  return mode === "point" || mode === "linestring" || mode === "polygon" || mode === "rectangle" || mode === "freehand";
}

export function createInteractionManager(options: {
  map: Map;
  view: View;
  registry: LayerRegistry;
  codec: FeatureCodec;
  coordinateSpace: MapCoordinateSpace;
  getActiveLayerId: () => string | null;
  getPickArmed: () => boolean;
  readOnly: boolean;
  onSourceCommitted: () => void;
  onSelectionChange: () => void;
  onMeasureReadout?: (readout: MeasureReadout | null) => void;
  onDiagnostic?: (code: string, detail: string) => void;
}): InteractionManager {
  const { map, view, registry, codec } = options;
  let currentMode: VectorDrawMode = options.readOnly ? "static" : "select";
  let activeLayerId = options.getActiveLayerId();
  let draw: Draw | null = null;
  let snap: Snap | null = null;
  let snapOptions: SnapOptions = { ...DEFAULT_SNAP };
  let measureSketch: Feature<Geometry> | null = null;

  const overlaySource = new VectorSource({ wrapX: false });
  const overlayLayer = new VectorLayer({
    source: overlaySource,
    style(feature) {
      if (feature.get("kind") === "snap-indicator") {
        return new Style({
          image: new CircleStyle({
            radius: 5,
            fill: new Fill({ color: "rgba(213, 171, 108, 0.85)" }),
            stroke: new Stroke({ color: "#f3d39a", width: 2 }),
          }),
        });
      }
      return new Style({
        stroke: new Stroke({ color: "#d5ab6c", width: 2, lineDash: [8, 6] }),
        fill: new Fill({ color: "rgba(213, 171, 108, 0.12)" }),
        image: new CircleStyle({
          radius: 4,
          fill: new Fill({ color: "#d5ab6c" }),
          stroke: new Stroke({ color: "#f3d39a", width: 1.5 }),
        }),
      });
    },
    zIndex: 10_000,
  });
  map.addLayer(overlayLayer);

  const featureSelectable = (feature: Feature<Geometry>) => {
    const layerId = String(feature.get("daenaLayerId") ?? "");
    const layer = registry.layerById(layerId);
    if (layerId === BASE_LAYER_ID) return currentMode === "static";
    if (!layer || layer.kind !== "vector" || !layer.defaultVisible) return false;
    if (currentMode === "static") return true;
    return !layer.locked;
  };

  const select = new Select({
    layers: (layer) => registry.isSelectableVectorLayer(layer),
    hitTolerance: 6,
    multi: true,
    condition: () => !options.getPickArmed() && (currentMode === "select" || currentMode === "static"),
    filter: featureSelectable,
  });
  const modify = new Modify({
    features: select.getFeatures(),
    pixelTolerance: 10,
    deleteCondition: altKeyOnly,
    filter(feature) {
      const layerId = String(feature.get("daenaLayerId") ?? "");
      return layerAcceptsEdits(registry.layerById(layerId));
    },
  });
  const translate = new Translate({ features: select.getFeatures(), hitTolerance: 6 });
  const dragBox = new DragBox({ condition: platformModifierKeyOnly });

  if (!options.readOnly) {
    map.addInteraction(select);
    map.addInteraction(modify);
    map.addInteraction(translate);
    map.addInteraction(dragBox);
  } else {
    map.addInteraction(select);
    select.setActive(true);
    modify.setActive(false);
    translate.setActive(false);
    dragBox.setActive(false);
  }

  const activeEditableLayer = (): VectorLayerDefinition | null => {
    const layer = registry.layerById(activeLayerId ?? "");
    return layerAcceptsEdits(layer) ? layer : null;
  };

  const clearOverlaySketch = () => {
    overlaySource.clear(true);
    measureSketch = null;
  };

  const clearMeasureReadout = () => {
    clearOverlaySketch();
    options.onMeasureReadout?.(null);
  };

  const removeDrawingInteractions = () => {
    if (draw) map.removeInteraction(draw);
    if (snap) map.removeInteraction(snap);
    draw = null;
    snap = null;
  };

  const attachSnap = () => {
    if (!snapOptions.enabled) return;
    snap = new Snap({
      source: registry.snapSource,
      edge: snapOptions.edge,
      vertex: snapOptions.vertex,
      intersection: snapOptions.intersection,
    });
    map.addInteraction(snap);
  };

  const updateSnapIndicator = (coordinate: number[] | null) => {
    overlaySource.getFeatures().forEach((feature) => {
      if (feature.get("kind") === "snap-indicator") overlaySource.removeFeature(feature);
    });
    if (!coordinate || !snapOptions.enabled) return;
    let closest: number[] | null = null;
    let best = Infinity;
    for (const feature of registry.snapSource.getFeatures()) {
      const geometry = feature.getGeometry();
      if (!geometry) continue;
      const candidate = geometry.getClosestPoint(coordinate);
      const dx = candidate[0] - coordinate[0];
      const dy = candidate[1] - coordinate[1];
      const distanceSq = dx * dx + dy * dy;
      if (distanceSq < best) {
        best = distanceSq;
        closest = candidate;
      }
    }
    if (!closest || best > 256) return;
    const indicator = new Feature(new Point(closest));
    indicator.set("kind", "snap-indicator");
    overlaySource.addFeature(indicator);
  };

  const formatDistance = (value: number) => {
    const units = unitsForCoordinateSpace(options.coordinateSpace);
    return formatMeasurement(value, units.length);
  };

  const formatArea = (value: number) => {
    const units = unitsForCoordinateSpace(options.coordinateSpace);
    return formatMeasurement(value, units.area);
  };

  const emitMeasureFromGeometry = (geometry: Geometry) => {
    const type = geometry.getType();
    if (type === "LineString") {
      const line = geometry as import("ol/geom/LineString.js").default;
      const coords = line.getCoordinates().map((coord) => viewToAuthored(coord, options.coordinateSpace));
      const value =
        currentMode === "measure-distance" && coords.length >= 2
          ? pointDistance(coords[0], coords[coords.length - 1], options.coordinateSpace)
          : pathLength(coords, options.coordinateSpace);
      const last = coords[coords.length - 1];
      options.onMeasureReadout?.({
        label: currentMode === "measure-distance" ? formatDistance(value) : formatDistance(value),
        point: last,
      });
      return;
    }
    if (type === "Polygon") {
      const polygon = geometry as import("ol/geom/Polygon.js").default;
      const ring =
        polygon
          .getLinearRing(0)
          ?.getCoordinates()
          .map((coord) => viewToAuthored(coord, options.coordinateSpace)) ?? [];
      const value = polygonArea(ring, options.coordinateSpace);
      const last = ring[ring.length - 1];
      options.onMeasureReadout?.({ label: formatArea(value), point: last });
    }
  };

  const pruneSelection = () => {
    const collection = select.getFeatures();
    const keep = collection.getArray().filter((feature) => featureSelectable(feature as Feature<Geometry>));
    if (keep.length === collection.getLength()) return;
    collection.clear();
    for (const feature of keep) collection.push(feature);
    options.onSelectionChange();
  };

  const configureMode = (mode: VectorDrawMode) => {
    removeDrawingInteractions();
    clearMeasureReadout();
    currentMode = options.readOnly ? "static" : mode;
    pruneSelection();
    if (options.readOnly) {
      select.setActive(true);
      modify.setActive(false);
      translate.setActive(false);
      dragBox.setActive(false);
      return;
    }
    const editable = activeEditableLayer();
    const selecting = currentMode === "select";
    const measuring = isMeasureMode(currentMode);
    select.setActive(currentMode === "static" || selecting);
    modify.setActive(selecting);
    translate.setActive(selecting);
    dragBox.setActive(selecting);

    if (measuring) {
      const drawType =
        currentMode === "measure-area" ? "Polygon" : currentMode === "measure-distance" ? "LineString" : "LineString";
      draw = new Draw({
        source: overlaySource,
        type: drawType,
        maxPoints: currentMode === "measure-distance" ? 2 : undefined,
        stopClick: true,
      });
      draw.on("drawstart", (event) => {
        measureSketch = event.feature as Feature<Geometry>;
        overlaySource.clear(true);
      });
      draw.on("drawend", (event) => {
        const geometry = (event.feature as Feature<Geometry>).getGeometry();
        if (geometry) emitMeasureFromGeometry(geometry);
      });
      map.addInteraction(draw);
      attachSnap();
      return;
    }

    if (!editable || currentMode === "static" || currentMode === "select") {
      if (selecting && snapOptions.enabled) attachSnap();
      return;
    }

    if (!isDrawMode(currentMode)) return;

    const drawMode = currentMode;
    const source = registry.sourceFor(editable.id);
    if (!source) return;
    draw = new Draw({
      source,
      type:
        drawMode === "point"
          ? "Point"
          : drawMode === "linestring"
            ? "LineString"
            : drawMode === "rectangle"
              ? "Circle"
              : "Polygon",
      geometryFunction: drawMode === "rectangle" ? createBox() : undefined,
      freehand: drawMode === "freehand",
      trace: drawMode !== "freehand",
      traceSource: registry.snapSource,
      stopClick: true,
    });
    draw.on("drawend", (event) => {
      const feature = event.feature as Feature<Geometry>;
      feature.setId(crypto.randomUUID());
      feature.setProperties({
        daenaLayerId: editable.id,
        kind: kindForDrawMode(drawMode),
        name: null,
      });
      if (drawMode === "freehand") {
        const converted = codec.toVectorFeature(feature, editable.id);
        if (!converted) {
          queueMicrotask(() => source.removeFeature(feature));
          options.onDiagnostic?.("vector.geometry.invalid", "Freehand geometry could not be represented.");
          return;
        }
        const simplified = simplifyFreehandGeometry(converted.geometry, view.getZoom() ?? 0);
        if ("error" in simplified) {
          queueMicrotask(() => source.removeFeature(feature));
          options.onDiagnostic?.(simplified.error, "Freehand geometry exceeded the editor budget or was invalid.");
          return;
        }
        feature.setGeometry(codec.readGeometry(simplified));
      }
      queueMicrotask(() => options.onSourceCommitted());
    });
    map.addInteraction(draw);
    attachSnap();
  };

  select.on("select", () => options.onSelectionChange());
  dragBox.on("boxend", () => {
    if (currentMode !== "select") return;
    const extent = dragBox.getGeometry().getExtent();
    select.getFeatures().clear();
    registry.forEachVectorFeature((feature) => {
      const layerId = String(feature.get("daenaLayerId") ?? "");
      const layer = registry.layerById(layerId);
      if (!layerAcceptsEdits(layer)) return;
      const geometry = feature.getGeometry();
      if (!geometry) return;
      const featureExtent = geometry.getExtent();
      if (
        featureExtent[0] <= extent[2] &&
        featureExtent[2] >= extent[0] &&
        featureExtent[1] <= extent[3] &&
        featureExtent[3] >= extent[1]
      ) {
        select.getFeatures().push(feature);
      }
    });
    options.onSelectionChange();
  });
  modify.on("modifyend", () => options.onSourceCommitted());
  translate.on("translateend", () => options.onSourceCommitted());

  map.on("pointermove", (event) => {
    if (event.dragging || options.getPickArmed()) return;
    const shouldIndicate =
      snapOptions.enabled && (isDrawMode(currentMode) || currentMode === "select" || isMeasureMode(currentMode));
    updateSnapIndicator(shouldIndicate ? event.coordinate : null);
  });

  configureMode(currentMode);

  return {
    select,
    configureMode,
    setActiveLayerId(layerId) {
      activeLayerId = layerId;
    },
    setSnapOptions(next) {
      snapOptions = { ...next };
      configureMode(currentMode);
    },
    clearMeasure: clearMeasureReadout,
    clearSelection() {
      select.getFeatures().clear();
      options.onSelectionChange();
    },
    dispose() {
      removeDrawingInteractions();
      select.getFeatures().clear();
      overlaySource.clear(true);
      map.removeLayer(overlayLayer);
    },
    currentMode: () => currentMode,
  };
}
