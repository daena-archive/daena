import { PHYSICAL_RASTER_OVERSAMPLE, physicalGridRowForRasterRow } from "../native-vector/coordinates.ts";

export type ClimateOverlayMode = "off" | "annual" | "nh-summer" | "nh-winter" | "freeze";

export type PhysicalRasterPaintOptions = {
  iceVisible?: boolean;
  lakesVisible?: boolean;
  climateOverlay?: ClimateOverlayMode;
  climateAnnualCentiC?: number[];
  climateNhSummerCentiC?: number[];
  climateNhWinterCentiC?: number[];
};

export type PhysicalRasterProducts = {
  width: number;
  height: number;
  seaLevelMm: number;
  waterLevelMm: number[];
  hillshadePpm: number[];
  bathymetryMm: number[];
  lakeCells?: boolean[];
  iceCells?: boolean[];
};

/** Isolated sinks smaller than this stay land so one-cell puddles do not speckle continents. */
export const MIN_VISIBLE_INLAND_WATER_CELLS = 8;

function lerp(first: number, second: number, t: number): number {
  return first + (second - first) * t;
}

function temperatureTint(centiC: number): [number, number, number] {
  const t = Math.max(0, Math.min(1, (centiC + 3_500) / 7_500));
  if (t < 0.5) {
    const u = t * 2;
    return [lerp(40, 220, u), lerp(80, 220, u), lerp(180, 160, u)];
  }
  const u = (t - 0.5) * 2;
  return [lerp(220, 196, u), lerp(220, 64, u), lerp(160, 48, u)];
}

function mixRgb(
  first: [number, number, number],
  second: [number, number, number],
  t: number,
): [number, number, number] {
  return [lerp(first[0], second[0], t), lerp(first[1], second[1], t), lerp(first[2], second[2], t)];
}

function landTint(heightMm: number): [number, number, number] {
  if (heightMm < 180_000) {
    const t = Math.max(0, heightMm) / 180_000;
    return [lerp(92, 138, t), lerp(132, 154, t), lerp(64, 82, t)];
  }
  if (heightMm < 800_000) {
    const t = (heightMm - 180_000) / 620_000;
    return [lerp(138, 176, t), lerp(154, 140, t), lerp(82, 78, t)];
  }
  if (heightMm < 2_200_000) {
    const t = (heightMm - 800_000) / 1_400_000;
    return [lerp(176, 132, t), lerp(140, 96, t), lerp(78, 58, t)];
  }
  const t = Math.min(1, (heightMm - 2_200_000) / 1_800_000);
  return [lerp(132, 232, t), lerp(96, 228, t), lerp(58, 224, t)];
}

function floodComponent(width: number, height: number, start: number, wet: boolean[], seen: Uint8Array): number[] {
  const cells: number[] = [];
  const stack = [start];
  seen[start] = 1;
  while (stack.length) {
    const index = stack.pop() as number;
    cells.push(index);
    const row = Math.floor(index / width);
    const column = index - row * width;
    const neighbors = [
      column === 0 ? index + width - 1 : index - 1,
      column === width - 1 ? index - width + 1 : index + 1,
      row > 0 ? index - width : -1,
      row < height - 1 ? index + width : -1,
    ];
    for (const next of neighbors) {
      if (next < 0 || seen[next] || !wet[next]) continue;
      seen[next] = 1;
      stack.push(next);
    }
  }
  return cells;
}

export function classifyPhysicalWater(
  width: number,
  height: number,
  bathymetryMm: number[],
  lakeCells?: boolean[],
): { ocean: boolean[]; inland: boolean[] } {
  const count = width * height;
  const ocean = Array.from({ length: count }, () => false);
  const inland = Array.from({ length: count }, () => false);
  if (count === 0) return { ocean, inland };

  const belowSea = Array.from({ length: count }, (_, index) => (bathymetryMm[index] ?? 0) > 0);
  const seen = new Uint8Array(count);
  let largest: number[] = [];
  for (let index = 0; index < count; index += 1) {
    if (!belowSea[index] || seen[index]) continue;
    const cells = floodComponent(width, height, index, belowSea, seen);
    if (cells.length > largest.length) largest = cells;
  }
  for (const cell of largest) ocean[cell] = true;

  const inlandWet = Array.from({ length: count }, (_, index) => {
    if (ocean[index]) return false;
    return Boolean(lakeCells?.[index]) || belowSea[index];
  });
  seen.fill(0);
  for (let index = 0; index < count; index += 1) {
    if (!inlandWet[index] || seen[index]) continue;
    const cells = floodComponent(width, height, index, inlandWet, seen);
    if (cells.length < MIN_VISIBLE_INLAND_WATER_CELLS) continue;
    for (const cell of cells) inland[cell] = true;
  }
  return { ocean, inland };
}

