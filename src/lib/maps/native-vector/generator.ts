import { contours } from "d3-contour";

export const GENERATOR_VERSION = 1 as const;
export const CANDIDATE_COUNT = 6;
const GRID_WIDTH = 512;
const GRID_HEIGHT = 256;
const VALUE_COUNT = GRID_WIDTH * GRID_HEIGHT;
const LON_MIN = -170;
const LON_SPAN = 340;
const LAT_MAX = 75;
const LAT_SPAN = 150;
const SCALE = 1_000_000;
const ANTIMERIDIAN = 180_000_000;
const MIN_GRID_AREA = 4;
const BASE_RX = [0.38, 0.34, 0.3, 0.27, 0.24, 0.22, 0.2, 0.18];
const NOISE_AMPLITUDE = {
  low: [1, 0.35, 0.12, 0.04, 0.01],
  medium: [1, 0.5, 0.25, 0.125, 0.0625],
  high: [1, 0.65, 0.42, 0.27, 0.18],
} as const;
const SIMPLIFY_THRESHOLD = { low: 2.25, medium: 1, high: 0.25 } as const;
const ISLAND_COUNT = { none: 0, low: 4, medium: 10, high: 20 } as const;

export type CoastlineRoughness = "low" | "medium" | "high";
export type IslandFrequency = "none" | "low" | "medium" | "high";

export type NativeGeneratorSettings = {
  generatorVersion: 1;
  seed: number;
  landPercent: number;
  continentCount: number;
  coastlineRoughness: CoastlineRoughness;
  islandFrequency: IslandFrequency;
};

export type NativeGeneratorCandidate = {
  index: number;
  seed: number;
  collection: string;
  svg: string;
};

export const DEFAULT_GENERATOR_SETTINGS: NativeGeneratorSettings = {
  generatorVersion: GENERATOR_VERSION,
  seed: 831429,
  landPercent: 40,
  continentCount: 3,
  coastlineRoughness: "medium",
  islandFrequency: "medium",
};

export function mix32(value: number) {
  let x = value >>> 0;
  x ^= x >>> 16;
  x = Math.imul(x, 0x85ebca6b);
  x ^= x >>> 13;
  x = Math.imul(x, 0xc2b2ae35);
  return (x ^ (x >>> 16)) >>> 0;
}

export function next(state: number): [number, number] {
  state = (state + 0x6d2b79f5) >>> 0;
  let t = state;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return [((t ^ (t >>> 14)) >>> 0) / 0x1_0000_0000, state];
}

export function candidateSeed(seed: number, index: number) {
  return mix32((seed >>> 0) ^ Math.imul(index + 1, 0x9e3779b9));
}

export function validateGeneratorSettings(settings: NativeGeneratorSettings): string | null {
  if (settings.generatorVersion !== 1) return "vector.source.unsupported-version";
  if (!Number.isInteger(settings.seed) || settings.seed < 0 || settings.seed > 0xffff_ffff) {
    return "vector.generator.invalid-settings";
  }
  if (!Number.isInteger(settings.landPercent) || settings.landPercent < 15 || settings.landPercent > 70) {
    return "vector.generator.invalid-settings";
  }
  if (!Number.isInteger(settings.continentCount) || settings.continentCount < 1 || settings.continentCount > 8) {
    return "vector.generator.invalid-settings";
  }
  if (!["low", "medium", "high"].includes(settings.coastlineRoughness)) return "vector.generator.invalid-settings";
  if (!["none", "low", "medium", "high"].includes(settings.islandFrequency)) {
    return "vector.generator.invalid-settings";
  }
  return null;
}

export function generationProvenance(settings: NativeGeneratorSettings) {
  return {
    id: "daena-landmass",
    version: 1,
    seed: settings.seed >>> 0,
    settings: {
      landPercent: settings.landPercent,
      continentCount: settings.continentCount,
      coastlineRoughness: settings.coastlineRoughness,
      islandFrequency: settings.islandFrequency,
    },
  };
}

type Micro = { lon: number; lat: number };
type Ring = Micro[];
type Polygon = { exterior: Ring; holes: Ring[]; area: bigint };

