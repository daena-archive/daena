import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-physical-benchmark-"));
const timeBinary = "/usr/bin/time";
const useTime = existsSync(timeBinary);

function runTimed(args) {
  const commandArgs = useTime ? [process.platform === "darwin" ? "-l" : "-v", "cargo", ...args] : ["cargo", ...args];
  const result = spawnSync(useTime ? timeBinary : "cargo", commandArgs, {
    cwd: root,
    encoding: "utf8",
  });
  let summary;
  try {
    summary = JSON.parse(result.stdout.trim().split("\n").at(-1));
  } catch {
    throw new Error(`${args.join(" ")} failed:\n${result.stdout}\n${result.stderr}`);
  }
  const resident =
    process.platform === "darwin"
      ? result.stderr.match(/(\d+)\s+maximum resident set size/i)?.[1]
      : result.stderr.match(/Maximum resident set size \(kbytes\):\s+(\d+)/i)?.[1];
  const elapsed =
    process.platform === "darwin"
      ? result.stderr.match(/([\d.]+)\s+real/i)?.[1]
      : result.stderr.match(/Elapsed \(wall clock\) time[^:]*:\s+([\d:.]+)/i)?.[1];
  return {
    summary,
    wallTimeMs: elapsed
      ? process.platform === "darwin"
        ? Number(elapsed) * 1_000
        : elapsed.includes(":")
          ? elapsed.split(":").reduce((total, value) => total * 60 + Number(value), 0) * 1_000
          : Number(elapsed) * 1_000
      : null,
    peakResidentBytes: resident ? Number(resident) * (process.platform === "darwin" ? 1 : 1024) : null,
  };
}

try {
  const defaultSource = join(temp, "default.pworld");
  const defaultGeojson = join(temp, "default.geojson");
  const maximumSource = join(temp, "maximum.pworld");
  const maximumGeojson = join(temp, "maximum.geojson");
  const defaults = runTimed([
    "run",
    "--release",
    "--quiet",
    "--manifest-path",
    "crates/daena-physical-spike/Cargo.toml",
    "--locked",
    "--offline",
    "--",
    "--source",
    defaultSource,
    "--geojson",
    defaultGeojson,
  ]);
  const maximum = runTimed([
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
    maximumSource,
    "--geojson",
    maximumGeojson,
  ]);
  for (const [label, run, sourcePath, geojsonPath] of [
    ["default", defaults, defaultSource, defaultGeojson],
    ["maximum", maximum, maximumSource, maximumGeojson],
  ]) {
    const sourceBytes = readFileSync(sourcePath).length;
    const geojsonBytes = readFileSync(geojsonPath).length;
    assert.equal(sourceBytes, run.summary.sourceBytes, `${label} source size mismatch`);
    assert.equal(geojsonBytes, run.summary.geojsonBytes, `${label} derived size mismatch`);
    assert.ok(run.summary.generationMs < 2_000, `${label} generation exceeded 2 seconds`);
  }
  console.log(
    JSON.stringify({
      platform: process.platform,
      arch: process.arch,
      default: {
        ...defaults.summary,
        wallTimeMs: defaults.wallTimeMs,
        peakResidentBytes: defaults.peakResidentBytes,
      },
      maximum: {
        ...maximum.summary,
        wallTimeMs: maximum.wallTimeMs,
        peakResidentBytes: maximum.peakResidentBytes,
      },
      peakMemoryMeasurement: useTime && (defaults.peakResidentBytes !== null || maximum.peakResidentBytes !== null),
    }),
  );
} finally {
  rmSync(temp, { recursive: true, force: true });
}
