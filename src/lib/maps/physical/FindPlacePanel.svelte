<script lang="ts">
import { ChevronRight } from "@lucide/svelte";
import type { FindPlaceCandidate, FindPlaceQuery, FindPlaceResult } from "$lib/project/types";

const FALLBACK_BIOMES = [
  { id: 1, name: "permanent ice" },
  { id: 2, name: "tundra" },
  { id: 3, name: "alpine" },
  { id: 4, name: "cold grassland" },
  { id: 5, name: "temperate grassland" },
  { id: 6, name: "desert" },
  { id: 7, name: "shrubland" },
  { id: 8, name: "temperate forest" },
  { id: 9, name: "tropical forest" },
];

type Preset = {
  id: string;
  label: string;
  landOnly: boolean;
  values: Record<string, string>;
  soft: string[];
};

const PRESETS: Preset[] = [
  {
    id: "warm-wet-lowland",
    label: "Warm wet lowland",
    landOnly: true,
    values: {
      altitudeMin: "0",
      altitudeMax: "400",
      tempMin: "18",
      tempMax: "32",
      rainMin: "800",
      slopeMax: "8",
      coastMax: "80",
    },
    soft: ["coast"],
  },
  {
    id: "cold-valley",
    label: "Cold mountain valley",
    landOnly: true,
    values: {
      altitudeMin: "800",
      altitudeMax: "2800",
      tempMin: "-8",
      tempMax: "8",
      freshMax: "40",
      slopeMin: "2",
      slopeMax: "25",
    },
    soft: ["slope"],
  },
  {
    id: "dry-plateau",
    label: "Dry inland plateau",
    landOnly: true,
    values: { altitudeMin: "400", altitudeMax: "1800", rainMax: "400", aridityMin: "50", coastMin: "500" },
    soft: [],
  },
  {
    id: "safe-coast",
    label: "Low-hazard coast",
    landOnly: true,
    values: { coastMax: "30", quakeMax: "20", volcanoMax: "20", stormMax: "25" },
    soft: ["storm"],
  },
  {
    id: "storm-coast",
    label: "Tropical storm coast",
    landOnly: true,
    values: { coastMax: "40", tempMin: "20", stormMin: "20", biomeClass: "9" },
    soft: ["biome"],
  },
];

let {
  variant = "editor",
  disabled = false,
  searching = false,
  error = "",
  climateAvailable = true,
  hazardsAvailable = true,
  biomeLegend = [],
  islandOptions = [],
  result = null,
  selectedId = null,
  onsearch,
  onselect,
  onpin,
  onclear,
}: {
  variant?: "editor" | "studio";
  disabled?: boolean;
  searching?: boolean;
  error?: string;
  climateAvailable?: boolean;
  hazardsAvailable?: boolean;
  biomeLegend?: { id: number; name: string }[];
  islandOptions?: { id: number; cells: number }[];
  result?: FindPlaceResult | null;
  selectedId?: number | null;
  onsearch: (query: FindPlaceQuery) => void;
  onselect: (candidate: FindPlaceCandidate) => void;
  onpin: (candidate: FindPlaceCandidate) => void;
  onclear: () => void;
} = $props();

let simpleElevation = $state("");
let simpleTemperature = $state("");
let simpleRain = $state("");
let simpleWater = $state("");
let simpleHazard = $state("");
let landOnly = $state(true);
let altitudeMin = $state("");
let altitudeMax = $state("");
let latitudeMin = $state("");
let latitudeMax = $state("");
let slopeMin = $state("");
let slopeMax = $state("");
let tempMin = $state("");
let tempMax = $state("");
let rainMin = $state("");
let rainMax = $state("");
let humidityMin = $state("");
let humidityMax = $state("");
let aridityMin = $state("");
let aridityMax = $state("");
let coastMin = $state("");
let coastMax = $state("");
let freshMin = $state("");
let freshMax = $state("");
let biomeClass = $state("");
let islandId = $state("");
let ice = $state("");
let quakeMin = $state("");
let quakeMax = $state("");
let volcanoMin = $state("");
let volcanoMax = $state("");
let stormMin = $state("");
let stormMax = $state("");
let maxCandidates = $state("12");
let minCells = $state("4");
let altitudeSoft = $state(false);
let latitudeSoft = $state(false);
let slopeSoft = $state(false);
let tempSoft = $state(false);
let rainSoft = $state(false);
let humiditySoft = $state(false);
let ariditySoft = $state(false);
let coastSoft = $state(false);
let freshSoft = $state(false);
let biomeSoft = $state(false);
let islandSoft = $state(false);
let iceSoft = $state(false);
let quakeSoft = $state(false);
let volcanoSoft = $state(false);
let stormSoft = $state(false);
let formError = $state("");
let compareId = $state<number | null>(null);
let expandedId = $state<number | null>(null);

