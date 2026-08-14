import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-physical-benchmark-"));
const timeBinary = "/usr/bin/time";
const useTime = existsSync(timeBinary);

function sampleProcessTreeResidentBytes(rootPid) {
  const result = spawnSync("ps", ["-axo", "pid=,ppid=,rss="], { encoding: "utf8" });
  if (result.status !== 0) return null;
  const processes = result.stdout
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => line.trim().split(/\s+/).map(Number))
    .filter(([pid, parentPid, rss]) => Number.isFinite(pid) && Number.isFinite(parentPid) && Number.isFinite(rss));
  const children = new Map();
  for (const [pid, parentPid, rss] of processes) {
    const siblings = children.get(parentPid) ?? [];
    siblings.push([pid, rss]);
    children.set(parentPid, siblings);
  }
  const pending = [rootPid];
  const visited = new Set();
  let residentKb = 0;
  while (pending.length > 0) {
    const pid = pending.pop();
    if (visited.has(pid)) continue;
    visited.add(pid);
    const process = processes.find(([processPid]) => processPid === pid);
    if (process) residentKb += process[2];
    for (const [childPid] of children.get(pid) ?? []) pending.push(childPid);
  }
  return residentKb > 0 ? residentKb * 1024 : null;
}

function runSampled(args) {
  return new Promise((resolve, reject) => {
    const startedAt = Date.now();
    const child = spawn("cargo", args, { cwd: root, encoding: "utf8" });
    let stdout = "";
    let stderr = "";
    let peakResidentBytes = 0;
    let timer;
    const sample = () => {
      const resident = sampleProcessTreeResidentBytes(child.pid);
      if (resident !== null) peakResidentBytes = Math.max(peakResidentBytes, resident);
      if (!child.killed) timer = setTimeout(sample, 10);
    };
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    sample();
    child.on("close", (status) => {
      clearTimeout(timer);
      const resident = sampleProcessTreeResidentBytes(child.pid);
      if (resident !== null) peakResidentBytes = Math.max(peakResidentBytes, resident);
      if (status !== 0) {
        reject(new Error(`${args.join(" ")} failed:\n${stdout}\n${stderr}`));
        return;
      }
      try {
        const summary = JSON.parse(stdout.trim().split("\n").at(-1));
        resolve({
          summary,
          wallTimeMs: Date.now() - startedAt,
          peakResidentBytes: peakResidentBytes || null,
        });
      } catch (error) {
        reject(new Error(`${args.join(" ")} produced invalid output: ${error}\n${stdout}\n${stderr}`));
      }
    });
  });
}

async function runTimed(args) {
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
  const measured = {
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
  return measured.peakResidentBytes === null ? runSampled(args) : measured;
}

async function main() {
  try {
    const defaultSource = join(temp, "default.pworld");
    const defaultGeojson = join(temp, "default.geojson");
    const maximumSource = join(temp, "maximum.pworld");
    const maximumGeojson = join(temp, "maximum.geojson");
    const defaults = await runTimed([
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
    const maximum = await runTimed([
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
        peakMemoryMeasurement: defaults.peakResidentBytes !== null || maximum.peakResidentBytes !== null,
      }),
    );
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

await main();
