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
import { altKeyOnly, platformModifierKeyOnly } from "ol/events/condition.js";
import { kindForDrawMode, simplifyFreehandGeometry } from "../native-vector/geometry";
import { BASE_LAYER_ID, layerAcceptsEdits, type VectorDrawMode, type VectorLayerDefinition } from "../native-vector/types";
import type { FeatureCodec } from "./feature-codec";
import type { LayerRegistry } from "./layer-registry";

export type InteractionManager = {
  select: Select;
  configureMode: (mode: VectorDrawMode) => void;
  setActiveLayerId: (layerId: string | null) => void;
  clearSelection: () => void;
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

  const removeDrawingInteractions = () => {
    if (draw) map.removeInteraction(draw);
    if (snap) map.removeInteraction(snap);
    draw = null;
    snap = null;
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
    select.setActive(currentMode === "static" || selecting);
    modify.setActive(selecting);
    translate.setActive(selecting);
    dragBox.setActive(selecting);
    if (!editable || currentMode === "static" || currentMode === "select") return;
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
    snap = new Snap({ source: registry.snapSource, edge: true, vertex: true, intersection: true });
    map.addInteraction(snap);
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

  configureMode(currentMode);

  return {
    select,
    configureMode,
    setActiveLayerId(layerId) {
      activeLayerId = layerId;
    },
    clearSelection() {
      select.getFeatures().clear();
      options.onSelectionChange();
    },
    dispose() {
      removeDrawingInteractions();
      select.getFeatures().clear();
    },
    currentMode: () => currentMode,
  };
}