const biomes = $derived(biomeLegend.length > 0 ? biomeLegend : FALLBACK_BIOMES);
const climateOff = $derived(disabled || !climateAvailable);
const hazardsOff = $derived(disabled || !hazardsAvailable);

function parseOptional(value: string, label: string): number | undefined {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  const parsed = Number(trimmed);
  if (!Number.isFinite(parsed)) throw new Error(`${label} must be a number`);
  return parsed;
}

function optionalCenti(value: string, label: string): number | undefined {
  const parsed = parseOptional(value, label);
  return parsed === undefined ? undefined : Math.round(parsed * 100);
}

function optionalPercentPpm(value: string, label: string): number | undefined {
  const parsed = parseOptional(value, label);
  return parsed === undefined ? undefined : Math.round(parsed * 10_000);
}

function milliDeg(value: string, label: string): number | undefined {
  const parsed = parseOptional(value, label);
  return parsed === undefined ? undefined : Math.round(parsed * 1_000);
}

function collectSoft() {
  return (
    [
      ["altitude", altitudeSoft],
      ["latitude", latitudeSoft],
      ["slope", slopeSoft],
      ["temperature", tempSoft],
      ["precipitation", rainSoft],
      ["humidity", humiditySoft],
      ["aridity", ariditySoft],
      ["coast", coastSoft],
      ["freshwater", freshSoft],
      ["biome", biomeSoft],
      ["island", islandSoft],
      ["ice", iceSoft],
      ["earthquake", quakeSoft],
      ["volcanic", volcanoSoft],
      ["storm", stormSoft],
    ] as Array<[string, boolean]>
  )
    .filter(([, on]) => on)
    .map(([key]) => key);
}

function applyElevation(value: string) {
  simpleElevation = value;
  if (value === "lowland") {
    altitudeMin = "0";
    altitudeMax = "400";
  } else if (value === "upland") {
    altitudeMin = "400";
    altitudeMax = "1500";
  } else if (value === "highland") {
    altitudeMin = "1500";
    altitudeMax = "";
  } else if (value !== "custom") {
    altitudeMin = "";
    altitudeMax = "";
  }
}

function applyTemperature(value: string) {
  simpleTemperature = value;
  if (value === "hot") {
    tempMin = "18";
    tempMax = "";
  } else if (value === "mild") {
    tempMin = "8";
    tempMax = "20";
  } else if (value === "cold") {
    tempMin = "";
    tempMax = "8";
  } else if (value !== "custom") {
    tempMin = "";
    tempMax = "";
  }
}

function applyRain(value: string) {
  simpleRain = value;
  if (value === "wet") {
    rainMin = "800";
    rainMax = "";
  } else if (value === "moderate") {
    rainMin = "400";
    rainMax = "800";
  } else if (value === "dry") {
    rainMin = "";
    rainMax = "400";
  } else if (value !== "custom") {
    rainMin = "";
    rainMax = "";
  }
}

function applyWater(value: string) {
  simpleWater = value;
  if (value === "custom") return;
  coastMin = "";
  coastMax = "";
  freshMin = "";
  freshMax = "";
  if (value === "coastal") coastMax = "50";
  else if (value === "inland") coastMin = "200";
  else if (value === "freshwater") freshMax = "40";
}

