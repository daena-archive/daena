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
  run(
    ["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "--offline", "--lib", "atlas_studio"],
    { timeout: 180_000 },
  );
  for (const name of [
    "daena-atlas-relief.v1.json",
    "daena-atlas-antique.v1.json",
    "daena-atlas-political.v1.json",
    "daena-atlas-biome.v1.json",
    "daena-atlas-temperature.v1.json",
    "daena-atlas-precipitation.v1.json",
    "daena-atlas-bathymetry.v1.json",
    "daena-atlas-hydrology.v1.json",
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
  assert.match(panel, /seed\?:/);
  const editor = readFileSync(join(root, "src/lib/maps/native-vector/NativeVectorMapEditor.svelte"), "utf8");
  assert.match(editor, /atlasCapabilities/);
  assert.match(editor, /supportsStudio/);
  assert.match(editor, /AtlasStudioView/);
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
  assert.ok(source.length > 0);

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
  const biome = render(256, 128, join(temp, "atlas-biome.png"), {
    args: ["--style", "daena-atlas-biome"],
  });
  assert.equal(biome.summary.styleId, "daena-atlas-biome");
  assert.notEqual(present.hash, biome.hash);
  const temperature = render(256, 128, join(temp, "atlas-temperature.png"), {
    args: ["--style", "daena-atlas-temperature"],
  });
  assert.equal(temperature.summary.styleId, "daena-atlas-temperature");
  assert.notEqual(present.hash, temperature.hash);
  const rainfall = render(256, 128, join(temp, "atlas-precipitation.png"), {
    args: ["--style", "daena-atlas-precipitation"],
  });
  assert.equal(rainfall.summary.styleId, "daena-atlas-precipitation");
  assert.notEqual(biome.hash, rainfall.hash);
  const bathymetry = render(256, 128, join(temp, "atlas-bathymetry.png"), {
    args: ["--style", "daena-atlas-bathymetry"],
  });
  assert.equal(bathymetry.summary.styleId, "daena-atlas-bathymetry");
  assert.notEqual(present.hash, bathymetry.hash);
  const hydrology = render(256, 128, join(temp, "atlas-hydrology.png"), {
    args: ["--style", "daena-atlas-hydrology"],
  });
  assert.equal(hydrology.summary.styleId, "daena-atlas-hydrology");
  assert.notEqual(present.hash, hydrology.hash);

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
  assert.equal(cold.summary.rendererVersion, 11);
  assert.ok(cold.summary.tributaryCount >= 0);
  const adr = readFileSync(join(root, "docs/adr/0036-atlas-rendering-iteration-4.md"), "utf8");
  assert.match(adr, /atlas-only/);
  assert.match(adr, /\.daena\/cache\/atlas/);
  const studioAdr = readFileSync(join(root, "docs/adr/0037-atlas-studio-iteration-0.md"), "utf8");
  assert.match(studioAdr, /Web Mercator XYZ/);
  assert.match(studioAdr, /north-origin/);
  const studioHost = readFileSync(join(root, "docs/adr/0038-atlas-studio-iteration-1.md"), "utf8");
  assert.match(studioHost, /AtlasStudioSessionRequestV1/);
  assert.match(studioHost, /atlas-studio/);
  assert.match(studioHost, /Access-Control-Allow-Origin|CORS/);
  const studioComposition = readFileSync(join(root, "docs/adr/0039-atlas-studio-iteration-2.md"), "utf8");
  assert.match(studioComposition, /AtlasStudioSessionRequestV1/);
  assert.match(studioComposition, /calendar-year/);
  assert.match(studioComposition, /priority=prefetch/);
  assert.match(studioComposition, /current-view|regional Web Mercator/);
  const terrainSpike = readFileSync(join(root, "docs/adr/0040-atlas-studio-iteration-3.md"), "utf8");
  assert.match(terrainSpike, /experimental detail algorithm `2`/);
  assert.match(terrainSpike, /daena-atlas-detail-v2/);
  assert.match(terrainSpike, /hierarchical-relief/);
  assert.match(terrainSpike, /mountain-orometry/);
  assert.match(terrainSpike, /not listed in capabilities/);
  assert.match(terrainSpike, /mountain influence|mountain-influence/);
  const drainageSpike = readFileSync(join(root, "docs/adr/0041-atlas-studio-iteration-4.md"), "utf8");
  assert.match(drainageSpike, /experimental refined drainage/);
  assert.match(drainageSpike, /refined-drainage/);
  assert.match(drainageSpike, /multi-scale-erosion/);
  assert.match(drainageSpike, /atlas:tributary:v2/);
  assert.match(drainageSpike, /not listed in capabilities/);
  assert.match(drainageSpike, /Priority-Flood|priority-flood|Priority-Flood-style/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/amplify.rs"), "utf8"), /build_amplification_model/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/control.rs"), "utf8"), /sample_mountain_influence/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/refine.rs"), "utf8"), /build_refined_hydrology/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/refine.rs"), "utf8"), /atlas:valley:v\{ATLAS_DERIVED_DRAINAGE_VERSION\}/);
  assert.equal(readFileSync(join(root, "crates/daena-atlas/src/refine.rs"), "utf8").includes("AtlasDiskCache"), false);
  assert.match(drainageSpike, /atlas:valley:v2/);
  const atlasCargo = readFileSync(join(root, "crates/daena-atlas/Cargo.toml"), "utf8");
  assert.equal(atlasCargo.includes("noise-rs") || atlasCargo.includes("geo =") || atlasCargo.includes("image ="), false);
  const studioView = readFileSync(join(root, "src/lib/maps/atlas/AtlasStudioView.svelte"), "utf8");
  assert.match(studioView, /maplibre-gl/);
  assert.match(studioView, /atlasStudioOpen/);
  assert.match(studioView, /setMaxParallelImageRequests|maxParallelImageRequests/);
  assert.match(studioView, /isTransientTileError/);
  assert.match(studioView, /atlasStudioInspect/);
  assert.match(studioView, /priority=prefetch/);
  assert.match(studioView, /calendar-year/);
  assert.equal(studioView.includes("getCanvas"), false);
  assert.equal(studioView.includes("new Date"), false);
  assert.equal(studioView.includes("algorithmVersion: 6"), true);
  assert.equal(studioView.includes("algorithmVersion: 1"), false);
  assert.equal(studioView.includes("algorithmVersion: 2"), false);
  assert.equal(studioView.includes("algorithmVersion: 3"), false);
  assert.equal(studioView.includes("algorithmVersion: 4"), false);
  assert.equal(studioView.includes("algorithmVersion: 5"), false);
  assert.equal(studioView.includes("tributary:v2"), false);
  assert.equal(studioView.includes("valley:v2"), false);
  assert.match(studioView, /Skip to map/);
  assert.match(studioView, /aria-keyshortcuts/);
  assert.match(studioView, /atlas\.studio\.stale/);
  assert.match(studioView, /detail algorithm 6/);
  assert.match(studioView, /Regenerate disposable Atlas cache/);
  assert.match(studioView, /Atlas-only derived drainage/);
  assert.match(studioView, /currentViewExportHeight/);
  const studioRelease = readFileSync(join(root, "docs/adr/0042-atlas-studio-iteration-5.md"), "utf8");
  assert.match(studioRelease, /release hardening/);
  assert.match(studioRelease, /current_view_export_request/);
  assert.match(studioRelease, /Experimental paths are retained/);
  assert.match(studioRelease, /deferred/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/studio.rs"), "utf8"), /current_view_export_request/);
  const productionCutover = readFileSync(join(root, "docs/adr/0043-atlas-production-algorithm-2.md"), "utf8");
  assert.match(productionCutover, /Detail algorithm \| `2`/);
  assert.match(productionCutover, /Derived drainage \| `2`/);
  assert.match(productionCutover, /Renderer \| `7`/);
  assert.match(productionCutover, /build_detail_model/);
  const productionV4 = readFileSync(join(root, "docs/adr/0045-atlas-production-algorithm-4.md"), "utf8");
  assert.match(productionV4, /Detail algorithm \| `4`/);
  assert.match(productionV4, /Derived drainage \| `4`/);
  assert.match(productionV4, /Renderer \| `9`/);
  assert.match(productionV4, /daena-atlas-detail-v4/);
  const productionV5 = readFileSync(join(root, "docs/adr/0046-atlas-production-algorithm-5.md"), "utf8");
  assert.match(productionV5, /Detail algorithm \| `5`/);
  assert.match(productionV5, /Derived drainage \| `5`/);
  assert.match(productionV5, /Renderer \| `10`/);
  assert.match(productionV5, /daena-atlas-detail-v5/);
  assert.match(productionV5, /920/);
  const productionV6 = readFileSync(join(root, "docs/adr/0047-atlas-production-algorithm-6.md"), "utf8");
  assert.match(productionV6, /Detail algorithm \| `6`/);
  assert.match(productionV6, /Derived drainage \| `6`/);
  assert.match(productionV6, /Renderer \| `11`/);
  assert.match(productionV6, /daena-atlas-detail-v6/);
  assert.match(productionV6, /780/);
  const productionBiome = readFileSync(join(root, "docs/adr/0048-atlas-biome-style.md"), "utf8");
  assert.match(productionBiome, /daena-atlas-biome/);
  assert.match(productionBiome, /biomeForest/);
  const productionThematic = readFileSync(join(root, "docs/adr/0049-atlas-thematic-styles.md"), "utf8");
  assert.match(productionThematic, /daena-atlas-temperature/);
  assert.match(productionThematic, /daena-atlas-precipitation/);
  assert.match(productionThematic, /daena-atlas-bathymetry/);
  assert.match(productionThematic, /daena-atlas-hydrology/);
  const productionPhysical13 = readFileSync(join(root, "docs/adr/0050-physical-generator-13-orogeny-landmass-poles.md"), "utf8");
  assert.match(productionPhysical13, /Generator version is\n  `13`/);
  assert.match(productionPhysical13, /relative_speed \/ 600_000/);
  assert.match(readFileSync(join(root, "crates/daena-physical-spike/src/lib.rs"), "utf8"), /GENERATOR_VERSION: u32 = 13/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/lib.rs"), "utf8"), /ATLAS_DETAIL_ALGORITHM_VERSION: u32 = 6/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/lib.rs"), "utf8"), /ATLAS_DERIVED_DRAINAGE_VERSION: u32 = 6/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/lib.rs"), "utf8"), /ATLAS_RENDERER_VERSION: u32 = 11/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/amplify.rs"), "utf8"), /synthesize_coastline/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/amplify.rs"), "utf8"), /RIDGE_SYNTHESIS_MM: i32 = 780_000/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/amplify.rs"), "utf8"), /SecondaryRidge/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/amplify.rs"), "utf8"), /follow_ascent/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/erosion.rs"), "utf8"), /priority_fill_pits/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/refine.rs"), "utf8"), /DepositionKind/);
  assert.match(readFileSync(join(root, "crates/daena-atlas/src/erosion.rs"), "utf8"), /apply_scale_erosion/);

  const report = {
    preview: { ...preview.summary, sha256: preview.hash, peakResidentBytes: preview.peakResidentBytes },
    export4k: { ...mid.summary, sha256: mid.hash, peakResidentBytes: mid.peakResidentBytes },
    export8k: { ...max.summary, sha256: max.hash, peakResidentBytes: max.peakResidentBytes },
    epochs: {
      past: past.hash,
      present: present.hash,
      future: future.hash,
      antique: antique.hash,
      biome: biome.hash,
      temperature: temperature.hash,
      rainfall: rainfall.hash,
      bathymetry: bathymetry.hash,
      hydrology: hydrology.hash,
    },
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
