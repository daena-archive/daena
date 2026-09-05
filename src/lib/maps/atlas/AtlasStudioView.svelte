<script lang="ts">
import { onDestroy, onMount } from "svelte";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import Map from "ol/Map.js";
import View from "ol/View.js";
import TileLayer from "ol/layer/Tile.js";
import XYZ from "ol/source/XYZ.js";
import { defaults as defaultInteractions } from "ol/interaction/defaults.js";
import { fromLonLat, toLonLat, transformExtent } from "ol/proj.js";
import "ol/ol.css";
import {
  project,
  ATLAS_STUDIO_PROGRESS_EVENT,
  type AtlasRenderCapabilities,
  type AtlasRenderRequest,
  type AtlasStudioInspectHit,
  type AtlasStudioProgress,
  type AtlasStudioSessionStatus,
  type AtlasStudioSurfaceSample,
} from "$lib/project/client";
import type { MapLayerDefinition } from "../native-vector/types";
import MapViewControls from "../native-vector/MapViewControls.svelte";
import { bindMapLifecycle, type MapLifecycle } from "../openlayers/lifecycle";
import { createAtlasRenderCompletionTracker } from "./render-completion.ts";
import MapLayerVisibilityList from "../MapLayerVisibilityList.svelte";
import { ATLAS_DETAIL_ALGORITHM_VERSION, isAtlasLayerEnabledByDefault } from "./constants.ts";

const EPOCH_MIN = -100_000;
const EPOCH_MAX = 100_000;
const EPOCH_STEP = 10;
const VIEWER_ROLE_ALIASES: Record<string, string[]> = {
  ocean: ["ocean"],
  ice: ["ice"],
  lakes: ["lakes"],
  rivers: ["rivers"],
  coastlines: ["islands", "coastlines"],
  contours: ["bathymetric-contours", "contours"],
  "tectonic-plates": ["tectonic-plates"],
  "tectonic-boundaries": ["tectonic-boundaries"],
  "volcanic-centers": ["volcanic-centers"],
  watersheds: ["watersheds"],
};

