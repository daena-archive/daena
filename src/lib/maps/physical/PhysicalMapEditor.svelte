<script lang="ts">
import { onMount, tick } from "svelte";
import { Mountain } from "@lucide/svelte";
import WorkspaceTopbar from "$lib/layout/WorkspaceTopbar.svelte";
import {
  project,
  type Entity,
  type PhysicalGenerationInput,
  type PhysicalClimateProducts,
  type PhysicalHydrologyProducts,
  type PhysicalJobStatus,
} from "$lib/project/client";
import { paintPhysicalSurface, type ClimateOverlayMode, type PhysicalRasterPaintOptions } from "./raster";
import {
  EARTH_RADIUS_METRES,
  earthLikePlanetary,
  insolationPpm,
  markPlanetaryCustom,
  orbitalPeriodSeconds,
  planetaryFromPreset,
  surfaceGravityMilliG,
  validatePlanetary,
  withOrbitalPeriodSeconds,
  type PlanetaryConfiguration,
  type PlanetaryPreset,
} from "./planetary";
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
let planetary = $state<PlanetaryConfiguration>(earthLikePlanetary());
let advancedPlanet = $state(false);
let yearLengthError = $state<string | null>(null);
let status = $state<PhysicalJobStatus | null>(null);
let hydrology = $state<PhysicalHydrologyProducts | null>(null);
let climate = $state<PhysicalClimateProducts | null>(null);
let climateOverlay = $state<ClimateOverlayMode>("annual");
let raster = $state<HTMLCanvasElement | null>(null);
let notice = $state("");
let busy = $state(false);
let activeJobId: string | null = null;
let preview = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let helpOpen = $state(false);
let helpSeen = $state(false);
let layers = $state<VectorLayerDefinition[]>([
  {
    id: BASE_LAYER_ID,
    kind: "vector",
    name: "Physical base",
    order: 0,
    defaultVisible: false,
    locked: true,
    opacity: 1,
    blendMode: "normal",
    selector: {},
    style: DEFAULT_VECTOR_LAYER_STYLE,
  },
  {
    id: "ice",
    kind: "vector",
    name: "Ice",
    order: 1,
    defaultVisible: false,
    locked: true,
    opacity: 1,
    blendMode: "normal",
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
    opacity: 1,
    blendMode: "normal",
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
    opacity: 1,
    blendMode: "normal",
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
    opacity: 1,
    blendMode: "normal",
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
    opacity: 1,
    blendMode: "normal",
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
    opacity: 1,
    blendMode: "normal",
    selector: {},
    style: { fill: "#e0bb78", fillOpacity: 0.18, stroke: "#f0d39b", strokeWidth: 0.7, pointRadius: 2 },
  },
]);

function publish(nextStatus: string, detail: unknown = null) {
  onstate?.(nextStatus, detail);
}

function physicalRasterPaintOptions(): PhysicalRasterPaintOptions {
  return {
    iceVisible: layers.find((layer) => layer.id === "ice")?.defaultVisible ?? false,
    lakesVisible: layers.find((layer) => layer.id === "lakes")?.defaultVisible ?? false,
    climateOverlay: climate ? climateOverlay : "off",
    climateAnnualCentiC: climate?.temperatureCentiC,
    climateNhSummerCentiC: climate?.temperatureNhSummerCentiC,
    climateNhWinterCentiC: climate?.temperatureNhWinterCentiC,
  };
}

function formatCentiC(value: number) {
  return `${(value / 100).toFixed(1)} °C`;
}

function climateSummary() {
  if (!climate) return "";
  const metrics = climate.metrics;
  const freeze =
    metrics.permanentlyFrozenLandPpm > 50_000
      ? "Permanent freeze is plausible on a large share of land."
      : metrics.seasonallyFrozenLandPpm > 50_000
        ? "Seasonal freeze is plausible; summers thaw most of that land."
        : "Little or no land stays below freezing across the year.";
  return `Warmest ${formatCentiC(metrics.maximumSeasonalTemperatureCentiC)}, coldest ${formatCentiC(metrics.minimumSeasonalTemperatureCentiC)}, typical annual range ${formatCentiC(metrics.meanSeasonalRangeCentiC)}. High land stays colder than its latitude. Northern-summer solstice is the warmer-orbit season. ${freeze}`;
}

