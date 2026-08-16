import type { FilterSpecification, StyleSpecification } from "maplibre-gl";
import { BASE_LAYER_ID, type VectorFeature, type VectorFeatureCollection, type VectorLayerDefinition } from "./types";

export const BASE_SOURCE_ID = "daena-base";
export const AUTHORED_SOURCE_ID = "daena-authored";
export const IMAGE_SOURCE_ID = "daena-preview";
export const IMAGE_LAYER_ID = "daena-preview-overlay";

export function emptyFeatureCollection(): VectorFeatureCollection {
  return { type: "FeatureCollection", features: [] };
}

export function splitVectorSources(
  collection: VectorFeatureCollection,
  activeLayerId: string | null,
): { base: VectorFeatureCollection; authored: VectorFeatureCollection } {
  const base: VectorFeature[] = [];
  const authored: VectorFeature[] = [];
  for (const feature of collection.features) {
    if (feature.properties.daenaLayerId === BASE_LAYER_ID) base.push(feature);
    else if (feature.properties.daenaLayerId !== activeLayerId) authored.push(feature);
  }
  return {
    base: { type: "FeatureCollection", features: base },
    authored: { type: "FeatureCollection", features: authored },
  };
}

export function layerFilter(layerId: string): FilterSpecification {
  return ["==", ["get", "daenaLayerId"], layerId];
}

export function nativeBaseLayerVisibility(layers: readonly VectorLayerDefinition[]): "visible" | "none" {
  const base = layers.find((layer) => layer.id === BASE_LAYER_ID);
  return base?.defaultVisible ? "visible" : "none";
}

export function nativeVectorStyle(layers: readonly VectorLayerDefinition[]): StyleSpecification {
  const baseVisibility = nativeBaseLayerVisibility(layers);
  const style: StyleSpecification = {
    version: 8,
    sources: {
      [BASE_SOURCE_ID]: { type: "geojson", data: emptyFeatureCollection() },
      [AUTHORED_SOURCE_ID]: { type: "geojson", data: emptyFeatureCollection() },
    },
    layers: [
      { id: "daena-background", type: "background", paint: { "background-color": "#0d1b2a" } },
      {
        id: "daena-base-fill",
        type: "fill",
        source: BASE_SOURCE_ID,
        layout: { visibility: baseVisibility },
        paint: { "fill-color": "#c9a96e", "fill-opacity": 0.92 },
      },
      {
        id: "daena-base-line",
        type: "line",
        source: BASE_SOURCE_ID,
        layout: { visibility: baseVisibility },
        paint: { "line-color": "#8a7048", "line-width": 1.25 },
      },
    ],
  };

  for (const layer of [...layers].sort((left, right) => left.order - right.order || left.id.localeCompare(right.id))) {
    const filter = layerFilter(layer.id);
    style.layers.push(
      {
        id: `daena-vector-${layer.id}-fill`,
        type: "fill",
        source: AUTHORED_SOURCE_ID,
        filter,
        layout: { visibility: layer.defaultVisible ? "visible" : "none" },
        paint: { "fill-color": layer.style.fill, "fill-opacity": layer.style.fillOpacity },
      },
      {
        id: `daena-vector-${layer.id}-line`,
        type: "line",
        source: AUTHORED_SOURCE_ID,
        filter,
        layout: { visibility: layer.defaultVisible ? "visible" : "none" },
        paint: { "line-color": layer.style.stroke, "line-width": layer.style.strokeWidth },
      },
      {
        id: `daena-vector-${layer.id}-point`,
        type: "circle",
        source: AUTHORED_SOURCE_ID,
        filter,
        layout: { visibility: layer.defaultVisible ? "visible" : "none" },
        paint: {
          "circle-color": layer.style.fill,
          "circle-radius": layer.style.pointRadius,
          "circle-stroke-color": layer.style.stroke,
          "circle-stroke-width": Math.min(layer.style.strokeWidth, 4),
        },
      },
    );
  }

  style.layers.push(
    {
      id: "daena-hover-fill",
      type: "fill",
      source: AUTHORED_SOURCE_ID,
      paint: {
        "fill-color": "#f3d39a",
        "fill-opacity": ["case", ["boolean", ["feature-state", "hover"], false], 0.18, 0],
      },
    },
    {
      id: "daena-hover-line",
      type: "line",
      source: AUTHORED_SOURCE_ID,
      paint: {
        "line-color": "#f3d39a",
        "line-width": 2,
        "line-opacity": ["case", ["boolean", ["feature-state", "hover"], false], 1, 0],
      },
    },
    {
      id: "daena-selection-fill",
      type: "fill",
      source: AUTHORED_SOURCE_ID,
      paint: {
        "fill-color": "#d5ab6c",
        "fill-opacity": ["case", ["boolean", ["feature-state", "selected"], false], 0.22, 0],
      },
    },
    {
      id: "daena-selection-line",
      type: "line",
      source: AUTHORED_SOURCE_ID,
      paint: {
        "line-color": "#d5ab6c",
        "line-width": 2.5,
        "line-opacity": ["case", ["boolean", ["feature-state", "selected"], false], 1, 0],
      },
    },
    {
      id: "daena-selection-point",
      type: "circle",
      source: AUTHORED_SOURCE_ID,
      paint: {
        "circle-color": "#d5ab6c",
        "circle-radius": 7,
        "circle-stroke-color": "#f3d39a",
        "circle-stroke-width": 2,
        "circle-opacity": ["case", ["boolean", ["feature-state", "selected"], false], 1, 0],
        "circle-stroke-opacity": ["case", ["boolean", ["feature-state", "selected"], false], 1, 0],
      },
    },
  );

  return style;
}

export function styleContainsRemoteUrl(style: StyleSpecification): boolean {
  return /https?:\/\//i.test(JSON.stringify(style).replace(/blob:[^"]+/g, ""));
}
