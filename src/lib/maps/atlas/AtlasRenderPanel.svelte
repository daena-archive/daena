<script lang="ts">
import { onDestroy, onMount } from "svelte";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  project,
  ATLAS_PROGRESS_EVENT,
  type AtlasJobStatus,
  type AtlasRenderCapabilities,
  type AtlasRenderRequest,
} from "$lib/project/client";
import type { MapLayerDefinition } from "../native-vector/types";

let {
  mapId,
  epochOffsetYears = 0,
  seed = null,
  viewerLayers = [],
  onclose,
}: {
  mapId: string;
  epochOffsetYears?: number;
  seed?: AtlasRenderRequest | null;
  viewerLayers?: Pick<MapLayerDefinition, "id" | "name" | "defaultVisible">[];
  onclose?: () => void;
} = $props();

const PREVIEW_WIDTH = 1024;
const PREVIEW_HEIGHT = 512;
const PRESETS = [
  { label: "2K", width: 2048, height: 1024 },
  { label: "Print 4K", width: 4096, height: 2048 },
  { label: "Print 8K", width: 8192, height: 4096 },
] as const;
const EPOCH_MIN = -100_000;
const EPOCH_MAX = 100_000;
const EPOCH_STEP = 10;

const VIEWER_ROLE_ALIASES: Record<string, string[]> = {
  ocean: ["ocean"],
  ice: ["ice"],
  lakes: ["lakes"],
  rivers: ["rivers"],
  coastlines: ["islands", "coastlines"],
  contours: ["bathymetric-contours", "contours"],
  "tectonic-plates": ["tectonic-plates"],
  "tectonic-boundaries": ["tectonic-boundaries"],
  "volcanic-centers": ["volcanic-centers"],
  watersheds: ["watersheds"],
};

function viewerLayerName(atlasLayerId: string): string | null {
  const aliases = VIEWER_ROLE_ALIASES[atlasLayerId];
  if (!aliases || viewerLayers.length === 0) return null;
  const matched = viewerLayers.find((layer) => aliases.includes(layer.id));
  return matched?.name ?? null;
}

function viewerLayerEnabled(atlasLayerId: string): boolean | null {
  const aliases = VIEWER_ROLE_ALIASES[atlasLayerId];
  if (!aliases || viewerLayers.length === 0) return null;
  const matched = viewerLayers.filter((layer) => aliases.includes(layer.id));
  if (matched.length === 0) return null;
  return matched.some((layer) => layer.defaultVisible);
}

function styleLabel(id: string) {
  switch (id) {
    case "daena-atlas-relief":
      return "Elevation";
    case "daena-atlas-biome":
      return "Biomes";
    case "daena-atlas-temperature":
      return "Temperature";
    case "daena-atlas-precipitation":
      return "Rainfall";
    case "daena-atlas-bathymetry":
      return "Bathymetry";
    case "daena-atlas-hydrology":
      return "Hydrology";
    case "daena-atlas-antique":
      return "Antique";
    case "daena-atlas-political":
      return "Political";
    default:
      return id;
  }
}

let capabilities = $state<AtlasRenderCapabilities | null>(null);
let styleId = $state("daena-atlas-relief");
let widthPx = $state(2048);
let heightPx = $state(1024);
let dpi = $state(300);
let offsetYears = $state(0);
$effect(() => {
  offsetYears = epochOffsetYears;
});
let timeKind = $state<"physical-offset-year" | "calendar-year">("physical-offset-year");
let authoredYear = $state(1);
let presetName = $state("Atlas preset");
let layers = $state<Array<{ id: string; name: string; enabled: boolean }>>([]);
let previewJob = $state<AtlasJobStatus | null>(null);
let exportJob = $state<AtlasJobStatus | null>(null);
let previewUrl = $state("");
let notice = $state("");
let busyPreview = $state(false);
let unlisten: UnlistenFn | undefined;
let previewTimer: ReturnType<typeof setTimeout> | undefined;
let pollTimer: ReturnType<typeof setInterval> | undefined;
let previewRequestId = "";
let exportRequestId = crypto.randomUUID();
let seedProjection = $state<"equirectangular" | "web-mercator">("equirectangular");
let seedExtent = $state<AtlasRenderRequest["extent"] | null>(null);
let seedUnlock = $state(false);

