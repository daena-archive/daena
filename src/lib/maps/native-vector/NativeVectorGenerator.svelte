<script lang="ts">
import { onDestroy, onMount } from "svelte";
import type { Entity } from "$lib/project/client";
import { project } from "$lib/project/client";
import {
  DEFAULT_GENERATOR_SETTINGS,
  generationProvenance,
  type NativeGeneratorCandidate,
  type NativeGeneratorSettings,
} from "./generator";

let {
  oncreated,
  oncancel,
  autostartImport = false,
}: {
  oncreated?: (map: Entity) => void;
  oncancel?: () => void;
  autostartImport?: boolean;
} = $props();

let settings = $state<NativeGeneratorSettings>({ ...DEFAULT_GENERATOR_SETTINGS });
let seedText = $state(String(DEFAULT_GENERATOR_SETTINGS.seed));
let mapName = $state("Untitled map");
let candidates = $state<NativeGeneratorCandidate[]>([]);
let selected = $state<number | null>(null);
let busy = $state(false);
let accepting = $state(false);
let message = $state("");
let requestId = 0;
let worker: Worker | null = null;
let WorkerCtor: (new () => Worker) | null = null;
let workerReady: Promise<Worker> | null = null;

function parseSeed(value: string) {
  const parsed = Number.parseInt(value.trim(), 10);
  if (!Number.isInteger(parsed) || parsed < 0 || parsed > 0xffff_ffff) return null;
  return parsed;
}

function applySeed() {
  const parsed = parseSeed(seedText);
  if (parsed === null) {
    message = "Seed must be an integer from 0 to 4294967295.";
    seedText = String(settings.seed);
    return false;
  }
  settings = { ...settings, seed: parsed };
  seedText = String(parsed);
  return true;
}

async function ensureWorker() {
  if (worker) return worker;
  workerReady ??= (async () => {
    if (!WorkerCtor) {
      const module = await import("./generator.worker.js?worker");
      WorkerCtor = module.default;
    }
    worker = new WorkerCtor();
    worker.onmessage = (event: MessageEvent) => {
      const data = event.data as {
        type: string;
        requestId: number;
        candidates?: NativeGeneratorCandidate[];
        message?: string;
      };
      if (data.requestId !== requestId) return;
      busy = false;
      if (data.type === "error") {
        message = data.message ?? "Generation failed.";
        candidates = [];
        selected = null;
        return;
      }
      candidates = data.candidates ?? [];
      selected = candidates.length ? 0 : null;
      message = candidates.length ? "" : "No land polygons survived these settings.";
    };
    worker.onerror = (event) => {
      busy = false;
      message = event.message || "Generation worker failed.";
    };
    return worker;
  })();
  return workerReady;
}

function generate() {
  if (!applySeed()) return;
  busy = true;
  message = "";
  selected = null;
  candidates = [];
  const current = ++requestId;
  void ensureWorker()
    .then((instance) => {
      if (current !== requestId) return;
      instance.postMessage({ type: "generate", requestId: current, settings: { ...settings } });
    })
    .catch((cause) => {
      if (current !== requestId) return;
      busy = false;
      message = cause instanceof Error ? cause.message : String(cause);
    });
}

async function copySeed() {
  try {
    await navigator.clipboard.writeText(String(settings.seed));
    message = "Seed copied.";
  } catch {
    message = "Could not copy the seed.";
  }
}

async function pasteSeed() {
  try {
    seedText = (await navigator.clipboard.readText()).trim();
    if (applySeed()) generate();
  } catch {
    message = "Could not paste a seed.";
  }
}

async function accept() {
  if (selected === null || !candidates[selected] || accepting) return;
  accepting = true;
  message = "";
  try {
    const imported = await project.acceptVectorMap(
      mapName.trim() || "Untitled map",
      candidates[selected].collection,
      generationProvenance(settings),
    );
    await oncreated?.(imported.entity);
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
  } finally {
    accepting = false;
  }
}

function stopWorker() {
  worker?.terminate();
  worker = null;
  workerReady = null;
}

function cancel() {
  stopWorker();
  oncancel?.();
}

async function importImage() {
  const source = await project.pickFile();
  if (typeof source !== "string") {
    if (autostartImport) oncancel?.();
    return;
  }
  accepting = true;
  message = "";
  try {
    const imported = await project.importImageMapFile(source);
    await oncreated?.(imported.entity);
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
  } finally {
    accepting = false;
  }
}

onMount(() => {
  if (autostartImport) void importImage();
  else generate();
});

onDestroy(() => {
  stopWorker();
});
</script>