export function paintPhysicalSurface(
  products: PhysicalRasterProducts,
  options: PhysicalRasterPaintOptions = {},
): HTMLCanvasElement {
  const iceVisible = options.iceVisible ?? true;
  const lakesVisible = options.lakesVisible ?? true;
  const climateOverlay = options.climateOverlay ?? "off";
  const canvas = document.createElement("canvas");
  canvas.width = products.width;
  canvas.height = products.height * PHYSICAL_RASTER_OVERSAMPLE;
  const context = canvas.getContext("2d");
  if (!context) return canvas;
  const water = classifyPhysicalWater(products.width, products.height, products.bathymetryMm, products.lakeCells);
  const pixels = context.createImageData(canvas.width, canvas.height);
  for (let canvasRow = 0; canvasRow < canvas.height; canvasRow += 1) {
    const sourceRow = physicalGridRowForRasterRow(canvasRow, canvas.height, products.height);
    for (let column = 0; column < products.width; column += 1) {
      const index = sourceRow * products.width + column;
      const shade = (products.hillshadePpm[index] ?? 500_000) / 1_000_000;
      const light = 0.42 + 0.58 * shade;
      const offset = (canvasRow * products.width + column) * 4;
      let red: number;
      let green: number;
      let blue: number;
      if (water.ocean[index]) {
        const depth = Math.min(1, (products.bathymetryMm[index] ?? 0) / 4_000_000);
        red = Math.round((18 + (1 - depth) * 36) * (0.75 + 0.25 * shade));
        green = Math.round((52 + (1 - depth) * 70) * (0.75 + 0.25 * shade));
        blue = Math.round((96 + (1 - depth) * 62) * (0.8 + 0.2 * shade));
      } else if (iceVisible && products.iceCells?.[index]) {
        red = Math.round((228 + 18 * shade) * (0.88 + 0.12 * shade));
        green = Math.round((236 + 12 * shade) * (0.9 + 0.1 * shade));
        blue = Math.round((244 + 8 * shade) * (0.92 + 0.08 * shade));
      } else if (lakesVisible && water.inland[index]) {
        red = Math.round(42 * light);
        green = Math.round(132 * light);
        blue = Math.round(138 * light);
      } else {
        const heightMm = Math.max(0, (products.waterLevelMm[index] ?? products.seaLevelMm) - products.seaLevelMm);
        let [landRed, landGreen, landBlue] = landTint(heightMm);
        if (climateOverlay !== "off") {
          const annual = options.climateAnnualCentiC?.[index] ?? 0;
          const summer = options.climateNhSummerCentiC?.[index] ?? annual;
          const winter = options.climateNhWinterCentiC?.[index] ?? annual;
          if (climateOverlay === "freeze") {
            const cold = Math.min(summer, winter);
            const warm = Math.max(summer, winter);
            if (warm < 0) [landRed, landGreen, landBlue] = [210, 230, 245];
            else if (cold < 0)
              [landRed, landGreen, landBlue] = mixRgb([landRed, landGreen, landBlue], [170, 210, 230], 0.55);
          } else {
            const value = climateOverlay === "nh-summer" ? summer : climateOverlay === "nh-winter" ? winter : annual;
            [landRed, landGreen, landBlue] = mixRgb([landRed, landGreen, landBlue], temperatureTint(value), 0.72);
          }
        }
        red = Math.round(landRed * light);
        green = Math.round(landGreen * light);
        blue = Math.round(landBlue * light);
      }
      pixels.data[offset] = Math.min(255, red);
      pixels.data[offset + 1] = Math.min(255, green);
      pixels.data[offset + 2] = Math.min(255, blue);
      pixels.data[offset + 3] = 255;
    }
  }
  context.putImageData(pixels, 0, 0);
  return canvas;
}