function applySeed(next: AtlasRenderRequest | null) {
  if (!next) return;
  styleId = next.styleId;
  widthPx = next.widthPx;
  heightPx = next.heightPx;
  dpi = next.dpi;
  offsetYears = next.offsetYears;
  timeKind = next.timeKind;
  if (typeof next.authoredYear === "number") authoredYear = next.authoredYear;
  const ids = new Set(next.activeLayerIds);
  if (layers.length > 0) {
    layers = layers.map((layer) => ({ ...layer, enabled: ids.has(layer.id) }));
  }
  seedProjection = next.projection;
  seedExtent = next.extent;
  seedUnlock = next.unlockAspect;
}

function parseEpochYears(raw: string) {
  const digits = raw.replace(/[^\d]/g, "");
  const value = digits ? Number(digits) : 0;
  return Math.min(EPOCH_MAX, value);
}

function clampEpoch(offset: number, step = 1) {
  const snapped = step > 1 ? Math.round(offset / step) * step : Math.round(offset);
  return Math.min(EPOCH_MAX, Math.max(EPOCH_MIN, snapped));
}

function setOffsetYears(next: number) {
  offsetYears = clampEpoch(next);
  schedulePreview();
}

function setOffsetYearsAbs(raw: string) {
  const magnitude = parseEpochYears(raw);
  setOffsetYears(offsetYears < 0 ? -magnitude : magnitude);
}

function exportBusy() {
  return Boolean(
    exportJob &&
    exportJob.state !== "ready-to-save" &&
    exportJob.state !== "saved" &&
    exportJob.state !== "failed" &&
    exportJob.state !== "cancelled",
  );
}

function formatEpoch(offset: number) {
  if (offset === 0) return "at epoch";
  if (offset < 0) return "years before epoch";
  return "years after epoch";
}

function request(width: number, height: number): AtlasRenderRequest {
  return {
    schemaVersion: 1,
    offsetYears,
    algorithmVersion: 6,
    level: "detailed",
    variant: 0,
    styleId,
    widthPx: width,
    heightPx: height,
    dpi,
    format: "png",
    projection: seedProjection,
    extent: seedExtent ?? {
      westLonMicro: -180_000_000,
      southLatMicro: seedProjection === "web-mercator" ? -85_051_129 : -90_000_000,
      eastLonMicro: 180_000_000,
      northLatMicro: seedProjection === "web-mercator" ? 85_051_129 : 90_000_000,
    },
    unlockAspect: seedUnlock,
    activeLayerIds: layers.filter((layer) => layer.enabled).map((layer) => layer.id),
    timeKind,
    authoredYear: timeKind === "calendar-year" ? authoredYear : null,
    bindingRevision: null,
  };
}

function applyStatus(status: AtlasJobStatus) {
  if (status.mapEntityId !== mapId) return;
  if (status.kind === "preview") {
    if (status.requestId !== previewRequestId && status.state !== "cancelled") return;
    previewJob = status;
    busyPreview = status.state !== "ready-to-save" && status.state !== "failed" && status.state !== "cancelled";
    if (status.previewToken && status.state === "ready-to-save") {
      previewUrl = convertFileSrc(status.previewToken);
    }
    if (status.error && status.state === "failed") notice = status.error;
  } else {
    if (status.requestId !== exportRequestId && status.state !== "cancelled") return;
    exportJob = status;
    if (status.error && status.state === "failed") notice = status.error;
  }
  if (
    status.capturedContentGeneration != null &&
    status.currentContentGeneration != null &&
    status.currentContentGeneration > status.capturedContentGeneration
  ) {
    notice =
      "The project changed after this render. The image is from the captured generation. Render again to update.";
  }
}

