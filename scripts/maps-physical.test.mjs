import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-physical-"));
const sourcePath = join(temp, "world.pworld");
const geojsonPath = join(temp, "derived.geojson");
const largeSourcePath = join(temp, "world-large.pworld");
const largeGeojsonPath = join(temp, "derived-large.geojson");

function run(args, timeout = 120_000) {
  const result = spawnSync("cargo", args, { cwd: root, encoding: "utf8", timeout });
  if (result.status !== 0) {
    throw new Error(`${args.join(" ")} failed:\n${result.stdout}\n${result.stderr}`);
  }
  return result.stdout.trim().split("\n").at(-1);
}

function cli(resolution, source, geojson, extra = []) {
  return JSON.parse(
    run([
      "run",
      "--quiet",
      ...extra,
      "--manifest-path",
      "crates/daena-physical/Cargo.toml",
      "--locked",
      "--offline",
      "--",
      "--resolution",
      String(resolution),
      "--preset",
      "earthlike",
      "--source",
      source,
      "--geojson",
      geojson,
    ]),
  );
}

// DAENAPW2 header: 252 fixed bytes plus a six-step schedule (96 bytes), then
// one i32 elevation per canonical cube cell (6 * n^2).
function sourceBytes(resolution) {
  return 252 + 6 * 16 + 4 * 6 * resolution ** 2;
}

function checkGeoJson(path) {
  const bytes = readFileSync(path);
  const geojson = JSON.parse(bytes);
  assert.equal(geojson.type, "FeatureCollection");
  assert.ok(geojson.features.length > 0);
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
  return bytes.length;
}

try {
  run(["test", "--manifest-path", "crates/daena-physical/Cargo.toml", "--locked", "--offline"]);

  const small = cli(64, sourcePath, geojsonPath);
  const source = readFileSync(sourcePath);
  const smallGeojsonBytes = checkGeoJson(geojsonPath);
  assert.equal(small.resolution, 64);
  assert.equal(source.subarray(0, 8).toString("latin1"), "DAENAPW2", "the CLI writes the v2 source");
  assert.equal(source.length, sourceBytes(64), "DAENAPW2 source size is computed from the canonical record");
  assert.equal(small.width, 1024, "the derived product view is the production 1024x512 view");
  assert.equal(small.height, 512);
  assert.equal(small.sourceBytes, source.length);
  assert.equal(small.geojsonBytes, smallGeojsonBytes);
  assert.equal(small.geojsonFeatures, JSON.parse(readFileSync(geojsonPath, "utf8")).features.length);
  assert.ok(small.landFraction > 0.05 && small.landFraction < 0.6, `unexpected land fraction ${small.landFraction}`);
  assert.equal(small.preset, "earthlike");

  const large = cli(512, largeSourcePath, largeGeojsonPath, ["--release"]);
  const largeSource = readFileSync(largeSourcePath);
  const largeGeojsonBytes = checkGeoJson(largeGeojsonPath);
  assert.equal(large.resolution, 512);
  assert.equal(large.width, 1024);
  assert.equal(large.height, 512);
  assert.equal(largeSource.length, sourceBytes(512));
  assert.equal(large.sourceBytes, largeSource.length);
  assert.equal(large.geojsonBytes, largeGeojsonBytes);
  assert.ok(large.geojsonFeatures > 0);
  assert.ok(large.generationMs < 15_000, `512 generation exceeded budget: ${large.generationMs}ms`);
  console.log(
    `physical engine v2 check passed on ${process.platform}/${process.arch}: n=64 source=${small.sourceBytes} bytes geojson=${small.geojsonBytes} bytes; n=512 source=${large.sourceBytes} bytes geojson=${large.geojsonBytes} bytes in ${large.generationMs.toFixed(1)}ms`,
  );
} finally {
  rmSync(temp, { recursive: true, force: true });
}
