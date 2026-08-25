import type { VectorFeature, VectorFeatureCollection, VectorLayerDefinition } from "../native-vector/types";

/** Plate carrée: x is longitude, y is latitude, both in degrees on a 360×180 world. */
export function lonLatToEquirectangular(longitude: number, latitude: number): [number, number] {
  return [longitude + 180, 90 - latitude];
}

function projectRing(ring: number[][]): string {
  return ring
    .map((position, index) => {
      const [x, y] = lonLatToEquirectangular(position[0] ?? 0, position[1] ?? 0);
      return `${index === 0 ? "M" : "L"}${x} ${y}`;
    })
    .join(" ");
}

export function geometryToSvg(feature: VectorFeature): { kind: "path" | "point"; d?: string; x?: number; y?: number } {
  const geometry = feature.geometry;
  if (geometry.type === "Point") {
    const [x, y] = lonLatToEquirectangular(geometry.coordinates[0] ?? 0, geometry.coordinates[1] ?? 0);
    return { kind: "point", x, y };
  }
  if (geometry.type === "LineString") {
    return { kind: "path", d: projectRing(geometry.coordinates) };
  }
  if (geometry.type === "Polygon") {
    return { kind: "path", d: geometry.coordinates.map((ring) => `${projectRing(ring)} Z`).join(" ") };
  }
  return {
    kind: "path",
    d: geometry.coordinates.flatMap((polygon) => polygon.map((ring) => `${projectRing(ring)} Z`)).join(" "),
  };
}

export function visiblePhysicalFeatures(collection: VectorFeatureCollection, layers: readonly VectorLayerDefinition[]) {
  const visible = new Map(layers.filter((layer) => layer.defaultVisible).map((layer) => [layer.id, layer] as const));
  return collection.features.flatMap((feature) => {
    const layer = visible.get(feature.properties.daena.layerId);
    if (!layer) return [];
    return [{ feature, layer, svg: geometryToSvg(feature) }];
  });
}
