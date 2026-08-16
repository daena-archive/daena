import assert from "node:assert/strict";
import { DEFAULT_GENERATOR_SETTINGS, generateCandidates } from "../src/lib/maps/native-vector/generator.ts";

function isoperimetric(ring) {
  let area = 0;
  let peri = 0;
  for (let index = 0; index < ring.length - 1; index += 1) {
    const [x1, y1] = ring[index];
    const [x2, y2] = ring[index + 1];
    area += x1 * y2 - x2 * y1;
    peri += Math.hypot(x2 - x1, y2 - y1);
  }
  area = Math.abs(area) / 2;
  if (peri === 0) return 1;
  return (4 * Math.PI * area) / (peri * peri);
}

function ringArea(ring) {
  let area = 0;
  for (let index = 0; index < ring.length - 1; index += 1) {
    area += ring[index][0] * ring[index + 1][1] - ring[index + 1][0] * ring[index][1];
  }
  return Math.abs(area) / 2;
}

const first = generateCandidates(DEFAULT_GENERATOR_SETTINGS);
const second = generateCandidates(DEFAULT_GENERATOR_SETTINGS);
assert.equal(first.length, 6);
assert.deepEqual(
  first.map((candidate) => ({ seed: candidate.seed, collection: candidate.collection, svg: candidate.svg })),
  second.map((candidate) => ({ seed: candidate.seed, collection: candidate.collection, svg: candidate.svg })),
);
for (const candidate of first) {
  assert.equal(candidate.collection.includes('"id":'), false);
  assert.match(candidate.collection, /"properties":\{\}/);
  assert.equal(
    candidate.svg.startsWith(
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 340 150"><path fill="#c9a96e" fill-rule="evenodd" d="',
    ),
    true,
  );
  assert.equal(candidate.svg.endsWith("</svg>"), true);
  assert.equal(
    candidate.svg.includes("http://") &&
      candidate.svg.replace('xmlns="http://www.w3.org/2000/svg"', "").includes("http://"),
    false,
  );
  const collection = JSON.parse(candidate.collection);
  assert.ok(collection.features.length > 0, "candidate must produce land");
  const polygonAreas = collection.features.map((feature) => {
    const [exterior, ...holes] = feature.geometry.coordinates;
    return ringArea(exterior) - holes.reduce((sum, hole) => sum + ringArea(hole), 0);
  });
  const generatedLandPercent = (polygonAreas.reduce((sum, area) => sum + area, 0) / (340 * 150)) * 100;
  assert.ok(Math.abs(generatedLandPercent - DEFAULT_GENERATOR_SETTINGS.landPercent) < 0.6);
  assert.ok(polygonAreas.filter((area) => area > 500).length >= DEFAULT_GENERATOR_SETTINGS.continentCount);
  assert.ok(
    polygonAreas.some((area) => area >= 4 && area < 250),
    "medium islands must produce detached islets",
  );
  const largest = collection.features.reduce(
    (best, feature) => {
      const ring = feature.geometry.coordinates[0];
      const score = Math.abs(
        ring.reduce((sum, point, index, all) => {
          if (index === all.length - 1) return sum;
          return sum + point[0] * all[index + 1][1] - all[index + 1][0] * point[1];
        }, 0),
      );
      return score > best.score ? { score, ring } : best;
    },
    { score: 0, ring: null },
  );
  const roundness = isoperimetric(largest.ring);
  assert.ok(roundness < 0.72, `largest landmass is too round (${roundness})`);
}

console.log("native vector v1 generator checks passed");
