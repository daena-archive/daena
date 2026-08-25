import type Feature from "ol/Feature.js";
import GeoJSON from "ol/format/GeoJSON.js";
import type Geometry from "ol/geom/Geometry.js";
import VectorSource from "ol/source/Vector.js";
import { drawModeForGeometry, kindForDrawMode } from "../native-vector/geometry";
import {
  BASE_LAYER_ID,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorKind,
} from "../native-vector/types";
import { worldProjection } from "./projection";

export const geoJsonFormat = new GeoJSON({
  dataProjection: worldProjection,
  featureProjection: worldProjection,
});

function featureKind(value: unknown, geometry: VectorFeature["geometry"]): VectorKind {
  if (
    value === "land" ||
    value === "lake" ||
    value === "region" ||
    value === "route" ||
    value === "marker" ||
    value === "custom"
  ) {
    return value;
  }
  return kindForDrawMode(drawModeForGeometry(geometry));
}

/** Flatten nested daena props onto OL feature properties for editing interactions. */
export function readOlFeatures(collection: VectorFeatureCollection): Feature<Geometry>[] {
  const features = geoJsonFormat.readFeatures(collection as Parameters<GeoJSON["readFeatures"]>[0]) as Feature<Geometry>[];
  for (const feature of features) {
    const daena = feature.get("daena") as
      | { layerId?: unknown; semanticType?: unknown; name?: unknown }
      | undefined;
    if (daena && typeof daena === "object") {
      feature.setProperties({
        daenaLayerId: typeof daena.layerId === "string" ? daena.layerId : BASE_LAYER_ID,
        kind: typeof daena.semanticType === "string" ? daena.semanticType : "custom",
        name: typeof daena.name === "string" ? daena.name : null,
      });
      feature.unset("daena");
    }
  }
  return features;
}

/** Encode an OL feature back to nested Daena GeoJSON. */
export function toVectorFeature(feature: Feature<Geometry>, fallbackLayerId: string): VectorFeature | null {
  const object = geoJsonFormat.writeFeatureObject(feature) as {
    id?: string | number;
    properties?: Record<string, unknown> | null;
    geometry?: VectorFeature["geometry"] | null;
  };
  const geometry = object.geometry;
  if (
    !geometry ||
    (geometry.type !== "Point" &&
      geometry.type !== "MultiPoint" &&
      geometry.type !== "LineString" &&
      geometry.type !== "MultiLineString" &&
      geometry.type !== "Polygon" &&
      geometry.type !== "MultiPolygon")
  ) {
    return null;
  }
  const properties = object.properties ?? {};
  const layerId =
    typeof properties.daenaLayerId === "string"
      ? properties.daenaLayerId
      : typeof (properties.daena as { layerId?: unknown } | undefined)?.layerId === "string"
        ? (properties.daena as { layerId: string }).layerId
        : fallbackLayerId;
  const kind = featureKind(
    properties.kind ?? (properties.daena as { semanticType?: unknown } | undefined)?.semanticType,
    geometry,
  );
  const name =
    typeof properties.name === "string"
      ? properties.name
      : typeof (properties.daena as { name?: unknown } | undefined)?.name === "string"
        ? (properties.daena as { name: string }).name
        : null;
  return {
    type: "Feature",
    id: String(feature.getId() ?? object.id ?? crypto.randomUUID()),
    properties: {
      daena: {
        layerId,
        semanticType: kind,
        name,
        style: null,
        label: null,
        custom: {},
      },
    },
    geometry,
  };
}

export function collectionFromSource(
  source: VectorSource<Feature<Geometry>>,
  fallbackLayerId: string,
): VectorFeatureCollection {
  return {
    type: "FeatureCollection",
    features: source
      .getFeatures()
      .map((feature) => toVectorFeature(feature, fallbackLayerId))
      .filter((feature): feature is VectorFeature => feature !== null)
      .sort((left, right) => left.id.localeCompare(right.id)),
  };
}

export function collectionBounds(collection: VectorFeatureCollection): [number, number, number, number] | null {
  const features = readOlFeatures(collection);
  if (features.length === 0) return null;
  const extent = new VectorSource({ features }).getExtent();
  return extent && extent.every(Number.isFinite) ? (extent as [number, number, number, number]) : null;
}

export function collectionSignature(collection: VectorFeatureCollection): string {
  return JSON.stringify(collection);
}
