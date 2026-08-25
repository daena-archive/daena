import type Feature from "ol/Feature.js";
import type Geometry from "ol/geom/Geometry.js";
import type Map from "ol/Map.js";
import type VectorLayer from "ol/layer/Vector.js";
import type { MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps";
import { authoredToNormalized, viewToAuthored } from "../editor/coordinate-space";
import type { FeatureCodec } from "./feature-codec";

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
  space: MapCoordinateSpace,
  codec: FeatureCodec,
): MapAnchor {
  const converted = feature ? codec.toVectorFeature(feature, fallbackLayerId) : null;
  if (converted) {
    const positions = converted.geometry.coordinates.flat(Infinity) as number[];
    return {
      kind: "provider-feature",
      provider: "daena-openlayers",
      featureKind: "geojson-feature",
      featureId: converted.id,
      fallbackPoint:
        positions.length >= 2
          ? authoredToNormalized(positions[0], positions[1], space)
          : authoredToNormalized(...viewToAuthored(coordinate, space), space),
    };
  }
  const authored = viewToAuthored(coordinate, space);
  const normalized = authoredToNormalized(authored[0], authored[1], space);
  return {
    kind: "point",
    point: [normalized[0], normalized[1]],
  };
}
