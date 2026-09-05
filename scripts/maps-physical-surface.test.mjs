import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { compile } from "svelte/compiler";

import { classifyPhysicalWater, MIN_VISIBLE_INLAND_WATER_CELLS } from "../src/lib/maps/physical/raster.ts";

assert.equal(MIN_VISIBLE_INLAND_WATER_CELLS, 8);
assert.deepEqual(classifyPhysicalWater(0, 0, []), { ocean: [], inland: [] });

const width = 7;
const height = 4;
const count = width * height;
const bathymetry = Array.from({ length: count }, () => 0);
const lakeCells = Array.from({ length: count }, () => false);
for (let row = 0; row < height; row += 1) {
  bathymetry[row * width] = 1;
  bathymetry[row * width + 1] = 1;
  lakeCells[row * width + 3] = true;
  lakeCells[row * width + 4] = true;
}
lakeCells[6] = true;

const water = classifyPhysicalWater(width, height, bathymetry, lakeCells);
assert.equal(water.ocean.filter(Boolean).length, 8, "the largest connected bathymetry component is ocean");
assert.equal(water.inland.filter(Boolean).length, 8, "bounded lake components remain visible");
assert.equal(water.inland[6], false, "isolated one-cell sinks stay suppressed");
assert.equal(
  water.ocean.some((value, index) => value && water.inland[index]),
  false,
);

const root = resolve(import.meta.dirname, "..");
for (const path of [
  "src/lib/maps/physical/PhysicalMapEditor.svelte",
  "src/lib/maps/physical/PhysicalWorldView.svelte",
]) {
  const source = readFileSync(resolve(root, path), "utf8");
  compile(source, { filename: resolve(root, path), css: "injected" });
  assert.doesNotMatch(source, /https?:\/\//i, `${path} must not embed remote resources`);
}

const editor = readFileSync(resolve(root, "src/lib/maps/physical/PhysicalMapEditor.svelte"), "utf8");
assert.match(editor, /id: "winds"/);
assert.match(editor, /id: "currents"/);
assert.match(editor, /order: 14/);
assert.match(editor, /order: 15/);
assert.match(editor, /layers\.find\(\(layer\) => layer\.id === "winds"\)\?\.defaultVisible/);
assert.match(editor, /layers\.find\(\(layer\) => layer\.id === "currents"\)\?\.defaultVisible/);
const raster = readFileSync(resolve(root, "src/lib/maps/physical/raster.ts"), "utf8");
assert.match(raster, /const fillWind = isWindOverlay/);
assert.match(raster, /const arrowWind = windArrowMode/);
assert.match(raster, /function sampleWind/);
assert.match(raster, /function sampleCurrent/);
assert.match(raster, /currentsVisible/);
assert.match(raster, /function precipitationTint/);
assert.match(raster, /function humidityTint/);
assert.match(raster, /function aridityTint/);
assert.match(raster, /function biomeTint/);
assert.match(raster, /function stormTint/);
assert.match(raster, /climateStormSuitabilityPpm/);
assert.doesNotMatch(raster, /Math\.max\(options\.climateStormSuitabilityPpm/);
assert.match(editor, /value="precipitation"/);
assert.match(editor, /value="humidity"/);
assert.match(editor, /value="aridity"/);
assert.match(editor, /value="biome"/);
assert.match(editor, /value="storm"/);
assert.match(editor, /value="storm-track"/);
assert.match(editor, /inspectClimate/);
assert.match(editor, /meanHumidityPpm/);
assert.match(editor, /meanLandAridityPpm/);
assert.match(editor, /dominantLandBiome/);
assert.match(editor, /biomeClass/);
assert.match(editor, /biomeLegend/);
assert.match(editor, /physical-biome-legend/);

console.log("physical surface behavior and component checks passed");
