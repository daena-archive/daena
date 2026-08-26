import type Feature from "ol/Feature.js";
import GeoJSON from "ol/format/GeoJSON.js";
import type Geometry from "ol/geom/Geometry.js";
import type Projection from "ol/proj/Projection.js";
import VectorSource from "ol/source/Vector.js";
import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps";
import { authoredToView, mapPositions, viewToAuthored } from "../editor/coordinate-space";
import { drawModeForGeometry, kindForDrawMode } from "../native-vector/geometry";
import {
  BASE_LAYER_ID,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorKind,
} from "../native-vector/types";

export type FeatureCodec = {
  format: GeoJSON;
  readOlFeatures: (collection: VectorFeatureCollection) => Feature<Geometry>[];
  toVectorFeature: (feature: Feature<Geometry>, fallbackLayerId: string) => VectorFeature | null;
  collectionFromSource: (source: VectorSource<Feature<Geometry>>, fallbackLayerId: string) => VectorFeatureCollection;
  collectionFromSources: (sources: readonly VectorSource<Feature<Geometry>>[]) => VectorFeatureCollection;
  collectionBounds: (collection: VectorFeatureCollection) => [number, number, number, number] | null;
  readGeometry: (geometry: VectorFeature["geometry"]) => Geometry;
};

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

export function createGeoJsonFormat(projection: Projection): GeoJSON {
  return new GeoJSON({
    dataProjection: projection,
    featureProjection: projection,
  });
}

export function createFeatureCodec(space: MapCoordinateSpace, projection: Projection): FeatureCodec {
  const format = createGeoJsonFormat(projection);
  const toView = (position: number[]) => authoredToView(position, space);
  const toAuthored = (position: number[]) => viewToAuthored(position, space);

  const readOlFeatures = (collection: VectorFeatureCollection): Feature<Geometry>[] => {
    const viewCollection = {
      type: "FeatureCollection",
      features: collection.features.map((feature) => ({
        ...feature,
        geometry: mapPositions(feature.geometry, toView),
      })),
    };
    const features = format.readFeatures(
      viewCollection as Parameters<GeoJSON["readFeatures"]>[0],
    ) as Feature<Geometry>[];
    for (const feature of features) {
      const daena = feature.get("daena") as
        | {
            layerId?: unknown;
            semanticType?: unknown;
            name?: unknown;
            style?: unknown;
            label?: unknown;
            custom?: unknown;
          }
        | undefined;
      if (daena && typeof daena === "object") {
        feature.setProperties({
          daenaLayerId: typeof daena.layerId === "string" ? daena.layerId : BASE_LAYER_ID,
          kind: typeof daena.semanticType === "string" ? daena.semanticType : "custom",
          name: typeof daena.name === "string" ? daena.name : null,
          daenaStyle: daena.style ?? null,
          daenaLabel: daena.label ?? null,
          daenaCustom:
            daena.custom && typeof daena.custom === "object" && !Array.isArray(daena.custom) ? daena.custom : {},
        });
        feature.unset("daena");
      }
    }
    return features;
  };

  const toVectorFeature = (feature: Feature<Geometry>, fallbackLayerId: string): VectorFeature | null => {
    const object = format.writeFeatureObject(feature) as {
      id?: string | number;
      properties?: Record<string, unknown> | null;
      geometry?: VectorFeature["geometry"] | null;
    };
    const geometry = object.geometry ? mapPositions(object.geometry, toAuthored) : null;
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
    const nested = properties.daena as { style?: unknown; label?: unknown; custom?: unknown } | undefined;
    const style = properties.daenaStyle ?? nested?.style;
    const label = properties.daenaLabel ?? nested?.label;
    const custom = properties.daenaCustom ?? nested?.custom;
    return {
      type: "Feature",
      id: String(feature.getId() ?? object.id ?? crypto.randomUUID()),
      properties: {
        daena: {
          layerId,
          semanticType: kind,
          name,
          style:
            style && typeof style === "object" && !Array.isArray(style)
              ? (style as VectorFeature["properties"]["daena"]["style"])
              : null,
          label:
            label && typeof label === "object" && !Array.isArray(label)
              ? (label as VectorFeature["properties"]["daena"]["label"])
              : null,
          custom:
            custom && typeof custom === "object" && !Array.isArray(custom)
              ? (custom as VectorFeature["properties"]["daena"]["custom"])
              : {},
        },
      },
      geometry,
    };
  };

  return {
    format,
    readOlFeatures,
    toVectorFeature,
    collectionFromSource(source, fallbackLayerId) {
      return {
        type: "FeatureCollection",
        features: source
          .getFeatures()
          .map((feature) => toVectorFeature(feature, fallbackLayerId))
          .filter((feature): feature is VectorFeature => feature !== null)
          .sort((left, right) => left.id.localeCompare(right.id)),
      };
    },
    collectionFromSources(sources) {
      const features = sources.flatMap((source) =>
        source
          .getFeatures()
          .map((feature) => toVectorFeature(feature, BASE_LAYER_ID))
          .filter((feature): feature is VectorFeature => feature !== null),
      );
      return {
        type: "FeatureCollection",
        features: features.sort((left, right) => left.id.localeCompare(right.id)),
      };
    },
    collectionBounds(collection) {
      const features = readOlFeatures(collection);
      if (features.length === 0) return null;
      const extent = new VectorSource({ features }).getExtent();
      return extent && extent.every(Number.isFinite) ? (extent as [number, number, number, number]) : null;
    },
    readGeometry(geometry) {
      return format.readGeometry(mapPositions(geometry, toView) as Parameters<GeoJSON["readGeometry"]>[0]);
    },
  };
}

export function collectionSignature(collection: VectorFeatureCollection): string {
  return JSON.stringify(collection);
}