let {
  mapId,
  viewerLayers = [],
  stage = $bindable("Opening Atlas Studio…"),
  onexport,
  onready,
}: {
  mapId: string;
  viewerLayers?: Pick<MapLayerDefinition, "id" | "name" | "defaultVisible">[];
  stage?: string;
  onexport?: (request: AtlasRenderRequest) => void;
  onready?: (api: {
    refresh: () => void;
    requestRegenerate: () => void;
    toggleHelp: () => void;
    exportView: () => AtlasRenderRequest | null;
  }) => void;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let session = $state<AtlasStudioSessionStatus | null>(null);
let capabilities = $state<AtlasRenderCapabilities | null>(null);
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
let surface = $state<AtlasStudioSurfaceSample | null>(null);
let confirmCache = $state(false);
let showHelp = $state(false);
let viewZoom = $state(1);
let worldMinZoom = $state(0);
let unlisten: UnlistenFn | undefined;
let map: Map | null = null;
let tileSource: XYZ | null = null;
let mapLifecycle: MapLifecycle | undefined;
let opening = false;
let reopenPending = false;
let debounce: ReturnType<typeof setTimeout> | undefined;
let inspectHover: ReturnType<typeof setTimeout> | undefined;
let inspectSeq = 0;
let statusTimer: ReturnType<typeof setInterval> | undefined;
let prefetchTimer: ReturnType<typeof setTimeout> | undefined;
let mountedControls = false;
const renderCompletion = createAtlasRenderCompletionTracker();

function deviceScale() {
  // CPU-rendered 2x tiles cost four times as much for a small interactive
  // sharpness gain. Static exports retain their requested detail.
  return 1;
}

function formatEpoch(offset: number) {
  if (offset === 0) return "at epoch";
  if (offset < 0) return "years before epoch";
  return "years after epoch";
}

function parseEpochYears(raw: string) {
  const digits = raw.replace(/[^\d]/g, "");
  const value = digits ? Number(digits) : 0;
  return Math.min(EPOCH_MAX, value);
}

function clampEpoch(offset: number, step = 1) {
  const snapped = step > 1 ? Math.round(offset / step) * step : Math.round(offset);
  return Math.min(EPOCH_MAX, Math.max(EPOCH_MIN, snapped));
}

function wrapLon(value: number) {
  return ((((value + 180) % 360) + 360) % 360) - 180;
}

function fillWidthZoom(width: number, tileSize: number) {
  return Math.max(0, Math.log2(Math.max(1, width) / Math.max(1, tileSize)));
}

function overviewZoom(width: number, tileSize: number) {
  return Math.max(0, fillWidthZoom(width, tileSize) - 1);
}

function applyWorldConstraints() {
  const container = host;
  const status = session;
  worldMinZoom = 0;
  map?.getView().setMinZoom(0);
  if (!container || !status) return 0;
  return overviewZoom(container.clientWidth, status.tileSize);
}

function mapCenterLonLat(): [number, number] {
  const center = map?.getView().getCenter();
  return center ? (toLonLat(center) as [number, number]) : [0, 20];
}

function mapZoom() {
  return map?.getView().getZoom() ?? 0;
}

function mapLonLatExtent(): [number, number, number, number] {
  if (!map) return [-180, -85, 180, 85];
  const size = map.getSize();
  if (!size) return [-180, -85, 180, 85];
  return transformExtent(map.getView().calculateExtent(size), "EPSG:3857", "EPSG:4326") as [
    number,
    number,
    number,
    number,
  ];
}

function viewerLayerName(atlasLayerId: string): string | null {
  const aliases = VIEWER_ROLE_ALIASES[atlasLayerId];
  if (!aliases || viewerLayers.length === 0) return null;
  const matched = viewerLayers.find((layer) => aliases.includes(layer.id));
  return matched?.name ?? null;
}

function viewerLayerEnabled(atlasLayerId: string): boolean | null {
  const aliases = VIEWER_ROLE_ALIASES[atlasLayerId];
  if (!aliases || viewerLayers.length === 0) return null;
  const matched = viewerLayers.filter((layer) => aliases.includes(layer.id));
  if (matched.length === 0) return null;
  // Hidden physical-map layers should not disable Atlas defaults.
  return matched.some((layer) => layer.defaultVisible) ? true : null;
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

function studioRequest() {
  return {
    schemaVersion: 1,
    mapEntityId: mapId,
    offsetYears,
    algorithmVersion: ATLAS_DETAIL_ALGORITHM_VERSION,
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
      name: viewerLayerName(layer.id) ?? layer.name,
      enabled: viewerLayerEnabled(layer.id) ?? (layer.defaultVisible || isAtlasLayerEnabledByDefault(layer.id)),
    }));
}

async function openSession() {
  if (opening) {
    reopenPending = true;
    return;
  }
  opening = true;
  reopenPending = false;
  loading = true;
  error = "";
  stale = "";
  hits = [];
  stage = "Snapshotting…";
  try {
    if (!capabilities) await loadCapabilities();
    const previous = session;
    const keepCenter = map ? mapCenterLonLat() : null;
    const keepZoom = map?.getView().getZoom();
    const next = await project.atlasStudioOpen(studioRequest(), deviceScale());
    session = next;
    styleId = next.styleId;
    offsetYears = next.offsetYears;
    if (map && tileSource) {
      stage = "Updating map…";
      configureTileSource(tileSource, next);
      watchRenderCompletion(next, () => tileSource?.refresh());
    } else {
      stage = "Mounting map…";
      mapLifecycle?.dispose();
      mapLifecycle = undefined;
      map = null;
      tileSource = null;
      const overview = applyWorldConstraints();
      const center: [number, number] = keepCenter ?? [0, 20];
      mountMap(next, { center, zoom: keepZoom ?? overview });
    }
    if (previous && previous.sessionToken !== next.sessionToken) {
      void project.atlasStudioClose(previous.sessionToken).catch(() => undefined);
    }
    queueMicrotask(() => map?.updateSize());
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
    loading = false;
  } finally {
    opening = false;
    if (reopenPending) queueMicrotask(() => void openSession());
  }
}

function scheduleSession() {
  if (!mountedControls) return;
  if (debounce) clearTimeout(debounce);
  debounce = setTimeout(() => void openSession(), 300);
}