<section class="generator" aria-label="Generate a native vector map">
  <header>
    <div>
      <span>NATIVE VECTOR MAP</span>
      <strong>Generate landmass</strong>
    </div>
    <div class="header-actions">
      <button type="button" class="quiet" onclick={cancel}>Cancel</button>
      <button type="button" class="quiet" disabled={busy || accepting} onclick={() => void importImage()}
        >{accepting && autostartImport ? "Importing…" : "Import image"}</button>
      <button type="button" class="primary" disabled={selected === null || busy || accepting} onclick={() => void accept()}
        >{accepting ? "Accepting…" : "Accept candidate"}</button>
    </div>
  </header>
  <div class="body">
    <form
      class="controls"
      onsubmit={(event) => {
        event.preventDefault();
        generate();
      }}>
      <label>
        Map name
        <input bind:value={mapName} maxlength="120" autocomplete="off" />
      </label>
      <fieldset>
        <legend>Seed</legend>
        <div class="seed-row">
          <input
            aria-label="Generator seed"
            inputmode="numeric"
            bind:value={seedText}
            onchange={applySeed} />
          <button type="button" onclick={() => void copySeed()}>Copy</button>
          <button type="button" onclick={() => void pasteSeed()}>Paste</button>
        </div>
      </fieldset>
      <label>
        Land percent
        <input
          type="range"
          min="15"
          max="70"
          step="1"
          bind:value={settings.landPercent}
          aria-valuemin={15}
          aria-valuemax={70}
          aria-valuenow={settings.landPercent} />
        <span>{settings.landPercent}%</span>
      </label>
      <label>
        Continents
        <input type="range" min="1" max="8" step="1" bind:value={settings.continentCount} />
        <span>{settings.continentCount}</span>
      </label>
      <label>
        Coastline
        <select bind:value={settings.coastlineRoughness}>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
      </label>
      <label>
        Islands
        <select bind:value={settings.islandFrequency}>
          <option value="none">None</option>
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
      </label>
      <button type="submit" class="primary" disabled={busy}>{busy ? "Generating…" : "Regenerate"}</button>
    </form>
    <div class="candidates" role="radiogroup" aria-label="Landmass candidates" aria-busy={busy}>
      {#each candidates as candidate (candidate.index)}
        <label class="card" class:selected={selected === candidate.index}>
          <input type="radio" name="vector-candidate" value={candidate.index} bind:group={selected} />
          <span class="visually-hidden">Candidate {candidate.index + 1}, seed {candidate.seed}</span>
          {@html candidate.svg}
          <em>Candidate {candidate.index + 1}</em>
        </label>
      {/each}
      {#if busy && !candidates.length}
        <p>Generating six candidates…</p>
      {/if}
    </div>
  </div>
  {#if message}
    <p class="status" role="status">{message}</p>
  {/if}
</section>

<style>
.generator {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  background: #17211d;
  color: #edf2ec;
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid #405047;
  background: #202c27;
}
header span {
  font-size: 10px;
  letter-spacing: 0.12em;
  color: #b8c8bc;
}
.header-actions,
.seed-row {
  display: flex;
  gap: 6px;
}
.body {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  min-height: 0;
  flex: 1;
}
.controls {
  display: grid;
  align-content: start;
  gap: 12px;
  padding: 14px;
  overflow: auto;
  border-right: 1px solid #405047;
  background: #202c27;
}
label,
fieldset {
  display: grid;
  gap: 6px;
  margin: 0;
  border: 0;
  padding: 0;
}
input,
select,
button {
  border: 0;
  border-radius: 7px;
  padding: 8px 10px;
  background: #31443b;
  color: inherit;
}
.primary {
  background: #d5ab6c;
  color: #17211d;
}
.quiet {
  background: transparent;
  border: 1px solid #4b5a51;
}
.candidates {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
  padding: 16px;
  overflow: auto;
}
.card {
  position: relative;
  display: grid;
  gap: 8px;
  padding: 10px;
  border: 1px solid #4b5a51;
  border-radius: 8px;
  background: #111a16;
}
.card.selected {
  border-color: #d5ab6c;
}
.card :global(svg) {
  width: 100%;
  height: auto;
  background: #0b3d5c;
  border-radius: 4px;
}
.card input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}
.card em {
  font-style: normal;
  font-size: 12px;
  color: #b8c8bc;
}
.status {
  margin: 0;
  padding: 10px 16px;
  color: #f5a49c;
  border-top: 1px solid #405047;
}
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
}
button:focus-visible,
input:focus-visible,
select:focus-visible,
.card:focus-within {
  outline: 2px solid #f3d39a;
  outline-offset: 2px;
}
@media (prefers-reduced-motion: reduce) {
  .generator,
  .generator * {
    transition: none !important;
    animation: none !important;
  }
}
</style>
