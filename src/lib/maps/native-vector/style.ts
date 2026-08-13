import type { FilterSpecification, StyleSpecification } from "maplibre-gl";
import { BASE_LAYER_ID, type VectorFeature, type VectorFeatureCollection, type VectorLayerDefinition } from "./types";

export const BASE_SOURCE_ID = "daena-base";
export const AUTHORED_SOURCE_ID = "daena-authored";

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

export function nativeVectorStyle(layers: readonly VectorLayerDefinition[]): StyleSpecification {
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
        paint: { "fill-color": "#c9a96e", "fill-opacity": 0.92 },
      },
      {
        id: "daena-base-line",
        type: "line",
        source: BASE_SOURCE_ID,
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

  return style;
}

export function styleContainsRemoteUrl(style: StyleSpecification): boolean {
  return JSON.stringify(style).search(/https?:\/\//i) >= 0;
}
