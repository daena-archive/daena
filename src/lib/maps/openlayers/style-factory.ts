import type { FeatureLike } from "ol/Feature.js";
import CircleStyle from "ol/style/Circle.js";
import Fill from "ol/style/Fill.js";
import RegularShape from "ol/style/RegularShape.js";
import Stroke from "ol/style/Stroke.js";
import Style from "ol/style/Style.js";
import Text from "ol/style/Text.js";
import type { MapLabelV2, MapStyleV2 } from "../../../../packages/plugin-sdk/src/maps";
import {
  BASE_LAYER_ID,
  DEFAULT_VECTOR_LAYER_STYLE,
  featureLayerId,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "../native-vector/types";

export type FeatureStyleState = { hovered: boolean; selected: boolean; zoom?: number };

const styleCache = new Map<string, Style>();
const MAX_STYLE_CACHE_ENTRIES = 2_048;

function colorWithOpacity(color: string, opacity: number) {
  if (!/^#[0-9a-f]{6}$/i.test(color)) return color;
  return `rgba(${Number.parseInt(color.slice(1, 3), 16)}, ${Number.parseInt(color.slice(3, 5), 16)}, ${Number.parseInt(color.slice(5, 7), 16)}, ${opacity})`;
}

function baseVisible(layers: readonly VectorLayerDefinition[]) {
  return layers.find((layer) => layer.id === BASE_LAYER_ID)?.defaultVisible ?? true;
}

function resolvedStyle(feature: FeatureLike, layer: VectorLayerDefinition): MapStyleV2 {
  const override = feature.get("daenaStyle");
  const styleOverride =
    override && typeof override === "object" && !Array.isArray(override) ? (override as Partial<MapStyleV2>) : {};
  return {
    ...DEFAULT_VECTOR_LAYER_STYLE,
    ...layer.style,
    ...styleOverride,
    strokeDash: styleOverride.strokeDash ?? layer.style.strokeDash ?? DEFAULT_VECTOR_LAYER_STYLE.strokeDash,
    label: styleOverride.label ?? layer.style.label ?? DEFAULT_VECTOR_LAYER_STYLE.label,
  };
}

function resolvedLabel(feature: FeatureLike, style: MapStyleV2): MapLabelV2 | null {
  const direct = feature.get("daenaLabel");
  if (direct && typeof direct === "object" && !Array.isArray(direct)) return direct as MapLabelV2;
  return style.label ?? null;
}

function labelText(feature: FeatureLike, label: MapLabelV2 | null): string {
  if (!label) return "";
  if (label.source === "explicit") return label.text?.trim() ?? "";
  const name = feature.get("name");
  return typeof name === "string" ? name.trim() : "";
}

function labelVisible(label: MapLabelV2 | null, zoom: number | undefined): boolean {
  if (!label || zoom === undefined) return Boolean(label);
  return (label.minZoom == null || zoom >= label.minZoom) && (label.maxZoom == null || zoom <= label.maxZoom);
}

function markerImage(icon: string | null | undefined, radius: number, fill: Fill, stroke: Stroke) {
  if (icon === "square") return new RegularShape({ points: 4, radius, angle: Math.PI / 4, fill, stroke });
  if (icon === "diamond") return new RegularShape({ points: 4, radius, fill, stroke });
  if (icon === "triangle") return new RegularShape({ points: 3, radius, rotation: 0, fill, stroke });
  if (icon === "star") return new RegularShape({ points: 5, radius, radius2: radius / 2.2, fill, stroke });
  return new CircleStyle({ radius, fill, stroke });
}

function cacheStyle(key: string, factory: () => Style): Style {
  const cached = styleCache.get(key);
  if (cached) return cached;
  const created = factory();
  if (styleCache.size >= MAX_STYLE_CACHE_ENTRIES) styleCache.clear();
  styleCache.set(key, created);
  return created;
}

export function nativeFeatureStyle(
  feature: FeatureLike,
  layers: readonly VectorLayerDefinition[],
  state: FeatureStyleState,
): Style | undefined {
  const layerId = feature.get("daenaLayerId") as string | undefined;
  if (layerId === BASE_LAYER_ID) {
    if (!baseVisible(layers)) return undefined;
    const name = feature.get("name");
    const text = typeof name === "string" ? name.trim() : "";
    const key = JSON.stringify(["base", text, state.hovered, state.selected]);
    return cacheStyle(
      key,
      () =>
        new Style({
          fill: new Fill({ color: state.selected ? "rgba(213, 171, 108, 0.92)" : "rgba(201, 169, 110, 0.92)" }),
          stroke: new Stroke({ color: state.hovered ? "#f3d39a" : "#8a7048", width: state.selected ? 2.5 : 1.25 }),
          text: text
            ? new Text({
                text,
                offsetY: -14,
                font: "600 12px system-ui",
                fill: new Fill({ color: "#f7f0e5" }),
                stroke: new Stroke({ color: "rgba(13, 27, 42, 0.9)", width: 3 }),
                overflow: true,
              })
            : undefined,
        }),
    );
  }

  const layer = layers.find((candidate) => candidate.id === layerId);
  if (!layer?.defaultVisible) return undefined;
  const style = resolvedStyle(feature, layer);
  const label = resolvedLabel(feature, style);
  const text = labelVisible(label, state.zoom) ? labelText(feature, label) : "";
  const key = JSON.stringify([style, label, text, state.hovered, state.selected]);
  return cacheStyle(key, () => {
    const fillColor = state.selected ? "rgba(213, 171, 108, 0.56)" : colorWithOpacity(style.fill, style.fillOpacity);
    const strokeColor =
      state.hovered || state.selected ? "#f3d39a" : colorWithOpacity(style.stroke, style.strokeOpacity ?? 1);
    const width = state.selected
      ? Math.max(2.5, style.strokeWidth)
      : state.hovered
        ? Math.max(2, style.strokeWidth)
        : style.strokeWidth;
    const fill = new Fill({ color: fillColor });
    const stroke = new Stroke({
      color: strokeColor,
      width,
      lineDash: style.strokeDash ? [...style.strokeDash] : undefined,
    });
    const radius = state.selected
      ? Math.max(style.iconSize ? style.iconSize / 2 : 0, 7, style.pointRadius)
      : Math.max(style.iconSize ? style.iconSize / 2 : 0, style.pointRadius);
    return new Style({
      fill,
      stroke,
      image: markerImage(style.icon, radius, fill, new Stroke({ color: strokeColor, width: Math.min(width, 4) })),
      text:
        text && label
          ? new Text({
              text,
              font: `600 ${label.size}px system-ui`,
              fill: new Fill({ color: label.color }),
              stroke: new Stroke({ color: label.haloColor, width: label.haloWidth }),
              placement: label.placement === "line" ? "line" : "point",
              offsetX: label.offset[0],
              offsetY: label.offset[1],
              rotation: (label.rotation * Math.PI) / 180,
              overflow: true,
            })
          : undefined,
    });
  });
}

export function clearFeatureStyleCache() {
  styleCache.clear();
}

export function featureStyleCacheSize() {
  return styleCache.size;
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
    if (!layer.locked || snapTargetLayerIds.has(layer.id)) enabled.add(layer.id);
  }
  if (baseVisible(layers)) enabled.add(BASE_LAYER_ID);
  return {
    type: "FeatureCollection",
    features: collection.features.filter((feature) => enabled.has(featureLayerId(feature))),
  };
}
