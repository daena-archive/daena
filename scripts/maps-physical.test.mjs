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
  assert.equal(sha256(source), "sha256:f8ca89a55844e63719dcbfda4f8c23bb6877fe54b94e579ae188da5f08b9a5a3");
  assert.equal(sha256(geojsonBytes), "sha256:595940c7a82721b960a4cfbb646d5d896a177297cdcbd8d607cc44c8806b82aa");
  assert.equal(summary.width, 64);
  assert.equal(summary.height, 32);
  assert.equal(summary.sourceBytes, source.length);
  assert.equal(summary.geojsonBytes, geojsonBytes.length);
  assert.ok(Math.abs(summary.landFraction - 0.3) < 0.04);
  assert.equal(geojson.type, "FeatureCollection");
  assert.ok(geojson.features.length > 0);
  assert.ok(geojson.features.length <= 32768);
  assert.equal(JSON.stringify(geojson).includes("http"), false);
  function positions(value) {
    if (!Array.isArray(value)) return [];
    if (typeof value[0] === "number" && typeof value[1] === "number") return [value];
    return value.flatMap((item) => positions(item));
  }
  for (const feature of geojson.features) {
    assert.ok(["Point", "LineString", "Polygon", "MultiPolygon"].includes(feature.geometry.type));
    for (const [longitude, latitude] of positions(feature.geometry.coordinates)) {
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
  assert.equal(maxSummary.sourceBytes, 79918);
  assert.equal(maxSummary.geojsonBytes, 8358988);
  assert.equal(maxSummary.geojsonFeatures, 29264);
  assert.ok(maxSummary.generationMs < 2000, `maximum generation exceeded budget: ${maxSummary.generationMs}ms`);
  assert.equal(sha256(maxSource), "sha256:4552626373917f8d07a1508d91f8ad4828946b79c91a53851575c20cb00bd7b2");
  assert.equal(sha256(maxGeojson), "sha256:ff3e3ea70315016f32d5fa4a73169ae1149e73f63a635547aa157a42a3fcf20b");
  console.log(
    `physical map v6 source check passed on ${process.platform}/${process.arch}: default=${source.length}/${geojsonBytes.length} bytes, maximum=${maxSource.length}/${maxGeojson.length} bytes in ${maxSummary.generationMs.toFixed(1)}ms`,
  );
} finally {
  rmSync(temp, { recursive: true, force: true });
}