function configureTileSource(source: XYZ, status: AtlasStudioSessionStatus) {
  source.setTileUrlFunction((tileCoordinate) => {
    if (!tileCoordinate) return undefined;
    const [z, x, y] = tileCoordinate;
    const width = 2 ** z;
    const wrappedX = ((x % width) + width) % width;
    const url = status.tileUrlTemplate
      .replace("{z}", String(z))
      .replace("{x}", String(wrappedX))
      .replace("{y}", String(y));
    return tileUrlAllowed(url, status.sessionToken) ? url : undefined;
  });
}

function watchRenderCompletion(status: AtlasStudioSessionStatus, prepare: () => void = () => {}) {
  const target = map;
  if (!target) return;
  renderCompletion.watch(
    (complete) => target.once("rendercomplete", complete),
    () => {
      prepare();
      target.render();
    },
    () => session?.sessionToken === status.sessionToken,
    () => {
      loading = false;
      stage = "Ready";
      schedulePrefetch(status);
    },
  );
}

function mountMap(status: AtlasStudioSessionStatus, initial?: { center: [number, number]; zoom: number }) {
  const container = host;
  if (!container) return;
  tileSource = new XYZ({
    projection: "EPSG:3857",
    tileSize: status.tileSize,
    minZoom: 0,
    maxZoom: status.maxZoom,
    wrapX: true,
  });
  configureTileSource(tileSource, status);
  try {
    map = new Map({
      target: container,
      layers: [new TileLayer({ source: tileSource, preload: 1 })],
      view: new View({
        projection: "EPSG:3857",
        center: fromLonLat(initial?.center ?? [0, 20]),
        zoom: initial?.zoom ?? applyWorldConstraints(),
        minZoom: 0,
        maxZoom: status.maxZoom,
        multiWorld: true,
        constrainResolution: false,
      }),
      controls: [],
      interactions: defaultInteractions({ altShiftDragRotate: false, pinchRotate: false }),
    });
  } catch (cause) {
    error = cause instanceof Error ? cause.message : "OpenLayers failed to create the Atlas view.";
    loading = false;
    return;
  }
  map.on("pointermove", (event) => {
    const [longitude, latitude] = toLonLat(event.coordinate);
    cursor = `${longitude.toFixed(4)}°, ${latitude.toFixed(4)}°`;
    scheduleInspect(longitude, latitude);
  });
  map.on("singleclick", (event) => {
    if (inspectHover) clearTimeout(inspectHover);
    const [longitude, latitude] = toLonLat(event.coordinate);
    inspectAt(longitude, latitude);
  });
  map.on("moveend", () => {
    if (!map) return;
    const zoom = mapZoom();
    if (viewZoom !== zoom) viewZoom = zoom;
    schedulePrefetch(status);
  });
  watchRenderCompletion(status);
  mapLifecycle = bindMapLifecycle(map, container, () => applyWorldConstraints());
}

function schedulePrefetch(status: AtlasStudioSessionStatus) {
  if (prefetchTimer) clearTimeout(prefetchTimer);
  prefetchTimer = setTimeout(() => prefetchRing(status), 200);
}

function prefetchRing(status: AtlasStudioSessionStatus) {
  if (!map || !session || session.sessionToken !== status.sessionToken) return;
  const z = Math.min(status.maxZoom, Math.max(0, Math.floor(mapZoom())));
  const [west, south, east, north] = mapLonLatExtent();
  const n = 2 ** z;
  const lonToX = (lon: number) => Math.floor(((lon + 180) / 360) * n);
  const latToY = (lat: number) => {
    const sin = Math.sin((lat * Math.PI) / 180);
    const y = 0.5 - Math.log((1 + sin) / (1 - sin)) / (4 * Math.PI);
    return Math.floor(Math.min(n - 1, Math.max(0, y * n)));
  };
  const visibleMinX = lonToX(west);
  const visibleMaxX = lonToX(east);
  const visibleMinY = Math.max(0, latToY(north));
  const visibleMaxY = Math.min(n - 1, latToY(south));
  const minX = visibleMinX - 1;
  const maxX = visibleMaxX + 1;
  const minY = Math.max(0, visibleMinY - 1);
  const maxY = Math.min(n - 1, visibleMaxY + 1);
  const template = status.tileUrlTemplate;
  let requested = 0;
  for (let x = minX; x <= maxX; x += 1) {
    const wrapped = ((x % n) + n) % n;
    for (let y = minY; y <= maxY; y += 1) {
      if (x >= visibleMinX && x <= visibleMaxX && y >= visibleMinY && y <= visibleMaxY) continue;
      const url = `${template.replace("{z}", String(z)).replace("{x}", String(wrapped)).replace("{y}", String(y))}&priority=prefetch`;
      void fetch(url).catch(() => undefined);
      requested += 1;
      if (requested >= 8) return;
    }
  }
}