function compactKernel(dx: number, dy: number, rx: number, ry: number) {
  const q = (dx / rx) * (dx / rx) + (dy / ry) * (dy / ry);
  return q < 1 ? (1 - q) * (1 - q) : 0;
}

function lattice(candidate: number, ix: number, iy: number) {
  return (2 * mix32(candidate ^ Math.imul(ix, 0x1f123bb5) ^ Math.imul(iy, 0x5f356495))) / 0x1_0000_0000 - 1;
}

function valueNoise(candidate: number, x: number, y: number, frequency: number) {
  const dx = x * frequency;
  const dy = y * frequency;
  const ix = Math.floor(dx);
  const iy = Math.floor(dy);
  const tx = dx - ix;
  const ty = dy - iy;
  const sx = tx * tx * (3 - 2 * tx);
  const sy = ty * ty * (3 - 2 * ty);
  const v00 = lattice(candidate, ix, iy);
  const v10 = lattice(candidate, ix + 1, iy);
  const v01 = lattice(candidate, ix, iy + 1);
  const v11 = lattice(candidate, ix + 1, iy + 1);
  const v0 = v00 + (v10 - v00) * sx;
  const v1 = v01 + (v11 - v01) * sx;
  return v0 + (v1 - v0) * sy;
}

function toMicro(value: number) {
  const scaled = value * SCALE;
  const rounded = scaled < 0 ? -Math.round(-scaled) : Math.round(scaled);
  return rounded === 0 ? 0 : rounded;
}

export function formatMicro(value: number) {
  const sign = value < 0 ? "-" : "";
  const abs = Math.abs(value);
  const whole = Math.floor(abs / SCALE);
  let frac = String(abs % SCALE).padStart(6, "0");
  if (frac === "000000") return `${sign}${whole}`;
  while (frac.endsWith("0")) frac = frac.slice(0, -1);
  return `${sign}${whole}.${frac}`;
}

function compareMicro(left: Micro, right: Micro) {
  return left.lon - right.lon || left.lat - right.lat;
}

function cyclicKey(open: Ring, start: number) {
  return Array.from({ length: open.length }, (_, offset) => open[(start + offset) % open.length]);
}

function seqLess(left: Ring, right: Ring) {
  for (let index = 0; index < left.length; index += 1) {
    const cmp = compareMicro(left[index], right[index]);
    if (cmp !== 0) return cmp < 0;
  }
  return false;
}

function rotateLexGrid(ring: number[][]) {
  let best = 0;
  let bestSeq = ring;
  for (let start = 1; start < ring.length; start += 1) {
    const seq = Array.from({ length: ring.length }, (_, offset) => ring[(start + offset) % ring.length]);
    let less = false;
    for (let index = 0; index < seq.length; index += 1) {
      const dx = seq[index][0] - bestSeq[index][0];
      const dy = seq[index][1] - bestSeq[index][1];
      if (dx !== 0 || dy !== 0) {
        less = dx < 0 || (dx === 0 && dy < 0);
        break;
      }
    }
    if (less) {
      best = start;
      bestSeq = seq;
    }
  }
  return best === 0 ? ring.slice() : Array.from({ length: ring.length }, (_, offset) => ring[(best + offset) % ring.length]);
}

function perpendicularDistanceSquared(point: number[], start: number[], end: number[]) {
  const dx = end[0] - start[0];
  const dy = end[1] - start[1];
  if (dx === 0 && dy === 0) {
    const ox = point[0] - start[0];
    const oy = point[1] - start[1];
    return ox * ox + oy * oy;
  }
  const t = Math.max(0, Math.min(1, ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / (dx * dx + dy * dy)));
  const px = point[0] - (start[0] + t * dx);
  const py = point[1] - (start[1] + t * dy);
  return px * px + py * py;
}

