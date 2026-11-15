<script lang="ts">
import { onMount, tick } from "svelte";
import { Mountain } from "@lucide/svelte";
import WorkspaceTopbar from "$lib/layout/WorkspaceTopbar.svelte";
import {
  project,
  type Entity,
  type PhysicalGenerationInput,
  type PhysicalClimateProducts,
  type PhysicalHazardProducts,
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
import type { MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import PhysicalWorldView from "./PhysicalWorldView.svelte";
import NativeVectorMapEditor from "../native-vector/NativeVectorMapEditor.svelte";
import { parseVectorCollection } from "../native-vector/source";
import { physicalViewIndex } from "../native-vector/coordinates";
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

function worldPresetHint(preset: string) {
  switch (preset) {
    case "earthlike":
      return "Temperate oceans and continents, close to Earth.";
    case "frozen":
      return "A cold world with wide ice sheets and tundra.";
    case "hothouse":
      return "A hot, humid world with warm poles.";
    case "desert":
      return "Dry continents with little rainfall.";
    case "archipelago":
      return "Scattered islands and shallow seas.";
    case "water-world":
      return "Mostly ocean with small island groups.";
    case "pangea":
      return "One broad supercontinent.";
    case "shattered":
      return "Many fragments separated by narrow seaways.";
    default:
      return "Pick the land fraction yourself.";
  }
}

const WORLD_PRESETS = [
  "earthlike",
  "frozen",
  "hothouse",
  "desert",
  "archipelago",
  "water-world",
  "pangea",
  "shattered",
  "custom",
] as const;

let name = $state("Physical World");
let seed = $state(nextPhysicalSeed());
let worldPreset = $state<string>("earthlike");
let resolution = $state<"512" | "1024">("512");
let landFractionPercent = $state(30);
let planetary = $state<PlanetaryConfiguration>(earthLikePlanetary());
let advancedPlanet = $state(false);
let yearLengthError = $state<string | null>(null);
let status = $state<PhysicalJobStatus | null>(null);
let hydrology = $state<PhysicalHydrologyProducts | null>(null);
let climate = $state<PhysicalClimateProducts | null>(null);
let hazards = $state<PhysicalHazardProducts | null>(null);
let climateOverlay = $state<ClimateOverlayMode>("annual");
let climateSample = $state<string>("");
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
    order: 13,
    defaultVisible: false,
    locked: true,
    opacity: 1,
    blendMode: "normal",
    selector: {},
    style: { fill: "#e8f2f8", fillOpacity: 0.82, stroke: "#c5d8e6", strokeWidth: 0.4, pointRadius: 2 },
  },
  {
    id: "winds",
    kind: "vector",
    name: "Winds",
    order: 14,
    defaultVisible: false,
    locked: true,
    opacity: 1,
    blendMode: "normal",
    selector: {},
    style: { fill: "#d69434", fillOpacity: 0.18, stroke: "#f2d9a8", strokeWidth: 0.7, pointRadius: 2 },
  },
  {
    id: "earthquake-hazard",
    kind: "vector",
    name: "Earthquake hazard",
    order: 16,
    defaultVisible: false,
    locked: true,
    opacity: 1,
    blendMode: "normal",
    selector: {},
    style: { fill: "#d86a5c", fillOpacity: 0.2, stroke: "#f0b0a6", strokeWidth: 0.6, pointRadius: 2 },
  },
  {
    id: "volcanic-hazard",
    kind: "vector",
    name: "Volcanic hazard",
    order: 17,
    defaultVisible: false,
    locked: true,
    opacity: 1,
    blendMode: "normal",
    selector: {},
    style: { fill: "#e09a4a", fillOpacity: 0.2, stroke: "#f3cf9a", strokeWidth: 0.6, pointRadius: 2 },
  },
  {
    id: "volcanic-centers",
    kind: "vector",
    name: "Volcanoes",
    order: 18,
    defaultVisible: false,
    locked: true,
    opacity: 1,
    blendMode: "normal",
    selector: {},
    style: { fill: "#f0a050", fillOpacity: 0.8, stroke: "#3a2418", strokeWidth: 0.7, pointRadius: 2.4 },
  },
  {
    id: "ocean",
    kind: "vector",
    name: "Ocean",
    order: 1,
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
    order: 9,
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
    order: 10,
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
    order: 12,
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
    windsVisible: Boolean(climate) && (layers.find((layer) => layer.id === "winds")?.defaultVisible ?? false),
    earthquakeHazardVisible:
      Boolean(hazards) && (layers.find((layer) => layer.id === "earthquake-hazard")?.defaultVisible ?? false),
    volcanicHazardVisible:
      Boolean(hazards) && (layers.find((layer) => layer.id === "volcanic-hazard")?.defaultVisible ?? false),
    volcanicSourcesVisible:
      Boolean(hazards) && (layers.find((layer) => layer.id === "volcanic-centers")?.defaultVisible ?? false),
    climateOverlay: climate ? climateOverlay : "off",
    climateAnnualCentiC: climate?.temperatureCentiC,
    climateNhSummerCentiC: climate?.temperatureNhSummerCentiC,
    climateNhWinterCentiC: climate?.temperatureNhWinterCentiC,
    climateWindEastMilli: climate?.windEastMilli,
    climateWindNorthMilli: climate?.windNorthMilli,
    climateWindEastNhSummerMilli: climate?.windEastNhSummerMilli,
    climateWindNorthNhSummerMilli: climate?.windNorthNhSummerMilli,
    climateWindEastNhWinterMilli: climate?.windEastNhWinterMilli,
    climateWindNorthNhWinterMilli: climate?.windNorthNhWinterMilli,
    climatePrecipitationMm: climate?.precipitationMmPerYear,
    climatePrecipitationNhSummerMm: climate?.precipitationNhSummerMm,
    climatePrecipitationNhWinterMm: climate?.precipitationNhWinterMm,
    climateAridityPpm: climate?.aridityPpm,
    climateBiomeClass: climate?.biomeClass,
    climateBiomeFill: climateBiomeFills(),
    climateStormSuitabilityPpm: climate?.stormSuitabilityPpm,
    climateStormTrackPpm: climate?.stormTrackPpm,
    earthquakeHazardPpm: hazards?.earthquakeHazardPpm,
    volcanicHazardPpm: hazards?.volcanicHazardPpm,
    volcanicSources: hazards?.volcanicSources,
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
  const itcz = `${(metrics.itczLatitudeMilliDeg / 1000).toFixed(1)}°`;
  const easterlies = Math.round(metrics.easterlyCellPpm / 10_000);
  const strength =
    metrics.meanWindSpeedMilli < 400 ? "light" : metrics.meanWindSpeedMilli < 1_200 ? "moderate" : "strong";
  const rain = `${metrics.meanPrecipitationMmPerYear.toLocaleString("en-US")} mm/year typical rainfall`;
  const dryness =
    metrics.meanLandAridityPpm < 200_000
      ? "humid land"
      : metrics.meanLandAridityPpm < 500_000
        ? "sub-humid land"
        : metrics.meanLandAridityPpm < 800_000
          ? "semi-arid land"
          : "arid land";
  const seasonRain =
    metrics.meanSeasonalPrecipitationRangeMm > 0
      ? ` Solstice rainfall typically differs by ${metrics.meanSeasonalPrecipitationRangeMm.toLocaleString("en-US")} mm.`
      : "";
  const biome = biomeLegendEntry(metrics.dominantLandBiome).name;
  const stormsPerYear = (metrics.expectedStormsPerYearMilli ?? 0) / 1_000;
  const storms =
    metrics.stormProneOceanPpm > 50_000
      ? ` Tropical-cyclone-like genesis covers about ${Math.round(metrics.stormProneOceanPpm / 10_000)}% of ocean, about ${stormsPerYear.toFixed(1)} storms per year.`
      : " Tropical-cyclone genesis is limited.";
  return `Warmest ${formatCentiC(metrics.maximumSeasonalTemperatureCentiC)}, coldest ${formatCentiC(metrics.minimumSeasonalTemperatureCentiC)}, typical annual range ${formatCentiC(metrics.meanSeasonalRangeCentiC)}. High land stays colder than its latitude. Northern-summer solstice is the warmer-orbit season. Prevailing ${strength} winds (not local weather): ITCZ near ${itcz}, easterlies over ${easterlies}% of the globe. ${rain}; ${dryness}.${seasonRain} Typical land biome is ${biome}.${storms} Rainfall follows winds, mountains, and warmer seas. Aridity is evaporative demand unmet by rain. Biomes are a derived reading of those conditions, not painted decoration. Storm fields are climatology, not a weather forecast. ${freeze}`;
}

function formatAridityLabel(ppm: number) {
  if (ppm < 200_000) return "humid";
  if (ppm < 500_000) return "sub-humid";
  if (ppm < 800_000) return "semi-arid";
  return "arid";
}

function climateStats(): Array<{ label: string; value: string }> {
  if (!climate) return [];
  const metrics = climate.metrics;
  const strength =
    metrics.meanWindSpeedMilli < 400 ? "light" : metrics.meanWindSpeedMilli < 1_200 ? "moderate" : "strong";
  const stormsPerYear = (metrics.expectedStormsPerYearMilli ?? 0) / 1_000;
  return [
    { label: "Warmest", value: formatCentiC(metrics.maximumSeasonalTemperatureCentiC) },
    { label: "Coldest", value: formatCentiC(metrics.minimumSeasonalTemperatureCentiC) },
    { label: "Range", value: formatCentiC(metrics.meanSeasonalRangeCentiC) },
    { label: "Rainfall", value: `${metrics.meanPrecipitationMmPerYear.toLocaleString("en-US")} mm/yr` },
    { label: "Land", value: formatAridityLabel(metrics.meanLandAridityPpm) },
    { label: "Winds", value: strength },
    { label: "Biome", value: biomeLegendEntry(metrics.dominantLandBiome).name },
    { label: "Storms", value: `${stormsPerYear.toFixed(1)}/yr` },
    {
      label: "Growing",
      value: `${(((metrics.meanLandGrowingSeasonPpm ?? 0) / 1_000_000) * 12).toFixed(1)} mo`,
    },
    {
      label: "Dry / wet",
      value: `${(((metrics.meanLandDrySeasonPpm ?? 0) / 1_000_000) * 12).toFixed(1)} / ${(((metrics.meanLandWetSeasonPpm ?? 0) / 1_000_000) * 12).toFixed(1)} mo`,
    },
  ];
}

function hydrologyStats(): Array<{ label: string; value: string }> {
  if (!hydrology) return [];
  const metrics = hydrology.metrics;
  return [
    { label: "Lakes", value: metrics.lakeCount.toLocaleString("en-US") },
    { label: "Rivers", value: metrics.riverCount.toLocaleString("en-US") },
    { label: "Watersheds", value: metrics.watershedCount.toLocaleString("en-US") },
    { label: "Coast", value: `${metrics.coastlineSegmentCount.toLocaleString("en-US")} seg` },
    { label: "Islands", value: metrics.islandCount.toLocaleString("en-US") },
  ];
}

function hazardStats(): Array<{ label: string; value: string }> {
  if (!hazards) return [];
  const metrics = hazards.metrics;
  return [
    { label: "Volcanoes", value: metrics.volcanicSourceCount.toLocaleString("en-US") },
    { label: "Active", value: metrics.nonDormantSourceCount.toLocaleString("en-US") },
    { label: "Quake peak", value: `${Math.round(metrics.earthquakeMaxPpm / 10_000)}%` },
    { label: "Eruption peak", value: `${Math.round(metrics.volcanicMaxPpm / 10_000)}%` },
  ];
}

function climateOverlayHint(): string | null {
  const hints: string[] = [];
  const windsOn =
    layers.find((layer) => layer.id === "winds")?.defaultVisible ||
    climateOverlay === "wind" ||
    climateOverlay === "wind-nh-summer" ||
    climateOverlay === "wind-nh-winter";
  if (windsOn) {
    hints.push(
      "Blue is easterly, amber is westerly. Arrows are prevailing direction on this coarse grid, not a weather forecast.",
    );
  }
  if (
    climateOverlay === "precipitation" ||
    climateOverlay === "precipitation-nh-summer" ||
    climateOverlay === "precipitation-nh-winter"
  ) {
    hints.push("Tan is dry, teal is wet. Rainfall follows winds, mountains, and warmer seas, not a weather forecast.");
  } else if (climateOverlay === "aridity") {
    hints.push("Aridity is evaporative demand unmet by rainfall. Green is humid, tan is dry.");
  } else if (climateOverlay === "biome") {
    hints.push(
      "Biomes are classified from temperature, rainfall, aridity, seasonality, and elevation. Permanent ice is climate freeze, not ice-sheet cover. Click a cell to see its class and sampled conditions.",
    );
  } else if (climateOverlay === "storm") {
    hints.push(
      "Formation zones only: warm tropical ocean with enough rotation and fetch from land. Seasonal wind-vector shear can suppress genesis. Amber is potential. Climatology, not a forecast.",
    );
  } else if (climateOverlay === "storm-track") {
    hints.push(
      "Track corridors only: prevailing winds with a poleward drift, then decay inland. Not a specific storm.",
    );
  }
  if (layers.find((layer) => layer.id === "earthquake-hazard")?.defaultVisible) {
    hints.push("Earthquake hazard is the long-run M ≥ 4 potential on this view. Redder is higher.");
  }
  if (layers.find((layer) => layer.id === "volcanic-hazard")?.defaultVisible) {
    hints.push("Volcanic hazard is the long-run eruption potential. Amber is higher.");
  }
  if (layers.find((layer) => layer.id === "volcanic-centers")?.defaultVisible) {
    hints.push("Triangles mark engine volcanic sources. Colour shows activity class.");
  }
  return hints.length ? hints.join(" ") : null;
}

function biomeLegendEntry(biomeClass: number) {
  return (
    climate?.biomeLegend?.find((entry) => entry.id === biomeClass) ?? {
      id: biomeClass,
      name: "unclassified",
      reason: "",
      fill: "#78787c",
    }
  );
}

function biomeFill(entry: { fill: string }): [number, number, number] {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(entry.fill);
  if (!match) return [120, 120, 124];
  return [parseInt(match[1], 16), parseInt(match[2], 16), parseInt(match[3], 16)];
}

function climateBiomeFills() {
  const fills: [number, number, number][] = [];
  for (const entry of climate?.biomeLegend ?? []) {
    fills[entry.id] = biomeFill(entry);
  }
  return fills;
}

function inspectClimate(anchor: MapAnchor) {
  if (!climate) {
    climateSample = "";
    return;
  }
  const point =
    anchor.kind === "point" ? anchor.point : anchor.kind === "provider-feature" ? anchor.fallbackPoint : null;
  if (!point) return;
  const index = physicalViewIndex(point[0], point[1], climate.width, climate.height);
  const rain = climate.precipitationMmPerYear[index] ?? 0;
  const annualT = climate.temperatureCentiC[index] ?? 0;
  const aridityPpm = climate.aridityPpm[index] ?? 0;
  const aridity = formatAridityLabel(aridityPpm);
  const biomeClass = climate.biomeClass[index] ?? 99;
  const entry = biomeLegendEntry(biomeClass);
  const heightM = Math.round(
    ((hydrology?.waterLevelMm[index] ?? hydrology?.seaLevelMm ?? 0) - (hydrology?.seaLevelMm ?? 0)) / 1_000,
  );
  const iceCover = hydrology?.iceCells?.[index] ?? false;
  const iceNote = iceCover ? " Ice cover is present." : "";
  const storm = Math.round((climate.stormSuitabilityPpm[index] ?? 0) / 10_000);
  const track = Math.round((climate.stormTrackPpm[index] ?? 0) / 10_000);
  const intensity = Math.round((climate.stormIntensityPpm[index] ?? 0) / 10_000);
  const quake = Math.round((hazards?.earthquakeHazardPpm[index] ?? 0) / 10_000);
  const volcano = Math.round((hazards?.volcanicHazardPpm[index] ?? 0) / 10_000);
  climateSample = `${entry.name}. Annual temperature ${formatCentiC(annualT)}, rainfall ${rain.toLocaleString("en-US")} mm/year, ${aridity} (${Math.round(aridityPpm / 10_000)}% aridity). ${heightM} m above sea.${iceNote} Storm genesis ${storm}%, track ${track}%, intensity ${intensity}%. Earthquake hazard ${quake}%, volcanic ${volcano}%.`;
}

function headline() {
  if (status?.state === "completed") return "World ready — accept to save it";
  return "Generate a world, then accept it";
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
    hazards = await project.physicalMapDerivedHazards(mapId);
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
      hazards = await project.physicalMapHazards(jobId);
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
  hazards = null;
  preview = { type: "FeatureCollection", features: [] };
  destroyPreview();
  try {
    const input: PhysicalGenerationInput = {
      seed,
      settings: {
        resolution: resolution === "1024" ? 1024 : 512,
        radiusMetres: planetary.radiusMetres,
        targetLandFractionPpm:
          worldPreset === "custom" ? Math.round(Math.min(95, Math.max(5, landFractionPercent)) * 10_000) : 0,
        preset: worldPreset,
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
    <div class="physical-layout">
      <aside class="physical-sidebar" aria-label="World setup">
        <section class="physical-section" aria-labelledby="physical-world-heading">
          <h2 id="physical-world-heading">World</h2>
          <label class="physical-field"
            >Map name<input bind:value={name} disabled={busy} maxlength="120" placeholder="Physical World" /></label>
          <div class="physical-seed-row">
            <label class="physical-field physical-seed-field"
              >Seed<input type="number" bind:value={seed} disabled={busy} min="0" max="4294967295" /></label>
            <button
              class="quiet-button physical-seed-reroll"
              type="button"
              onclick={randomSeed}
              disabled={busy}
              title="Pick a new random seed">Reroll seed</button>
          </div>
          <label class="physical-field"
            >World preset<select bind:value={worldPreset} disabled={busy}>
              {#each WORLD_PRESETS as preset}
                <option value={preset}
                  >{preset === "custom"
                    ? "Custom land fraction"
                    : preset
                        .split("-")
                        .map((part) => part[0].toUpperCase() + part.slice(1))
                        .join(" ")}</option>
              {/each}
            </select></label>
          <p class="physical-hint">{worldPresetHint(worldPreset)}</p>
          <label class="physical-field"
            >Resolution<select bind:value={resolution} disabled={busy}>
              <option value="512">512 (standard)</option>
              <option value="1024">1024 (detailed)</option>
            </select></label>
          {#if worldPreset === "custom"}
            <label class="physical-field"
              >Land fraction<input
                type="number"
                bind:value={landFractionPercent}
                disabled={busy}
                min="5"
                max="95"
                step="1" /></label>
            <p class="physical-hint">
              Percent of the surface above sea level. Named presets use their own land fraction.
            </p>
          {/if}
          <p class="physical-hint">
            The generated world is the world you accept: coasts, elevation, climate, ice, rivers, and hazards are final.
            Generation takes about 2 seconds at 512 and about 4 seconds at 1024.
          </p>
        </section>
        <section class="physical-section" aria-labelledby="physical-planet-heading">
          <h2 id="physical-planet-heading">Planet</h2>
          <label class="physical-field"
            >Planet<select value={planetary.preset} disabled={busy} onchange={onPlanetPresetChange}>
              <option value="earth-like">Earth-like</option>
              <option value="low-tilt">Mild seasons</option>
              <option value="high-tilt">Strong seasons</option>
              <option value="slow-rotating">Long days</option>
              <option value="close-orbit">Close orbit</option>
              <option value="custom">Custom</option>
            </select></label>
          <p class="physical-hint">Presets cover most worlds. Use Planet details for custom tilt, days, and orbit.</p>
        </section>
        <section class="physical-section" aria-labelledby="physical-climate-heading">
          <h2 id="physical-climate-heading">Climate view</h2>
          {#if climate}
            <label class="physical-field"
              >Climate view<select bind:value={climateOverlay} onchange={() => rebuildRaster(hydrology)}>
                <option value="off">Terrain only</option>
                <option value="annual">Annual temperature</option>
                <option value="nh-summer">Northern-summer solstice</option>
                <option value="nh-winter">Northern-winter solstice</option>
                <option value="freeze">Freeze</option>
                <option value="wind">Prevailing wind</option>
                <option value="wind-nh-summer">Northern-summer wind</option>
                <option value="wind-nh-winter">Northern-winter wind</option>
                <option value="precipitation">Annual rainfall</option>
                <option value="precipitation-nh-summer">Northern-summer rainfall</option>
                <option value="precipitation-nh-winter">Northern-winter rainfall</option>
                <option value="aridity">Aridity</option>
                <option value="biome">Biome</option>
                <option value="storm">Storm genesis</option>
                <option value="storm-track">Storm tracks</option>
              </select></label>
            <div class="physical-check-row">
              <label class="physical-check"
                ><input
                  type="checkbox"
                  checked={layers.find((layer) => layer.id === "winds")?.defaultVisible ?? false}
                  disabled={busy}
                  onchange={(event) => {
                    const next = event.currentTarget.checked;
                    layers = layers.map((layer) => (layer.id === "winds" ? { ...layer, defaultVisible: next } : layer));
                    rebuildRaster(hydrology);
                  }} />
                Winds</label>
              {#if hazards}
                <label class="physical-check"
                  ><input
                    type="checkbox"
                    checked={layers.find((layer) => layer.id === "earthquake-hazard")?.defaultVisible ?? false}
                    disabled={busy}
                    onchange={(event) => {
                      const next = event.currentTarget.checked;
                      layers = layers.map((layer) =>
                        layer.id === "earthquake-hazard" ? { ...layer, defaultVisible: next } : layer,
                      );
                      rebuildRaster(hydrology);
                    }} />
                  Earthquakes</label>
                <label class="physical-check"
                  ><input
                    type="checkbox"
                    checked={layers.find((layer) => layer.id === "volcanic-hazard")?.defaultVisible ?? false}
                    disabled={busy}
                    onchange={(event) => {
                      const next = event.currentTarget.checked;
                      layers = layers.map((layer) =>
                        layer.id === "volcanic-hazard" ? { ...layer, defaultVisible: next } : layer,
                      );
                      rebuildRaster(hydrology);
                    }} />
                  Volcanic hazard</label>
                <label class="physical-check"
                  ><input
                    type="checkbox"
                    checked={layers.find((layer) => layer.id === "volcanic-centers")?.defaultVisible ?? false}
                    disabled={busy}
                    onchange={(event) => {
                      const next = event.currentTarget.checked;
                      layers = layers.map((layer) =>
                        layer.id === "volcanic-centers" ? { ...layer, defaultVisible: next } : layer,
                      );
                      rebuildRaster(hydrology);
                    }} />
                  Volcanoes</label>
              {/if}
            </div>
            {#if climateStats().length > 0}
              <dl class="physical-stat-grid">
                {#each climateStats() as stat}
                  <div class="physical-stat">
                    <dt>{stat.label}</dt>
                    <dd>{stat.value}</dd>
                  </div>
                {/each}
              </dl>
            {/if}
            {#if hydrologyStats().length > 0}
              <dl class="physical-stat-grid">
                {#each hydrologyStats() as stat}
                  <div class="physical-stat">
                    <dt>{stat.label}</dt>
                    <dd>{stat.value}</dd>
                  </div>
                {/each}
              </dl>
            {/if}
            {#if hazardStats().length > 0}
              <dl class="physical-stat-grid">
                {#each hazardStats() as stat}
                  <div class="physical-stat">
                    <dt>{stat.label}</dt>
                    <dd>{stat.value}</dd>
                  </div>
                {/each}
              </dl>
            {/if}
            {#if climateOverlayHint()}
              <p class="physical-hint">{climateOverlayHint()}</p>
            {/if}
            {#if climateOverlay === "biome" && climate.biomeLegend}
              <p class="physical-planet-readout physical-biome-legend">
                {#each climate.biomeLegend as entry}
                  <span
                    ><span class="physical-biome-swatch" style={`background: ${entry.fill}`}></span>{entry.name}</span>
                {/each}
              </p>
            {/if}
            <details class="physical-details">
              <summary>Full climate summary</summary>
              <p class="physical-planet-readout">{climateSummary()}</p>
            </details>
          {:else}
            <p class="physical-hint">Generate a world to unlock temperature, rainfall, biome, and storm views.</p>
          {/if}
        </section>
        <section class="physical-section" aria-labelledby="physical-planet-details-heading">
          <h2 id="physical-planet-details-heading">Planet details</h2>
          <button
            class="quiet-button physical-details-toggle"
            type="button"
            onclick={() => (advancedPlanet = !advancedPlanet)}
            disabled={busy}
            aria-expanded={advancedPlanet}>{advancedPlanet ? "Hide planet details" : "Planet details"}</button>
          {#if advancedPlanet}
            <div class="physical-planet-panel">
              <p class="physical-planet-readout">
                Stored with the world. These settings now drive temperature, seasons, and prevailing winds. Figures are
                generated world physics, not a precise scientific prediction.
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
                  ).toFixed(1)}x sunlight · about {(gravityG / 1000).toFixed(1)} g. Year length sets orbital distance from
                  the star's mass.
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
                        patchPlanetaryNumber(event, (value) => ({
                          starLuminosityPpm: Math.round(value * 1_000_000),
                        }))} />
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
                        patchPlanetaryNumber(event, (value) => ({
                          semiMajorAxisMilliAu: Math.round(value * 1_000_000),
                        }))} />
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
        </section>
      </aside>
      <div class="physical-main">
        <div class="physical-statusbar" role="status">
          {#if busy}
            <span class="physical-status-progress"
              >{status?.stage ?? "Starting"}…{#if status && status.total > 0}
                {status.completed} / {status.total}{/if}</span>
          {:else if status?.state === "completed"}
            <span class="physical-status-progress"
              >Seed {seed} · {worldPreset} · {resolution} · {preview.features.length} features</span>
          {:else}
            <span class="physical-status-progress">Choose a seed and planet, then generate.</span>
          {/if}
          <div class="physical-map-help-anchor">
            {#if helpOpen}
              <button type="button" class="physical-map-help-backdrop" aria-label="Close help" onclick={closeHelp}
              ></button>
            {/if}
            <button
              type="button"
              class="physical-map-help"
              class:unread={!helpSeen}
              aria-expanded={helpOpen}
              aria-controls="physical-map-help-panel"
              aria-label="About this world"
              onclick={toggleHelp}>?</button>
            {#if helpOpen}
              <div
                id="physical-map-help-panel"
                class="physical-map-help-panel"
                role="dialog"
                aria-labelledby="physical-map-help-title"
                aria-modal="false">
                <strong id="physical-map-help-title">About this world</strong>
                <p>
                  This view is the generated world at the chosen resolution. Accepting saves these coasts, elevation,
                  climate, ice, rivers, and hazards as the map’s physical shape. Pan and zoom to explore before you
                  accept.
                </p>
                <p class="physical-map-help-note">
                  You can’t edit the base world directly; copy any region into an editable layer to change it. Planet
                  settings drive temperature and seasons; results are generated world physics, not precise scientific
                  prediction.
                </p>
                <button type="button" class="physical-map-help-dismiss" onclick={closeHelp}>Got it</button>
              </div>
            {/if}
          </div>
        </div>
        {#if notice}<p class="map-reconcile-notice" role="alert">{notice}</p>{/if}
        {#if climate}
          <p class="physical-inspector">
            {climateSample || "Click the map to inspect temperature, rainfall, aridity, biome, and hazards at a cell."}
          </p>
        {/if}
        <div class="native-vector-map">
          <PhysicalWorldView
            collection={preview}
            {layers}
            {raster}
            showRaster
            pickArmed={Boolean(climate)}
            onMapPick={inspectClimate} />
          {#if busy}
            <div class="physical-map-stage" role="status">
              <strong>{status?.stage ?? "Starting"}…</strong>
              {#if status && status.total > 0}<span>{status.completed} / {status.total}</span>{/if}
              <span class="physical-stage-hint">You can cancel; nothing is saved until you accept.</span>
            </div>
          {:else if preview.features.length === 0 && !raster}
            <div class="physical-map-empty-hint">
              <p>
                No world yet. Set the seed, preset, and planet on the left, then click
                <strong>Generate world</strong> below.
              </p>
            </div>
          {/if}
        </div>
      </div>
    </div>
    <footer class="physical-map-actions">
      <span class="physical-actions-status" role="status">
        {#if status?.state === "completed"}World ready — review, reroll, or accept.{:else if busy}Generating…{:else}Not
          generated yet.{/if}
      </span>
      {#if status?.state === "completed"}
        <button
          class="primary-button"
          type="button"
          onclick={() => void accept()}
          disabled={busy}
          title="Save this world as a new map">Accept world</button>
        <button
          class="quiet-button"
          type="button"
          onclick={() => void generate()}
          disabled={busy}
          title="Generate another world with the current settings">Reroll</button>
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

.physical-map-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.9rem 1rem;
}

.physical-layout {
  display: grid;
  grid-template-columns: 21rem minmax(0, 1fr);
  min-height: 0;
  flex: 1;
}

.physical-sidebar {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  min-height: 0;
  overflow-y: auto;
  padding: 0.9rem 1rem 1.1rem;
  border-right: 1px solid rgb(255 255 255 / 12%);
  background: rgb(255 255 255 / 2%);
}

.physical-section {
  display: grid;
  gap: 0.6rem;
  padding: 0.75rem 0.8rem;
  border: 1px solid rgb(255 255 255 / 12%);
  border-radius: 0.6rem;
  background: rgb(255 255 255 / 3%);
}

.physical-section h2 {
  margin: 0;
  color: #f7f0e5;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.physical-field {
  display: grid;
  gap: 0.3rem;
  font-size: 0.78rem;
}

.physical-field input,
.physical-field select {
  border: 1px solid rgb(255 255 255 / 18%);
  border-radius: 0.35rem;
  background: rgb(255 255 255 / 7%);
  color: inherit;
  padding: 0.45rem 0.55rem;
  font: inherit;
  min-width: 0;
  width: 100%;
}

.physical-seed-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.5rem;
  align-items: end;
}

.physical-seed-field {
  min-width: 0;
}

.physical-seed-reroll {
  white-space: nowrap;
}

.physical-details-toggle {
  justify-self: start;
}

.physical-check-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 1rem;
}

.physical-check {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.78rem;
}

.physical-check input {
  width: 1rem;
  height: 1rem;
  accent-color: #c9a96e;
}

.physical-hint {
  margin: 0;
  color: #c4b8a8;
  font-size: 0.74rem;
  line-height: 1.5;
}

.physical-stat-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.45rem;
  margin: 0;
  padding: 0;
}

.physical-stat {
  display: grid;
  gap: 0.15rem;
  padding: 0.45rem 0.5rem;
  border: 1px solid rgb(255 255 255 / 10%);
  border-radius: 0.45rem;
  background: rgb(0 0 0 / 18%);
}

.physical-stat dt {
  color: #c4b8a8;
  font-size: 0.66rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.physical-stat dd {
  margin: 0;
  color: #f7f0e5;
  font-size: 0.76rem;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.physical-details {
  border-top: 1px solid rgb(255 255 255 / 10%);
  padding-top: 0.5rem;
}

.physical-details summary {
  cursor: pointer;
  font-size: 0.78rem;
  color: #d9d0c3;
}

.physical-details p {
  margin: 0.5rem 0 0;
}

.physical-main {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

.physical-statusbar {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-wrap: wrap;
  padding: 0.65rem 1rem;
  border-bottom: 1px solid rgb(255 255 255 / 10%);
  background: rgb(0 0 0 / 18%);
  font-size: 0.76rem;
}

.physical-status-progress {
  color: #d9d0c3;
}

.physical-inspector {
  margin: 0;
  padding: 0.55rem 1rem;
  border-bottom: 1px solid rgb(255 255 255 / 8%);
  color: #d9d0c3;
  font-size: 0.76rem;
  line-height: 1.5;
}

.physical-planet-panel {
  display: grid;
  gap: 0.65rem;
  padding-top: 0.25rem;
}

.physical-planet-panel label {
  display: grid;
  grid-template-columns: minmax(7rem, 10rem) minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.6rem;
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
  width: 100%;
  min-width: 0;
}

.physical-planet-readout {
  margin: 0;
  color: #d9d0c3;
  font-size: 0.76rem;
  line-height: 1.5;
}

.physical-biome-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem 0.75rem;
  margin: 0;
}

.physical-biome-swatch {
  display: inline-block;
  width: 0.7rem;
  height: 0.7rem;
  margin-right: 0.3rem;
  vertical-align: -0.1rem;
  border: 1px solid rgb(255 255 255 / 20%);
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
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: 0.6rem;
  margin-top: 0.6rem;
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
  position: relative;
  z-index: 6;
  margin-left: auto;
  flex: none;
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

.physical-stage-hint {
  font-size: 0.74rem !important;
  color: #c4b8a8 !important;
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
  flex-wrap: wrap;
}

.physical-actions-status {
  margin-right: auto;
  color: #d9d0c3;
  font-size: 0.76rem;
}

@media (max-width: 960px) {
  .physical-layout {
    grid-template-columns: minmax(0, 1fr);
  }

  .physical-sidebar {
    border-right: 0;
    border-bottom: 1px solid rgb(255 255 255 / 12%);
    max-height: none;
    overflow: visible;
  }

  .physical-stat-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .native-vector-map {
    min-height: 320px;
  }
}
</style>