function applyHazard(value: string) {
  simpleHazard = value;
  if (value === "custom") return;
  quakeMin = "";
  quakeMax = "";
  volcanoMin = "";
  volcanoMax = "";
  stormMin = "";
  stormMax = "";
  if (value === "low") {
    quakeMax = "20";
    volcanoMax = "20";
  } else if (value === "stormy") stormMin = "20";
}

function matchChoice(min: string, max: string, buckets: Record<string, [string, string]>) {
  for (const [id, [lo, hi]] of Object.entries(buckets)) {
    if (min === lo && max === hi) return id;
  }
  return min || max ? "custom" : "";
}

function syncSimpleFromNumbers() {
  simpleElevation = matchChoice(altitudeMin, altitudeMax, {
    lowland: ["0", "400"],
    upland: ["400", "1500"],
    highland: ["1500", ""],
  });
  simpleTemperature = matchChoice(tempMin, tempMax, {
    hot: ["18", ""],
    mild: ["8", "20"],
    cold: ["", "8"],
  });
  simpleRain = matchChoice(rainMin, rainMax, {
    wet: ["800", ""],
    moderate: ["400", "800"],
    dry: ["", "400"],
  });
  if (coastMax === "50" && !coastMin && !freshMax) simpleWater = "coastal";
  else if (coastMin === "200" && !coastMax && !freshMax) simpleWater = "inland";
  else if (freshMax === "40" && !coastMin && !coastMax) simpleWater = "freshwater";
  else simpleWater = coastMin || coastMax || freshMin || freshMax ? "custom" : "";
  if (quakeMax === "20" && volcanoMax === "20" && !stormMin) simpleHazard = "low";
  else if (stormMin === "20" && !quakeMax && !volcanoMax) simpleHazard = "stormy";
  else simpleHazard = quakeMin || quakeMax || volcanoMin || volcanoMax || stormMin || stormMax ? "custom" : "";
}

function resetFilters() {
  simpleElevation = "";
  simpleTemperature = "";
  simpleRain = "";
  simpleWater = "";
  simpleHazard = "";
  landOnly = true;
  altitudeMin = "";
  altitudeMax = "";
  latitudeMin = "";
  latitudeMax = "";
  slopeMin = "";
  slopeMax = "";
  tempMin = "";
  tempMax = "";
  rainMin = "";
  rainMax = "";
  humidityMin = "";
  humidityMax = "";
  aridityMin = "";
  aridityMax = "";
  coastMin = "";
  coastMax = "";
  freshMin = "";
  freshMax = "";
  biomeClass = "";
  islandId = "";
  ice = "";
  quakeMin = "";
  quakeMax = "";
  volcanoMin = "";
  volcanoMax = "";
  stormMin = "";
  stormMax = "";
  maxCandidates = "12";
  minCells = "4";
  altitudeSoft = false;
  latitudeSoft = false;
  slopeSoft = false;
  tempSoft = false;
  rainSoft = false;
  humiditySoft = false;
  ariditySoft = false;
  coastSoft = false;
  freshSoft = false;
  biomeSoft = false;
  islandSoft = false;
  iceSoft = false;
  quakeSoft = false;
  volcanoSoft = false;
  stormSoft = false;
  formError = "";
}

function applyPreset(preset: Preset) {
  resetFilters();
  landOnly = preset.landOnly;
  const assign: Record<string, (value: string) => void> = {
    altitudeMin: (value) => (altitudeMin = value),
    altitudeMax: (value) => (altitudeMax = value),
    tempMin: (value) => (tempMin = value),
    tempMax: (value) => (tempMax = value),
    rainMin: (value) => (rainMin = value),
    rainMax: (value) => (rainMax = value),
    slopeMin: (value) => (slopeMin = value),
    slopeMax: (value) => (slopeMax = value),
    coastMin: (value) => (coastMin = value),
    coastMax: (value) => (coastMax = value),
    freshMax: (value) => (freshMax = value),
    aridityMin: (value) => (aridityMin = value),
    quakeMax: (value) => (quakeMax = value),
    volcanoMax: (value) => (volcanoMax = value),
    stormMin: (value) => (stormMin = value),
    stormMax: (value) => (stormMax = value),
    biomeClass: (value) => (biomeClass = value),
  };
  for (const [key, value] of Object.entries(preset.values)) assign[key]?.(value);
  altitudeSoft = preset.soft.includes("altitude");
  latitudeSoft = preset.soft.includes("latitude");
  slopeSoft = preset.soft.includes("slope");
  tempSoft = preset.soft.includes("temperature");
  rainSoft = preset.soft.includes("precipitation");
  humiditySoft = preset.soft.includes("humidity");
  ariditySoft = preset.soft.includes("aridity");
  coastSoft = preset.soft.includes("coast");
  freshSoft = preset.soft.includes("freshwater");
  biomeSoft = preset.soft.includes("biome");
  islandSoft = preset.soft.includes("island");
  iceSoft = preset.soft.includes("ice");
  quakeSoft = preset.soft.includes("earthquake");
  volcanoSoft = preset.soft.includes("volcanic");
  stormSoft = preset.soft.includes("storm");
  syncSimpleFromNumbers();
}

