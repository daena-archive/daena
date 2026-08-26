import type { FeatureLike } from "ol/Feature.js";
import CircleStyle from "ol/style/Circle.js";
import Fill from "ol/style/Fill.js";
import Stroke from "ol/style/Stroke.js";
import Style from "ol/style/Style.js";
import Text from "ol/style/Text.js";
import { BASE_LAYER_ID, featureLayerId, type VectorFeatureCollection, type VectorLayerDefinition } from "./types";

function colorWithOpacity(color: string, opacity: number) {
  if (!color.startsWith("#") || (color.length !== 7 && color.length !== 4)) return color;
  const digits =
    color.length === 4
      ? color
          .slice(1)
          .split("")
          .map((digit) => `${digit}${digit}`)
      : color.slice(1).match(/.{2}/g);
  if (!digits || digits.length !== 3) return color;
  return `rgba(${Number.parseInt(digits[0], 16)}, ${Number.parseInt(digits[1], 16)}, ${Number.parseInt(digits[2], 16)}, ${opacity})`;
}

function baseVisible(layers: readonly VectorLayerDefinition[]) {
  const base = layers.find((layer) => layer.id === BASE_LAYER_ID);
  return base ? base.defaultVisible : true;
}

export function visibleUnlockedFeatures(
  collection: VectorFeatureCollection,
  layers: readonly VectorLayerDefinition[],
): VectorFeatureCollection {
  return snapTargetFeatures(collection, layers);
}

export function snapTargetFeatures(
  collection: VectorFeatureCollection,
  layers: readonly VectorLayerDefinition[],
  snapTargetLayerIds: ReadonlySet<string> = new Set(),
): VectorFeatureCollection {
  const enabled = new Set<string>();
  for (const layer of layers) {
    if (!layer.defaultVisible) continue;
    if (!layer.locked) enabled.add(layer.id);
    else if (snapTargetLayerIds.has(layer.id)) enabled.add(layer.id);
  }
  if (baseVisible(layers)) enabled.add(BASE_LAYER_ID);
  return {
    type: "FeatureCollection",
    features: collection.features.filter((feature) => enabled.has(featureLayerId(feature))),
  };
}

export function nativeFeatureStyle(
  feature: FeatureLike,
  layers: readonly VectorLayerDefinition[],
  state: { hovered: boolean; selected: boolean },
): Style | undefined {
  const name = feature.get("name");
  const text =
    typeof name === "string" && name.trim()
      ? new Text({
          text: name.trim(),
          offsetY: -14,
          font: "600 12px system-ui",
          fill: new Fill({ color: "#f7f0e5" }),
          stroke: new Stroke({ color: "rgba(13, 27, 42, 0.9)", width: 3 }),
          overflow: true,
        })
      : undefined;
  const layerId = feature.get("daenaLayerId") as string | undefined;
  if (layerId === BASE_LAYER_ID) {
    if (!baseVisible(layers)) return undefined;
    return new Style({
      fill: new Fill({ color: state.selected ? "rgba(213, 171, 108, 0.92)" : "rgba(201, 169, 110, 0.92)" }),
      stroke: new Stroke({ color: state.hovered ? "#f3d39a" : "#8a7048", width: state.selected ? 2.5 : 1.25 }),
      text,
    });
  }

  const layer = layers.find((candidate) => candidate.id === layerId);
  if (!layer?.defaultVisible) return undefined;
  const fill = state.selected
    ? "rgba(213, 171, 108, 0.56)"
    : colorWithOpacity(layer.style.fill, layer.style.fillOpacity);
  const stroke =
    state.hovered || state.selected ? "#f3d39a" : colorWithOpacity(layer.style.stroke, layer.style.strokeOpacity ?? 1);
  const width = state.selected
    ? Math.max(2.5, layer.style.strokeWidth)
    : state.hovered
      ? Math.max(2, layer.style.strokeWidth)
      : layer.style.strokeWidth;
  return new Style({
    fill: new Fill({ color: fill }),
    stroke: new Stroke({ color: stroke, width }),
    image: new CircleStyle({
      radius: state.selected ? Math.max(7, layer.style.pointRadius) : layer.style.pointRadius,
      fill: new Fill({
        color: state.selected ? "#d5ab6c" : colorWithOpacity(layer.style.fill, layer.style.fillOpacity),
      }),
      stroke: new Stroke({ color: stroke, width: Math.min(width, 4) }),
    }),
    text,
  });
}