async function loadCapabilities() {
  capabilities = await project.atlasCapabilities(mapId);
  if (!capabilities.supported) {
    notice = "This map provider does not support atlas rendering.";
    return;
  }
  styleId = capabilities.styles[0] ?? "daena-atlas-relief";
  if (capabilities.calendarBinding) {
    authoredYear = capabilities.calendarBinding.calendarReferenceYear;
  }
  layers = capabilities.layers.map((layer) => {
    const fromViewer = viewerLayerEnabled(layer.id);
    return {
      id: layer.id,
      name: viewerLayerName(layer.id) ?? layer.name,
      enabled: fromViewer ?? layer.defaultVisible,
    };
  });
  applySeed(seed);
  if (viewerLayers.length > 0) {
    layers = layers.map((layer) => ({
      ...layer,
      name: viewerLayerName(layer.id) ?? layer.name,
      enabled: viewerLayerEnabled(layer.id) ?? layer.enabled,
    }));
  }
  schedulePreview();
}

function schedulePreview() {
  if (!capabilities?.supported) return;
  if (previewTimer) clearTimeout(previewTimer);
  previewTimer = setTimeout(() => {
    previewTimer = undefined;
    void runPreview();
  }, 280);
}

async function runPreview() {
  if (!capabilities?.supported) return;
  previewRequestId = crypto.randomUUID();
  busyPreview = true;
  notice = "";
  try {
    const status = await project.atlasPreviewBegin(
      mapId,
      request(PREVIEW_WIDTH, PREVIEW_HEIGHT),
      previewRequestId as `${string}-${string}-${string}-${string}-${string}`,
    );
    applyStatus(status);
  } catch (error) {
    busyPreview = false;
    notice = error instanceof Error ? error.message : String(error);
  }
}

async function runExport() {
  if (exportBusy()) return;
  notice = "";
  exportRequestId = crypto.randomUUID();
  try {
    const status = await project.atlasRenderBegin(mapId, request(widthPx, heightPx), exportRequestId);
    applyStatus(status);
    if (status.requestId !== exportRequestId) exportJob = null;
  } catch (error) {
    notice = error instanceof Error ? error.message : String(error);
  }
}

async function saveExport() {
  if (!exportJob) return;
  try {
    const status = await project.atlasArtifactSave(exportJob.jobId);
    applyStatus(status);
    if (status.state === "saved") notice = "Saved atlas PNG.";
  } catch (error) {
    notice = error instanceof Error ? error.message : String(error);
  }
}

async function savePreset() {
  notice = "";
  try {
    const fields = await project.listFields(mapId);
    const current = fields.find((field) => field.namespace === "maps" && field.key === "atlasPresets");
    const presets = Array.isArray((current?.value as { presets?: unknown[] } | undefined)?.presets)
      ? ([...(current?.value as { presets: unknown[] }).presets] as Record<string, unknown>[])
      : [];
    presets.push({
      id: crypto.randomUUID(),
      name: presetName.trim() || "Atlas preset",
      time:
        timeKind === "calendar-year"
          ? { kind: "calendar-year", authoredYear }
          : { kind: "physical-offset-year", offsetYears },
      detail: { algorithmVersion: 6, level: "detailed", variant: 0 },
      style: { id: styleId, version: 1 },
      activeLayerIds: layers.filter((layer) => layer.enabled).map((layer) => layer.id),
      viewport: { kind: "world", projection: "equirectangular" },
      output: { widthPx, heightPx, dpi, format: "png" },
    });
    await project.setField({
      entity_id: mapId,
      namespace: "maps",
      key: "atlasPresets",
      value: { schemaVersion: 1, presets },
      revision: current?.revision ?? "",
    });
    capabilities = await project.atlasCapabilities(mapId);
    notice = "Saved atlas preset.";
  } catch (error) {
    notice = error instanceof Error ? error.message : String(error);
  }
}