function headline() {
  if (status?.state === "completed") return "Preview ready — accept this world";
  return "Generate a globe, then accept it";
}

function randomSeed() {
  seed = nextPhysicalSeed(seed);
}

function applyPlanetPreset(preset: PlanetaryPreset) {
  yearLengthError = null;
  planetary = planetaryFromPreset(preset);
}

function setPlanetary(patch: Partial<PlanetaryConfiguration>) {
  yearLengthError = null;
  planetary = markPlanetaryCustom({ ...planetary, ...patch });
}

function numberFromEvent(event: Event) {
  if (!(event.currentTarget instanceof HTMLInputElement)) return null;
  const value = Number(event.currentTarget.value);
  return Number.isFinite(value) ? value : null;
}

function onPlanetPresetChange(event: Event) {
  const value = event.currentTarget instanceof HTMLSelectElement ? event.currentTarget.value : "earth-like";
  applyPlanetPreset(value as PlanetaryPreset);
}

const planetError = $derived(validatePlanetary(planetary));
const yearSeconds = $derived(orbitalPeriodSeconds(planetary));
const yearDays = $derived(yearSeconds == null ? null : yearSeconds / 86_400);
const sunlight = $derived(insolationPpm(planetary));
const gravityG = $derived(surfaceGravityMilliG(planetary));
const localDaysInYear = $derived(
  yearSeconds == null || planetary.rotationPeriodSeconds <= 0 ? null : yearSeconds / planetary.rotationPeriodSeconds,
);

function setYearLengthDays(days: number | null) {
  if (days == null || days <= 0) {
    yearLengthError = "Year length must be a positive number of Earth days.";
    return;
  }
  const next = withOrbitalPeriodSeconds(planetary, days * 86_400);
  if (next) {
    yearLengthError = null;
    planetary = next;
    return;
  }
  yearLengthError = "That year length is outside the supported orbital range for this star mass.";
}

function patchPlanetaryNumber(event: Event, map: (value: number) => Partial<PlanetaryConfiguration>) {
  const value = numberFromEvent(event);
  if (value == null) return;
  setPlanetary(map(value));
}

function destroyPreview() {
  raster = null;
}

function rebuildRaster(products: PhysicalHydrologyProducts | null) {
  raster = products ? paintPhysicalSurface(products, physicalRasterPaintOptions()) : null;
}

function toggleHelp() {
  helpOpen = !helpOpen;
  if (helpOpen) helpSeen = true;
}

function closeHelp() {
  helpOpen = false;
}

