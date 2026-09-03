import type { MapLabelV2 } from "../../../../packages/plugin-sdk/src/maps";
import type { VectorFeature, VectorFeatureCollection } from "./types";

export function cloneCollection(collection: VectorFeatureCollection): VectorFeatureCollection {
  // `draft` is Svelte state and may be a reactive Proxy after an edit. The
  // browser structured-clone algorithm rejects that proxy, while GeoJSON is
  // intentionally JSON-shaped and can be copied safely at this boundary.
  return JSON.parse(JSON.stringify(collection)) as VectorFeatureCollection;
}

export function offsetGeometry(geometry: VectorFeature["geometry"], dx: number, dy: number): VectorFeature["geometry"] {
  const shift = (coords: number[]): number[] => [coords[0] + dx, coords[1] + dy, ...coords.slice(2)];
  const walk = (value: unknown, depth: number): unknown => {
    if (depth === 0) return shift(value as number[]);
    return (value as unknown[]).map((item) => walk(item, depth - 1));
  };
  switch (geometry.type) {
    case "Point":
      return { type: "Point", coordinates: shift(geometry.coordinates) };
    case "MultiPoint":
      return { type: "MultiPoint", coordinates: walk(geometry.coordinates, 1) as number[][] };
    case "LineString":
      return { type: "LineString", coordinates: walk(geometry.coordinates, 1) as number[][] };
    case "MultiLineString":
      return { type: "MultiLineString", coordinates: walk(geometry.coordinates, 2) as number[][][] };
    case "Polygon":
      return { type: "Polygon", coordinates: walk(geometry.coordinates, 2) as number[][][] };
    case "MultiPolygon":
      return { type: "MultiPolygon", coordinates: walk(geometry.coordinates, 3) as number[][][][] };
  }
}

export function selectedMetadataSnapshot(feature: VectorFeature) {
  return {
    name: feature.properties.daena.name,
    semanticType: feature.properties.daena.semanticType,
    style: feature.properties.daena.style,
    label: feature.properties.daena.label,
    custom: feature.properties.daena.custom,
  };
}

export function defaultFeatureLabel(feature: VectorFeature): MapLabelV2 {
  return {
    source: "name",
    text: null,
    size: 12,
    color: "#f7f0e5",
    haloColor: "#0d1b2a",
    haloWidth: 3,
    placement: feature.geometry.type === "LineString" || feature.geometry.type === "MultiLineString" ? "line" : "point",
    offset: [0, -14],
    rotation: 0,
    minZoom: null,
    maxZoom: null,
  };
}

export function featureVertexCount(feature: VectorFeature) {
  return (feature.geometry.coordinates.flat(Infinity) as number[]).length / 2;
}
