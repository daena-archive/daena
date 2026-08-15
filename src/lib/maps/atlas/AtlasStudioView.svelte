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
  type AtlasStudioProgress,
  type AtlasStudioSessionStatus,
} from "$lib/project/client";

if (typeof maplibregl.setWorkerUrl === "function") maplibregl.setWorkerUrl(workerUrl);

let {
  mapId,
  onexport,
}: {
  mapId: string;
  onexport?: () => void;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let session = $state<AtlasStudioSessionStatus | null>(null);
let stage = $state("Opening Atlas Studio…");
let error = $state("");
let cursor = $state("—");
let loading = $state(true);
let unlisten: UnlistenFn | undefined;
let map: MapLibreMap | null = null;
let opening = false;
let resizeObserver: ResizeObserver | undefined;

function deviceScale() {
  const ratio = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  return ratio >= 1.5 ? 2 : 1;
}

function studioRequest() {
  return {
    schemaVersion: 1,
    mapEntityId: mapId,
    offsetYears: 0,
    algorithmVersion: 1,
    level: "detailed" as const,
    variant: 0,
    styleId: "daena-atlas-relief",
    activeLayerIds: ["ice", "lakes", "ocean", "relief"],
    projection: "web-mercator",
    timeKind: "physical-offset-year" as const,
    authoredYear: null,
  };
}

function tileUrlAllowed(url: string, token: string) {
  if (url.includes("://") && /^https?:\/\//i.test(url) && !url.includes("atlas-studio.localhost")) {
    return false;
  }
  return url.includes(token) && (url.startsWith("atlas-studio:") || url.includes("atlas-studio.localhost"));
}

function isTransientTileError(message: string) {
  return /AJAXError|Load failed|503|408|queue is full|resource-limit|Failed to fetch|access control/i.test(
    message,
  );
}

async function openSession() {
  if (opening) return;
  opening = true;
  loading = true;
  error = "";
  stage = "Snapshotting…";
  try {
    if (session) {
      await project.atlasStudioClose(session.sessionToken).catch(() => undefined);
      session = null;
    }
    map?.remove();
    map = null;
    const next = await project.atlasStudioOpen(studioRequest(), deviceScale());
    session = next;
    stage = "Mounting map…";
    mountMap(next);
    queueMicrotask(() => map?.resize());
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
    loading = false;
  } finally {
    opening = false;
  }
}

function mountMap(status: AtlasStudioSessionStatus) {
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
      center: [0, 20],
      zoom: 1,
      minZoom: 0,
      maxZoom: status.maxZoom,
      maxPitch: 0,
      pitchWithRotate: false,
      renderWorldCopies: true,
      attributionControl: false,
      fadeDuration: 0,
      maxParallelImageRequests: 8,
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
  map.on("dataloading", () => {
    if (!error) loading = true;
  });
  map.on("idle", () => {
    loading = false;
    stage = "Ready";
    map?.resize();
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

async function regenerate() {
  error = "";
  stage = "Regenerating cache…";
  try {
    await project.atlasStudioRegenerateCache();
    await openSession();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

onMount(() => {
  void listen<AtlasStudioProgress>(ATLAS_STUDIO_PROGRESS_EVENT, (event) => {
    if (event.payload.mapEntityId !== mapId) return;
    stage = `${event.payload.stage} · ${event.payload.completed}/${event.payload.total}`;
  }).then((fn) => {
    unlisten = fn;
  });
  void openSession();
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
  <header>
    <div>
      <strong>Atlas Studio</strong>
      <span role="status">{error ? "Error" : loading ? stage : "Relief · reference epoch"}</span>
    </div>
    <div class="actions">
      <output aria-live="polite">{cursor}</output>
      <button type="button" onclick={() => void openSession()}>Refresh Atlas</button>
      <button type="button" onclick={() => void regenerate()}>Regenerate cache</button>
      <button type="button" onclick={() => onexport?.()}>Export</button>
    </div>
  </header>
  <div class="frame">
    {#if error}
      <p class="error" role="alert">
        {error}
        <button type="button" onclick={() => void openSession()}>Retry</button>
      </p>
    {:else if loading}
      <p class="loading" role="status">{stage}</p>
    {/if}
    <div class="viewport" bind:this={host} role="application" aria-label="Atlas Studio map"></div>
  </div>
</section>

<style>
.studio {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-height: 0;
  width: 100%;
  height: 100%;
  flex: 1;
  background: #0d1b2a;
  color: #edf2ec;
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
.loading {
  position: absolute;
  z-index: 1;
  margin: 0;
  padding: 8px 12px;
  font: 12px/1.4 system-ui;
  pointer-events: none;
}
.error button {
  pointer-events: auto;
}
.error {
  color: #f5a49c;
}
.loading {
  color: #d5ab6c;
}
@media (prefers-reduced-motion: reduce) {
  .studio,
  .studio * {
    transition: none !important;
    animation: none !important;
  }
}
</style>