function submit() {
  formError = "";
  try {
    const query: FindPlaceQuery = {
      landOnly,
      altitudeMMin: parseOptional(altitudeMin, "Altitude min"),
      altitudeMMax: parseOptional(altitudeMax, "Altitude max"),
      latitudeMilliDegMin: milliDeg(latitudeMin, "Latitude min"),
      latitudeMilliDegMax: milliDeg(latitudeMax, "Latitude max"),
      slopePpmMin: optionalPercentPpm(slopeMin, "Slope min"),
      slopePpmMax: optionalPercentPpm(slopeMax, "Slope max"),
      temperatureCentiCMin: climateAvailable ? optionalCenti(tempMin, "Temp min") : undefined,
      temperatureCentiCMax: climateAvailable ? optionalCenti(tempMax, "Temp max") : undefined,
      precipitationMmMin: climateAvailable ? parseOptional(rainMin, "Rain min") : undefined,
      precipitationMmMax: climateAvailable ? parseOptional(rainMax, "Rain max") : undefined,
      humidityPpmMin: climateAvailable ? optionalPercentPpm(humidityMin, "Humidity min") : undefined,
      humidityPpmMax: climateAvailable ? optionalPercentPpm(humidityMax, "Humidity max") : undefined,
      aridityPpmMin: climateAvailable ? optionalPercentPpm(aridityMin, "Aridity min") : undefined,
      aridityPpmMax: climateAvailable ? optionalPercentPpm(aridityMax, "Aridity max") : undefined,
      biomeClass: !climateAvailable || biomeClass === "" ? undefined : Number(biomeClass),
      islandId: parseOptional(islandId, "Landmass id"),
      ice: ice === "" ? undefined : ice === "yes",
      coastDistanceKmMin: parseOptional(coastMin, "Coast min"),
      coastDistanceKmMax: parseOptional(coastMax, "Coast max"),
      freshwaterDistanceKmMin: parseOptional(freshMin, "Fresh water min"),
      freshwaterDistanceKmMax: parseOptional(freshMax, "Fresh water max"),
      earthquakeHazardPpmMin: hazardsAvailable ? optionalPercentPpm(quakeMin, "Quake min") : undefined,
      earthquakeHazardPpmMax: hazardsAvailable ? optionalPercentPpm(quakeMax, "Quake max") : undefined,
      volcanicHazardPpmMin: hazardsAvailable ? optionalPercentPpm(volcanoMin, "Volcano min") : undefined,
      volcanicHazardPpmMax: hazardsAvailable ? optionalPercentPpm(volcanoMax, "Volcano max") : undefined,
      stormSuitabilityPpmMin: climateAvailable ? optionalPercentPpm(stormMin, "Storm min") : undefined,
      stormSuitabilityPpmMax: climateAvailable ? optionalPercentPpm(stormMax, "Storm max") : undefined,
      maxCandidates: parseOptional(maxCandidates, "Max regions") ?? 12,
      minCells: parseOptional(minCells, "Min cells") ?? 4,
      soft: collectSoft(),
    };
    onsearch(query);
  } catch (cause) {
    formError = cause instanceof Error ? cause.message : String(cause);
  }
}