function simplifyGridRing(ring: number[][], threshold: number) {
  const open = ring[0][0] === ring[ring.length - 1][0] && ring[0][1] === ring[ring.length - 1][1] ? ring.slice(0, -1) : ring.slice();
  if (open.length < 3) return null;
  const rotated = rotateLexGrid(open).map((point, original) => ({ point, original }));
  while (rotated.length > 3) {
    let removeAt = 1;
    let best = Infinity;
    let bestOriginal = rotated[1].original;
    for (let index = 1; index < rotated.length - 1; index += 1) {
      const distance = perpendicularDistanceSquared(
        rotated[index].point,
        rotated[index - 1].point,
        rotated[index + 1].point,
      );
      if (distance < best || (distance === best && rotated[index].original < bestOriginal)) {
        best = distance;
        bestOriginal = rotated[index].original;
        removeAt = index;
      }
    }
    if (best >= threshold) break;
    rotated.splice(removeAt, 1);
  }
  if (rotated.length < 3) return null;
  return rotated.map((item) => item.point);
}

function gridArea(open: number[][]) {
  let area = 0;
  for (let index = 0; index < open.length; index += 1) {
    const a = open[index];
    const b = open[(index + 1) % open.length];
    area += a[0] * b[1] - b[0] * a[1];
  }
  return area / 2;
}

function gridToLonLat(point: number[]): [number, number] {
  return [LON_MIN + (point[0] / GRID_WIDTH) * LON_SPAN, LAT_MAX - (point[1] / GRID_HEIGHT) * LAT_SPAN];
}

function signedArea(ring: Ring) {
  let area = 0n;
  for (let index = 0; index < ring.length - 1; index += 1) {
    const a = ring[index];
    const b = ring[index + 1];
    area += BigInt(a.lon) * BigInt(b.lat) - BigInt(b.lon) * BigInt(a.lat);
  }
  return area;
}

function orient(a: Micro, b: Micro, c: Micro) {
  return (BigInt(b.lon) - BigInt(a.lon)) * (BigInt(c.lat) - BigInt(a.lat)) - (BigInt(b.lat) - BigInt(a.lat)) * (BigInt(c.lon) - BigInt(a.lon));
}

function onSegment(a: Micro, b: Micro, c: Micro) {
  return c.lon >= Math.min(a.lon, b.lon) && c.lon <= Math.max(a.lon, b.lon) && c.lat >= Math.min(a.lat, b.lat) && c.lat <= Math.max(a.lat, b.lat);
}

function segmentsIntersect(a: Micro, b: Micro, c: Micro, d: Micro) {
  if ((a.lon === c.lon && a.lat === c.lat) || (a.lon === d.lon && a.lat === d.lat) || (b.lon === c.lon && b.lat === c.lat) || (b.lon === d.lon && b.lat === d.lat)) {
    return false;
  }
  const o1 = Number(orient(a, b, c) === 0n ? 0n : orient(a, b, c) > 0n ? 1n : -1n);
  const o2 = Number(orient(a, b, d) === 0n ? 0n : orient(a, b, d) > 0n ? 1n : -1n);
  const o3 = Number(orient(c, d, a) === 0n ? 0n : orient(c, d, a) > 0n ? 1n : -1n);
  const o4 = Number(orient(c, d, b) === 0n ? 0n : orient(c, d, b) > 0n ? 1n : -1n);
  if (o1 !== o2 && o3 !== o4) return true;
  return (o1 === 0 && onSegment(a, b, c)) || (o2 === 0 && onSegment(a, b, d)) || (o3 === 0 && onSegment(c, d, a)) || (o4 === 0 && onSegment(c, d, b));
}

function crossesAntimeridian(a: Micro, b: Micro) {
  return Math.abs(a.lon - b.lon) > ANTIMERIDIAN;
}

function dedupClose(positions: Micro[]) {
  const unique: Micro[] = [];
  for (const position of positions) {
    const previous = unique[unique.length - 1];
    if (!previous || previous.lon !== position.lon || previous.lat !== position.lat) unique.push(position);
  }
  if (unique.length && (unique[0].lon !== unique[unique.length - 1].lon || unique[0].lat !== unique[unique.length - 1].lat)) {
    unique.push({ ...unique[0] });
  }
  return unique;
}

