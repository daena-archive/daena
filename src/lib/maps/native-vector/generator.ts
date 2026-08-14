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
const BASE_RX = [0.27, 0.235, 0.2, 0.185, 0.17, 0.16, 0.15, 0.14];
const NOISE_AMPLITUDE = {
  low: [1, 0.35, 0.12, 0.04, 0.01],
  medium: [1, 0.5, 0.25, 0.125, 0.0625],
  high: [1, 0.65, 0.42, 0.27, 0.18],
} as const;
const SIMPLIFY_THRESHOLD = { low: 0.35, medium: 0.06, high: 0.015 } as const;
const ARCHIPELAGO_COUNT = { none: 0, low: 2, medium: 4, high: 7 } as const;
const CONTINENT_LOBES = 8;
const BAY_COUNT = 4;
const PLACEMENT_ATTEMPTS = 48;
const CONTINENT_RADIUS_SCALE = 1.18;
const NOISE_STRENGTH = { low: 0.18, medium: 0.28, high: 0.38 } as const;
const SHORELINE_BAND = { low: 0.26, medium: 0.2, high: 0.14 } as const;
const WARP_X = 0x51ed;
const WARP_Y = 0xa31c;
const MASS_MASK = 0xc0a5;

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
  landPercent: 30,
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
  if (settings.generatorVersion !== GENERATOR_VERSION) return "vector.source.unsupported-version";
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
    version: GENERATOR_VERSION,
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

type Kernel = { cx: number; cy: number; rx: number; ry: number; ux: number; uy: number; shear: number };
type ContinentAnchor = Kernel;

function compactKernel(dx: number, dy: number, rx: number, ry: number, shear: number) {
  const wx = dx + shear * dy;
  const q = (wx / rx) * (wx / rx) + (dy / ry) * (dy / ry);
  return q < 1 ? (1 - q) * (1 - q) : 0;
}

function kernelCoordinates(kernel: Kernel, x: number, y: number): [number, number] {
  const dx = x - kernel.cx;
  const dy = y - kernel.cy;
  const along = dx * kernel.ux + dy * kernel.uy;
  const across = -dx * kernel.uy + dy * kernel.ux;
  return [along + kernel.shear * across, across];
}

function compactOrientedKernel(kernel: Kernel, x: number, y: number, radiusScale = 1) {
  const [along, across] = kernelCoordinates(kernel, x, y);
  return compactKernel(along, across, kernel.rx * radiusScale, kernel.ry * radiusScale, 0);
}

function signedOrientedKernel(kernel: Kernel, x: number, y: number) {
  const [along, across] = kernelCoordinates(kernel, x, y);
  const q = (along / kernel.rx) * (along / kernel.rx) + (across / kernel.ry) * (across / kernel.ry);
  return 1 - q;
}

function maxCompactKernel(kernels: readonly Kernel[], x: number, y: number, scale = 1, radiusScale = 1) {
  let field = 0;
  for (const kernel of kernels) field = Math.max(field, compactOrientedKernel(kernel, x, y, radiusScale) * scale);
  return field;
}

function maxSignedKernel(kernels: readonly Kernel[], x: number, y: number) {
  let field = -Infinity;
  for (const kernel of kernels) field = Math.max(field, signedOrientedKernel(kernel, x, y));
  return field;
}

function unitVector(x: number, y: number): [number, number] {
  const length = Math.hypot(x, y);
  return length < 1e-9 ? [1, 0] : [x / length, y / length];
}

function domainWarp(candidate: number, x: number, y: number): [number, number] {
  const warpX = valueNoise(candidate ^ WARP_X, x, y, 2) * 0.09 + valueNoise(candidate ^ WARP_X, x, y, 5) * 0.035;
  const warpY = valueNoise(candidate ^ WARP_Y, x, y, 2) * 0.07 + valueNoise(candidate ^ WARP_Y, x, y, 5) * 0.03;
  return [x + warpX, y + warpY];
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
  return best === 0
    ? ring.slice()
    : Array.from({ length: ring.length }, (_, offset) => ring[(best + offset) % ring.length]);
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
  const open =
    ring[0][0] === ring[ring.length - 1][0] && ring[0][1] === ring[ring.length - 1][1]
      ? ring.slice(0, -1)
      : ring.slice();
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
  return (
    (BigInt(b.lon) - BigInt(a.lon)) * (BigInt(c.lat) - BigInt(a.lat)) -
    (BigInt(b.lat) - BigInt(a.lat)) * (BigInt(c.lon) - BigInt(a.lon))
  );
}

