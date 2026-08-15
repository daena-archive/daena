import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-atlas-spike-"));
const sourcePath = join(temp, "world.pworld");
const previewPath = join(temp, "atlas-2048.png");
const exportPath = join(temp, "atlas-4096.png");
const maxPath = join(temp, "atlas-8192.png");
const manifest = "crates/daena-atlas/Cargo.toml";

function sha256(bytes) {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function run(args, extra = {}) {
  const result = spawnSync("cargo", args, {
    cwd: root,
    encoding: "utf8",
    timeout: extra.timeout ?? 180_000,
  });
  if (result.status !== 0) {
    throw new Error(`${args.join(" ")} failed:\n${result.stdout}\n${result.stderr}`);
  }
  return result.stdout.trim().split("\n").at(-1);
}

function render(width, height, output, extra = {}, release = true) {
  const args = [
    "run",
    "--quiet",
    "--manifest-path",
    manifest,
    "--locked",
    "--offline",
  ];
  if (release) args.push("--release");
  args.push(
    "--",
    "--width",
    String(width),
    "--height",
    String(height),
    "--source",
    sourcePath,
    "--output",
    output,
    ...(extra.args ?? []),
  );
  const timeBin = "/usr/bin/time";
  const useTime = existsSync(timeBin);
  const result = spawnSync(useTime ? timeBin : "cargo", useTime ? ["-l", "cargo", ...args] : args, {
    cwd: root,
    encoding: "utf8",
    timeout: release ? 300_000 : 180_000,
  });
  const line = result.stdout.trim().split("\n").at(-1);
  let summary;
  try {
    summary = JSON.parse(line);
  } catch (error) {
    throw new Error(`${args.join(" ")} failed:\n${result.stdout}\n${result.stderr}\n${error}`);
  }
  const png = readFileSync(output);
  assert.equal(summary.width, width);
  assert.equal(summary.height, height);
  assert.equal(summary.pngBytes, png.length);
  assert.equal(png.subarray(0, 8).toString("hex"), "89504e470d0a1a0a");
  assert.equal(Buffer.from(png).includes(Buffer.from("http://")), false);
  assert.equal(Buffer.from(png).includes(Buffer.from("https://")), false);
  const rssMatch = /(\d+)\s+maximum resident set size/.exec(result.stderr);
  return {
    summary,
    png,
    hash: sha256(png),
    peakResidentBytes: rssMatch ? Number(rssMatch[1]) : null,
  };
}

try {
  run(["test", "--manifest-path", manifest, "--locked", "--offline"], { timeout: 300_000 });
  run(
    ["test", "--manifest-path", "crates/daena-core/Cargo.toml", "--locked", "--offline", "maps::atlas"],
    { timeout: 180_000 },
  );
  run(
    ["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "--offline", "--lib", "atlas_jobs"],
    { timeout: 180_000 },
  );
  for (const name of ["daena-atlas-relief.v1.json", "daena-atlas-antique.v1.json"]) {
    const style = readFileSync(join(root, "docs/maps/atlas/styles", name), "utf8");
    assert.equal(style.toLowerCase().includes("http://"), false);
    assert.equal(style.toLowerCase().includes("https://"), false);
    assert.equal(style.toLowerCase().includes("javascript"), false);
    assert.equal(style.toLowerCase().includes("shader"), false);
    JSON.parse(style);
  }
  const licenses = readFileSync(join(root, "docs/maps/atlas/LICENSES.md"), "utf8");
  assert.match(licenses, /daena-atlas-bitmap-5x7/);
  assert.match(licenses, /No runtime URL/);
  const panel = readFileSync(join(root, "src/lib/maps/atlas/AtlasRenderPanel.svelte"), "utf8");
  assert.equal(panel.includes("maplibre"), false);
  assert.equal(panel.includes("getCanvas"), false);
  assert.match(panel, /convertFileSrc/);
  const editor = readFileSync(join(root, "src/lib/maps/native-vector/NativeVectorMapEditor.svelte"), "utf8");
  assert.match(editor, /atlasCapabilities/);
  assert.equal(/atlasSupported = descriptor\?\.provider/.test(editor), false);
  const physical = JSON.parse(
    run(
      [
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
      ],
      { timeout: 120_000 },
    ),
  );
  assert.equal(physical.width, 64);
  assert.equal(physical.height, 32);
  const source = readFileSync(sourcePath);
  assert.equal(sha256(source), "sha256:f520abeaf54426178f6c208879341991fe611cd676073d060a844a27a89d7a2e");

  const preview = render(2048, 1024, previewPath);
  const again = render(2048, 1024, join(temp, "atlas-2048-repeat.png"));
  assert.equal(preview.hash, again.hash);
  const mid = render(4096, 2048, exportPath);
  const max = render(8192, 4096, maxPath);
  assert.notEqual(preview.hash, mid.hash);
  assert.notEqual(mid.hash, max.hash);

  const past = render(256, 128, join(temp, "atlas-past.png"), { args: ["--offset-years", "-8000"] });
  const present = render(256, 128, join(temp, "atlas-present.png"), { args: ["--offset-years", "0"] });
  const future = render(256, 128, join(temp, "atlas-future.png"), { args: ["--offset-years", "8000"] });
  assert.equal(past.summary.offsetYears, -8000);
  assert.equal(present.summary.offsetYears, 0);
  assert.equal(future.summary.offsetYears, 8000);
  assert.notEqual(past.hash, present.hash);
  assert.notEqual(present.hash, future.hash);
  const antique = render(256, 128, join(temp, "atlas-antique.png"), {
    args: ["--style", "daena-atlas-antique"],
  });
  assert.equal(antique.summary.styleId, "daena-atlas-antique");
  assert.notEqual(present.hash, antique.hash);

  const report = {
    preview: { ...preview.summary, sha256: preview.hash, peakResidentBytes: preview.peakResidentBytes },
    export4k: { ...mid.summary, sha256: mid.hash, peakResidentBytes: mid.peakResidentBytes },
    export8k: { ...max.summary, sha256: max.hash, peakResidentBytes: max.peakResidentBytes },
    epochs: { past: past.hash, present: present.hash, future: future.hash, antique: antique.hash },
  };
  writeFileSync(join(temp, "atlas-budgets.json"), `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report));
} finally {
  rmSync(temp, { recursive: true, force: true });
}
