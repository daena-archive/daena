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
      "--width",
      "64",
      "--height",
      "32",
      "--source",
      sourcePath,
      "--geojson",
      geojsonPath,
    ]),
  );
  const source = readFileSync(sourcePath);
  const geojsonBytes = readFileSync(geojsonPath);
  const geojson = JSON.parse(geojsonBytes);
  assert.equal(sha256(source), "sha256:6e9a13df19859f2f0d6978526abf60d20354c23e3ba6c5acd22360e510f429c2");
  assert.equal(sha256(geojsonBytes), "sha256:616e1becb901ab7ee14df22a3af7ec9691e0568abf0f66e4ab9df14ea2bebdce");
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
  assert.equal(maxSummary.width, 384);
  assert.equal(maxSummary.height, 192);
  assert.equal(maxSummary.sourceBytes, 575147);
  assert.equal(maxSummary.geojsonBytes, 15141142);
  assert.equal(maxSummary.geojsonFeatures, 52330);
  assert.ok(maxSummary.generationMs < 8000, `maximum generation exceeded budget: ${maxSummary.generationMs}ms`);
  assert.equal(sha256(maxSource), "sha256:60eef9eddaae3d6855dcd26ca3c57360d84b980ffdacbedf3e23afa246c4f36f");
  assert.equal(sha256(maxGeojson), "sha256:42bf33b4dad992cfe11e33f6f3331a54639b1f9eff5ae409db7467759c8e7e4c");
  console.log(
    `physical map v12 source check passed on ${process.platform}/${process.arch}: default=${source.length}/${geojsonBytes.length} bytes, maximum=${maxSource.length}/${maxGeojson.length} bytes in ${maxSummary.generationMs.toFixed(1)}ms`,
  );
} finally {
  rmSync(temp, { recursive: true, force: true });
}
