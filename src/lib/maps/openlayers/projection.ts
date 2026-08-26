import Projection from "ol/proj/Projection.js";
import { get as getProjection } from "ol/proj.js";
import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps.ts";
import { DEFAULT_WORLD_SPACE, WORLD_EXTENT, extentOf, wrapXOf, type Extent4 } from "../editor/coordinate-space.ts";

export { WORLD_EXTENT };

export const WORLD_RESOLUTIONS = Array.from({ length: 13 }, (_, zoom) => 360 / 256 / 2 ** zoom);

export const worldProjection = new Projection({
  code: "DAENA:WORLD",
  units: "degrees",
  extent: WORLD_EXTENT,
  worldExtent: WORLD_EXTENT,
  getPointResolution: (resolution) => resolution,
});

function sameExtent(left: Extent4, right: Extent4): boolean {
  return left[0] === right[0] && left[1] === right[1] && left[2] === right[2] && left[3] === right[3];
}

export function projectionFromCoordinateSpace(space: MapCoordinateSpace): Projection {
  const extent = extentOf(space);
  if (
    space.kind === "world" &&
    sameExtent(extent, WORLD_EXTENT) &&
    space.units.id === "world-unit" &&
    space.units.metresPerUnit == null &&
    !space.wrapX
  ) {
    return worldProjection;
  }
  if (space.kind === "geographic") {
    if (space.wrapX) {
      const geographic = getProjection("EPSG:4326");
      if (geographic) return geographic;
    }
    return new Projection({
      code: `DAENA:GEOGRAPHIC:${extent.join(",")}`,
      units: "degrees",
      extent,
      worldExtent: extent,
      global: space.wrapX,
      getPointResolution: (resolution) => resolution,
    });
  }
  if (space.kind === "image") {
    return new Projection({
      code: `DAENA:IMAGE:${extent.join(",")}`,
      units: "pixels",
      extent,
      worldExtent: extent,
      metersPerUnit: 1,
      getPointResolution: (resolution) => resolution,
    });
  }
  const metres = space.units.metresPerUnit;
  return new Projection({
    code: `DAENA:WORLD:${extent.join(",")}:${space.units.id}:${metres ?? "none"}:${space.wrapX ? "wrap" : "nowrap"}`,
    units: metres != null ? "m" : "pixels",
    extent,
    worldExtent: extent,
    global: wrapXOf(space),
    metersPerUnit: metres ?? undefined,
    getPointResolution: (resolution) => resolution,
  });
}

export function resolutionsForCoordinateSpace(space: MapCoordinateSpace, levels = 19): number[] {
  const [minX, , maxX] = extentOf(space);
  const width = Math.max(maxX - minX, Number.EPSILON);
  return Array.from({ length: levels }, (_, zoom) => width / 256 / 2 ** zoom);
}

export function viewExtentForCoordinateSpace(space: MapCoordinateSpace): Extent4 {
  return extentOf(space);
}

export function maxZoomForCoordinateSpace(space: MapCoordinateSpace): number {
  return space.kind === "image" ? 18 : 12;
}

export function defaultCoordinateSpace(): MapCoordinateSpace {
  return DEFAULT_WORLD_SPACE;
}
