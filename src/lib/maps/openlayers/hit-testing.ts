import type Feature from "ol/Feature.js";
import type Geometry from "ol/geom/Geometry.js";
import type Map from "ol/Map.js";
import type VectorLayer from "ol/layer/Vector.js";
import type { MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import { lonLatToNormalized } from "../native-vector/coordinates";
import { toVectorFeature } from "./feature-codec";

export function featureAtPixel(map: Map, vectorLayer: VectorLayer, pixel: number[]): Feature<Geometry> | undefined {
  return map.forEachFeatureAtPixel(
    pixel,
    (feature, layer) => (layer === vectorLayer ? (feature as Feature<Geometry>) : undefined),
    { hitTolerance: 6, layerFilter: (layer) => layer === vectorLayer },
  );
}

export function anchorForFeature(
  feature: Feature<Geometry> | undefined,
  coordinate: number[],
  fallbackLayerId: string,
): MapAnchor {
  const converted = feature ? toVectorFeature(feature, fallbackLayerId) : null;
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
  return {
    kind: "point",
    point: [Math.max(0, Math.min(1, normalized[0])), Math.max(0, Math.min(1, normalized[1]))],
  };
}
