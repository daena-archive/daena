<script lang="ts">
import { onDestroy, onMount } from "svelte";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import maplibregl from "maplibre-gl/dist/maplibre-gl-csp.js";
import workerUrl from "maplibre-gl/dist/maplibre-gl-csp-worker.js?url";
import "maplibre-gl/dist/maplibre-gl.css";
import type { Map as MapLibreMap, StyleSpecification } from "maplibre-gl";
import {
  project,
  ATLAS_STUDIO_PROGRESS_EVENT,
  type AtlasRenderCapabilities,
  type AtlasRenderRequest,
  type AtlasStudioInspectHit,
  type AtlasStudioProgress,
  type AtlasStudioSessionStatus,
} from "$lib/project/client";

if (typeof maplibregl.setWorkerUrl === "function") maplibregl.setWorkerUrl(workerUrl);
if (typeof maplibregl.setMaxParallelImageRequests === "function") maplibregl.setMaxParallelImageRequests(8);

let {
  mapId,
  onexport,
}: {
  mapId: string;
  onexport?: (request: AtlasRenderRequest) => void;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let session = $state<AtlasStudioSessionStatus | null>(null);
let capabilities = $state<AtlasRenderCapabilities | null>(null);
let stage = $state("Opening Atlas Studio…");
let error = $state("");
let stale = $state("");
let cursor = $state("—");
let loading = $state(true);
let styleId = $state("daena-atlas-relief");
let offsetYears = $state(0);
let timeKind = $state<"physical-offset-year" | "calendar-year">("physical-offset-year");
let authoredYear = $state(1);
let layers = $state<Array<{ id: string; name: string; enabled: boolean }>>([]);
let hits = $state<AtlasStudioInspectHit[]>([]);
let confirmCache = $state(false);
let showHelp = $state(false);
let unlisten: UnlistenFn | undefined;
let map: MapLibreMap | null = null;
let opening = false;
let resizeObserver: ResizeObserver | undefined;
let debounce: ReturnType<typeof setTimeout> | undefined;
let statusTimer: ReturnType<typeof setInterval> | undefined;
let prefetchTimer: ReturnType<typeof setTimeout> | undefined;
let mountedControls = false;

function deviceScale() {
  const ratio = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  return ratio >= 1.5 ? 2 : 1;
}

function formatEpoch(offset: number) {
  if (offset === 0) return "Reference epoch";
  if (offset < 0) return `${Math.abs(offset)} years before reference`;
  return `${offset} years after reference`;
}

function prefersReducedMotion() {
  return typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

const STUDIO_DIAGNOSTICS: Record<string, { title: string; action: string }> = {
  "atlas.studio.request.invalid": {
    title: "This Atlas Studio request is not valid.",
    action: "Refresh Atlas and use a supported style, epoch, and layer set.",
  },
  "atlas.studio.tile.invalid": {
    title: "That map tile request is not valid.",
    action: "Pan or zoom back into the supported range, then retry.",
  },
  "atlas.studio.resource-limit": {
    title: "Atlas Studio is busy or at a resource limit.",
    action: "Wait for visible tiles; a full queue is retryable and is not a sticky error.",
  },
  "atlas.studio.cancelled": {
    title: "Atlas work was cancelled.",
    action: "Refresh Atlas if the map is still open.",
  },
  "atlas.studio.unsupported": {
    title: "Atlas Studio is not available for this map.",
    action: "Enable Maps and open an accepted physical map.",
  },
  "atlas.studio.stale": {
    title: "The project changed after this Atlas session.",
    action: "Refresh Atlas to capture the current generation.",
  },
  "atlas.studio.expired": {
    title: "This Atlas session expired.",
    action: "Refresh Atlas to open a new session.",
  },
  "atlas.studio.tile.failed": {
    title: "Atlas Studio failed to draw a tile.",
    action: "Retry. If it continues, regenerate the disposable cache.",
  },
  "atlas.studio.protocol.denied": {
    title: "Atlas Studio refused that tile request.",
    action: "Refresh Atlas. Do not paste file paths into the map.",
  },
};

function explainStudioError(raw: string) {
  const code = raw.split(":")[0]?.trim() ?? "";
  const mapped = STUDIO_DIAGNOSTICS[code];
  if (mapped) return { code, ...mapped };
  return {
    code: code.startsWith("atlas.studio.") ? code : "atlas.studio.failed",
    title: raw || "Atlas Studio could not complete that action.",
    action: "Retry. If it continues, Refresh Atlas or regenerate the disposable cache.",
  };
}

function derivedExplanation(hit: AtlasStudioInspectHit) {
  if (hit.kind === "derived-tributary") {
    return "Atlas-only derived drainage. It is not canonical Physical Map data and cannot be edited or promoted from Studio.";
  }
  if (hit.derived) {
    return "Presentation overlay from the captured Atlas snapshot. It is not a Physical Map edit.";
  }
  return "Authored or semantic map feature from the captured project snapshot.";
}

function currentViewExportHeight(west: number, south: number, east: number, north: number, widthPx: number) {
  const latSpan = Math.max(1, north - south);
  let lonSpan = (east - west + 360_000_000) % 360_000_000;
  if (lonSpan === 0) lonSpan = 360_000_000;
  return Math.max(256, Math.min(2048, Math.round((widthPx * latSpan) / Math.max(1, lonSpan))));
}

function studioRequest() {
  return {
    schemaVersion: 1,
    mapEntityId: mapId,
    offsetYears,
    algorithmVersion: 2,
    level: "detailed" as const,
    variant: 0,
    styleId,
    activeLayerIds: layers.filter((layer) => layer.enabled).map((layer) => layer.id),
    projection: "web-mercator",
    timeKind,
    authoredYear: timeKind === "calendar-year" ? authoredYear : null,
  };
}

function tileUrlAllowed(url: string, token: string) {
  if (url.includes("://") && /^https?:\/\//i.test(url) && !url.includes("atlas-studio.localhost")) {
    return false;
  }
  return url.includes(token) && (url.startsWith("atlas-studio:") || url.includes("atlas-studio.localhost"));
}

function isTransientTileError(message: string) {
  return /AJAXError|Load failed|503|408|queue is full|resource-limit|Failed to fetch|access control|prefetch deferred/i.test(
    message,
  );
}

function toMicro(value: number) {
  return Math.round(value * 1_000_000);
}

async function loadCapabilities() {
  capabilities = await project.atlasCapabilities(mapId);
  styleId = capabilities.styles.includes("daena-atlas-relief")
    ? "daena-atlas-relief"
    : (capabilities.styles[0] ?? "daena-atlas-relief");
  if (capabilities.calendarBinding) {
    authoredYear = capabilities.calendarBinding.calendarReferenceYear;
  }
  layers = capabilities.layers
    .filter((layer) => layer.id !== "frame")
    .map((layer) => ({
      id: layer.id,
      name: layer.name,
      enabled: layer.defaultVisible && layer.id !== "frame",
    }));
}

async function openSession() {
  if (opening) return;
  opening = true;
  loading = true;
  error = "";
  stale = "";
  hits = [];
  stage = "Snapshotting…";
  try {
    if (!capabilities) await loadCapabilities();
    if (session) {
      await project.atlasStudioClose(session.sessionToken).catch(() => undefined);
      session = null;
    }
    const keepCenter = map?.getCenter();
    const keepZoom = map?.getZoom();
    map?.remove();
    map = null;
    const next = await project.atlasStudioOpen(studioRequest(), deviceScale());
    session = next;
    styleId = next.styleId;
    offsetYears = next.offsetYears;
    stage = "Mounting map…";
    mountMap(next, keepCenter ? { center: [keepCenter.lng, keepCenter.lat], zoom: keepZoom ?? 1 } : undefined);
    queueMicrotask(() => map?.resize());
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
    loading = false;
  } finally {
    opening = false;
  }
}

function scheduleSession() {
  if (!mountedControls) return;
  if (debounce) clearTimeout(debounce);
  debounce = setTimeout(() => void openSession(), 300);
}

function mountMap(
  status: AtlasStudioSessionStatus,
  view?: { center: [number, number]; zoom: number },
) {
  const container = host;
  if (!container) return;
  const style: StyleSpecification = {
    version: 8,
    sources: {
      "atlas-studio": {
        type: "raster",
        tiles: [status.tileUrlTemplate],
        tileSize: status.tileSize,
        minzoom: 0,
        maxzoom: status.maxZoom,
        scheme: "xyz",
      },
    },
    layers: [
      { id: "atlas-background", type: "background", paint: { "background-color": "#0d1b2a" } },
      { id: "atlas-relief", type: "raster", source: "atlas-studio" },
    ],
  };
  try {
    map = new maplibregl.Map({
      container,
      style,
      center: view?.center ?? [0, 20],
      zoom: view?.zoom ?? 1,
      minZoom: 0,
      maxZoom: status.maxZoom,
      maxPitch: 0,
      pitchWithRotate: false,
      renderWorldCopies: true,
      attributionControl: false,
      fadeDuration: 0,
      keyboard: true,
      transformRequest(url) {
        if (!tileUrlAllowed(url, status.sessionToken)) {
          throw new Error("Atlas Studio rejects remote tile, glyph, sprite, and telemetry URLs");
        }
        return { url };
      },
    });
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "MapLibre failed to create a WebGL2 context.";
    loading = false;
    return;
  }
  map.on("load", () => map?.resize());
  map.on("mousemove", (event) => {
    cursor = `${event.lngLat.lng.toFixed(4)}°, ${event.lngLat.lat.toFixed(4)}°`;
  });
  map.on("click", (event) => {
    inspectAt(event.lngLat.lng, event.lngLat.lat);
  });
  map.on("moveend", () => schedulePrefetch(status));
  map.on("idle", () => {
    loading = false;
    stage = "Ready";
    map?.resize();
    schedulePrefetch(status);
  });
  map.on("dataloading", () => {
    if (!error) loading = true;
  });
  map.on("error", (event) => {
    const message = event.error?.message ?? "";
    if (isTransientTileError(message)) return;
    if (message.includes("reject") || message.includes("WebGL")) {
      error = message || "Atlas Studio failed to load a tile.";
      loading = false;
    }
  });
  resizeObserver?.disconnect();
  resizeObserver = new ResizeObserver(() => map?.resize());
  resizeObserver.observe(container);
}

function schedulePrefetch(status: AtlasStudioSessionStatus) {
  if (prefetchTimer) clearTimeout(prefetchTimer);
  prefetchTimer = setTimeout(() => prefetchRing(status), 200);
}

function prefetchRing(status: AtlasStudioSessionStatus) {
  if (!map || !session || session.sessionToken !== status.sessionToken) return;
  const z = Math.min(status.maxZoom, Math.max(0, Math.floor(map.getZoom())));
  const bounds = map.getBounds();
  const n = 2 ** z;
  const lonToX = (lon: number) => Math.floor(((lon + 180) / 360) * n);
  const latToY = (lat: number) => {
    const sin = Math.sin((lat * Math.PI) / 180);
    const y = 0.5 - Math.log((1 + sin) / (1 - sin)) / (4 * Math.PI);
    return Math.floor(Math.min(n - 1, Math.max(0, y * n)));
  };
  const minX = lonToX(bounds.getWest()) - 1;
  const maxX = lonToX(bounds.getEast()) + 1;
  const minY = Math.max(0, latToY(bounds.getNorth()) - 1);
  const maxY = Math.min(n - 1, latToY(bounds.getSouth()) + 1);
  const template = status.tileUrlTemplate;
  for (let x = minX; x <= maxX; x += 1) {
    const wrapped = ((x % n) + n) % n;
    for (let y = minY; y <= maxY; y += 1) {
      const url = `${template.replace("{z}", String(z)).replace("{x}", String(wrapped)).replace("{y}", String(y))}&priority=prefetch`;
      void fetch(url).catch(() => undefined);
    }
  }
}

function currentViewExport(): AtlasRenderRequest | null {
  if (!map) return null;
  const bounds = map.getBounds();
  const west = toMicro(bounds.getWest());
  const east = toMicro(bounds.getEast());
  const south = Math.max(-85_051_129, toMicro(bounds.getSouth()));
  const north = Math.min(85_051_129, toMicro(bounds.getNorth()));
  const widthPx = 2048;
  const heightPx = currentViewExportHeight(west, south, east, north, widthPx);
  return {
    schemaVersion: 1,
    offsetYears,
    algorithmVersion: 2,
    level: "detailed",
    variant: 0,
    styleId,
    widthPx,
    heightPx,
    dpi: 72,
    format: "png",
    projection: "web-mercator",
    extent: {
      westLonMicro: west,
      southLatMicro: south,
      eastLonMicro: east === 180_000_000 ? 180_000_000 : east,
      northLatMicro: north,
    },
    unlockAspect: true,
    activeLayerIds: layers.filter((layer) => layer.enabled).map((layer) => layer.id),
    timeKind,
    authoredYear: timeKind === "calendar-year" ? authoredYear : null,
    bindingRevision: null,
  };
}

async function regenerate() {
  error = "";
  confirmCache = false;
  stage = "Regenerating cache…";
  try {
    await project.atlasStudioRegenerateCache();
    await openSession();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

function inspectAt(lng: number, lat: number) {
  const token = session?.sessionToken;
  if (!token || !map) return;
  void project
    .atlasStudioInspect(token, toMicro(lng), toMicro(lat), Math.floor(map.getZoom()))
    .then((next) => {
      hits = next;
    })
    .catch(() => {
      hits = [];
    });
}

function onViewportKey(event: KeyboardEvent) {
  if (!map) return;
  const reduced = prefersReducedMotion();
  const animate = !reduced;
  if (event.key === "ArrowLeft" || event.key === "ArrowRight" || event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault();
    const step = event.shiftKey ? 120 : 48;
    const dx = event.key === "ArrowLeft" ? -step : event.key === "ArrowRight" ? step : 0;
    const dy = event.key === "ArrowUp" ? -step : event.key === "ArrowDown" ? step : 0;
    map.panBy([dx, dy], { animate, duration: reduced ? 0 : 200 });
  } else if (event.key === "+" || event.key === "=") {
    event.preventDefault();
    map.zoomIn({ animate, duration: reduced ? 0 : 200 });
  } else if (event.key === "-" || event.key === "_") {
    event.preventDefault();
    map.zoomOut({ animate, duration: reduced ? 0 : 200 });
  } else if (event.key === "0" || event.key === "Home") {
    event.preventDefault();
    map.jumpTo({ center: [0, 20], zoom: 1 });
  } else if (event.key === "Enter") {
    event.preventDefault();
    const center = map.getCenter();
    inspectAt(center.lng, center.lat);
  } else if (event.key === "Escape") {
    event.preventDefault();
    hits = [];
  } else if (event.key === "?" || event.key.toLowerCase() === "h") {
    if (!event.metaKey && !event.ctrlKey) {
      event.preventDefault();
      showHelp = !showHelp;
    }
  }
}

async function applyPreset(id: string) {
  if (!id) return;
  const fields = await project.listFields(mapId);
  const current = fields.find((field) => field.namespace === "maps" && field.key === "atlasPresets");
  const presets = ((current?.value as { presets?: Array<Record<string, unknown>> } | undefined)?.presets ?? []);
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
  const ids = new Set((preset.activeLayerIds as string[] | undefined) ?? []);
  layers = layers.map((layer) => ({ ...layer, enabled: ids.has(layer.id) }));
  await openSession();
}

onMount(() => {
  void listen<AtlasStudioProgress>(ATLAS_STUDIO_PROGRESS_EVENT, (event) => {
    if (event.payload.mapEntityId !== mapId) return;
    stage = `${event.payload.stage} · ${event.payload.completed}/${event.payload.total}`;
  }).then((fn) => {
    unlisten = fn;
  });
  statusTimer = setInterval(() => {
    const token = session?.sessionToken;
    if (!token) return;
    void project
      .atlasStudioStatus(token)
      .then((next) => {
        if (next.errorCode === "atlas.studio.stale") {
          stale = next.error ?? "The project changed after this Atlas session.";
        }
      })
      .catch(() => undefined);
  }, 2000);
  void loadCapabilities()
    .then(() => {
      mountedControls = true;
      return openSession();
    })
    .catch((cause) => {
      error = cause instanceof Error ? cause.message : String(cause);
      loading = false;
    });
});

$effect(() => {
  const container = host;
  const status = session;
  if (!container || !status || map) return;
  mountMap(status);
  queueMicrotask(() => map?.resize());
});

onDestroy(() => {
  unlisten?.();
  if (debounce) clearTimeout(debounce);
  if (statusTimer) clearInterval(statusTimer);
  if (prefetchTimer) clearTimeout(prefetchTimer);
  resizeObserver?.disconnect();
  resizeObserver = undefined;
  map?.remove();
  map = null;
  if (session) {
    void project.atlasStudioClose(session.sessionToken).catch(() => undefined);
  }
});
</script>

<section class="studio" aria-label="Atlas Studio">
  <a class="skip" href="#atlas-studio-map">Skip to map</a>
  <header>
    <div>
      <strong>Atlas Studio</strong>
      <span role="status">{error ? "Error" : loading ? stage : `${styleId} · ${formatEpoch(offsetYears)}`}</span>
    </div>
    <div class="actions">
      <output aria-live="polite">{cursor}</output>
      <button type="button" onclick={() => void openSession()}>Refresh Atlas</button>
      <button type="button" onclick={() => (confirmCache = true)}>Regenerate cache</button>
      <button
        type="button"
        onclick={() => {
          const request = currentViewExport();
          if (request) onexport?.(request);
        }}>Export</button>
      <button type="button" aria-expanded={showHelp} onclick={() => (showHelp = !showHelp)}>Keyboard help</button>
    </div>
  </header>
  <div class="body">
    <aside aria-label="Atlas Studio controls">
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
        <select bind:value={styleId} onchange={() => scheduleSession()}>
          {#each capabilities?.styles ?? [] as id}
            <option value={id}>{id}</option>
          {/each}
        </select>
      </label>
      <label>
        World time
        <input
          type="range"
          min="-100000"
          max="100000"
          step="1"
          bind:value={offsetYears}
          disabled={timeKind === "calendar-year"}
          oninput={() => scheduleSession()} />
        <output>{formatEpoch(offsetYears)}</output>
      </label>
      {#if capabilities?.timeModes.includes("calendar-year")}
        <label>
          Time mode
          <select bind:value={timeKind} onchange={() => scheduleSession()}>
            <option value="physical-offset-year">Physical offset</option>
            <option value="calendar-year">Authored year</option>
          </select>
        </label>
        {#if timeKind === "calendar-year"}
          <label>
            Authored year
            <input type="number" bind:value={authoredYear} oninput={() => scheduleSession()} />
          </label>
        {/if}
      {/if}
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
                scheduleSession();
              }} />
            {layer.name}
          </label>
        {/each}
      </fieldset>
      <section class="provenance" aria-label="Atlas provenance">
        <strong>Provenance</strong>
        <p>The Physical Map is canonical. Atlas geography is derived, deterministic, and disposable.</p>
        <p>Released products: detail algorithm 2 · drainage 2 · renderer 6.</p>
        <p>Regenerate cache deletes only validated files under the project Atlas cache. Canonical maps, presets, and checkpoints stay unchanged.</p>
      </section>
      {#if confirmCache}
        <div class="confirm" role="alertdialog" aria-labelledby="atlas-cache-title" aria-describedby="atlas-cache-copy">
          <strong id="atlas-cache-title">Regenerate disposable Atlas cache?</strong>
          <p id="atlas-cache-copy">This removes derived Atlas cache files only. It does not change canonical project files.</p>
          <div class="actions">
            <button type="button" onclick={() => void regenerate()}>Regenerate now</button>
            <button type="button" onclick={() => (confirmCache = false)}>Cancel</button>
          </div>
        </div>
      {/if}
      {#if showHelp}
        <section class="help" aria-label="Keyboard shortcuts">
          <strong>Keyboard</strong>
          <ul>
            <li>Arrows pan (Shift for a larger step)</li>
            <li>+ / − zoom</li>
            <li>Home or 0 resets the view</li>
            <li>Enter inspects the map center</li>
            <li>Escape clears inspection</li>
          </ul>
        </section>
      {/if}
      {#if hits.length > 0}
        <div class="inspect" role="region" aria-label="Feature inspection">
          <strong>Inspect</strong>
          {#each hits as hit}
            <p>
              <span>{hit.label ?? hit.id}</span>
              <small>{hit.kind}{hit.derived ? " · derived" : ""}</small>
              <small>{derivedExplanation(hit)}</small>
            </p>
          {/each}
        </div>
      {/if}
    </aside>
    <div class="frame">
      {#if error}
        {@const diagnostic = explainStudioError(error)}
        <p class="error" role="alert">
          <strong>{diagnostic.title}</strong>
          <span>{diagnostic.action}</span>
          {#if diagnostic.code}<code>{diagnostic.code}</code>{/if}
          <button type="button" onclick={() => void openSession()}>Retry</button>
        </p>
      {:else if stale}
        {@const diagnostic = explainStudioError(stale.includes("atlas.studio.stale") ? stale : `atlas.studio.stale: ${stale}`)}
        <p class="stale" role="status">
          <strong>{diagnostic.title}</strong>
          <span>{diagnostic.action}</span>
          <button type="button" onclick={() => void openSession()}>Refresh Atlas</button>
        </p>
      {:else if loading}
        <p class="loading" role="status">{stage}</p>
      {/if}
      <div
        class="viewport"
        id="atlas-studio-map"
        bind:this={host}
        role="application"
        tabindex="0"
        aria-label="Atlas Studio map"
        aria-keyshortcuts="ArrowLeft ArrowRight ArrowUp ArrowDown + - Home Enter Escape"
        onkeydown={onViewportKey}></div>
    </div>
  </div>
</section>

<style>
.studio {
  position: relative;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-height: 0;
  width: 100%;
  height: 100%;
  flex: 1;
  background: #0d1b2a;
  color: #edf2ec;
}
.body {
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr);
  min-height: 0;
  height: 100%;
}
aside {
  display: grid;
  align-content: start;
  gap: 10px;
  overflow: auto;
  padding: 10px;
  background: #1b2822;
  border-right: 1px solid #405047;
  font: 12px/1.4 system-ui;
}
aside label,
aside fieldset {
  display: grid;
  gap: 4px;
}
aside fieldset {
  border: 1px solid #405047;
  border-radius: 8px;
}
aside select,
aside input[type="number"] {
  border: 1px solid #405047;
  border-radius: 6px;
  padding: 6px;
  background: #0f1a16;
  color: #edf2ec;
}
.inspect p {
  margin: 0;
  display: grid;
}
.inspect small {
  color: #b8c8bc;
}
.frame {
  position: relative;
  display: flex;
  min-width: 0;
  min-height: 360px;
  height: 100%;
  background: #0d1b2a;
}
header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 12px;
  background: #1b2822;
  border-bottom: 1px solid #405047;
}
header span,
output {
  display: block;
  font: 12px/1.4 system-ui;
  color: #b8c8bc;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
button {
  border: 0;
  border-radius: 7px;
  padding: 7px 10px;
  background: #31443b;
  color: #edf2ec;
  font: 700 12px system-ui;
  cursor: pointer;
}
.viewport {
  flex: 1;
  min-width: 0;
  min-height: 0;
  height: 100%;
}
.viewport :global(.maplibregl-map) {
  width: 100%;
  height: 100%;
}
.error,
.loading,
.stale {
  position: absolute;
  z-index: 1;
  margin: 0;
  padding: 8px 12px;
  font: 12px/1.4 system-ui;
  pointer-events: none;
  display: grid;
  gap: 4px;
  max-width: min(420px, calc(100% - 24px));
}
.error button,
.stale button {
  pointer-events: auto;
  justify-self: start;
}
.error {
  color: #f5a49c;
}
.stale,
.loading {
  color: #d5ab6c;
}
.error code,
.stale code {
  font: 11px/1.3 ui-monospace, monospace;
  color: #b8c8bc;
}
.skip {
  position: absolute;
  left: 8px;
  top: 8px;
  z-index: 3;
  transform: translateY(-200%);
  padding: 6px 10px;
  background: #edf2ec;
  color: #0f1a16;
  font: 700 12px system-ui;
  border-radius: 6px;
}
.skip:focus {
  transform: none;
}
.provenance,
.help,
.confirm {
  display: grid;
  gap: 6px;
  padding: 8px;
  border: 1px solid #405047;
  border-radius: 8px;
}
.provenance p,
.help li,
.confirm p {
  margin: 0;
  color: #b8c8bc;
}
.help ul {
  margin: 0;
  padding-left: 1.2em;
}
button:focus-visible,
a:focus-visible,
select:focus-visible,
input:focus-visible,
.viewport:focus-visible {
  outline: 2px solid #edf2ec;
  outline-offset: 2px;
}
@media (prefers-reduced-motion: reduce) {
  .studio,
  .studio * {
    transition: none !important;
    animation: none !important;
  }
}
</style>