async function applyPreset(id: string) {
  const fields = await project.listFields(mapId);
  const current = fields.find((field) => field.namespace === "maps" && field.key === "atlasPresets");
  const presets = (current?.value as { presets?: Array<Record<string, unknown>> } | undefined)?.presets ?? [];
  const preset = presets.find((item) => item.id === id);
  if (!preset) return;
  const time = preset.time as { kind?: string; offsetYears?: number; authoredYear?: number } | undefined;
  if (time?.kind === "calendar-year" && typeof time.authoredYear === "number") {
    timeKind = "calendar-year";
    authoredYear = time.authoredYear;
  } else if (typeof time?.offsetYears === "number") {
    timeKind = "physical-offset-year";
    offsetYears = time.offsetYears;
  }
  const style = preset.style as { id?: string } | undefined;
  if (style?.id) styleId = style.id;
  const output = preset.output as { widthPx?: number; heightPx?: number; dpi?: number } | undefined;
  if (typeof output?.widthPx === "number") widthPx = output.widthPx;
  if (typeof output?.heightPx === "number") heightPx = output.heightPx;
  if (typeof output?.dpi === "number") dpi = output.dpi;
  const ids = new Set((preset.activeLayerIds as string[] | undefined) ?? []);
  layers = layers.map((layer) => ({ ...layer, enabled: ids.has(layer.id) }));
  schedulePreview();
}

async function cancel(job: AtlasJobStatus | null) {
  if (!job) return;
  applyStatus(await project.atlasJobCancel(job.jobId));
}

onMount(() => {
  applySeed(seed);
  void loadCapabilities();
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") onclose?.();
  };
  window.addEventListener("keydown", onKey);
  void listen<AtlasJobStatus>(ATLAS_PROGRESS_EVENT, (event) => applyStatus(event.payload)).then((fn) => {
    unlisten = fn;
  });
  pollTimer = setInterval(() => {
    const jobs = [previewJob, exportJob].filter((job): job is AtlasJobStatus => Boolean(job));
    for (const job of jobs) {
      void project
        .atlasJobStatus(job.jobId)
        .then(applyStatus)
        .catch(() => undefined);
    }
  }, 2000);
  return () => window.removeEventListener("keydown", onKey);
});

onDestroy(() => {
  unlisten?.();
  if (previewTimer) clearTimeout(previewTimer);
  if (pollTimer) clearInterval(pollTimer);
  if (previewJob) void project.atlasArtifactDiscard(previewJob.jobId).catch(() => undefined);
  if (exportJob) {
    void project.atlasArtifactDiscard(exportJob.jobId).catch(() => undefined);
    void project.atlasJobCancel(exportJob.jobId).catch(() => undefined);
  }
});
</script>

