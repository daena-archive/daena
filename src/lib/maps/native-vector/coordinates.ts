/** MapLibre Web Mercator latitude limit used by adapter version 1. */
export const WEB_MERCATOR_MAX_LAT = 85.05112878;

export function lonLatToNormalized(longitude: number, latitude: number): [number, number] {
  return [(longitude + 180) / 360, (90 - latitude) / 180];
}

export function normalizedToLonLat(x: number, y: number): [number, number] {
  return [x * 360 - 180, 90 - y * 180];
}