function currentViewExportHeight(west: number, south: number, east: number, north: number, widthPx: number) {
  const latSpan = Math.max(1, north - south);
  let lonSpan = (east - west + 360_000_000) % 360_000_000;
  if (lonSpan === 0) lonSpan = 360_000_000;
  return Math.max(256, Math.min(2048, Math.round((widthPx * latSpan) / Math.max(1, lonSpan))));
}

function isWorldOverviewView() {
  if (!map) return true;
  return mapZoom() <= applyWorldConstraints() + 0.05;
}

function worldExportRequest(): AtlasRenderRequest {
  return {
    schemaVersion: 1,
    offsetYears,
    algorithmVersion: ATLAS_DETAIL_ALGORITHM_VERSION,
    level: "detailed",
    variant: 0,
    styleId,
    widthPx: 2048,
    heightPx: 1024,
    dpi: 72,
    format: "png",
    projection: "equirectangular",
    extent: {
      westLonMicro: -180_000_000,
      southLatMicro: -90_000_000,
      eastLonMicro: 180_000_000,
      northLatMicro: 90_000_000,
    },
    unlockAspect: false,
    activeLayerIds: layers.filter((layer) => layer.enabled).map((layer) => layer.id),
    timeKind,
    authoredYear: timeKind === "calendar-year" ? authoredYear : null,
    bindingRevision: null,
  };
}

