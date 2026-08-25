export function lonLatToNormalized(longitude: number, latitude: number): [number, number] {
  return [(longitude + 180) / 360, (90 - latitude) / 180];
}

export function normalizedToLonLat(x: number, y: number): [number, number] {
  return [x * 360 - 180, 90 - y * 180];
}

export type ImageOverlayCoordinates = [[number, number], [number, number], [number, number], [number, number]];
export const PHYSICAL_RASTER_OVERSAMPLE = 1;

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
    [-180, 90],
    [180, 90],
    [180, -90],
    [-180, -90],
  ];
}

/** Convert a north-origin canvas row to the generator's south-origin source grid. */
export function physicalGridRowForRasterRow(canvasRow: number, canvasHeight: number, sourceHeight: number): number {
  if (sourceHeight <= 1 || canvasHeight <= 0) return 0;
  const row = Math.floor(((canvasRow + 0.5) / canvasHeight) * sourceHeight);
  return Math.max(0, Math.min(sourceHeight - 1, sourceHeight - 1 - row));
}
