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

let {
  mapId,
  epochOffsetYears = 0,
  seed = null,
  onclose,
}: {
  mapId: string;
  epochOffsetYears?: number;
  seed?: AtlasRenderRequest | null;
  onclose?: () => void;
} = $props();

const PRESETS = [
  { label: "Preview 2K", width: 2048, height: 1024 },
  { label: "Print 4K", width: 4096, height: 2048 },
  { label: "Print 8K", width: 8192, height: 4096 },
] as const;

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

function formatEpoch(offset: number) {
  if (offset === 0) return "Reference epoch";
  if (offset < 0) return `${Math.abs(offset)} years before reference`;
  return `${offset} years after reference`;
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
  if (status.kind === "preview") {
    if (status.requestId !== previewRequestId && status.state !== "cancelled") return;
    previewJob = status;
    busyPreview = status.state !== "ready-to-save" && status.state !== "failed" && status.state !== "cancelled";
    if (status.previewToken && status.state === "ready-to-save") {
      previewUrl = convertFileSrc(status.previewToken);
    }
    if (status.error && status.state === "failed") notice = status.error;
  } else {
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
  layers = capabilities.layers.map((layer) => ({
    id: layer.id,
    name: layer.name,
    enabled: layer.defaultVisible,
  }));
  applySeed(seed);
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
      request(2048, 1024),
      previewRequestId as `${string}-${string}-${string}-${string}-${string}`,
    );
    applyStatus(status);
  } catch (error) {
    busyPreview = false;
    notice = error instanceof Error ? error.message : String(error);
  }
}

async function runExport() {
  notice = "";
  try {
    const status = await project.atlasRenderBegin(mapId, request(widthPx, heightPx), crypto.randomUUID());
    applyStatus(status);
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
});

onDestroy(() => {
  unlisten?.();
  if (previewTimer) clearTimeout(previewTimer);
  if (pollTimer) clearInterval(pollTimer);
  if (previewJob) void project.atlasArtifactDiscard(previewJob.jobId).catch(() => undefined);
});
</script>

<section class="atlas-panel" aria-label="Render Atlas Map">
  <header>
    <strong>Render Atlas Map</strong>
    <button type="button" onclick={() => onclose?.()}>Close</button>
  </header>
  {#if previewUrl}
    <img src={previewUrl} alt="Atlas preview" />
  {:else}
    <div class="preview-empty">{busyPreview ? "Rendering preview…" : "Preview will appear here"}</div>
  {/if}
  {#if previewJob}
    <p class="status" role="status">
      Preview · {previewJob.stage} · {previewJob.completed}/{previewJob.total}
    </p>
  {/if}
  <label>
    World time
    <input
      type="range"
      min="-100000"
      max="100000"
      step="1"
      bind:value={offsetYears}
      oninput={() => schedulePreview()} />
    <output>{formatEpoch(offsetYears)}</output>
  </label>
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
  <label>
    Style
    <select bind:value={styleId} onchange={() => schedulePreview()}>
      {#each capabilities?.styles ?? [] as id}
        <option value={id}>{styleLabel(id)}</option>
      {/each}
    </select>
  </label>
  <fieldset>
    <legend>Layers</legend>
    {#each layers as layer, index}
      <label>
        <input
          type="checkbox"
          checked={layer.enabled}
          onchange={(event) => {
            layers[index].enabled = event.currentTarget.checked;
            layers = layers;
            schedulePreview();
          }} />
        {layer.name}
      </label>
    {/each}
    <p>
      Rivers include atlas-only minor tributaries. They are not canonical geography and are never promoted
      automatically.
    </p>
  </fieldset>
  <label>
    Size
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
  {#if exportJob}
    <p class="status" role="status">Export · {exportJob.state} · {exportJob.stage}</p>
  {/if}
  {#if notice}<p class="notice">{notice}</p>{/if}
  <div class="actions">
    <button type="button" class="primary" onclick={() => void runExport()}>Render</button>
    <button type="button" onclick={() => void cancel(exportJob ?? previewJob)}>Cancel render</button>
    <button type="button" disabled={exportJob?.state !== "ready-to-save"} onclick={() => void saveExport()}
      >Save</button>
    <input bind:value={presetName} aria-label="Preset name" />
    <button type="button" onclick={() => void savePreset()}>Save preset</button>
    <button type="button" onclick={() => onclose?.()}>Close</button>
  </div>
</section>

<style>
.atlas-panel {
  display: grid;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid #405047;
  background: #1b2822;
  color: #edf2ec;
  font: 13px/1.4 system-ui;
}
header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
img,
.preview-empty {
  width: 100%;
  max-height: 280px;
  object-fit: contain;
  background: #101814;
  border-radius: 8px;
}
.preview-empty {
  min-height: 120px;
  display: grid;
  place-items: center;
  color: #b8c8bc;
}
label,
fieldset {
  display: grid;
  gap: 4px;
}
fieldset {
  border: 1px solid #405047;
  border-radius: 8px;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
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
button.primary,
button:disabled {
  background: #d5ab6c;
  color: #243126;
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