<div class="atlas-modal" role="dialog" aria-modal="true" aria-label="Export atlas">
  <button type="button" class="atlas-backdrop" aria-label="Close export" onclick={() => onclose?.()}></button>
  <section class="atlas-panel">
    <header>
      <div>
        <span>ATLAS EXPORT</span>
        <strong>Render atlas map</strong>
      </div>
      <button type="button" onclick={() => onclose?.()}>Close</button>
    </header>
    <div class="preview-frame">
      {#if previewUrl}
        <img src={previewUrl} alt="Atlas preview" />
      {:else}
        <div class="preview-empty">Preview will appear here</div>
      {/if}
      {#if busyPreview || (previewJob && previewJob.state !== "ready-to-save" && previewJob.state !== "failed" && previewJob.state !== "cancelled")}
        <div class="map-busy" role="status">
          <strong>{previewJob?.stage ?? "Rendering preview…"}</strong>
          {#if previewJob}<span>{previewJob.completed} / {previewJob.total}</span>{/if}
        </div>
      {/if}
    </div>
    <p class="preview-note">
      Rivers include atlas-only minor tributaries. They are not canonical geography and are never promoted
      automatically.
    </p>
    <div class="epoch-control" aria-label="World epoch">
      <input
        type="range"
        min={EPOCH_MIN}
        max={EPOCH_MAX}
        step={EPOCH_STEP}
        value={offsetYears}
        aria-label="Epoch offset"
        oninput={(event) => setOffsetYears(clampEpoch(Number(event.currentTarget.value), EPOCH_STEP))} />
      <input
        class="epoch-year"
        type="text"
        inputmode="numeric"
        autocomplete="off"
        spellcheck="false"
        value={Math.abs(offsetYears).toLocaleString("en-US")}
        aria-label="Years from epoch"
        onchange={(event) => setOffsetYearsAbs(event.currentTarget.value)} />
      <span>{formatEpoch(offsetYears)}</span>
    </div>
    {#if capabilities?.timeModes.includes("calendar-year")}
      <label>
        Time mode
        <select bind:value={timeKind} onchange={() => schedulePreview()}>
          <option value="physical-offset-year">Physical offset</option>
          <option value="calendar-year">Authored year</option>
        </select>
      </label>
      {#if timeKind === "calendar-year"}
        <label>
          Authored year
          <input type="number" bind:value={authoredYear} oninput={() => schedulePreview()} />
        </label>
      {/if}
    {/if}
    {#if (capabilities?.presets.length ?? 0) > 0}
      <label>
        Preset
        <select onchange={(event) => void applyPreset(event.currentTarget.value)}>
          <option value="">Apply a saved preset</option>
          {#each capabilities?.presets ?? [] as preset}
            <option value={preset.id}>{preset.name}</option>
          {/each}
        </select>
      </label>
    {/if}
    <div class="style-row">
      <label>
        Style
        <select bind:value={styleId} onchange={() => schedulePreview()}>
          {#each capabilities?.styles ?? [] as id}
            <option value={id}>{styleLabel(id)}</option>
          {/each}
        </select>
      </label>
      <input bind:value={presetName} aria-label="Preset name" placeholder="Preset name" />
      <button type="button" onclick={() => void savePreset()}>Save preset</button>
    </div>
    <div class="layer-toggles" role="group" aria-label="Atlas layers">
      {#each layers as layer, index}
        <button
          class="layer-toggle"
          type="button"
          aria-pressed={layer.enabled}
          onclick={() => {
            layers[index].enabled = !layer.enabled;
            layers = layers;
            schedulePreview();
          }}>{layer.name}</button>
      {/each}
    </div>
    <div class="output-row">
      <label>
        Export size
        <select
          onchange={(event) => {
            const preset = PRESETS[Number(event.currentTarget.value)];
            if (!preset) return;
            widthPx = preset.width;
            heightPx = preset.height;
          }}>
          {#each PRESETS as preset, index}
            <option value={index} selected={preset.width === widthPx}
              >{preset.label} ({preset.width}×{preset.height})</option>
          {/each}
        </select>
      </label>
      <label>
        DPI metadata
        <input type="number" min="72" max="2400" bind:value={dpi} />
        <small>{(widthPx / dpi).toFixed(2)} × {(heightPx / dpi).toFixed(2)} in</small>
      </label>
    </div>
    {#if exportJob}
      <p class="status" role="status">Export · {exportJob.state} · {exportJob.stage}</p>
    {/if}
    {#if notice}<p class="notice">{notice}</p>{/if}
    <div class="actions">
      <button type="button" class="primary" disabled={exportBusy()} onclick={() => void runExport()}>Render</button>
      <button type="button" onclick={() => void cancel(exportJob ?? previewJob)}>Cancel render</button>
      <button type="button" disabled={exportJob?.state !== "ready-to-save"} onclick={() => void saveExport()}
        >Save</button>
    </div>
  </section>
</div>

<style>
.atlas-modal {
  position: fixed;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 1.5rem;
}
.atlas-backdrop {
  position: absolute;
  inset: 0;
  border: 0;
  border-radius: 0;
  background: rgb(8 14 12 / 62%);
  cursor: pointer;
}
.atlas-panel {
  position: relative;
  z-index: 1;
  display: grid;
  gap: 10px;
  width: min(52rem, 100%);
  max-height: calc(100vh - 3rem);
  overflow: auto;
  padding: 12px 16px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 12px;
  background: #1b2822;
  color: #edf2ec;
  font: 13px/1.4 system-ui;
  box-shadow: 0 16px 48px rgb(0 0 0 / 45%);
}
header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
}
header div {
  display: grid;
  gap: 2px;
}
header span {
  font-size: 10px;
  letter-spacing: 0.12em;
  color: #b8c8bc;
}
.preview-frame {
  position: relative;
  height: 320px;
  background: #101814;
  border-radius: 8px;
  overflow: hidden;
}
img,
.preview-empty {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.preview-empty {
  display: grid;
  place-items: center;
  color: #b8c8bc;
}
.preview-note {
  margin: 0;
  color: var(--theme-neutral-text-muted, #aebdb1);
  font-size: 12px;
}
.map-busy {
  position: absolute;
  inset: 0;
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 0.3rem;
  pointer-events: none;
  color: #f7f0e5;
  text-align: center;
  text-shadow: 0 1px 8px rgb(0 0 0 / 75%);
}
.map-busy strong {
  font: 600 1.05rem/1.3 inherit;
}
.map-busy span {
  color: #d9d0c3;
  font-size: 0.8rem;
}
.epoch-control {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 8px;
}
.epoch-control input[type="range"] {
  width: 140px;
  accent-color: #d5ab6c;
}
.epoch-year {
  width: 5.4em;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  padding: 4px 5px;
  background: #0f1a16;
  color: #edf2ec;
  font: 12px system-ui;
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.style-row {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  gap: 8px;
}
.output-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  align-items: start;
  gap: 8px;
}
.style-row label,
.output-row label {
  flex: 1 1 12rem;
  min-width: 0;
}
.output-row select,
.output-row input[type="number"] {
  box-sizing: border-box;
  height: 2.25rem;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  padding: 0 8px;
  background: #0f1a16;
  color: #edf2ec;
  font: 12px system-ui;
}
.output-row small {
  color: var(--theme-neutral-text-muted, #aebdb1);
}
.style-row input {
  min-width: 10rem;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  padding: 8px 10px;
  background: #0f1a16;
  color: #edf2ec;
  font: 12px system-ui;
}
.layer-toggles {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem 0.8rem;
}
label {
  display: grid;
  gap: 4px;
}
label select,
label input[type="number"] {
  min-width: 0;
  width: 100%;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
button {
  border: 0;
  border-radius: 7px;
  padding: 8px 10px;
  background: #31443b;
  color: #edf2ec;
  font: 700 12px system-ui;
  cursor: pointer;
}
.layer-toggle {
  padding: 0.3rem 0.65rem;
  border: 1px solid rgb(255 255 255 / 16%);
  border-radius: 999px;
  background: rgb(255 255 255 / 6%);
  color: #d9d0c3;
  font-weight: 600;
}
.layer-toggle:hover {
  border-color: rgb(255 255 255 / 28%);
  background: rgb(255 255 255 / 10%);
}
.layer-toggle[aria-pressed="true"] {
  border-color: var(--theme-warning-border, #c9a96e);
  background: #c9a96e;
  color: var(--brass-ink);
}
button.primary,
button:disabled {
  background: #d5ab6c;
  color: var(--brass-ink);
}
button:disabled {
  opacity: 0.45;
}
.notice,
.status {
  margin: 0;
  color: #d8e3d9;
}
</style>
