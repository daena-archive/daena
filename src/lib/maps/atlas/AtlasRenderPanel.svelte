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
  onclose,
}: {
  mapId: string;
  epochOffsetYears?: number;
  onclose?: () => void;
} = $props();

const PRESETS = [
  { label: "Preview 2K", width: 2048, height: 1024 },
  { label: "Print 4K", width: 4096, height: 2048 },
  { label: "Print 8K", width: 8192, height: 4096 },
] as const;

let capabilities = $state<AtlasRenderCapabilities | null>(null);
let styleId = $state("daena-atlas-relief");
let widthPx = $state(2048);
let heightPx = $state(1024);
let dpi = $state(300);
let offsetYears = $state(epochOffsetYears);
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

function formatEpoch(offset: number) {
  if (offset === 0) return "Reference epoch";
  if (offset < 0) return `${Math.abs(offset)} years before reference`;
  return `${offset} years after reference`;
}

function request(width: number, height: number): AtlasRenderRequest {
  return {
    schemaVersion: 1,
    offsetYears,
    algorithmVersion: 1,
    level: "detailed",
    variant: 0,
    styleId,
    widthPx: width,
    heightPx: height,
    dpi,
    format: "png",
    activeLayerIds: layers.filter((layer) => layer.enabled).map((layer) => layer.id),
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
    notice = "The project changed after this render. The image is from the captured generation. Render again to update.";
  }
}

async function loadCapabilities() {
  capabilities = await project.atlasCapabilities(mapId);
  if (!capabilities.supported) {
    notice = "This map provider does not support atlas rendering.";
    return;
  }
  styleId = capabilities.styles[0] ?? "daena-atlas-relief";
  layers = capabilities.layers.map((layer) => ({
    id: layer.id,
    name: layer.name,
    enabled: layer.defaultVisible,
  }));
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
    const status = await project.atlasPreviewBegin(mapId, request(2048, 1024), previewRequestId);
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

async function cancel(job: AtlasJobStatus | null) {
  if (!job) return;
  applyStatus(await project.atlasJobCancel(job.jobId));
}

onMount(() => {
  void loadCapabilities();
  void listen<AtlasJobStatus>(ATLAS_PROGRESS_EVENT, (event) => applyStatus(event.payload)).then((fn) => {
    unlisten = fn;
  });
  pollTimer = setInterval(() => {
    const jobs = [previewJob, exportJob].filter((job): job is AtlasJobStatus => Boolean(job));
    for (const job of jobs) {
      void project.atlasJobStatus(job.jobId).then(applyStatus).catch(() => undefined);
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
  <label>
    Style
    <select bind:value={styleId} onchange={() => schedulePreview()}>
      {#each capabilities?.styles ?? [] as id}
        <option value={id}>{id}</option>
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
        <option value={index} selected={preset.width === widthPx}>{preset.label} ({preset.width}×{preset.height})</option>
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
    <button type="button" disabled={exportJob?.state !== "ready-to-save"} onclick={() => void saveExport()}>Save</button>
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
