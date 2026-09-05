import { PHYSICAL_RASTER_OVERSAMPLE, physicalGridRowForRasterRow } from "../native-vector/coordinates.ts";

export type ClimateOverlayMode =
  | "off"
  | "annual"
  | "nh-summer"
  | "nh-winter"
  | "freeze"
  | "wind"
  | "wind-nh-summer"
  | "wind-nh-winter"
  | "precipitation"
  | "precipitation-nh-summer"
  | "precipitation-nh-winter"
  | "humidity"
  | "aridity"
  | "biome"
  | "storm"
  | "storm-track";

export type PhysicalRasterPaintOptions = {
  iceVisible?: boolean;
  lakesVisible?: boolean;
  windsVisible?: boolean;
  currentsVisible?: boolean;
  climateOverlay?: ClimateOverlayMode;
  climateAnnualCentiC?: number[];
  climateNhSummerCentiC?: number[];
  climateNhWinterCentiC?: number[];
  climateWindEastMilli?: number[];
  climateWindNorthMilli?: number[];
  climateWindEastNhSummerMilli?: number[];
  climateWindNorthNhSummerMilli?: number[];
  climateWindEastNhWinterMilli?: number[];
  climateWindNorthNhWinterMilli?: number[];
  climateCurrentEastMilli?: number[];
  climateCurrentNorthMilli?: number[];
  climatePrecipitationMm?: number[];
  climatePrecipitationNhSummerMm?: number[];
  climatePrecipitationNhWinterMm?: number[];
  climateHumidityPpm?: number[];
  climateAridityPpm?: number[];
  climateBiomeClass?: number[];
  climateBiomeFill?: [number, number, number][];
  climateStormSuitabilityPpm?: number[];
  climateStormTrackPpm?: number[];
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
const CURRENT_ARROW_MIN_MILLI = 80;

function lerp(first: number, second: number, t: number): number {
  return first + (second - first) * t;
}

function isWindOverlay(mode: ClimateOverlayMode): boolean {
  return mode === "wind" || mode === "wind-nh-summer" || mode === "wind-nh-winter";
}

function windArrowMode(options: PhysicalRasterPaintOptions, overlay: ClimateOverlayMode): ClimateOverlayMode | null {
  if (isWindOverlay(overlay)) return overlay;
  if (options.windsVisible) return "wind";
  return null;
}

function windComponents(
  options: PhysicalRasterPaintOptions,
  overlay: ClimateOverlayMode,
  index: number,
): [number, number] {
  if (overlay === "wind-nh-summer") {
    return [options.climateWindEastNhSummerMilli?.[index] ?? 0, options.climateWindNorthNhSummerMilli?.[index] ?? 0];
  }
  if (overlay === "wind-nh-winter") {
    return [options.climateWindEastNhWinterMilli?.[index] ?? 0, options.climateWindNorthNhWinterMilli?.[index] ?? 0];
  }
  return [options.climateWindEastMilli?.[index] ?? 0, options.climateWindNorthMilli?.[index] ?? 0];
}

function currentTint(eastMilli: number, northMilli: number): [number, number, number] {
  const speed = Math.hypot(eastMilli, northMilli);
  if (speed < 1) return [48, 196, 196];
  const u = eastMilli / speed;
  const v = northMilli / speed;
  const westward: [number, number, number] = [72, 112, 220];
  const eastward: [number, number, number] = [48, 196, 196];
  const northward: [number, number, number] = [232, 148, 64];
  const southward: [number, number, number] = [64, 188, 168];
  const zonal = mixRgb(westward, eastward, (u + 1) / 2);
  const meridional = v >= 0 ? northward : southward;
  return mixRgb(zonal, meridional, Math.abs(v) * 0.7);
}

function windTint(eastMilli: number, northMilli: number): [number, number, number] {
  const speed = Math.hypot(eastMilli, northMilli);
  if (speed < 1) return [72, 168, 232];
  const u = eastMilli / speed;
  const v = northMilli / speed;
  const easterly: [number, number, number] = [72, 168, 232];
  const westerly: [number, number, number] = [232, 156, 48];
  const northward: [number, number, number] = [64, 196, 168];
  const southward: [number, number, number] = [196, 96, 168];
  const zonal = mixRgb(easterly, westerly, (u + 1) / 2);
  const meridional = v >= 0 ? northward : southward;
  return mixRgb(zonal, meridional, Math.abs(v) * 0.7);
}

function precipitationTint(mm: number): [number, number, number] {
  const t = Math.max(0, Math.min(1, mm / 2_800));
  return [lerp(196, 32, t), lerp(168, 96, t), lerp(112, 176, t)];
}

function humidityTint(ppm: number): [number, number, number] {
  const t = Math.max(0, Math.min(1, ppm / 1_000_000));
  return [lerp(196, 48, t), lerp(176, 164, t), lerp(148, 188, t)];
}

function aridityTint(ppm: number): [number, number, number] {
  const t = Math.max(0, Math.min(1, ppm / 1_000_000));
  return [lerp(96, 210, t), lerp(148, 164, t), lerp(92, 96, t)];
}

function biomeTint(biomeClass: number, fills: [number, number, number][] | undefined): [number, number, number] {
  const fill = fills?.[biomeClass];
  if (fill) return fill;
  return [120, 120, 124];
}

function stormTint(ppm: number): [number, number, number] {
  const t = Math.max(0, Math.min(1, ppm / 1_000_000));
  return [lerp(48, 212, t), lerp(72, 96, t), lerp(112, 64, t)];
}

function isStormOverlay(mode: ClimateOverlayMode): boolean {
  return mode === "storm" || mode === "storm-track";
}

function moistureValue(options: PhysicalRasterPaintOptions, overlay: ClimateOverlayMode, index: number): number {
  if (overlay === "precipitation-nh-summer") return options.climatePrecipitationNhSummerMm?.[index] ?? 0;
  if (overlay === "precipitation-nh-winter") return options.climatePrecipitationNhWinterMm?.[index] ?? 0;
  if (overlay === "humidity") return options.climateHumidityPpm?.[index] ?? 0;
  if (overlay === "aridity") return options.climateAridityPpm?.[index] ?? 0;
  return options.climatePrecipitationMm?.[index] ?? 0;
}

function isMoistureOverlay(mode: ClimateOverlayMode): boolean {
  return (
    mode === "precipitation" ||
    mode === "precipitation-nh-summer" ||
    mode === "precipitation-nh-winter" ||
    mode === "humidity" ||
    mode === "aridity"
  );
}

function isBiomeOverlay(mode: ClimateOverlayMode): boolean {
  return mode === "biome";
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
  const fillWind = isWindOverlay(climateOverlay) ? climateOverlay : null;
  const arrowWind = windArrowMode(options, climateOverlay);
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
        let oceanRed = (18 + (1 - depth) * 36) * (0.75 + 0.25 * shade);
        let oceanGreen = (52 + (1 - depth) * 70) * (0.75 + 0.25 * shade);
        let oceanBlue = (96 + (1 - depth) * 62) * (0.8 + 0.2 * shade);
        if (fillWind) {
          const [east, north] = windComponents(options, fillWind, index);
          [oceanRed, oceanGreen, oceanBlue] = mixRgb([oceanRed, oceanGreen, oceanBlue], windTint(east, north), 0.22);
        } else if (isStormOverlay(climateOverlay)) {
          const value =
            climateOverlay === "storm-track"
              ? (options.climateStormTrackPpm?.[index] ?? 0)
              : (options.climateStormSuitabilityPpm?.[index] ?? 0);
          [oceanRed, oceanGreen, oceanBlue] = mixRgb([oceanRed, oceanGreen, oceanBlue], stormTint(value), 0.72);
        }
        red = Math.round(oceanRed);
        green = Math.round(oceanGreen);
        blue = Math.round(oceanBlue);
      } else if (iceVisible && products.iceCells?.[index]) {
        let iceRed = (228 + 18 * shade) * (0.88 + 0.12 * shade);
        let iceGreen = (236 + 12 * shade) * (0.9 + 0.1 * shade);
        let iceBlue = (244 + 8 * shade) * (0.92 + 0.08 * shade);
        if (fillWind) {
          const [east, north] = windComponents(options, fillWind, index);
          [iceRed, iceGreen, iceBlue] = mixRgb([iceRed, iceGreen, iceBlue], windTint(east, north), 0.16);
        }
        red = Math.round(iceRed);
        green = Math.round(iceGreen);
        blue = Math.round(iceBlue);
      } else if (lakesVisible && water.inland[index]) {
        let lakeRed = 42 * light;
        let lakeGreen = 132 * light;
        let lakeBlue = 138 * light;
        if (fillWind) {
          const [east, north] = windComponents(options, fillWind, index);
          [lakeRed, lakeGreen, lakeBlue] = mixRgb([lakeRed, lakeGreen, lakeBlue], windTint(east, north), 0.18);
        }
        red = Math.round(lakeRed);
        green = Math.round(lakeGreen);
        blue = Math.round(lakeBlue);
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
          } else if (fillWind) {
            const [east, north] = windComponents(options, fillWind, index);
            [landRed, landGreen, landBlue] = mixRgb([landRed, landGreen, landBlue], windTint(east, north), 0.22);
          } else if (isStormOverlay(climateOverlay)) {
            const value =
              climateOverlay === "storm-track"
                ? (options.climateStormTrackPpm?.[index] ?? 0)
                : (options.climateStormSuitabilityPpm?.[index] ?? 0);
            [landRed, landGreen, landBlue] = mixRgb([landRed, landGreen, landBlue], stormTint(value), 0.72);
          } else if (isBiomeOverlay(climateOverlay)) {
            [landRed, landGreen, landBlue] = mixRgb(
              [landRed, landGreen, landBlue],
              biomeTint(options.climateBiomeClass?.[index] ?? 99, options.climateBiomeFill),
              0.78,
            );
          } else if (isMoistureOverlay(climateOverlay)) {
            const value = moistureValue(options, climateOverlay, index);
            const tint =
              climateOverlay === "humidity"
                ? humidityTint(value)
                : climateOverlay === "aridity"
                  ? aridityTint(value)
                  : precipitationTint(value);
            [landRed, landGreen, landBlue] = mixRgb([landRed, landGreen, landBlue], tint, 0.72);
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
  if (arrowWind) {
    paintWindArrows(context, products, options, arrowWind);
  }
  if (options.currentsVisible) {
    paintCurrentArrows(context, products, options);
  }
  return canvas;
}

function sampleWind(
  options: PhysicalRasterPaintOptions,
  overlay: ClimateOverlayMode,
  width: number,
  height: number,
  column: number,
  row: number,
): [number, number] {
  const x = Math.max(0, Math.min(width - 1, column));
  const y = Math.max(0, Math.min(height - 1, row));
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  const x1 = Math.min(width - 1, x0 + 1);
  const y1 = Math.min(height - 1, y0 + 1);
  const tx = x - x0;
  const ty = y - y0;
  const a = windComponents(options, overlay, y0 * width + x0);
  const b = windComponents(options, overlay, y0 * width + x1);
  const c = windComponents(options, overlay, y1 * width + x0);
  const d = windComponents(options, overlay, y1 * width + x1);
  return [lerp(lerp(a[0], b[0], tx), lerp(c[0], d[0], tx), ty), lerp(lerp(a[1], b[1], tx), lerp(c[1], d[1], tx), ty)];
}

function paintWindArrows(
  context: CanvasRenderingContext2D,
  products: PhysicalRasterProducts,
  options: PhysicalRasterPaintOptions,
  overlay: ClimateOverlayMode,
) {
  const stepX = Math.max(2, Math.floor(products.width / 24));
  const stepY = Math.max(2, Math.floor(products.height / 12));
  const oversample = PHYSICAL_RASTER_OVERSAMPLE;
  context.lineWidth = 1.15;
  context.lineCap = "round";
  for (let row = Math.floor(stepY / 2); row < products.height; row += stepY) {
    for (let column = Math.floor(stepX / 2); column < products.width; column += stepX) {
      const jitter = ((column * 1103515245 + row * 12345) >>> 0) % 7;
      const sampleCol = column + (jitter - 3) * 0.22;
      const sampleRow = row + (((jitter * 3) % 7) - 3) * 0.22;
      const [east, north] = sampleWind(options, overlay, products.width, products.height, sampleCol, sampleRow);
      const speed = Math.hypot(east, north);
      if (speed < 80) continue;
      const [red, green, blue] = windTint(east, north);
      const alpha = 0.45 + Math.min(0.45, speed / 2_400);
      context.strokeStyle = `rgba(${Math.round(red)},${Math.round(green)},${Math.round(blue)},${alpha})`;
      const length = 2.4 + Math.min(5.2, speed / 420);
      const angle = Math.atan2(-north, east);
      const x = sampleCol + 0.5;
      const y = (sampleRow + 0.5) * oversample;
      const dx = Math.cos(angle) * length;
      const dy = Math.sin(angle) * length;
      context.beginPath();
      context.moveTo(x - dx, y - dy);
      context.lineTo(x + dx, y + dy);
      context.moveTo(x + dx, y + dy);
      context.lineTo(x + dx - Math.cos(angle - 0.45) * 2.1, y + dy - Math.sin(angle - 0.45) * 2.1);
      context.moveTo(x + dx, y + dy);
      context.lineTo(x + dx - Math.cos(angle + 0.45) * 2.1, y + dy - Math.sin(angle + 0.45) * 2.1);
      context.stroke();
    }
  }
}

function sampleCurrent(
  options: PhysicalRasterPaintOptions,
  width: number,
  height: number,
  column: number,
  row: number,
): [number, number] {
  const x = Math.max(0, Math.min(width - 1, column));
  const y = Math.max(0, Math.min(height - 1, row));
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  const x1 = Math.min(width - 1, x0 + 1);
  const y1 = Math.min(height - 1, y0 + 1);
  const tx = x - x0;
  const ty = y - y0;
  const at = (index: number): [number, number] => [
    options.climateCurrentEastMilli?.[index] ?? 0,
    options.climateCurrentNorthMilli?.[index] ?? 0,
  ];
  const a = at(y0 * width + x0);
  const b = at(y0 * width + x1);
  const c = at(y1 * width + x0);
  const d = at(y1 * width + x1);
  return [lerp(lerp(a[0], b[0], tx), lerp(c[0], d[0], tx), ty), lerp(lerp(a[1], b[1], tx), lerp(c[1], d[1], tx), ty)];
}

function paintCurrentArrows(
  context: CanvasRenderingContext2D,
  products: PhysicalRasterProducts,
  options: PhysicalRasterPaintOptions,
) {
  const stepX = Math.max(2, Math.floor(products.width / 24));
  const stepY = Math.max(2, Math.floor(products.height / 12));
  const oversample = PHYSICAL_RASTER_OVERSAMPLE;
  context.lineWidth = 1.2;
  context.lineCap = "round";
  for (let row = Math.floor(stepY / 2); row < products.height; row += stepY) {
    for (let column = Math.floor(stepX / 2); column < products.width; column += stepX) {
      const jitter = ((column * 1103515245 + row * 12345) >>> 0) % 7;
      const sampleCol = column + (jitter - 3) * 0.22;
      const sampleRow = row + (((jitter * 3) % 7) - 3) * 0.22;
      const [east, north] = sampleCurrent(options, products.width, products.height, sampleCol, sampleRow);
      const speed = Math.hypot(east, north);
      if (speed < CURRENT_ARROW_MIN_MILLI) continue;
      const [red, green, blue] = currentTint(east, north);
      const alpha = 0.48 + Math.min(0.42, speed / 2_400);
      context.strokeStyle = `rgba(${Math.round(red)},${Math.round(green)},${Math.round(blue)},${alpha})`;
      const length = 2.6 + Math.min(5.4, speed / 380);
      const angle = Math.atan2(-north, east);
      const x = sampleCol + 0.5;
      const y = (sampleRow + 0.5) * oversample;
      const dx = Math.cos(angle) * length;
      const dy = Math.sin(angle) * length;
      context.beginPath();
      context.moveTo(x - dx, y - dy);
      context.lineTo(x + dx, y + dy);
      context.moveTo(x + dx, y + dy);
      context.lineTo(x + dx - Math.cos(angle - 0.45) * 2.1, y + dy - Math.sin(angle - 0.45) * 2.1);
      context.moveTo(x + dx, y + dy);
      context.lineTo(x + dx - Math.cos(angle + 0.45) * 2.1, y + dy - Math.sin(angle + 0.45) * 2.1);
      context.stroke();
    }
  }
}
