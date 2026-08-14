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
  assert.equal(sha256(source), "sha256:f97034adb095764c663ca6d35cb42d9e085ae4b79ae9578faa4e3731fbfe63ee");
  assert.equal(sha256(geojsonBytes), "sha256:57119b95e8abe53e95b37ca76b866ac91fcafd0d6848c2ba5b75eccdf6a86aca");
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
  assert.equal(maxSummary.sourceBytes, 79918);
  assert.equal(maxSummary.geojsonBytes, 786649);
  assert.equal(maxSummary.geojsonFeatures, 3822);
  assert.ok(maxSummary.generationMs < 2000, `maximum generation exceeded budget: ${maxSummary.generationMs}ms`);
  assert.equal(sha256(maxSource), "sha256:c6f95e15279ba5b65cace5c2f67ce08b4fc5fde203e4caa07e878be077629745");
  assert.equal(sha256(maxGeojson), "sha256:a7d0f4a53f9f2992885f2d1b81506f6ea94ae6d001b915d5af719207eb8211c4");
  console.log(
    `physical map v4 source check passed on ${process.platform}/${process.arch}: default=${source.length}/${geojsonBytes.length} bytes, maximum=${maxSource.length}/${maxGeojson.length} bytes in ${maxSummary.generationMs.toFixed(1)}ms`,
  );
} finally {
  rmSync(temp, { recursive: true, force: true });
}
