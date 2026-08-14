import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-physical-spike-"));
const sourcePath = join(temp, "world.pworld");
const geojsonPath = join(temp, "coastline.geojson");
const maxSourcePath = join(temp, "world-max.pworld");
const maxGeojsonPath = join(temp, "coastline-max.geojson");

function sha256(bytes) {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function run(args) {
  const result = spawnSync("cargo", args, { cwd: root, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`${args.join(" ")} failed:\n${result.stdout}\n${result.stderr}`);
  }
  return result.stdout.trim().split("\n").at(-1);
}

try {
  run(["test", "--manifest-path", "crates/daena-physical-spike/Cargo.toml", "--locked", "--offline"]);
  const summary = JSON.parse(
    run([
      "run",
      "--quiet",
      "--manifest-path",
      "crates/daena-physical-spike/Cargo.toml",
      "--locked",
      "--offline",
      "--",
      "--source",
      sourcePath,
      "--geojson",
      geojsonPath,
    ]),
  );
  const source = readFileSync(sourcePath);
  const geojsonBytes = readFileSync(geojsonPath);
  const geojson = JSON.parse(geojsonBytes);
  assert.equal(sha256(source), "sha256:6ecf77ded12723d9cec4343c416c90e73cee5328a3e8a5333c0726ed10d2b1a7");
  assert.equal(sha256(geojsonBytes), "sha256:caf92dcc92c07d0bdcec70865c0ea4da9b25d444189cb31985ceadef7246eb31");
  assert.equal(summary.width, 64);
  assert.equal(summary.height, 32);
  assert.equal(summary.sourceBytes, source.length);
  assert.equal(summary.geojsonBytes, geojsonBytes.length);
  assert.ok(Math.abs(summary.landFraction - 0.3) < 0.04);
  assert.equal(geojson.type, "FeatureCollection");
  assert.ok(geojson.features.length > 0);
  assert.ok(geojson.features.length <= 32768);
  assert.equal(JSON.stringify(geojson).includes("http"), false);
  for (const feature of geojson.features) {
    assert.equal(feature.geometry.type, "LineString");
    for (const [longitude, latitude] of feature.geometry.coordinates) {
      assert.ok(longitude >= -180 && longitude <= 180);
      assert.ok(latitude >= -90 && latitude <= 90);
    }
  }
  const maxSummary = JSON.parse(
    run([
      "run",
      "--release",
      "--quiet",
      "--manifest-path",
      "crates/daena-physical-spike/Cargo.toml",
      "--locked",
      "--offline",
      "--",
      "--max",
      "--source",
      maxSourcePath,
      "--geojson",
      maxGeojsonPath,
    ]),
  );
  const maxSource = readFileSync(maxSourcePath);
  const maxGeojson = readFileSync(maxGeojsonPath);
  assert.equal(maxSummary.width, 128);
  assert.equal(maxSummary.height, 64);
  assert.equal(maxSummary.sourceBytes, 32816);
  assert.equal(maxSummary.geojsonBytes, 1406527);
  assert.equal(maxSummary.geojsonFeatures, 6812);
  assert.ok(maxSummary.generationMs < 2000, `maximum generation exceeded budget: ${maxSummary.generationMs}ms`);
  assert.equal(sha256(maxSource), "sha256:d2002207d4785ebf9fd86b82aabf3073cd0f1e32919055c9b5293cb6b37cd1a0");
  assert.equal(sha256(maxGeojson), "sha256:f0e994b3d0dfab8c234ea94871dc233b238c5707fe2377447806bc3fc4cb898b");
  console.log(
    `physical map iteration-0 spike passed: default=${source.length}/${geojsonBytes.length} bytes, maximum=${maxSource.length}/${maxGeojson.length} bytes in ${maxSummary.generationMs.toFixed(1)}ms`,
  );
} finally {
  rmSync(temp, { recursive: true, force: true });
}