$effect(() => {
  if (!helpOpen) return;
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") closeHelp();
  };
  window.addEventListener("keydown", onKey);
  return () => window.removeEventListener("keydown", onKey);
});

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
    climate = await project.physicalMapDerivedClimate(mapId);
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
      climate = await project.physicalMapClimate(jobId);
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
  const error = validatePlanetary(planetary);
  if (error) {
    notice = error;
    publish("error", { detail: notice });
    return;
  }
  busy = true;
  notice = "";
  hydrology = null;
  climate = null;
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
        radiusMetres: planetary.radiusMetres,
        targetLandFractionPpm: 300_000,
        planetary,
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
    <WorkspaceTopbar
      title="Physical world"
      subtitle={headline()}
      icon={Mountain}
      backLabel="Back to map details"
      onBack={() => void cancel()} />
    <div class="physical-map-controls">
      <label>Map name<input bind:value={name} disabled={busy} /></label>
      <label>Seed<input type="number" bind:value={seed} disabled={busy} min="0" max="4294967295" /></label>
      <label
        >Terrain age<select bind:value={evolutionPreset} disabled={busy}>
          <option value="young">Young</option>
          <option value="mature">Mature</option>
          <option value="old">Old</option>
        </select></label>
      <label
        >Planet<select value={planetary.preset} disabled={busy} onchange={onPlanetPresetChange}>
          <option value="earth-like">Earth-like</option>
          <option value="low-tilt">Mild seasons</option>
          <option value="high-tilt">Strong seasons</option>
          <option value="slow-rotating">Long days</option>
          <option value="close-orbit">Close orbit</option>
          <option value="custom">Custom</option>
        </select></label>
      <button class="quiet-button" type="button" onclick={randomSeed} disabled={busy}>Reroll seed</button>
      <button
        class="quiet-button"
        type="button"
        onclick={() => (advancedPlanet = !advancedPlanet)}
        disabled={busy}
        aria-expanded={advancedPlanet}>{advancedPlanet ? "Hide planet details" : "Planet details"}</button>
      {#if climate}
        <label
          >Climate view<select bind:value={climateOverlay} onchange={() => rebuildRaster(hydrology)}>
            <option value="off">Terrain only</option>
            <option value="annual">Annual temperature</option>
            <option value="nh-summer">Northern-summer solstice</option>
            <option value="nh-winter">Northern-winter solstice</option>
            <option value="freeze">Freeze</option>
          </select></label>
      {/if}
    </div>
    {#if climate}
      <p class="physical-planet-readout physical-climate-readout">{climateSummary()}</p>
    {/if}
    {#if advancedPlanet}
      <div class="physical-planet-panel">
        <p class="physical-planet-readout">
          Stored with the world. These settings now drive temperature and seasons. Figures are generated world physics,
          not a precise scientific prediction.
        </p>
        <label
          >Seasons (tilt)
          <input
            type="range"
            min="0"
            max="90"
            step="1"
            disabled={busy}
            value={Math.round(planetary.axialTiltMilliDeg / 1000)}
            oninput={(event) => patchPlanetaryNumber(event, (value) => ({ axialTiltMilliDeg: value * 1000 }))} />
          <span>{Math.round(planetary.axialTiltMilliDeg / 1000)}°</span>
        </label>
        <label
          >Hours in a day
          <input
            type="number"
            min="1"
            max="2160"
            step="1"
            disabled={busy}
            value={Math.round(planetary.rotationPeriodSeconds / 3600)}
            oninput={(event) =>
              patchPlanetaryNumber(event, (value) => ({ rotationPeriodSeconds: Math.max(1, value) * 3600 }))} />
        </label>
        <label
          >Year length (Earth days)
          <input
            type="number"
            min="4"
            max="200000"
            step="1"
            disabled={busy}
            value={yearDays == null ? "" : Math.round(yearDays)}
            oninput={(event) => setYearLengthDays(numberFromEvent(event))} />
        </label>
        <p class="physical-planet-readout">
          {#if yearLengthError}
            {yearLengthError}
          {:else if planetError}
            {planetError}
          {:else if yearDays != null && localDaysInYear != null && sunlight != null && gravityG != null}
            About {Math.round(yearDays)} Earth days / {Math.round(localDaysInYear)} local days · about {(
              sunlight / 1_000_000
            ).toFixed(1)}x sunlight · about {(gravityG / 1000).toFixed(1)} g. Year length sets orbital distance from the star's
            mass.
          {:else}
            Enter supported planetary values to see approximate year, sunlight, and gravity.
          {/if}
        </p>
        <details class="physical-planet-advanced">
          <summary>Advanced</summary>
          <div class="physical-planet-advanced-grid">
            <label
              >Star brightness
              <input
                type="number"
                min="0.01"
                max="100"
                step="0.01"
                disabled={busy}
                value={planetary.starLuminosityPpm / 1_000_000}
                oninput={(event) =>
                  patchPlanetaryNumber(event, (value) => ({ starLuminosityPpm: Math.round(value * 1_000_000) }))} />
            </label>
            <label
              >Distance (AU)
              <input
                type="number"
                min="0.05"
                max="50"
                step="0.01"
                disabled={busy}
                value={planetary.semiMajorAxisMilliAu / 1_000_000}
                oninput={(event) =>
                  patchPlanetaryNumber(event, (value) => ({ semiMajorAxisMilliAu: Math.round(value * 1_000_000) }))} />
            </label>
            <label
              >Star mass
              <input
                type="number"
                min="0.08"
                max="8"
                step="0.01"
                disabled={busy}
                value={planetary.starMassPpm / 1_000_000}
                oninput={(event) =>
                  patchPlanetaryNumber(event, (value) => ({ starMassPpm: Math.round(value * 1_000_000) }))} />
            </label>
            <label
              >Orbit stretch
              <input
                type="number"
                min="0"
                max="0.8"
                step="0.01"
                disabled={busy}
                value={planetary.eccentricityPpm / 1_000_000}
                oninput={(event) =>
                  patchPlanetaryNumber(event, (value) => ({ eccentricityPpm: Math.round(value * 1_000_000) }))} />
            </label>
            <label
              >Retained heat (°C)
              <input
                type="number"
                min="-50"
                max="50"
                step="1"
                disabled={busy}
                value={planetary.retainedHeatCentiC / 100}
                oninput={(event) =>
                  patchPlanetaryNumber(event, (value) => ({ retainedHeatCentiC: Math.round(value * 100) }))} />
            </label>
            <label
              >Reflectivity
              <input
                type="number"
                min="0.05"
                max="0.8"
                step="0.01"
                disabled={busy}
                value={planetary.bondAlbedoPpm / 1_000_000}
                oninput={(event) =>
                  patchPlanetaryNumber(event, (value) => ({ bondAlbedoPpm: Math.round(value * 1_000_000) }))} />
            </label>
            <label
              >Planet size (Earth radii)
              <input
                type="number"
                min="0.05"
                max="10"
                step="0.05"
                disabled={busy}
                value={planetary.radiusMetres / EARTH_RADIUS_METRES}
                oninput={(event) =>
                  patchPlanetaryNumber(event, (value) => ({
                    radiusMetres: Math.max(1, Math.round(value * EARTH_RADIUS_METRES)),
                  }))} />
            </label>
            <label
              >Density (kg/m³)
              <input
                type="number"
                min="1000"
                max="12000"
                step="10"
                disabled={busy}
                value={planetary.meanDensityKgM3}
                oninput={(event) => patchPlanetaryNumber(event, (value) => ({ meanDensityKgM3: value }))} />
            </label>
          </div>
        </details>
      </div>
    {/if}
    {#if notice}<p class="map-reconcile-notice" role="alert">{notice}</p>{/if}
    <div class="native-vector-map">
      <PhysicalWorldView collection={preview} {layers} {raster} showRaster />
      <div class="physical-map-help-anchor">
        {#if helpOpen}
          <button type="button" class="physical-map-help-backdrop" aria-label="Close help" onclick={closeHelp}></button>
        {/if}
        <button
          type="button"
          class="physical-map-help"
          class:unread={!helpSeen}
          aria-expanded={helpOpen}
          aria-controls="physical-map-help-panel"
          aria-label="About this preview"
          onclick={toggleHelp}>?</button>
        {#if helpOpen}
          <div
            id="physical-map-help-panel"
            class="physical-map-help-panel"
            role="dialog"
            aria-labelledby="physical-map-help-title"
            aria-modal="false">
            <strong id="physical-map-help-title">About this preview</strong>
            <p>
              This low-resolution view locks the world’s physical shape—coasts, elevation, climate, ice, and rivers. Pan
              and zoom to explore before you accept.
            </p>
            <p class="physical-map-help-note">
              The accepted map is a separate high-resolution render with much more detail. You can’t edit the base world
              directly; copy any region into an editable layer to change it. Planet settings drive temperature and
              seasons; results are generated world physics, not precise scientific prediction.
            </p>
            <button type="button" class="physical-map-help-dismiss" onclick={closeHelp}>Got it</button>
          </div>
        {/if}
      </div>
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

.physical-map-actions,
.physical-map-controls {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.9rem 1rem;
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

.physical-planet-panel {
  display: grid;
  gap: 0.65rem;
  padding: 0 1rem 0.85rem;
}

.physical-planet-panel label {
  display: grid;
  grid-template-columns: minmax(8rem, 12rem) minmax(8rem, 1fr) auto;
  align-items: center;
  gap: 0.65rem;
  font-size: 0.78rem;
}

.physical-planet-panel input[type="range"] {
  width: 100%;
}

.physical-planet-panel input[type="number"] {
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 0.35rem;
  background: rgb(255 255 255 / 7%);
  color: inherit;
  padding: 0.4rem 0.5rem;
}

.physical-planet-readout {
  margin: 0;
  color: #d9d0c3;
  font-size: 0.76rem;
}

.physical-climate-readout {
  padding: 0 1rem 0.75rem;
}

.physical-planet-advanced {
  border-top: 1px solid rgb(255 255 255 / 10%);
  padding-top: 0.55rem;
}

.physical-planet-advanced summary {
  cursor: pointer;
  font-size: 0.78rem;
  color: #d9d0c3;
}

.physical-planet-advanced-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  gap: 0.65rem;
  margin-top: 0.65rem;
}

.physical-planet-advanced-grid label {
  display: grid;
  grid-template-columns: 1fr;
  gap: 0.3rem;
}

.physical-map-editor .quiet-button,
.physical-map-editor .primary-button {
  cursor: pointer;
  font: inherit;
}

.physical-map-editor .quiet-button:focus-visible,
.physical-map-editor .primary-button:focus-visible {
  outline: 2px solid var(--theme-warning-border, #f3d39a);
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
  border: 1px solid var(--theme-warning-border, #d4b57a);
  border-radius: 0.45rem;
  background: #c9a96e;
  color: var(--brass-ink);
  font-weight: 700;
}

.physical-map-editor .primary-button:hover {
  background: #d8ba82;
}

.physical-map-editor .quiet-button:disabled,
.physical-map-editor .primary-button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.physical-map-editor .map-reconcile-notice {
  margin: 0;
  padding: 0.55rem 1rem;
}

.native-vector-map {
  position: relative;
  display: flex;
  min-height: 360px;
  min-width: 0;
  flex: 1;
}

.physical-map-help-anchor {
  position: absolute;
  top: 0.75rem;
  right: 0.75rem;
  z-index: 4;
}

.physical-map-help-backdrop {
  position: fixed;
  inset: 0;
  z-index: 3;
  border: 0;
  padding: 0;
  background: transparent;
  cursor: default;
}

.physical-map-help {
  position: relative;
  z-index: 5;
  display: grid;
  place-items: center;
  width: 1.75rem;
  height: 1.75rem;
  padding: 0;
  border: 1px solid rgb(255 255 255 / 22%);
  border-radius: 999px;
  background: rgb(13 27 42 / 72%);
  backdrop-filter: blur(6px);
  color: #f7f0e5;
  font: 700 0.78rem/1 inherit;
  cursor: pointer;
  box-shadow: 0 4px 14px rgb(0 0 0 / 28%);
}

.physical-map-help:hover,
.physical-map-help:focus-visible {
  border-color: rgb(255 255 255 / 34%);
  background: rgb(21 37 54 / 88%);
  color: #fff;
}

.physical-map-help:focus-visible {
  outline: 2px solid var(--theme-warning-border, #f3d39a);
  outline-offset: 2px;
}

.physical-map-help.unread::after {
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

.physical-map-help-panel {
  position: absolute;
  top: calc(100% + 0.5rem);
  right: 0;
  z-index: 5;
  display: grid;
  gap: 0.55rem;
  width: min(18.5rem, calc(100vw - 2.5rem));
  padding: 0.75rem 0.85rem;
  border: 1px solid rgb(255 255 255 / 16%);
  border-radius: 0.5rem;
  background: #152536;
  color: #f7f0e5;
  box-shadow: 0 10px 28px rgb(0 0 0 / 38%);
}

.physical-map-help-panel::before {
  content: "";
  position: absolute;
  top: -5px;
  right: 0.65rem;
  width: 10px;
  height: 10px;
  border-top: 1px solid rgb(255 255 255 / 16%);
  border-left: 1px solid rgb(255 255 255 / 16%);
  background: #152536;
  transform: rotate(45deg);
}

.physical-map-help-panel strong {
  font-size: 0.82rem;
  font-weight: 600;
}

.physical-map-help-panel p {
  margin: 0;
  font-size: 0.78rem;
  line-height: 1.45;
  color: #d9d0c3;
}

.physical-map-help-note {
  color: #c4b8a8 !important;
  font-size: 0.74rem !important;
}

.physical-map-help-dismiss {
  justify-self: start;
  margin-top: 0.15rem;
  padding: 0.35rem 0.65rem;
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 0.35rem;
  background: rgb(255 255 255 / 8%);
  color: #f7f0e5;
  font: inherit;
  font-size: 0.76rem;
  cursor: pointer;
}

.physical-map-help-dismiss:hover {
  border-color: rgb(255 255 255 / 28%);
  background: rgb(255 255 255 / 12%);
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
  color: var(--theme-warning-text, #c9a96e);
  font-size: 0.82rem;
  line-height: 1.5;
}

.physical-map-actions {
  justify-content: flex-end;
  border-top: 1px solid rgb(255 255 255 / 12%);
}
</style>