function canonicalRing(positions: Micro[], hole: boolean): Ring | null {
  const ring = dedupClose(positions);
  if (ring.length < 4) return null;
  const open = ring.slice(0, -1);
  if (open.length < 3) return null;
  for (let index = 0; index < ring.length - 1; index += 1) {
    if (crossesAntimeridian(ring[index], ring[index + 1])) return null;
  }
  const minLon = Math.min(...open.map((coord) => coord.lon));
  const maxLon = Math.max(...open.map((coord) => coord.lon));
  if (maxLon - minLon > ANTIMERIDIAN) return null;
  const n = open.length;
  for (let i = 0; i < n; i += 1) {
    const a = open[i];
    const b = open[(i + 1) % n];
    for (let j = i + 1; j < n; j += 1) {
      if (j === i || (j + 1) % n === i || (i + 1) % n === j) continue;
      if (segmentsIntersect(a, b, open[j], open[(j + 1) % n])) return null;
    }
  }
  const closed = [...open, { ...open[0] }];
  const area = signedArea(closed);
  if (area === 0n) return null;
  const clockwise = area < 0n;
  if (hole !== clockwise) open.reverse();
  let bestIndex = 0;
  let bestSeq = cyclicKey(open, 0);
  for (let index = 1; index < open.length; index += 1) {
    const seq = cyclicKey(open, index);
    if (seqLess(seq, bestSeq)) {
      bestSeq = seq;
      bestIndex = index;
    }
  }
  const rotated = cyclicKey(open, bestIndex);
  return [...rotated, { ...rotated[0] }];
}

function toMicros(open: number[][]) {
  return open.map((point) => {
    const [lon, lat] = gridToLonLat(point);
    return { lon: toMicro(lon), lat: toMicro(lat) };
  });
}

function serializeRing(ring: Ring) {
  return `[${ring.map((coord) => `[${formatMicro(coord.lon)},${formatMicro(coord.lat)}]`).join(",")}]`;
}

function serializePolygon(polygon: Polygon) {
  return `[${[polygon.exterior, ...polygon.holes].map(serializeRing).join(",")}]`;
}

function ringKey(ring: Ring) {
  return ring.map((coord) => `${coord.lon},${coord.lat}`).join(";");
}

function comparePolygons(left: Polygon, right: Polygon) {
  if (right.area !== left.area) return right.area > left.area ? 1 : -1;
  const leftKey = ringKey(left.exterior);
  const rightKey = ringKey(right.exterior);
  return leftKey < rightKey ? -1 : leftKey > rightKey ? 1 : 0;
}

function svgCoord(coord: Micro) {
  const lon = coord.lon / SCALE;
  const lat = coord.lat / SCALE;
  return `${formatMicro(toMicro(lon + 170))} ${formatMicro(toMicro(75 - lat))}`;
}