function currentViewExport(): AtlasRenderRequest | null {
  if (!map) return null;
  if (isWorldOverviewView()) return worldExportRequest();
  const [westDegrees, southDegrees, eastDegrees, northDegrees] = mapLonLatExtent();
  const west = toMicro(westDegrees);
  const east = toMicro(eastDegrees);
  const south = Math.max(-85_051_129, toMicro(southDegrees));
  const north = Math.min(85_051_129, toMicro(northDegrees));
  const widthPx = 2048;
  const heightPx = currentViewExportHeight(west, south, east, north, widthPx);
  return {
    schemaVersion: 1,
    offsetYears,
    algorithmVersion: ATLAS_DETAIL_ALGORITHM_VERSION,
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

function setViewZoom(next: number) {
  const zoom = Math.max(worldMinZoom, Math.min(session?.maxZoom ?? 8, next));
  viewZoom = zoom;
  if (!map) return;
  map.getView().setZoom(zoom);
}

function shiftMap(longitudeDegrees: number, latitudeDegrees = 0) {
  if (!map) return;
  const reduced = prefersReducedMotion();
  const current = mapCenterLonLat();
  const center: [number, number] = [
    wrapLon(current[0] + longitudeDegrees),
    Math.max(-85, Math.min(85, current[1] + latitudeDegrees)),
  ] as [number, number];
  if (reduced) map.getView().setCenter(fromLonLat(center));
  else map.getView().animate({ center: fromLonLat(center), duration: 250 });
}

function resetView() {
  const zoom = applyWorldConstraints();
  viewZoom = zoom;
  map?.getView().setCenter(fromLonLat([0, 20]));
  map?.getView().setZoom(zoom);
}

function titleCase(value: string) {
  return value ? `${value[0].toUpperCase()}${value.slice(1)}` : value;
}

function formatMetres(mm: number) {
  const metres = mm / 1000;
  const abs = Math.abs(metres);
  const text = abs >= 100 ? abs.toFixed(0) : abs.toFixed(1);
  return `${metres < 0 ? "−" : ""}${text} m`;
}

function formatElevation(elevationMm: number, waterSurfaceMm: number, surface: string) {
  const relative = elevationMm - waterSurfaceMm;
  const height = formatMetres(relative);
  if (surface === "ocean" || surface === "lake") {
    return relative < 0 ? `${formatMetres(-relative)} below water` : `${height} at water`;
  }
  return `${height} above water`;
}

function formatTemperature(centiC: number) {
  return `${(centiC / 100).toFixed(1)} °C`;
}

function formatWind(eastMilli: number, northMilli: number) {
  const speed = Math.hypot(eastMilli, northMilli);
  const strength = speed < 400 ? "light" : speed < 1_200 ? "moderate" : "strong";
  return `${strength} east ${(eastMilli / 1000).toFixed(1)}, north ${(northMilli / 1000).toFixed(1)}`;
}

function formatCurrent(eastMilli: number, northMilli: number) {
  const speed = Math.hypot(eastMilli, northMilli);
  if (speed < 1) return "none";
  return `annual east ${(eastMilli / 1000).toFixed(2)}, north ${(northMilli / 1000).toFixed(2)}`;
}

function formatDivergence(ppm: number) {
  if (ppm < -20_000) return "Converging";
  if (ppm > 20_000) return "Diverging";
  return "Neutral";
}

function inspectAt(lng: number, lat: number) {
  const token = session?.sessionToken;
  if (!token || !map) return;
  const seq = ++inspectSeq;
  void project
    .atlasStudioInspect(token, toMicro(lng), toMicro(lat), Math.floor(mapZoom()))
    .then((next) => {
      if (seq !== inspectSeq) return;
      hits = next.hits;
      surface = next.surface;
    })
    .catch(() => {
      if (seq !== inspectSeq) return;
      hits = [];
    });
}

function scheduleInspect(lng: number, lat: number) {
  if (inspectHover) clearTimeout(inspectHover);
  inspectHover = setTimeout(() => inspectAt(lng, lat), 240);
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
    const resolution = map.getView().getResolution() ?? 1;
    const center = map.getView().getCenter() ?? [0, 0];
    const next = [center[0] + dx * resolution, center[1] - dy * resolution];
    if (animate) map.getView().animate({ center: next, duration: 200 });
    else map.getView().setCenter(next);
  } else if (event.key === "+" || event.key === "=") {
    event.preventDefault();
    map.getView().animate({ zoom: mapZoom() + 1, duration: reduced ? 0 : 200 });
  } else if (event.key === "-" || event.key === "_") {
    event.preventDefault();
    map.getView().animate({ zoom: mapZoom() - 1, duration: reduced ? 0 : 200 });
  } else if (event.key === "0" || event.key === "Home") {
    event.preventDefault();
    map.getView().setCenter(fromLonLat([0, 20]));
    map.getView().setZoom(applyWorldConstraints());
  } else if (event.key === "Enter") {
    event.preventDefault();
    const center = mapCenterLonLat();
    inspectAt(center[0], center[1]);
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

function setOffsetYears(next: number) {
  offsetYears = clampEpoch(next, EPOCH_STEP);
  scheduleSession();
}

function setOffsetYearsAbs(raw: string) {
  const magnitude = parseEpochYears(raw);
  setOffsetYears(offsetYears < 0 ? -magnitude : magnitude);
}

async function applyPreset(id: string) {
  if (!id) return;
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
  onready?.({
    refresh: () => void openSession(),
    requestRegenerate: () => {
      confirmCache = true;
    },
    toggleHelp: () => {
      showHelp = !showHelp;
    },
    exportView: () => currentViewExport(),
  });
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
  queueMicrotask(() => map?.updateSize());
});

$effect(() => {
  const node = host;
  if (!node) return;
  const handler = (event: KeyboardEvent) => onViewportKey(event);
  node.addEventListener("keydown", handler);
  return () => node.removeEventListener("keydown", handler);
});

onDestroy(() => {
  renderCompletion.invalidate();
  unlisten?.();
  if (debounce) clearTimeout(debounce);
  if (inspectHover) clearTimeout(inspectHover);
  if (statusTimer) clearInterval(statusTimer);
  if (prefetchTimer) clearTimeout(prefetchTimer);
  mapLifecycle?.dispose();
  mapLifecycle = undefined;
  map = null;
  tileSource = null;
  if (session) {
    void project.atlasStudioClose(session.sessionToken).catch(() => undefined);
  }
});
</script>

<section class="studio" aria-label="Atlas Studio">
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
            <option value={id}>{styleLabel(id)}</option>
          {/each}
        </select>
      </label>
      <div class="epoch-control" aria-label="World epoch">
        <input
          type="range"
          min={EPOCH_MIN}
          max={EPOCH_MAX}
          step={EPOCH_STEP}
          value={offsetYears}
          aria-label="Epoch offset"
          disabled={timeKind === "calendar-year"}
          oninput={(event) => setOffsetYears(clampEpoch(Number(event.currentTarget.value), EPOCH_STEP))} />
        <input
          class="epoch-year"
          type="text"
          inputmode="numeric"
          autocomplete="off"
          spellcheck="false"
          value={Math.abs(offsetYears).toLocaleString("en-US")}
          aria-label="Years from epoch"
          disabled={timeKind === "calendar-year"}
          onchange={(event) => setOffsetYearsAbs(event.currentTarget.value)} />
        <span>{formatEpoch(offsetYears)}</span>
      </div>
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
      <MapLayerVisibilityList
        variant="studio"
        {layers}
        onToggle={(index) => {
          layers[index].enabled = !layers[index].enabled;
          layers = layers;
          scheduleSession();
        }} />
      <section class="place" aria-label="Place" aria-live="polite">
        <strong>Place</strong>
        <dl>
          <div>
            <dt>Coordinates</dt>
            <dd>{cursor}</dd>
          </div>
          {#if surface}
            <div>
              <dt>Elevation</dt>
              <dd>{formatElevation(surface.elevationMm, surface.waterSurfaceMm, surface.surface)}</dd>
            </div>
            <div>
              <dt>Temperature</dt>
              <dd>{formatTemperature(surface.temperatureCentiC)}</dd>
            </div>
            <div>
              <dt>Northern-summer solstice</dt>
              <dd>{formatTemperature(surface.temperatureNhSummerCentiC)}</dd>
            </div>
            <div>
              <dt>Northern-winter solstice</dt>
              <dd>{formatTemperature(surface.temperatureNhWinterCentiC)}</dd>
            </div>
            <div>
              <dt>Annual range</dt>
              <dd>{formatTemperature(surface.seasonalRangeCentiC)}</dd>
            </div>
            <div>
              <dt>Freeze</dt>
              <dd>
                {surface.freeze === "permanent" ? "Permanent" : surface.freeze === "seasonal" ? "Seasonal" : "None"}
              </dd>
            </div>
            <div>
              <dt>Climate</dt>
              <dd>{titleCase(surface.climate)}</dd>
            </div>
            <div>
              <dt>Prevailing wind</dt>
              <dd>{formatWind(surface.windEastMilli, surface.windNorthMilli)}</dd>
            </div>
            <div>
              <dt>Northern-summer wind</dt>
              <dd>{formatWind(surface.windEastNhSummerMilli, surface.windNorthNhSummerMilli)}</dd>
            </div>
            <div>
              <dt>Northern-winter wind</dt>
              <dd>{formatWind(surface.windEastNhWinterMilli, surface.windNorthNhWinterMilli)}</dd>
            </div>
            <div>
              <dt>Circulation</dt>
              <dd>{titleCase(surface.windBand)}</dd>
            </div>
            <div>
              <dt>Northern-summer circulation</dt>
              <dd>{titleCase(surface.windBandNhSummer)}</dd>
            </div>
            <div>
              <dt>Northern-winter circulation</dt>
              <dd>{titleCase(surface.windBandNhWinter)}</dd>
            </div>
            <div>
              <dt>Wind flow</dt>
              <dd>{formatDivergence(surface.windDivergencePpm)}</dd>
            </div>
            <div>
              <dt>Northern-summer wind flow</dt>
              <dd>{formatDivergence(surface.windDivergenceNhSummerPpm)}</dd>
            </div>
            <div>
              <dt>Northern-winter wind flow</dt>
              <dd>{formatDivergence(surface.windDivergenceNhWinterPpm)}</dd>
            </div>
            <div>
              <dt>Surface current</dt>
              <dd>{formatCurrent(surface.currentEastMilli, surface.currentNorthMilli)}</dd>
            </div>
            <div>
              <dt>Rainfall</dt>
              <dd>{surface.precipitationMm.toLocaleString("en-US")} mm/year</dd>
            </div>
            <div>
              <dt>Surface</dt>
              <dd>{titleCase(surface.surface)}</dd>
            </div>
            {#if surface.iceThicknessMm > 0}
              <div>
                <dt>Ice</dt>
                <dd>{formatMetres(surface.iceThicknessMm)}</dd>
              </div>
            {/if}
          {:else}
            <p>Move or click the map to sample this point.</p>
          {/if}
        </dl>
      </section>
      {#if confirmCache}
        <div class="confirm" role="alertdialog" aria-labelledby="atlas-cache-title" aria-describedby="atlas-cache-copy">
          <strong id="atlas-cache-title">Regenerate disposable Atlas cache?</strong>
          <p id="atlas-cache-copy">
            This removes derived Atlas cache files only. It does not change canonical project files.
          </p>
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
            <li>Drag or the pan pad to move the view</li>
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
        {@const diagnostic = explainStudioError(
          stale.includes("atlas.studio.stale") ? stale : `atlas.studio.stale: ${stale}`,
        )}
        <p class="stale" role="status">
          <strong>{diagnostic.title}</strong>
          <span>{diagnostic.action}</span>
          {#if diagnostic.code}<code>{diagnostic.code}</code>{/if}
          <button type="button" onclick={() => void openSession()}>Refresh Atlas</button>
        </p>
      {:else if loading}
        <div class="map-busy" role="status">
          <strong>{stage}</strong>
        </div>
      {/if}
      <div
        class="viewport"
        id="atlas-studio-map"
        bind:this={host}
        tabindex="-1"
        aria-label="Atlas Studio map"
        aria-keyshortcuts="ArrowLeft ArrowRight ArrowUp ArrowDown + - Home Enter Escape">
      </div>
      <MapViewControls
        zoom={viewZoom}
        min={worldMinZoom}
        max={session?.maxZoom ?? 8}
        onzoom={setViewZoom}
        onpan={shiftMap} />
    </div>
  </div>
</section>

<style>
.studio {
  position: relative;
  display: flex;
  min-height: 0;
  width: 100%;
  height: 100%;
  flex: 1;
  background: #0d1b2a;
  color: #edf2ec;
}
.body {
  display: grid;
  grid-template-columns: 240px minmax(0, 1fr);
  min-height: 0;
  height: 100%;
  width: 100%;
}
aside {
  display: grid;
  align-content: start;
  gap: 10px;
  overflow: auto;
  padding: 10px;
  background: #1b2822;
  border-right: 1px solid var(--theme-neutral-border-strong, #405047);
  font: 12px/1.4 system-ui;
}
aside label {
  display: grid;
  gap: 4px;
}
aside select,
aside input[type="number"] {
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  padding: 6px;
  background: #0f1a16;
  color: #edf2ec;
}
.epoch-control {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 8px;
}
.epoch-control input[type="range"],
aside input[type="range"] {
  width: 140px;
  accent-color: #d5ab6c;
}
.epoch-year {
  width: 5.4em;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  padding: 4px 5px;
  background: #0f1a16;
  color: #edf2ec;
  font: 12px system-ui;
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.place {
  display: grid;
  gap: 6px;
}
.place dl,
.place p {
  margin: 0;
}
.place div {
  display: grid;
  grid-template-columns: 7.2em minmax(0, 1fr);
  gap: 6px 10px;
  align-items: baseline;
}
.place dt {
  color: var(--theme-neutral-text-muted, #aebdb1);
  font-weight: 500;
}
.place dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
}
.place p {
  color: var(--theme-neutral-text-muted, #aebdb1);
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
.viewport :global(.ol-viewport) {
  width: 100%;
  height: 100%;
}
.error,
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
.stale {
  color: var(--theme-warning-text, #d5ab6c);
}
.error code,
.stale code {
  font:
    11px/1.3 ui-monospace,
    monospace;
  color: #b8c8bc;
}
.map-busy {
  position: absolute;
  z-index: 3;
  inset: 0;
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 0.3rem;
  pointer-events: none;
  color: #f7f0e5;
  text-align: center;
  text-shadow: 0 1px 8px rgb(0 0 0 / 75%);
}
.map-busy strong {
  font: 600 1.05rem/1.3 inherit;
}
.help,
.confirm {
  display: grid;
  gap: 6px;
  padding: 8px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 8px;
}
.help li,
.confirm p {
  margin: 0;
  color: #b8c8bc;
}
.help ul {
  margin: 0;
  padding-left: 1.2em;
}
aside small {
  color: var(--theme-neutral-text-muted, #aebdb1);
}
button:focus-visible,
select:focus-visible,
input:focus-visible,
.viewport:focus-visible {
  outline: 2px solid var(--theme-success-border, #edf2ec);
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