function biomeName(id: number) {
  return biomes.find((entry) => entry.id === id)?.name ?? "unclassified";
}

function toggleCompare(id: number) {
  compareId = compareId === id ? null : id;
}

function toggleExpanded(id: number) {
  expandedId = expandedId === id ? null : id;
}

function clearResults() {
  compareId = null;
  expandedId = null;
  onclear();
}

const compared = $derived(
  result && compareId !== null && selectedId !== null && compareId !== selectedId
    ? [
        result.candidates.find((candidate) => candidate.id === selectedId),
        result.candidates.find((candidate) => candidate.id === compareId),
      ].filter((candidate): candidate is FindPlaceCandidate => Boolean(candidate))
    : [],
);

function statLine(candidate: FindPlaceCandidate) {
  return `${candidate.means.altitudeM} m · ${(candidate.means.temperatureCentiC / 100).toFixed(1)} °C · ${candidate.means.precipitationMm} mm`;
}

const ELEVATION_CHIPS = [
  { id: "", label: "Any" },
  { id: "lowland", label: "Lowland" },
  { id: "upland", label: "Upland" },
  { id: "highland", label: "Highland" },
];
const TEMP_CHIPS = [
  { id: "", label: "Any" },
  { id: "hot", label: "Hot" },
  { id: "mild", label: "Mild" },
  { id: "cold", label: "Cold" },
];
const RAIN_CHIPS = [
  { id: "", label: "Any" },
  { id: "wet", label: "Wet" },
  { id: "moderate", label: "Moderate" },
  { id: "dry", label: "Dry" },
];
const WATER_CHIPS = [
  { id: "", label: "Any" },
  { id: "coastal", label: "Coastal" },
  { id: "inland", label: "Inland" },
  { id: "freshwater", label: "Fresh water" },
];
const HAZARD_CHIPS = [
  { id: "", label: "Any" },
  { id: "low", label: "Low hazard" },
  { id: "stormy", label: "Stormy" },
];
</script>