function onSegment(a: Micro, b: Micro, c: Micro) {
  return (
    c.lon >= Math.min(a.lon, b.lon) &&
    c.lon <= Math.max(a.lon, b.lon) &&
    c.lat >= Math.min(a.lat, b.lat) &&
    c.lat <= Math.max(a.lat, b.lat)
  );
}

function segmentsIntersect(a: Micro, b: Micro, c: Micro, d: Micro) {
  if (
    (a.lon === c.lon && a.lat === c.lat) ||
    (a.lon === d.lon && a.lat === d.lat) ||
    (b.lon === c.lon && b.lat === c.lat) ||
    (b.lon === d.lon && b.lat === d.lat)
  ) {
    return false;
  }
  const o1 = Number(orient(a, b, c) === 0n ? 0n : orient(a, b, c) > 0n ? 1n : -1n);
  const o2 = Number(orient(a, b, d) === 0n ? 0n : orient(a, b, d) > 0n ? 1n : -1n);
  const o3 = Number(orient(c, d, a) === 0n ? 0n : orient(c, d, a) > 0n ? 1n : -1n);
  const o4 = Number(orient(c, d, b) === 0n ? 0n : orient(c, d, b) > 0n ? 1n : -1n);
  if (o1 !== o2 && o3 !== o4) return true;
  return (
    (o1 === 0 && onSegment(a, b, c)) ||
    (o2 === 0 && onSegment(a, b, d)) ||
    (o3 === 0 && onSegment(c, d, a)) ||
    (o4 === 0 && onSegment(c, d, b))
  );
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
  if (
    unique.length &&
    (unique[0].lon !== unique[unique.length - 1].lon || unique[0].lat !== unique[unique.length - 1].lat)
  ) {
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
  const take = () => {
    let value: number;
    [value, state] = next(state);
    return value;
  };
  const values = new Float64Array(VALUE_COUNT);
  const macroValues = new Float64Array(VALUE_COUNT);
  const continentGroups: Kernel[][] = [];
  const continentAnchors: ContinentAnchor[] = [];
  const bayGroups: Kernel[][] = [];
  const baseRx = BASE_RX[settings.continentCount - 1];
  for (let count = 0; count < settings.continentCount; count += 1) {
    let cx = 0.5;
    let cy = 0.5;
    let bestScore = -Infinity;
    // Best-candidate placement keeps the requested bodies distinct instead of
    // allowing several random cores to collapse into one central supercontinent.
    for (let attempt = 0; attempt < PLACEMENT_ATTEMPTS; attempt += 1) {
      const candidateX = 0.1 + take() * 0.8;
      const candidateY = 0.14 + take() * 0.72;
      const edgeScore = Math.min(candidateX - 0.06, 0.94 - candidateX, candidateY - 0.08, 0.92 - candidateY) * 3;
      let separation = Infinity;
      for (const anchor of continentAnchors) {
        separation = Math.min(
          separation,
          Math.min(
            1,
            Math.hypot(
              (candidateX - anchor.cx) / (baseRx + anchor.rx),
              (candidateY - anchor.cy) / (baseRx + anchor.ry),
            ) / 1.9,
          ),
        );
      }
      const score = Math.min(edgeScore, separation);
      if (score > bestScore) {
        bestScore = score;
        cx = candidateX;
        cy = candidateY;
      }
    }
    const rx = baseRx * (0.78 + take() * 0.34);
    const aspect = 0.7 + take() * 0.4;
    const ry = rx * aspect;
    // Longitude covers more than twice the preview distance of latitude, so
    // temper the normalized x component to avoid implausibly wide continents.
    const [ux, uy] = unitVector((take() * 2 - 1) * 0.42, take() * 2 - 1);
    const shear = (take() * 2 - 1) * 0.32;
    const anchor = { cx, cy, rx, ry, ux, uy, shear };
    continentAnchors.push(anchor);
    const continentGroup = [anchor];
    continentGroups.push(continentGroup);
    const bayGroup: Kernel[] = [];
    bayGroups.push(bayGroup);

    // A tapered, gently wandering spine gives each mass a geological axis.
    // Overlapping side lobes become capes and peninsulas rather than radial bumps.
    for (let lobe = 0; lobe < CONTINENT_LOBES; lobe += 1) {
      const along = ((lobe + 0.5) / CONTINENT_LOBES) * 2 - 1 + (take() * 2 - 1) * 0.16;
      const side = (take() * 2 - 1) * (0.32 + 0.18 * Math.abs(along));
      const taper = 1 - Math.abs(along) * 0.3;
      const lobeRx = rx * (0.28 + take() * 0.18) * taper;
      const lobeRy = ry * (0.38 + take() * 0.34) * taper;
      const [lobeUx, lobeUy] = unitVector(ux - uy * side * 0.35, uy + ux * side * 0.35);
      const component = {
        cx: cx + ux * along * rx * 0.95 - uy * side * ry,
        cy: cy + uy * along * rx * 0.95 + ux * side * ry,
        rx: lobeRx,
        ry: lobeRy,
        ux: lobeUx,
        uy: lobeUy,
        shear: (take() * 2 - 1) * 0.42,
      };
      continentGroup.push(component);
    }

    // Subtractive kernels bite into alternating sides of the continental shelf.
    // They create gulfs, straits, and inland seas without scattering ocean noise.
    for (let bay = 0; bay < BAY_COUNT; bay += 1) {
      const along = -0.7 + ((bay + take()) / BAY_COUNT) * 1.4;
      const side = (bay + count) % 2 === 0 ? 1 : -1;
      const bayRx = rx * (0.16 + take() * 0.12);
      const bayRy = ry * (0.34 + take() * 0.24);
      const bayKernel = {
        cx: cx + ux * along * rx - uy * side * ry * (0.82 + take() * 0.2),
        cy: cy + uy * along * rx + ux * side * ry * (0.82 + take() * 0.2),
        rx: bayRx,
        ry: bayRy,
        ux,
        uy,
        shear: (take() * 2 - 1) * 0.25,
      };
      bayGroup.push(bayKernel);
    }
  }

  const islands: Kernel[] = [];
  const archipelagoCount = ARCHIPELAGO_COUNT[settings.islandFrequency];
  for (let cluster = 0; cluster < archipelagoCount; cluster += 1) {
    const anchor =
      continentAnchors[Math.min(continentAnchors.length - 1, Math.floor(take() * continentAnchors.length))];
    const [outX, outY] = unitVector(take() * 2 - 1, take() * 2 - 1);
    const tangentX = -outY;
    const tangentY = outX;
    const localX = anchor.ux * outX * anchor.rx - anchor.uy * outY * anchor.ry;
    const localY = anchor.uy * outX * anchor.rx + anchor.ux * outY * anchor.ry;
    const centerX = Math.max(0.05, Math.min(0.95, anchor.cx + localX * (1.08 + take() * 0.24)));
    const centerY = Math.max(0.07, Math.min(0.93, anchor.cy + localY * (1.08 + take() * 0.24)));
    const memberCount = 4 + Math.floor(take() * 5);
    const spacing = 0.02 + take() * 0.012;
    for (let member = 0; member < memberCount; member += 1) {
      const offset = member - (memberCount - 1) / 2;
      const curve = offset * offset * 0.002;
      const centerBias = 1 - Math.min(1, Math.abs(offset) / Math.max(1, memberCount / 2));
      const islandScale = 0.72 + centerBias * 0.58;
      const islandRx = (0.0045 + take() * 0.0075) * islandScale;
      const aspect = 0.55 + take() * 0.9;
      const crossJitter = (take() * 2 - 1) * (0.25 + (Math.abs(offset) / Math.max(1, memberCount)) * 0.45) * spacing;
      const [islandUx, islandUy] = unitVector(
        tangentX + outX * (take() * 0.5 - 0.25),
        tangentY + outY * (take() * 0.5 - 0.25),
      );
      islands.push({
        cx: centerX + tangentX * offset * spacing + outX * (curve + crossJitter) + (take() * 2 - 1) * 0.006,
        cy: centerY + tangentY * offset * spacing + outY * (curve + crossJitter) + (take() * 2 - 1) * 0.006,
        rx: islandRx,
        ry: islandRx * aspect,
        ux: islandUx,
        uy: islandUy,
        shear: (take() * 2 - 1) * 0.28,
      });
    }
  }
  const amplitudes = NOISE_AMPLITUDE[settings.coastlineRoughness];
  const amplitudeSum = amplitudes.reduce((sum, value) => sum + value, 0);
  // Normalize each compact continental field without lifting the shared ocean
  // baseline, then apply the requested land budget once across the whole map.
  const targetPerContinent = Math.min(0.7, (settings.landPercent / 100 / continentGroups.length) * 1.08);
  const groupThresholds = continentGroups.map((group, groupIndex) => {
    const sampleWidth = 128;
    const sampleHeight = 64;
    const samples = new Float64Array(sampleWidth * sampleHeight);
    for (let row = 0; row < sampleHeight; row += 1) {
      for (let column = 0; column < sampleWidth; column += 1) {
        const x = (column + 0.5) / sampleWidth;
        const y = (row + 0.5) / sampleHeight;
        const [sx, sy] = domainWarp(seed, x, y);
        const compact = maxCompactKernel(group, sx, sy, 1, CONTINENT_RADIUS_SCALE);
        samples[row * sampleWidth + column] =
          compact +
          valueNoise(seed ^ MASS_MASK ^ Math.imul(groupIndex + 1, 0x9e37), sx, sy, 3) * 0.2 * compact -
          maxCompactKernel(bayGroups[groupIndex], sx, sy, 4);
      }
    }
    samples.sort();
    const rank = Math.max(0, Math.min(samples.length - 1, Math.floor((1 - targetPerContinent) * samples.length)));
    return continentGroups.length === 1 ? 0 : Math.max(0, samples[rank]);
  });
  for (let row = 0; row < GRID_HEIGHT; row += 1) {
    for (let column = 0; column < GRID_WIDTH; column += 1) {
      const cellIndex = row * GRID_WIDTH + column;
      const x = (column + 0.5) / GRID_WIDTH;
      const y = (row + 0.5) / GRID_HEIGHT;
      const [sx, sy] = domainWarp(seed, x, y);
      let continentField = -Infinity;
      for (let groupIndex = 0; groupIndex < continentGroups.length; groupIndex += 1) {
        const compact = maxCompactKernel(continentGroups[groupIndex], sx, sy, 1, CONTINENT_RADIUS_SCALE);
        const groupField =
          compact +
          valueNoise(seed ^ MASS_MASK ^ Math.imul(groupIndex + 1, 0x9e37), sx, sy, 3) * 0.2 * compact -
          maxCompactKernel(bayGroups[groupIndex], sx, sy, 4) -
          groupThresholds[groupIndex];
        continentField = Math.max(continentField, groupField);
      }
      const islandField = islands.length === 0 ? -Infinity : maxSignedKernel(islands, sx, sy) * 0.9 + 0.25;
      const edgeDistance = Math.min(x, 1 - x, y, 1 - y);
      const edgePenalty = edgeDistance < 0.04 ? ((0.04 - edgeDistance) / 0.04) * 24 : 0;
      macroValues[cellIndex] = Math.max(continentField, islandField) - edgePenalty + cellIndex * 2 ** -40;
    }
  }
  const macroSorted = Float64Array.from(macroValues);
  macroSorted.sort();
  const macroRank = Math.floor((1 - settings.landPercent / 100) * VALUE_COUNT);
  const macroThreshold =
    macroRank <= 0
      ? macroSorted[0]
      : macroRank >= VALUE_COUNT
        ? macroSorted[VALUE_COUNT - 1]
        : (macroSorted[macroRank - 1] + macroSorted[macroRank]) / 2;
  const shorelineBand = SHORELINE_BAND[settings.coastlineRoughness];
  for (let row = 0; row < GRID_HEIGHT; row += 1) {
    for (let column = 0; column < GRID_WIDTH; column += 1) {
      const cellIndex = row * GRID_WIDTH + column;
      const x = (column + 0.5) / GRID_WIDTH;
      const y = (row + 0.5) / GRID_HEIGHT;
      const [sx, sy] = domainWarp(seed, x, y);
      let noise = 0;
      for (let octave = 0; octave < amplitudes.length; octave += 1) {
        noise += valueNoise(seed, sx, sy, 2 ** octave) * amplitudes[octave];
      }
      noise = (noise / amplitudeSum) * NOISE_STRENGTH[settings.coastlineRoughness];
      const shorelineWeight = Math.exp(-Math.abs(macroValues[cellIndex] - macroThreshold) / shorelineBand);
      const localNoise = noise * (0.16 + shorelineWeight * 0.84);
      values[cellIndex] = macroValues[cellIndex] + localNoise + cellIndex * 2 ** -40;
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
    .map(
      (polygon) =>
        `{"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":${serializePolygon(polygon)}}}`,
    )
    .join(",")}]}`;
  return { index, seed, collection, svg: svgPath(polygons) };
}

export function generateCandidates(settings: NativeGeneratorSettings): NativeGeneratorCandidate[] {
  const invalid = validateGeneratorSettings(settings);
  if (invalid) throw new Error(invalid);
  return Array.from({ length: CANDIDATE_COUNT }, (_, index) => generateOne(settings, index));
}
