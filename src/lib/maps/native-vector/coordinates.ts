/** MapLibre Web Mercator latitude limit used by adapter version 1. */
export const WEB_MERCATOR_MAX_LAT = 85.05112878;

export function lonLatToNormalized(longitude: number, latitude: number): [number, number] {
  return [(longitude + 180) / 360, (90 - latitude) / 180];
}

export function normalizedToLonLat(x: number, y: number): [number, number] {
  return [x * 360 - 180, 90 - y * 180];
}

export type ImageOverlayCoordinates = [[number, number], [number, number], [number, number], [number, number]];
export const PHYSICAL_RASTER_OVERSAMPLE = 8;

/** Generation/image overlay extent used by adapter version 1. */
export const IMAGE_OVERLAY_LON_SPAN = 340;
export const IMAGE_OVERLAY_LAT_SPAN = 150;

export function imageOverlayCoordinates(width: number, height: number): ImageOverlayCoordinates {
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

/** Physical grids cover the complete world; keep their raster and GeoJSON extents identical. */
export function physicalWorldOverlayCoordinates(): ImageOverlayCoordinates {
  return [
    [-180, WEB_MERCATOR_MAX_LAT],
    [180, WEB_MERCATOR_MAX_LAT],
    [180, -WEB_MERCATOR_MAX_LAT],
    [-180, -WEB_MERCATOR_MAX_LAT],
  ];
}

/** Convert a Web-Mercator canvas row to the source grid row at its pixel center. */
export function physicalGridRowForRasterRow(canvasRow: number, canvasHeight: number, sourceHeight: number): number {
  if (sourceHeight <= 1 || canvasHeight <= 0) return 0;
  const mercatorY = (canvasRow + 0.5) / canvasHeight;
  const mercatorN = Math.PI * (1 - 2 * mercatorY);
  const latitude = (Math.atan(Math.sinh(mercatorN)) * 180) / Math.PI;
  return Math.max(0, Math.min(sourceHeight - 1, Math.floor(((latitude + 90) / 180) * sourceHeight)));
}