{#snippet chips(
  label: string,
  value: string,
  options: { id: string; label: string }[],
  apply: (id: string) => void,
  off = false,
)}
  <div class="chip-field">
    <span>{label}</span>
    <div class="chips" role="group" aria-label={label}>
      {#each options as option (option.id || `${label}-any`)}
        <button type="button" class:active={value === option.id} disabled={off} onclick={() => apply(option.id)}>
          {option.label}
        </button>
      {/each}
      {#if value === "custom"}
        <button type="button" class="active" disabled>Custom</button>
      {/if}
    </div>
  </div>
{/snippet}

{#snippet query()}
  <p class="section-note">Presets fill filters. Chips and numbers stay editable.</p>
  <div class="find-presets" role="group" aria-label="Starting searches">
    {#each PRESETS as preset (preset.id)}
      <button type="button" class="chip" {disabled} onclick={() => applyPreset(preset)}>{preset.label}</button>
    {/each}
  </div>
  <label class="find-check"><input type="checkbox" bind:checked={landOnly} {disabled} /> Land only</label>
  {#if !climateAvailable}
    <p class="section-note">Climate filters are unavailable until climate has been derived.</p>
  {/if}
  {#if !hazardsAvailable}
    <p class="section-note">Earthquake and volcanic filters need hazard layers.</p>
  {/if}
  {@render chips("Elevation", simpleElevation, ELEVATION_CHIPS, applyElevation, disabled)}
  {@render chips("Temperature", simpleTemperature, TEMP_CHIPS, applyTemperature, climateOff)}
  {@render chips("Rainfall", simpleRain, RAIN_CHIPS, applyRain, climateOff)}
  {@render chips("Water", simpleWater, WATER_CHIPS, applyWater, disabled)}
  {@render chips(
    "Hazard",
    simpleHazard,
    HAZARD_CHIPS,
    applyHazard,
    disabled || (!climateAvailable && !hazardsAvailable),
  )}
  <label class="biome-field"
    ><span>Biome</span>
    <select bind:value={biomeClass} disabled={climateOff}>
      <option value="">Any</option>
      {#each biomes.filter((entry) => entry.id !== 0) as entry (entry.id)}
        <option value={String(entry.id)}>{entry.name}</option>
      {/each}
    </select>
  </label>
  <details class="advanced" oninput={syncSimpleFromNumbers}>
    <summary>Exact numbers</summary>
    <p class="section-note">Blank means any. Prefer ranks a match instead of excluding it.</p>
    <div class="detail-grid">
      <label><span>Altitude min m</span><input type="number" bind:value={altitudeMin} {disabled} /></label>
      <label><span>Altitude max m</span><input type="number" bind:value={altitudeMax} {disabled} /></label>
      <label><span>Temp min °C</span><input type="number" bind:value={tempMin} disabled={climateOff} /></label>
      <label><span>Temp max °C</span><input type="number" bind:value={tempMax} disabled={climateOff} /></label>
      <label><span>Rain min mm</span><input type="number" min="0" bind:value={rainMin} disabled={climateOff} /></label>
      <label><span>Rain max mm</span><input type="number" min="0" bind:value={rainMax} disabled={climateOff} /></label>
      <label
        ><span>Biome</span>
        <select bind:value={biomeClass} disabled={climateOff}>
          <option value="">Any</option>
          {#each biomes.filter((entry) => entry.id !== 0) as entry (entry.id)}
            <option value={String(entry.id)}>{entry.name}</option>
          {/each}
        </select>
      </label>
      <label
        ><span>Ice</span>
        <select bind:value={ice} {disabled}>
          <option value="">Any</option>
          <option value="no">No ice</option>
          <option value="yes">Ice cover</option>
        </select>
      </label>
      <label><span>Coast min km</span><input type="number" min="0" bind:value={coastMin} {disabled} /></label>
      <label><span>Coast max km</span><input type="number" min="0" bind:value={coastMax} {disabled} /></label>
      <label><span>Fresh water min km</span><input type="number" min="0" bind:value={freshMin} {disabled} /></label>
      <label><span>Fresh water max km</span><input type="number" min="0" bind:value={freshMax} {disabled} /></label>
      <label><span>Latitude min °</span><input type="number" bind:value={latitudeMin} {disabled} /></label>
      <label><span>Latitude max °</span><input type="number" bind:value={latitudeMax} {disabled} /></label>
      <label><span>Slope min %</span><input type="number" min="0" step="0.1" bind:value={slopeMin} {disabled} /></label>
      <label><span>Slope max %</span><input type="number" min="0" step="0.1" bind:value={slopeMax} {disabled} /></label>
      <label
        ><span>Humidity min %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={humidityMin}
          disabled={climateOff} /></label>
      <label
        ><span>Humidity max %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={humidityMax}
          disabled={climateOff} /></label>
      <label
        ><span>Aridity min %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={aridityMin}
          disabled={climateOff} /></label>
      <label
        ><span>Aridity max %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={aridityMax}
          disabled={climateOff} /></label>
      {#if islandOptions.length > 0}
        <label
          ><span>Landmass</span>
          <select bind:value={islandId} {disabled}>
            <option value="">Any</option>
            {#each islandOptions as island (island.id)}
              <option value={String(island.id)}>#{island.id} · {island.cells.toLocaleString("en-US")} cells</option>
            {/each}
          </select>
        </label>
      {/if}
      <label
        ><span>Quake min %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={quakeMin}
          disabled={hazardsOff} /></label>
      <label
        ><span>Quake max %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={quakeMax}
          disabled={hazardsOff} /></label>
      <label
        ><span>Volcano min %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={volcanoMin}
          disabled={hazardsOff} /></label>
      <label
        ><span>Volcano max %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={volcanoMax}
          disabled={hazardsOff} /></label>
      <label
        ><span>Storm min %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={stormMin}
          disabled={climateOff} /></label>
      <label
        ><span>Storm max %</span><input
          type="number"
          min="0"
          max="100"
          bind:value={stormMax}
          disabled={climateOff} /></label>
      <label
        ><span>Max regions</span><input type="number" min="1" max="12" bind:value={maxCandidates} {disabled} /></label>
      <label><span>Min cells</span><input type="number" min="1" bind:value={minCells} {disabled} /></label>
    </div>
    <div class="find-prefer">
      <label><input type="checkbox" bind:checked={altitudeSoft} {disabled} /> Altitude</label>
      <label><input type="checkbox" bind:checked={tempSoft} disabled={climateOff} /> Temperature</label>
      <label><input type="checkbox" bind:checked={rainSoft} disabled={climateOff} /> Rain</label>
      <label><input type="checkbox" bind:checked={biomeSoft} disabled={climateOff} /> Biome</label>
      <label><input type="checkbox" bind:checked={iceSoft} {disabled} /> Ice</label>
      <label><input type="checkbox" bind:checked={coastSoft} {disabled} /> Coast</label>
      <label><input type="checkbox" bind:checked={freshSoft} {disabled} /> Fresh water</label>
      <label><input type="checkbox" bind:checked={latitudeSoft} {disabled} /> Latitude</label>
      <label><input type="checkbox" bind:checked={slopeSoft} {disabled} /> Slope</label>
      <label><input type="checkbox" bind:checked={humiditySoft} disabled={climateOff} /> Humidity</label>
      <label><input type="checkbox" bind:checked={ariditySoft} disabled={climateOff} /> Aridity</label>
      {#if islandOptions.length > 0}
        <label><input type="checkbox" bind:checked={islandSoft} {disabled} /> Landmass</label>
      {/if}
      <label><input type="checkbox" bind:checked={quakeSoft} disabled={hazardsOff} /> Earthquake</label>
      <label><input type="checkbox" bind:checked={volcanoSoft} disabled={hazardsOff} /> Volcano</label>
      <label><input type="checkbox" bind:checked={stormSoft} disabled={climateOff} /> Storm</label>
    </div>
  </details>
  <div class="actions">
    <button type="button" class="primary" disabled={disabled || searching} onclick={submit}>
      {searching ? "Searching…" : "Search"}
    </button>
    <button type="button" {disabled} onclick={resetFilters}>Reset</button>
    <button type="button" disabled={disabled || (!result && !error && !formError)} onclick={clearResults}>
      Clear
    </button>
  </div>
  {#if formError || error}
    <p class="section-note find-error">{formError || error}</p>
  {/if}
{/snippet}

{#snippet matches()}
  {#if result}
    <p class="section-note">
      {result.candidateCount} region{result.candidateCount === 1 ? "" : "s"} · {result.matchedCells.toLocaleString(
        "en-US",
      )} matching cells
    </p>
    {#if compared.length === 2}
      <div class="find-compare">
        {#each compared as candidate (candidate.id)}
          <p>
            <strong>#{candidate.id}</strong>
            {statLine(candidate)}
            {#if candidate.preferences.length}<span>{candidate.preferences.join(" · ")}</span>{/if}
          </p>
        {/each}
      </div>
    {/if}
    {#if result.candidates.length === 0}
      <p class="empty-note">No region matches those criteria. Loosen a chip or open Exact numbers.</p>
    {:else}
      <ul class="find-results">
        {#each result.candidates as candidate (candidate.id)}
          <li class:active={selectedId === candidate.id} class:compare={compareId === candidate.id}>
            <button type="button" class="find-candidate" onclick={() => onselect(candidate)}>
              <strong>{biomeName(candidate.means.biomeClass)}</strong>
              <span>{candidate.areaKm2.toLocaleString("en-US")} km² · {candidate.reasons[0]}</span>
              <span>{statLine(candidate)}</span>
              {#if candidate.risks.length}
                <span class="find-risks">{candidate.risks.join(" · ")}</span>
              {/if}
              {#if expandedId === candidate.id}
                {#if candidate.satisfied.length}<span>{candidate.satisfied.join(" · ")}</span>{/if}
                {#if candidate.preferences.length}<span>{candidate.preferences.join(" · ")}</span>{/if}
                {#if candidate.nearMisses.length}<span>{candidate.nearMisses.join(" · ")}</span>{/if}
              {/if}
            </button>
            <div class="actions">
              <button type="button" onclick={() => toggleExpanded(candidate.id)}>
                {expandedId === candidate.id ? "Less" : "Why"}
              </button>
              <button
                type="button"
                class:active={compareId === candidate.id}
                onclick={() => toggleCompare(candidate.id)}>
                Compare
              </button>
              <button type="button" onclick={() => onpin(candidate)}>Pin</button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
{/snippet}

{#if variant === "studio"}
  <section class="find-place studio" aria-label="Find place">
    <div class="block">
      <span class="kicker">Query</span>
      {@render query()}
    </div>
    {#if result}
      <div class="block">
        <span class="kicker">Matches</span>
        {@render matches()}
      </div>
    {/if}
  </section>
{:else}
  <details class="find-place map-section-group">
    <summary>
      <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
      <strong>Find Place</strong>
      {#if result}
        <span class="section-count">{result.candidateCount}</span>
      {/if}
    </summary>
    <div class="section-body">
      {@render query()}
      {@render matches()}
    </div>
  </details>
{/if}

<style>
.find-place.studio {
  display: grid;
  gap: 10px;
}
.block {
  display: grid;
  gap: 8px;
  padding: 10px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 10px;
  background: rgb(255 255 255 / 3%);
}
.kicker {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #d5ab6c;
}
.section-body {
  display: grid;
  gap: 8px;
}
.section-note,
.empty-note {
  margin: 0;
  color: var(--theme-neutral-text-muted, #aebdb1);
  font-size: 11px;
  line-height: 1.45;
}
.empty-note {
  padding: 10px;
  border: 1px dashed rgb(255 255 255 / 14%);
  border-radius: 8px;
}
.find-presets,
.chips,
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.chip-field,
.biome-field {
  display: grid;
  gap: 4px;
}
.chip-field span,
.biome-field span,
.detail-grid label span,
.advanced summary {
  color: #d9d0c3;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.chips button,
.chip,
.actions button {
  min-height: 26px;
  padding: 3px 8px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 999px;
  background: #0f1a16;
  color: #edf2ec;
  font-size: 11px;
  cursor: pointer;
  transition:
    background 180ms ease,
    border-color 180ms ease,
    color 180ms ease;
}
.chips button.active,
.actions button.active {
  border-color: #d5ab6c;
  background: #d5ab6c;
  color: #1b2822;
}
.actions .primary {
  border-radius: 7px;
  border-color: #d5ab6c;
  background: #d5ab6c;
  color: #1b2822;
}
.chips button:disabled,
.chip:disabled,
.actions button:disabled {
  opacity: 0.45;
  cursor: default;
}
.find-check,
.find-prefer label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #edf2ec;
}
.biome-field select,
.detail-grid input,
.detail-grid select {
  width: 100%;
  min-width: 0;
  padding: 6px 8px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 7px;
  background: #0f1a16;
  color: #edf2ec;
  font-size: 11px;
}
.advanced {
  display: grid;
  gap: 8px;
}
.advanced summary {
  cursor: pointer;
  list-style: none;
}
.advanced summary::-webkit-details-marker {
  display: none;
}
.detail-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.detail-grid label {
  display: grid;
  gap: 4px;
  font-size: 10px;
}
.find-prefer {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 12px;
}
.find-error {
  color: var(--danger, #c45c48);
}
.find-compare {
  display: grid;
  gap: 6px;
  font-size: 11px;
}
.find-compare span {
  display: block;
  color: #aebdb1;
}
.find-results {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 6px;
}
.find-results li {
  display: grid;
  gap: 4px;
  padding: 4px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 8px;
  background: rgb(0 0 0 / 16%);
}
.find-results li.active {
  border-color: #d5ab6c;
  background: rgb(213 171 108 / 16%);
}
.find-results li.compare {
  box-shadow: inset 0 0 0 1px #7aa7d9;
}
.find-candidate {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  text-align: left;
  padding: 4px 6px;
  border: 0;
  background: none;
  color: inherit;
  cursor: pointer;
}
.find-candidate span {
  font-size: 11px;
  color: #aebdb1;
}
.find-risks {
  color: #d9a05c;
}
.actions button {
  border-radius: 7px;
}
</style>
