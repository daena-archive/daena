import type Feature from "ol/Feature.js";
import type Geometry from "ol/geom/Geometry.js";
import Draw from "ol/interaction/Draw.js";
import { createBox } from "ol/interaction/Draw.js";
import DragBox from "ol/interaction/DragBox.js";
import Modify from "ol/interaction/Modify.js";
import Select from "ol/interaction/Select.js";
import Snap from "ol/interaction/Snap.js";
import Translate from "ol/interaction/Translate.js";
import type Map from "ol/Map.js";
import type View from "ol/View.js";
import { platformModifierKeyOnly } from "ol/events/condition.js";
import { kindForDrawMode, simplifyFreehandGeometry } from "../native-vector/geometry";
import { BASE_LAYER_ID, type VectorDrawMode, type VectorLayerDefinition } from "../native-vector/types";
import type { FeatureCodec } from "./feature-codec";
import type { LayerRegistry } from "./layer-registry";

export type InteractionManager = {
  select: Select;
  configureMode: (mode: VectorDrawMode) => void;
  setActiveLayerId: (layerId: string | null) => void;
  dispose: () => void;
  currentMode: () => VectorDrawMode;
};

export function createInteractionManager(options: {
  map: Map;
  view: View;
  registry: LayerRegistry;
  codec: FeatureCodec;
  getActiveLayerId: () => string | null;
  getPickArmed: () => boolean;
  readOnly: boolean;
  onSourceCommitted: () => void;
  onSelectionChange: () => void;
  onDiagnostic?: (code: string, detail: string) => void;
}): InteractionManager {
  const { map, view, registry, codec } = options;
  let currentMode: VectorDrawMode = options.readOnly ? "static" : "select";
  let activeLayerId = options.getActiveLayerId();
  let draw: Draw | null = null;
  let snap: Snap | null = null;

  const select = new Select({
    layers: [registry.vectorLayer],
    hitTolerance: 6,
    multi: true,
    condition: () => !options.getPickArmed() && (currentMode === "select" || currentMode === "static"),
    filter(feature) {
      const layerId = feature.get("daenaLayerId");
      const layer = registry.layers.find((candidate) => candidate.id === layerId);
      if (layerId === BASE_LAYER_ID) return currentMode === "static";
      if (!layer || !layer.defaultVisible) return false;
      return currentMode === "static" || (layer.id === activeLayerId && !layer.locked);
    },
  });
  const modify = new Modify({ features: select.getFeatures(), pixelTolerance: 10 });
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
    const layer = registry.layers.find((candidate) => candidate.id === activeLayerId);
    return layer && layer.defaultVisible && !layer.locked ? layer : null;
  };

  const removeDrawingInteractions = () => {
    if (draw) map.removeInteraction(draw);
    if (snap) map.removeInteraction(snap);
    draw = null;
    snap = null;
  };

  const configureMode = (mode: VectorDrawMode) => {
    removeDrawingInteractions();
    currentMode = options.readOnly ? "static" : mode;
    if (options.readOnly) {
      select.setActive(true);
      modify.setActive(false);
      translate.setActive(false);
      dragBox.setActive(false);
      return;
    }
    const editable = activeEditableLayer();
    const selecting = currentMode === "select" && Boolean(editable);
    select.setActive(currentMode === "static" || selecting);
    modify.setActive(selecting);
    translate.setActive(selecting);
    dragBox.setActive(selecting);
    if (!editable || currentMode === "static" || currentMode === "select") return;
    const drawMode = currentMode;
    draw = new Draw({
      source: registry.source,
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
          queueMicrotask(() => registry.source.removeFeature(feature));
          options.onDiagnostic?.("vector.geometry.invalid", "Freehand geometry could not be represented.");
          return;
        }
        const simplified = simplifyFreehandGeometry(converted.geometry, view.getZoom() ?? 0);
        if ("error" in simplified) {
          queueMicrotask(() => registry.source.removeFeature(feature));
          options.onDiagnostic?.(simplified.error, "Freehand geometry exceeded the editor budget or was invalid.");
          return;
        }
        feature.setGeometry(codec.readGeometry(simplified));
      }
      queueMicrotask(() => options.onSourceCommitted());
    });
    map.addInteraction(draw);
    snap = new Snap({ source: registry.snapSource, edge: true, vertex: true, intersection: true });
    map.addInteraction(snap);
  };

  select.on("select", () => options.onSelectionChange());
  dragBox.on("boxend", () => {
    const editable = activeEditableLayer();
    if (currentMode !== "select" || !editable) return;
    const extent = dragBox.getGeometry().getExtent();
    select.getFeatures().clear();
    registry.source.forEachFeatureIntersectingExtent(extent, (feature) => {
      if (feature.get("daenaLayerId") === editable.id) select.getFeatures().push(feature);
    });
    options.onSelectionChange();
  });
  modify.on("modifyend", () => options.onSourceCommitted());
  translate.on("translateend", () => options.onSourceCommitted());

  configureMode(currentMode);

  return {
    select,
    configureMode,
    setActiveLayerId(layerId) {
      activeLayerId = layerId;
    },
    dispose() {
      removeDrawingInteractions();
      select.getFeatures().clear();
    },
    currentMode: () => currentMode,
  };
}
