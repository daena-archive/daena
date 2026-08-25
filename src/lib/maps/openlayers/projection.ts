import Projection from "ol/proj/Projection.js";

export const WORLD_EXTENT: [number, number, number, number] = [-180, -90, 180, 90];

export const WORLD_RESOLUTIONS = Array.from({ length: 13 }, (_, zoom) => 360 / 256 / 2 ** zoom);

export const worldProjection = new Projection({
  code: "DAENA:WORLD",
  units: "degrees",
  extent: WORLD_EXTENT,
  worldExtent: WORLD_EXTENT,
});