function svgPath(polygons: Polygon[]) {
  let d = "";
  for (const polygon of polygons) {
    for (const ring of [polygon.exterior, ...polygon.holes]) {
      d += `M${svgCoord(ring[0])}`;
      for (let index = 1; index < ring.length - 1; index += 1) d += `L${svgCoord(ring[index])}`;
      d += "Z";
    }
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 340 150"><path fill="#c9a96e" fill-rule="evenodd" d="${d}"/></svg>`;
}

function prepareRing(ring: number[][], threshold: number, hole: boolean) {
  const simplified = simplifyGridRing(ring, threshold);
  if (!simplified || Math.abs(gridArea(simplified)) < MIN_GRID_AREA) return null;
  return canonicalRing(toMicros(simplified), hole);
}

function generateOne(settings: NativeGeneratorSettings, index: number): NativeGeneratorCandidate {
  const seed = candidateSeed(settings.seed, index);
  let state = seed;
  const values = new Float64Array(VALUE_COUNT);
  const continents: { cx: number; cy: number; rx: number; ry: number }[] = [];
  const baseRx = BASE_RX[settings.continentCount - 1];
  for (let count = 0; count < settings.continentCount; count += 1) {
    let value: number;
    [value, state] = next(state);
    const cx = 0.1 + value * 0.8;
    [value, state] = next(state);
    const cy = 0.15 + value * 0.7;
    [value, state] = next(state);
    const rx = baseRx * (0.8 + value * 0.4);
    [value, state] = next(state);
    const aspect = 0.55 + value * 0.35;
    continents.push({ cx, cy, rx, ry: rx * aspect });
  }
  const islands: { cx: number; cy: number; rx: number; ry: number }[] = [];
  const islandCount = ISLAND_COUNT[settings.islandFrequency];
  for (let count = 0; count < islandCount; count += 1) {
    let value: number;
    [value, state] = next(state);
    const cx = 0.05 + value * 0.9;
    [value, state] = next(state);
    const cy = 0.1 + value * 0.8;
    [value, state] = next(state);
    const rx = 0.015 + value * 0.035;
    [value, state] = next(state);
    const aspect = 0.6 + value * 0.8;
    islands.push({ cx, cy, rx, ry: rx * aspect });
  }
  const amplitudes = NOISE_AMPLITUDE[settings.coastlineRoughness];
  const amplitudeSum = amplitudes.reduce((sum, value) => sum + value, 0);
  for (let row = 0; row < GRID_HEIGHT; row += 1) {
    for (let column = 0; column < GRID_WIDTH; column += 1) {
      const cellIndex = row * GRID_WIDTH + column;
      const x = (column + 0.5) / GRID_WIDTH;
      const y = (row + 0.5) / GRID_HEIGHT;
      let continentField = 0;
      for (const continent of continents) {
        continentField = Math.max(continentField, compactKernel(x - continent.cx, y - continent.cy, continent.rx, continent.ry));
      }
      let islandField = 0;
      for (const island of islands) {
        islandField = Math.max(islandField, compactKernel(x - island.cx, y - island.cy, island.rx, island.ry) * 0.55);
      }
      let noise = 0;
      for (let octave = 0; octave < amplitudes.length; octave += 1) {
        noise += valueNoise(seed, x, y, 2 ** octave) * amplitudes[octave];
      }
      noise = (noise / amplitudeSum) * 0.38;
      values[cellIndex] = Math.max(continentField, islandField) + noise + cellIndex * 2 ** -40;
    }
  }
  const sorted = Float64Array.from(values);
  sorted.sort();
  const rank = Math.floor((1 - settings.landPercent / 100) * VALUE_COUNT);
  const threshold =
    rank <= 0 ? sorted[0] : rank >= VALUE_COUNT ? sorted[VALUE_COUNT - 1] : (sorted[rank - 1] + sorted[rank]) / 2;
  const extracted = contours().size([GRID_WIDTH, GRID_HEIGHT]).smooth(true).thresholds([threshold])(values);
  const simplify = SIMPLIFY_THRESHOLD[settings.coastlineRoughness];
  const polygons: Polygon[] = [];
  for (const contour of extracted) {
    for (const member of contour.coordinates) {
      const exteriorOpen = simplifyGridRing(member[0], simplify);
      if (!exteriorOpen || Math.abs(gridArea(exteriorOpen)) < MIN_GRID_AREA) continue;
      const exterior = canonicalRing(toMicros(exteriorOpen), false);
      if (!exterior) continue;
      const holes: Ring[] = [];
      for (const hole of member.slice(1)) {
        const prepared = prepareRing(hole, simplify, true);
        if (prepared) holes.push(prepared);
      }
      holes.sort((left, right) => (ringKey(left) < ringKey(right) ? -1 : ringKey(left) > ringKey(right) ? 1 : 0));
      const signed = signedArea(exterior);
      const area = signed < 0n ? -signed : signed;
      polygons.push({ exterior, holes, area });
    }
  }
  polygons.sort(comparePolygons);
  const collection = `{"type":"FeatureCollection","features":[${polygons
    .map((polygon) => `{"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":${serializePolygon(polygon)}}}`)
    .join(",")}]}`;
  return { index, seed, collection, svg: svgPath(polygons) };
}

export function generateCandidates(settings: NativeGeneratorSettings): NativeGeneratorCandidate[] {
  const invalid = validateGeneratorSettings(settings);
  if (invalid) throw new Error(invalid);
  return Array.from({ length: CANDIDATE_COUNT }, (_, index) => generateOne(settings, index));
}
