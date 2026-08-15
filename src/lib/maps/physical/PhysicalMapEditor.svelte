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

let name = $state("Physical World");
let seed = $state(831429);
let evolutionPreset = $state<"young" | "mature" | "old">("mature");
let status = $state<PhysicalJobStatus | null>(null);
let hydrology = $state<PhysicalHydrologyProducts | null>(null);
let raster = $state<HTMLCanvasElement | null>(null);
let showHillshade = $state(true);
let notice = $state("");
let busy = $state(false);
let preview = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let layers = $state<VectorLayerDefinition[]>([
  {
    id: "base",
    kind: "vector",
    name: "Physical base",
    order: 0,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: DEFAULT_VECTOR_LAYER_STYLE,
  },
  {
    id: "tectonic-plates",
    kind: "vector",
    name: "Tectonic plates",
    order: 5,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#6c8ebf", fillOpacity: 0.12, stroke: "#5c7aa5", strokeWidth: 0.5, pointRadius: 2 },
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
    id: "bathymetry",
    kind: "vector",
    name: "Bathymetry",
    order: 7,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#4e89b5", fillOpacity: 0.12, stroke: "#386b91", strokeWidth: 0.35, pointRadius: 2 },
  },
  {
    id: "volcanic-centers",
    kind: "vector",
    name: "Volcanic centers",
    order: 8,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#ef9b4a", fillOpacity: 0.9, stroke: "#8f4c25", strokeWidth: 1, pointRadius: 5 },
  },
  {
    id: "earthquake-hazard",
    kind: "vector",
    name: "Earthquake hazard (generated)",
    order: 9,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#c95353", fillOpacity: 0.72, stroke: "#7f2525", strokeWidth: 0.8, pointRadius: 3 },
  },
  {
    id: "volcanic-hazard",
    kind: "vector",
    name: "Volcanic hazard (generated)",
    order: 10,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#f08a36", fillOpacity: 0.72, stroke: "#8f4c25", strokeWidth: 0.8, pointRadius: 3 },
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
    id: "watersheds",
    kind: "vector",
    name: "Watersheds",
    order: 13,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#9c80d1", fillOpacity: 0.08, stroke: "#bba7e5", strokeWidth: 0.45, pointRadius: 2 },
  },
  {
    id: "ocean",
    kind: "vector",
    name: "Ocean",
    order: 1,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#245c80", fillOpacity: 0.58, stroke: "#397da5", strokeWidth: 0.3, pointRadius: 2 },
  },
  {
    id: "land",
    kind: "vector",
    name: "Exposed land",
    order: 2,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#b99b62", fillOpacity: 0.55, stroke: "#d8bd83", strokeWidth: 0.45, pointRadius: 2 },
  },
  {
    id: "shelves",
    kind: "vector",
    name: "Continental shelves",
    order: 3,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#4f87a2", fillOpacity: 0.25, stroke: "#8db4c3", strokeWidth: 0.35, pointRadius: 2 },
  },
  {
    id: "bathymetric-contours",
    kind: "vector",
    name: "Bathymetric contours",
    order: 4,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#78b3ca", fillOpacity: 0, stroke: "#78b3ca", strokeWidth: 0.6, pointRadius: 2 },
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
  {
    id: "ice",
    kind: "vector",
    name: "Ice",
    order: 15,
    defaultVisible: false,
    locked: true,
    selector: {},
    style: { fill: "#e8f2f8", fillOpacity: 0.82, stroke: "#c5d8e6", strokeWidth: 0.4, pointRadius: 2 },
  },
]);

function publish(nextStatus: string, detail: unknown = null) {
  onstate?.(nextStatus, detail);
}

function randomSeed() {
  const values = new Uint32Array(1);
  crypto.getRandomValues(values);
  seed = values[0] ?? seed;
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

async function loadSavedMap() {
  if (!mapId) return;
  busy = true;
  try {
    preview = parseVectorCollection(new TextEncoder().encode(await project.physicalMapDerivedGeoJson(mapId)));
    hydrology = await project.physicalMapDerivedHydrology(mapId);
    await mountPreview();
  } catch (cause) {
    notice = cause instanceof Error ? cause.message : String(cause);
    publish("error", { detail: notice });
  } finally {
    busy = false;
  }
}

async function poll(jobId: string) {
  for (;;) {
    status = await project.physicalMapStatus(jobId);
    if (status.state !== "running" && status.state !== "cancelling") break;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  if (status.state === "completed") {
    preview = parseVectorCollection(new TextEncoder().encode(await project.physicalMapPreview(jobId)));
    hydrology = await project.physicalMapHydrology(jobId);
    await mountPreview();
    publish("preview-ready", { jobId });
  } else if (status.state !== "cancelled") {
    notice = status.error ?? "Physical generation failed";
    publish("error", { detail: notice });
  }
  busy = false;
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
    await poll(status.jobId);
  } catch (cause) {
    busy = false;
    notice = cause instanceof Error ? cause.message : String(cause);
    publish("error", { detail: notice });
  }
}

async function cancel() {
  if (!status || !busy) {
    oncancel?.();
    return;
  }
  await project.cancelPhysicalMap(status.jobId);
}

async function accept() {
  if (!status || status.state !== "completed" || busy) return;
  busy = true;
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
    if (status?.jobId && busy && (status.state === "running" || status.state === "cancelling")) {
      void project.cancelPhysicalMap(status.jobId);
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
        <span>PHYSICAL WORLD</span><strong
          >{busy ? `${status?.stage ?? "Starting"}…` : "One world, one preview"}</strong>
      </div>
      <button class="icon-button" type="button" aria-label="Cancel" onclick={() => void cancel()}>×</button>
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
    {#if status}<p class="physical-map-progress" role="status">
        {status.state === "completed" ? "Preview ready" : `${status.stage} · ${status.completed}/${status.total}`}
      </p>{/if}
    {#if notice}<p class="map-reconcile-notice" role="alert">{notice}</p>{/if}
    <div class="physical-layer-controls" aria-label="Physical diagnostic layers">
      <label
        ><input
          type="checkbox"
          checked={showHillshade}
          onchange={() => {
            showHillshade = !showHillshade;
          }} />
        Hillshade</label>
      {#each layers as layer}
        <label
          ><input type="checkbox" checked={layer.defaultVisible} onchange={() => toggleLayer(layer.id)} />
          {layer.name}</label>
      {/each}
    </div>
    <small class="physical-hazard-legend"
      >Hazard layers show relative generated rates; they are not real-world predictions.</small>
    <div class="native-vector-map">
      <PhysicalWorldView collection={preview} {layers} {raster} showRaster={showHillshade} />
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
  min-height: 560px;
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

.physical-map-controls select,
.physical-map-editor button {
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 0.35rem;
  font: inherit;
}

.physical-map-controls select {
  min-width: 10rem;
  background: rgb(255 255 255 / 7%);
  color: inherit;
  padding: 0.45rem 0.55rem;
}

.physical-map-editor button {
  cursor: pointer;
  padding: 0.45rem 0.7rem;
}

.physical-map-editor button:disabled {
  cursor: wait;
  opacity: 0.55;
}

.physical-map-progress,
.physical-map-editor .map-reconcile-notice {
  margin: 0;
  padding: 0.55rem 1rem;
}

.physical-layer-controls {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem 0.8rem;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid rgb(255 255 255 / 8%);
  color: #d9d0c3;
  font-size: 0.75rem;
}

.physical-layer-controls label {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.physical-hazard-legend {
  display: block;
  padding: 0.45rem 1rem;
  color: #b9c4c7;
  font-size: 0.72rem;
}

.native-vector-map {
  display: flex;
  min-height: 360px;
  min-width: 0;
  flex: 1;
}

.physical-map-actions {
  justify-content: flex-end;
  border-top: 1px solid rgb(255 255 255 / 12%);
}
</style>
