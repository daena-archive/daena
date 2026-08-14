/** MapLibre Web Mercator latitude limit used by adapter version 1. */
export const WEB_MERCATOR_MAX_LAT = 85.05112878;

export function lonLatToNormalized(longitude: number, latitude: number): [number, number] {
  return [(longitude + 180) / 360, (90 - latitude) / 180];
}

export function normalizedToLonLat(x: number, y: number): [number, number] {
  return [x * 360 - 180, 90 - y * 180];
}

/** Generation/image overlay extent used by adapter version 1. */
export const IMAGE_OVERLAY_LON_SPAN = 340;
export const IMAGE_OVERLAY_LAT_SPAN = 150;

export function imageOverlayCoordinates(
  width: number,
  height: number,
): [[number, number], [number, number], [number, number], [number, number]] {
  const extentAspect = IMAGE_OVERLAY_LON_SPAN / IMAGE_OVERLAY_LAT_SPAN;
  const imageAspect = width / Math.max(height, 1);
  let lonSpan = IMAGE_OVERLAY_LON_SPAN;
  let latSpan = IMAGE_OVERLAY_LAT_SPAN;
  if (imageAspect > extentAspect) latSpan = lonSpan / imageAspect;
  else lonSpan = latSpan * imageAspect;
  const west = -lonSpan / 2;
  const north = latSpan / 2;
  const east = west + lonSpan;
  const south = north - latSpan;
  return [
    [west, north],
    [east, north],
    [east, south],
    [west, south],
  ];
}
