<script lang="ts">
import { onMount, tick } from "svelte";
import {
  project,
  type Entity,
  type PhysicalGenerationInput,
  type PhysicalHydrologyProducts,
  type PhysicalJobStatus,
} from "$lib/project/client";
import { paintPhysicalSurface } from "./raster";
import PhysicalWorldView from "./PhysicalWorldView.svelte";
import NativeVectorMapEditor from "../native-vector/NativeVectorMapEditor.svelte";
import { parseVectorCollection } from "../native-vector/source";
import {
  BASE_LAYER_ID,
  DEFAULT_VECTOR_LAYER_STYLE,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "../native-vector/types";

let {
  mapId,
  oncreated,
  oncancel,
  onstate,
}: {
  mapId?: string;
  oncreated?: (map: Entity) => void;
  oncancel?: () => void;
  onstate?: (status: string, detail: unknown) => void;
} = $props();

function nextPhysicalSeed(fallback = 0) {
  const values = new Uint32Array(1);
  crypto.getRandomValues(values);
  return values[0] ?? fallback;
}

let name = $state("Physical World");
let seed = $state(nextPhysicalSeed());
let evolutionPreset = $state<"young" | "mature" | "old">("mature");
let status = $state<PhysicalJobStatus | null>(null);
let hydrology = $state<PhysicalHydrologyProducts | null>(null);
let raster = $state<HTMLCanvasElement | null>(null);
let notice = $state("");
let busy = $state(false);
let activeJobId: string | null = null;
let helpSeen = $state(false);
let preview = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let layers = $state<VectorLayerDefinition[]>([
  {
    id: BASE_LAYER_ID,
    kind: "vector",
    name: "Physical base",
    order: 0,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: DEFAULT_VECTOR_LAYER_STYLE,
  },
  {
    id: "ice",
    kind: "vector",
    name: "Ice",
    order: 1,
    defaultVisible: true,
    locked: true,
    selector: {},
    style: { fill: "#e8f2f8", fillOpacity: 0.82, stroke: "#c5d8e6", strokeWidth: 0.4, pointRadius: 2 },
  },
  {
    id: "ocean",
    kind: "vector",
    name: "Ocean",
    order: 2,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#245c80", fillOpacity: 0.58, stroke: "#397da5", strokeWidth: 0.3, pointRadius: 2 },
  },
  {
    id: "tectonic-boundaries",
    kind: "vector",
    name: "Plate boundaries",
    order: 6,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#d46a5e", fillOpacity: 0, stroke: "#d46a5e", strokeWidth: 2, pointRadius: 2 },
  },
  {
    id: "lakes",
    kind: "vector",
    name: "Lakes",
    order: 11,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#4d9ac2", fillOpacity: 0.72, stroke: "#b8e4f5", strokeWidth: 1, pointRadius: 2 },
  },
  {
    id: "rivers",
    kind: "vector",
    name: "Rivers",
    order: 12,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#71c7e8", fillOpacity: 0, stroke: "#71c7e8", strokeWidth: 1.5, pointRadius: 2 },
  },
  {
    id: "islands",
    kind: "vector",
    name: "Islands",
    order: 14,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#e0bb78", fillOpacity: 0.18, stroke: "#f0d39b", strokeWidth: 0.7, pointRadius: 2 },
  },
]);

function publish(nextStatus: string, detail: unknown = null) {
  onstate?.(nextStatus, detail);
}

function overlayLayers() {
  return layers.filter((layer) => layer.id !== BASE_LAYER_ID);
}

function headline() {
  if (status?.state === "completed") return "Preview ready — accept this world";
  return "Generate a globe, then accept it";
}

function randomSeed() {
  seed = nextPhysicalSeed(seed);
}

function destroyPreview() {
  raster = null;
}

function toggleLayer(layerId: string) {
  layers = layers.map((layer) => (layer.id === layerId ? { ...layer, defaultVisible: !layer.defaultVisible } : layer));
}

function rebuildRaster(products: PhysicalHydrologyProducts | null) {
  raster = products ? paintPhysicalSurface(products) : null;
}

async function mountPreview() {
  await tick();
  rebuildRaster(hydrology);
  publish("ready", { preview: true });
}

function parseDerivedVectorCollection(text: string): VectorFeatureCollection {
  let skipped = 0;
  const collection = parseVectorCollection(new TextEncoder().encode(text), {
    lenient: true,
    onSkipped: () => {
      skipped += 1;
    },
  });
  if (skipped > 0) {
    notice = `${skipped} preview feature${skipped === 1 ? "" : "s"} skipped because of degenerate geometry.`;
  }
  return collection;
}

async function loadSavedMap() {
  if (!mapId) return;
  busy = true;
  try {
    preview = parseDerivedVectorCollection(await project.physicalMapDerivedGeoJson(mapId));
    hydrology = await project.physicalMapDerivedHydrology(mapId);
    await mountPreview();
  } catch (cause) {
    notice = cause instanceof Error ? cause.message : String(cause);
    publish("error", { detail: notice });
  } finally {
    busy = false;
  }
}

async function cancelActiveJob(jobId: string) {
  activeJobId = null;
  try {
    await project.cancelPhysicalMap(jobId);
  } catch (cause) {
    notice = `Could not cancel the background generation: ${cause instanceof Error ? cause.message : String(cause)}`;
    publish("error", { detail: notice });
  }
}

async function poll(jobId: string) {
  try {
    for (;;) {
      status = await project.physicalMapStatus(jobId);
      if (status.state !== "running" && status.state !== "cancelling") break;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    if (status.state === "completed") {
      preview = parseDerivedVectorCollection(await project.physicalMapPreview(jobId));
      hydrology = await project.physicalMapHydrology(jobId);
      await mountPreview();
      publish("preview-ready", { jobId });
    } else if (status.state !== "cancelled") {
      notice = status.error ?? "Physical generation failed";
      publish("error", { detail: notice });
    }
  } catch (cause) {
    notice = cause instanceof Error ? cause.message : String(cause);
    publish("error", { detail: notice });
    if (activeJobId === jobId) {
      await cancelActiveJob(jobId);
    }
  }
}

async function generate() {
  if (busy) return;
  busy = true;
  notice = "";
  hydrology = null;
  preview = { type: "FeatureCollection", features: [] };
  destroyPreview();
  try {
    const input: PhysicalGenerationInput = {
      seed,
      retryIndex: 0,
      evolutionPreset,
      settings: {
        width: 384,
        height: 192,
        radiusMetres: 6_371_000,
        targetLandFractionPpm: 300_000,
      },
    };
    status = await project.generatePhysicalMap(input);
    activeJobId = status.jobId;
    await poll(status.jobId);
  } catch (cause) {
    notice = cause instanceof Error ? cause.message : String(cause);
    publish("error", { detail: notice });
  } finally {
    busy = false;
  }
}

async function cancel() {
  const jobId = activeJobId ?? status?.jobId ?? null;
  if (!jobId || !busy) {
    oncancel?.();
    return;
  }
  await cancelActiveJob(jobId);
}

async function accept() {
  if (!status || status.state !== "completed" || busy) return;
  busy = true;
  activeJobId = null;
  try {
    const accepted = await project.acceptPhysicalMap(status.jobId, name.trim() || "Physical World");
    oncreated?.(accepted.entity);
  } catch (cause) {
    busy = false;
    notice = cause instanceof Error ? cause.message : String(cause);
    publish("error", { detail: notice });
  }
}

onMount(() => {
  if (mapId) return;
  void loadSavedMap();
  return () => {
    if (activeJobId) {
      void cancelActiveJob(activeJobId);
    }
    destroyPreview();
  };
});
</script>

{#if mapId}
  <NativeVectorMapEditor {mapId} {oncancel} {onstate} />
{:else}
  <section class="native-vector-editor physical-map-editor" aria-label="Generate physical map">
    <header>
      <div>
        <span>PHYSICAL WORLD</span>
        <strong>{headline()}</strong>
      </div>
      <button class="icon-button" type="button" aria-label="Close" onclick={() => void cancel()}>×</button>
    </header>
    <div class="physical-map-controls">
      <label>Map name<input bind:value={name} disabled={busy} /></label>
      <label>Seed<input type="number" bind:value={seed} disabled={busy} min="0" max="4294967295" /></label>
      <label
        >Terrain age<select bind:value={evolutionPreset} disabled={busy}>
          <option value="young">Young</option>
          <option value="mature">Mature</option>
          <option value="old">Old</option>
        </select></label>
      <button class="quiet-button" type="button" onclick={randomSeed} disabled={busy}>Reroll seed</button>
    </div>
    {#if notice}<p class="map-reconcile-notice" role="alert">{notice}</p>{/if}
    <div class="physical-layer-controls">
      <div role="group" aria-label="Physical diagnostic layers">
        {#each overlayLayers() as layer (layer.id)}
          <button
            class="layer-toggle"
            type="button"
            aria-pressed={layer.defaultVisible}
            onclick={() => toggleLayer(layer.id)}>{layer.name}</button>
        {/each}
      </div>
      <span class="physical-map-help">
        <button type="button" aria-describedby="physical-map-hint" aria-label="About this preview" class:unread={!helpSeen} onmouseenter={() => { helpSeen = true; }} onfocus={() => { helpSeen = true; }}>?</button>
        <p id="physical-map-hint" role="tooltip">
          This preview locks the world’s physical shape—coasts, climate, ice, rivers, and the rest. The accepted,
          exportable map is a high-resolution render with far more detail and quality.
        </p>
      </span>
    </div>
    <div class="native-vector-map">
      <PhysicalWorldView collection={preview} {layers} {raster} showRaster />
      {#if busy}
        <div class="physical-map-stage" role="status">
          <strong>{status?.stage ?? "Starting"}…</strong>
          {#if status && status.total > 0}<span>{status.completed} / {status.total}</span>{/if}
        </div>
      {:else if preview.features.length === 0 && !raster}
        <div class="physical-map-empty-hint">
          <p>Click <strong>Generate world</strong> below to create a physical map.</p>
        </div>
      {/if}
    </div>
    <footer class="physical-map-actions">
      {#if status?.state === "completed"}
        <button class="primary-button" type="button" onclick={() => void accept()} disabled={busy}>Accept world</button>
        <button class="quiet-button" type="button" onclick={() => void generate()} disabled={busy}>Reroll</button>
      {:else if busy}
        <button class="quiet-button" type="button" onclick={() => void cancel()}>Cancel generation</button>
      {:else}
        <button class="primary-button" type="button" onclick={() => void generate()}>Generate world</button>
        <button class="quiet-button" type="button" onclick={() => oncancel?.()}>Back</button>
      {/if}
    </footer>
  </section>
{/if}

<style>
.physical-map-editor {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
  background: #0d1b2a;
  color: #f7f0e5;
}

.physical-map-editor header,
.physical-map-actions,
.physical-map-controls {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.9rem 1rem;
}

.physical-map-editor header {
  justify-content: space-between;
  border-bottom: 1px solid rgb(255 255 255 / 12%);
}

.physical-map-editor header div {
  display: grid;
  gap: 0.25rem;
}

.physical-map-editor header span {
  color: #c9a96e;
  font-size: 0.68rem;
  letter-spacing: 0.12em;
}

.physical-map-controls {
  flex-wrap: wrap;
}

.physical-map-controls label {
  display: grid;
  gap: 0.3rem;
  min-width: 10rem;
  font-size: 0.78rem;
}

.physical-map-controls input {
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 0.35rem;
  background: rgb(255 255 255 / 7%);
  color: inherit;
  padding: 0.45rem 0.55rem;
}

.physical-map-controls select {
  min-width: 10rem;
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 0.35rem;
  background: rgb(255 255 255 / 7%);
  color: inherit;
  padding: 0.45rem 0.55rem;
  font: inherit;
}

.physical-map-editor .icon-button,
.physical-map-editor .quiet-button,
.physical-map-editor .primary-button,
.physical-map-editor .layer-toggle {
  cursor: pointer;
  font: inherit;
}

.physical-map-editor .icon-button {
  display: grid;
  place-items: center;
  width: 2rem;
  height: 2rem;
  padding: 0;
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 0.35rem;
  background: transparent;
  color: #f7f0e5;
  font-size: 1.2rem;
  line-height: 1;
}

.physical-map-editor .icon-button:hover {
  background: rgb(255 255 255 / 10%);
}

.physical-map-editor .icon-button:focus-visible,
.physical-map-editor .quiet-button:focus-visible,
.physical-map-editor .primary-button:focus-visible,
.physical-map-editor .layer-toggle:focus-visible,
.physical-map-help button:focus-visible {
  outline: 2px solid #f3d39a;
  outline-offset: 2px;
}

.physical-map-editor .quiet-button {
  padding: 0.5rem 0.85rem;
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 0.45rem;
  background: rgb(255 255 255 / 8%);
  color: #f7f0e5;
}

.physical-map-editor .quiet-button:hover {
  border-color: rgb(255 255 255 / 28%);
  background: rgb(255 255 255 / 12%);
}

.physical-map-editor .primary-button {
  padding: 0.5rem 0.95rem;
  border: 1px solid #d4b57a;
  border-radius: 0.45rem;
  background: #c9a96e;
  color: #0d1b2a;
  font-weight: 700;
}

.physical-map-editor .primary-button:hover {
  background: #d8ba82;
}

.physical-map-editor .icon-button:disabled,
.physical-map-editor .quiet-button:disabled,
.physical-map-editor .primary-button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.physical-map-editor .layer-toggle {
  padding: 0.3rem 0.65rem;
  border: 1px solid rgb(255 255 255 / 16%);
  border-radius: 999px;
  background: rgb(255 255 255 / 6%);
  color: #d9d0c3;
}

.physical-map-editor .layer-toggle:hover {
  border-color: rgb(255 255 255 / 28%);
  background: rgb(255 255 255 / 10%);
}

.physical-map-editor .layer-toggle[aria-pressed="true"] {
  border-color: #c9a96e;
  background: #c9a96e;
  color: #0d1b2a;
  font-weight: 600;
}

.physical-map-editor .map-reconcile-notice {
  margin: 0;
  padding: 0.55rem 1rem;
}

.physical-layer-controls {
  position: relative;
  z-index: 3;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.45rem 0.8rem;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid rgb(255 255 255 / 8%);
  color: #d9d0c3;
  font-size: 0.75rem;
}

.physical-layer-controls > div {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem 0.8rem;
}

.physical-map-help {
  position: relative;
  margin-left: auto;
}

.physical-map-help button {
  position: relative;
  display: grid;
  place-items: center;
  width: 1.65rem;
  height: 1.65rem;
  padding: 0;
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 999px;
  background: rgb(255 255 255 / 6%);
  color: #d9d0c3;
  font: inherit;
  cursor: pointer;
}

.physical-map-help button:hover {
  border-color: rgb(255 255 255 / 28%);
  background: rgb(255 255 255 / 10%);
  color: #f7f0e5;
}

.physical-map-help button.unread::after {
  content: "";
  position: absolute;
  top: -2px;
  right: -2px;
  width: 7px;
  height: 7px;
  border-radius: 999px;
  background: #e8913a;
  box-shadow: 0 0 4px #e8913a;
}

.physical-map-help p {
  position: absolute;
  z-index: 3;
  right: calc(100% + 0.35rem);
  top: calc(100% + 0.35rem);
  display: none;
  width: min(22rem, calc(100vw - 2.5rem));
  margin: 0;
  padding: 0.65rem 0.75rem;
  border: 1px solid rgb(255 255 255 / 16%);
  border-radius: 0.45rem;
  background: #152536;
  color: #f7f0e5;
  box-shadow: 0 8px 24px rgb(0 0 0 / 35%);
  font-size: 0.78rem;
  font-weight: 400;
  line-height: 1.45;
}

.physical-map-help:hover p,
.physical-map-help:focus-within p {
  display: block;
}

.native-vector-map {
  position: relative;
  display: flex;
  min-height: 360px;
  min-width: 0;
  flex: 1;
}

.physical-map-stage {
  position: absolute;
  z-index: 2;
  inset: 0;
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 0.35rem;
  pointer-events: none;
  background: rgb(13 27 42 / 42%);
  color: #f7f0e5;
  text-align: center;
}

.physical-map-stage strong {
  font: 600 1.05rem/1.3 inherit;
}

.physical-map-stage span {
  color: #d9d0c3;
  font-size: 0.8rem;
}

.physical-map-empty-hint {
  position: absolute;
  inset: 0;
  display: grid;
  place-content: center;
  pointer-events: none;
  text-align: center;
}

.physical-map-empty-hint p {
  margin: 0;
  padding: 0.55rem 1rem;
  border-radius: 0.45rem;
  background: rgb(13 27 42 / 55%);
  color: #c9a96e;
  font-size: 0.82rem;
  line-height: 1.5;
}

.physical-map-actions {
  justify-content: flex-end;
  border-top: 1px solid rgb(255 255 255 / 12%);
}
</style>
