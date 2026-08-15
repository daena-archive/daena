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
  const format = extra.format ?? "png";
  if (format === "png") {
    assert.equal(summary.pngBytes, png.length);
    assert.equal(png.subarray(0, 8).toString("hex"), "89504e470d0a1a0a");
  } else {
    assert.equal(summary.artifactBytes, png.length);
    assert.equal(summary.format, format);
  }
  assert.equal(Buffer.from(png).includes(Buffer.from("http://")) && format === "png", false);
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
  for (const name of [
    "daena-atlas-relief.v1.json",
    "daena-atlas-antique.v1.json",
    "daena-atlas-political.v1.json",
  ]) {
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
  run(
    ["test", "--manifest-path", "crates/daena-core/Cargo.toml", "--locked", "--offline", "maps::calendar"],
    { timeout: 120_000 },
  );
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
  assert.equal(sha256(source), "sha256:6e9a13df19859f2f0d6978526abf60d20354c23e3ba6c5acd22360e510f429c2");

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

  const region = render(128, 64, join(temp, "atlas-region.png"), {
    args: ["--west", "0", "--south", "0", "--east", "90", "--north", "45"],
  });
  assert.equal(region.summary.projection, "equirectangular");
  assert.notEqual(region.hash, present.hash);

  const svg = render(128, 64, join(temp, "atlas.svg"), {
    args: ["--format", "svg"],
    format: "svg",
  });
  const svgText = readFileSync(join(temp, "atlas.svg"), "utf8");
  assert.match(svgText, /^<\?xml/);
  assert.match(svgText, /xmlns="http:\/\/www.w3.org\/2000\/svg"/);
  assert.equal(svgText.includes("<script"), false);
  assert.equal(svgText.includes("href=\"http://"), false);
  assert.equal(svg.summary.format, "svg");

  const pdf = render(128, 64, join(temp, "atlas.pdf"), {
    args: ["--format", "pdf", "--dpi", "72"],
    format: "pdf",
  });
  const pdfBytes = readFileSync(join(temp, "atlas.pdf"));
  assert.equal(pdfBytes.subarray(0, 8).toString(), "%PDF-1.4");
  assert.match(pdfBytes.toString("latin1"), /\/MediaBox \[0 0 128 64\]/);
  assert.equal(pdfBytes.includes(Buffer.from("/JS")), false);
  assert.equal(pdfBytes.includes(Buffer.from("/URI")), false);
  assert.equal(pdf.summary.format, "pdf");

  const cacheDir = join(temp, "atlas-cache");
  const cold = render(256, 128, join(temp, "atlas-cache-cold.png"), {
    args: ["--cache-dir", cacheDir],
  });
  const warm = render(256, 128, join(temp, "atlas-cache-warm.png"), {
    args: ["--cache-dir", cacheDir],
  });
  assert.equal(cold.hash, warm.hash);
  assert.equal(cold.summary.artifactCache, "miss");
  assert.equal(warm.summary.artifactCache, "hit");
  assert.equal(cold.summary.rendererVersion, 5);
  assert.ok(cold.summary.tributaryCount >= 0);
  const adr = readFileSync(join(root, "docs/adr/0036-atlas-rendering-iteration-4.md"), "utf8");
  assert.match(adr, /atlas-only/);
  assert.match(adr, /\.daena\/cache\/atlas/);

  const report = {
    preview: { ...preview.summary, sha256: preview.hash, peakResidentBytes: preview.peakResidentBytes },
    export4k: { ...mid.summary, sha256: mid.hash, peakResidentBytes: mid.peakResidentBytes },
    export8k: { ...max.summary, sha256: max.hash, peakResidentBytes: max.peakResidentBytes },
    epochs: { past: past.hash, present: present.hash, future: future.hash, antique: antique.hash },
    cache: {
      coldMs: cold.summary.renderMs,
      warmMs: warm.summary.renderMs,
      coldCache: cold.summary.artifactCache,
      warmCache: warm.summary.artifactCache,
      tributaryCount: cold.summary.tributaryCount,
    },
  };
  writeFileSync(join(temp, "atlas-budgets.json"), `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify(report));
} finally {
  rmSync(temp, { recursive: true, force: true });
}
