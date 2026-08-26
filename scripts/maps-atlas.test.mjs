import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-atlas-test-"));
const sourcePath = join(temp, "world.pworld");
const manifest = "crates/daena-atlas/Cargo.toml";

function runCargo(args, timeout = 180_000) {
  const result = spawnSync("cargo", args, { cwd: root, encoding: "utf8", timeout });
  if (result.status !== 0) throw new Error(`${args.join(" ")} failed:\n${result.stdout}\n${result.stderr}`);
  return result.stdout.trim().split("\n").at(-1);
}

function render(name, options = {}) {
  const output = join(temp, name);
  const width = options.width ?? 128;
  const height = options.height ?? 64;
  const format = options.format ?? "png";
  const stdout = runCargo(
    [
      "run",
      "--quiet",
      "--release",
      "--manifest-path",
      manifest,
      "--locked",
      "--offline",
      "--",
      "--source",
      sourcePath,
      "--output",
      output,
      "--width",
      String(width),
      "--height",
      String(height),
      ...(options.args ?? []),
    ],
    300_000,
  );
  const summary = JSON.parse(stdout);
  const bytes = readFileSync(output);
  assert.equal(summary.width, width);
  assert.equal(summary.height, height);
  assert.equal(summary.format, format);
  assert.equal(summary.artifactBytes ?? summary.pngBytes, bytes.length);
  assert.equal(bytes.includes(Buffer.from("https://")), false);
  return {
    bytes,
    hash: createHash("sha256").update(bytes).digest("hex"),
    summary,
  };
}

try {
  runCargo(["test", "--manifest-path", manifest, "--locked", "--offline"], 300_000);
  runCargo(["test", "--manifest-path", "crates/daena-core/Cargo.toml", "--locked", "--offline", "maps::atlas"]);
  runCargo(["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "--offline", "--lib", "atlas_"]);

  const physical = JSON.parse(
    runCargo([
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
    ]),
  );
  assert.equal(physical.width, 64);
  assert.equal(physical.height, 32);

  const present = render("present.png");
  const repeated = render("present-repeat.png");
  assert.equal(present.hash, repeated.hash, "the same captured request renders byte-identically");
  assert.ok(Number.isInteger(present.summary.rendererVersion));

  const past = render("past.png", { args: ["--offset-years", "-8000"] });
  const future = render("future.png", { args: ["--offset-years", "8000"] });
  assert.equal(past.summary.offsetYears, -8000);
  assert.equal(future.summary.offsetYears, 8000);
  assert.notEqual(past.hash, present.hash);
  assert.notEqual(future.hash, present.hash);

  for (const style of [
    "daena-atlas-antique",
    "daena-atlas-political",
    "daena-atlas-biome",
    "daena-atlas-temperature",
    "daena-atlas-precipitation",
    "daena-atlas-bathymetry",
    "daena-atlas-hydrology",
  ]) {
    const result = render(`${style}.png`, { args: ["--style", style] });
    assert.equal(result.summary.styleId, style);
    assert.notEqual(result.hash, present.hash, `${style} must produce a distinct composition`);
  }

  const region = render("region.png", {
    args: ["--west", "0", "--south", "0", "--east", "90", "--north", "45"],
  });
  assert.equal(region.summary.projection, "equirectangular");
  assert.notEqual(region.hash, present.hash);

  const svg = render("atlas.svg", { format: "svg", args: ["--format", "svg"] });
  const svgText = svg.bytes.toString("utf8");
  assert.match(svgText, /^<\?xml/);
  assert.match(svgText, /xmlns="http:\/\/www\.w3\.org\/2000\/svg"/);
  assert.doesNotMatch(svgText, /<script|href="https?:\/\//i);

  const pdf = render("atlas.pdf", { format: "pdf", args: ["--format", "pdf", "--dpi", "72"] });
  assert.equal(pdf.bytes.subarray(0, 8).toString(), "%PDF-1.4");
  assert.match(pdf.bytes.toString("latin1"), /\/MediaBox \[0 0 128 64\]/);
  assert.equal(pdf.bytes.includes(Buffer.from("/JS")), false);
  assert.equal(pdf.bytes.includes(Buffer.from("/URI")), false);

  const cacheDir = join(temp, "cache");
  const cold = render("cache-cold.png", { args: ["--cache-dir", cacheDir] });
  const warm = render("cache-warm.png", { args: ["--cache-dir", cacheDir] });
  assert.equal(cold.hash, warm.hash);
  assert.equal(cold.summary.artifactCache, "miss");
  assert.equal(warm.summary.artifactCache, "hit");

  console.log("Atlas deterministic render, style, format, epoch, region, and cache checks passed");
} finally {
  rmSync(temp, { recursive: true, force: true });
}
