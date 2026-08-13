import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { DEFAULT_GENERATOR_SETTINGS, generateCandidates } from "../src/lib/maps/native-vector/generator.ts";

const fixtureUrl = new URL("../docs/maps/native-vector-fixtures/phase2-generator.json", import.meta.url);

function sha256(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
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
  assert.equal(candidate.svg.startsWith('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 340 150"><path fill="#c9a96e" fill-rule="evenodd" d="'), true);
  assert.equal(candidate.svg.endsWith("</svg>"), true);
  assert.equal(candidate.svg.includes("http://") && candidate.svg.replace('xmlns="http://www.w3.org/2000/svg"', "").includes("http://"), false);
}

const actual = {
  generatorVersion: 1,
  settings: DEFAULT_GENERATOR_SETTINGS,
  candidates: first.map((candidate) => ({
    index: candidate.index,
    seed: candidate.seed,
    geometryHash: sha256(candidate.collection),
    thumbnailHash: sha256(candidate.svg),
  })),
};

if (process.env.DAENA_WRITE_GENERATOR_GOLDEN === "1") {
  writeFileSync(fixtureUrl, `${JSON.stringify(actual, null, 2)}\n`);
}

const expected = JSON.parse(readFileSync(fixtureUrl, "utf8"));
assert.deepEqual(actual, expected, "native vector generator golden hashes drifted");
console.log("native vector Phase 2 generator golden hashes matched");
