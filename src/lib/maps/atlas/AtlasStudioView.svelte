<script lang="ts">
import { onDestroy, onMount, tick, untrack } from "svelte";
import { Info, Layers, MapPin, Pencil, Route, Search, X } from "@lucide/svelte";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import Map from "ol/Map.js";
import View from "ol/View.js";
import Feature from "ol/Feature.js";
import LineString from "ol/geom/LineString.js";
import MultiLineString from "ol/geom/MultiLineString.js";
import Point from "ol/geom/Point.js";
import TileLayer from "ol/layer/Tile.js";
import VectorLayer from "ol/layer/Vector.js";
import VectorSource from "ol/source/Vector.js";
import XYZ from "ol/source/XYZ.js";
import CircleStyle from "ol/style/Circle.js";
import Fill from "ol/style/Fill.js";
import Stroke from "ol/style/Stroke.js";
import Style from "ol/style/Style.js";
import Text from "ol/style/Text.js";
import Draw from "ol/interaction/Draw.js";
import { defaults as defaultInteractions } from "ol/interaction/defaults.js";
import GeoJSON from "ol/format/GeoJSON.js";
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
  type MapPin as ProjectMapPin,
} from "$lib/project/client";
import type { MapLayerDefinition } from "../native-vector/types";
import MapViewControls from "../native-vector/MapViewControls.svelte";
import MapLocationLinkPanel from "../native-vector/MapLocationLinkPanel.svelte";
import FindPlacePanel from "../physical/FindPlacePanel.svelte";
import RouteSuggestPanel from "../physical/RouteSuggestPanel.svelte";
import {
  existingRoutePolylines,
  routeEndpoints,
  routeFeatureFromSuggestion,
  routeGeometryFromSuggestion,
  toMicrodegrees,
} from "../physical/route-suggest.ts";
import type {
  AtlasRegionProposal,
  FindPlaceCandidate,
  FindPlaceQuery,
  FindPlaceResult,
  RouteSuggestion,
  RouteSuggestResult,
} from "$lib/project/types";
import { PHYSICAL_PROVIDER, type MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import {
  PHYSICAL_COORDINATE_SPACE,
  authoredToNormalized,
  mapPositions,
  normalizedToAuthored,
  wrapGeographicPosition,
  wrapLongitude,
} from "../editor/coordinate-space";
import { bindMapLifecycle, type MapLifecycle } from "../openlayers/lifecycle";
import { createAtlasRenderCompletionTracker } from "./render-completion.ts";
import MapLayerVisibilityList from "../MapLayerVisibilityList.svelte";
import { ATLAS_DETAIL_ALGORITHM_VERSION, atlasStyleLabel, isAtlasLayerEnabledByDefault } from "./constants.ts";
import AtlasOverlayPanel from "./AtlasOverlayPanel.svelte";
import { simplifyFreehandGeometry } from "../native-vector/geometry.ts";
import { daenaProperties, featureName, type VectorFeature } from "../native-vector/types.ts";
import {
  combineModeFromModifiers,
  combineSelection,
  displayLandmassGeometry,
  featureFromSelection,
  hoverCoversPoint,
  hoverIsRedundant,
  hoverPreviewFeature,
  invertSelection,
  occupiedGeometries,
  HOVER_PREVIEW_ID,
  LANDMASS_HOVER_DELAY_MS,
  previewFeature,
  selectionFromProposal,
  selectionMinusOccupied,
  withGeometry,
  type LandmassSelection,
} from "../physical/landmass-selection.ts";
import {
  overlayFamilyLabel,
  overlayFeaturesForLayer,
  overlayLayerFamily,
  overlayPlaceRole,
  type AtlasOverlayAuthoring,
  type OverlayDrawTool,
} from "./overlay-family.ts";

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
  overlayAuthoring = null,
  stage = $bindable("Opening Atlas Studio…"),
  onexport,
  onready,
}: {
  mapId: string;
  viewerLayers?: Pick<MapLayerDefinition, "id" | "name" | "defaultVisible">[];
  overlayAuthoring?: AtlasOverlayAuthoring | null;
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
let picked = $state<{ lng: number; lat: number; x: number; y: number; flip: boolean } | null>(null);
let placeModal = $state(false);
let guideOpen = $state(false);
let pickedSample = $state<{ lng: number; lat: number; surface: AtlasStudioSurfaceSample } | null>(null);
let pickedHits = $state<AtlasStudioInspectHit[]>([]);
let modalSample = $state<{ lng: number; lat: number; surface: AtlasStudioSurfaceSample } | null>(null);
let sampledPoint = $state<{ lng: number; lat: number } | null>(null);
let pinSeq = 0;
const modalSurface = $derived(modalSample?.surface ?? null);
const modalCoords = $derived(modalSample ? `${modalSample.lng.toFixed(4)}°, ${modalSample.lat.toFixed(4)}°` : "");

const FIELD_GUIDE: Array<{ term: string; help: string }> = [
  {
    term: "Solstice temperatures",
    help: "Temperature at the northern-summer and northern-winter solstices: two representative seasonal states, not a daily forecast.",
  },
  {
    term: "Annual range",
    help: "How much the temperature typically swings across the year at this spot.",
  },
  {
    term: "Freeze",
    help: "Permanent means the land stays below freezing all year; seasonal means it freezes for part of the year and thaws.",
  },
  {
    term: "Prevailing wind",
    help: "The average wind direction and strength here. Think prevailing conditions, not today's weather.",
  },
  {
    term: "Circulation",
    help: "Which planet-scale air band sits overhead: Hadley (tropical), Ferrel (temperate), or polar.",
  },
  {
    term: "Wind flow",
    help: "Converging means winds flow together here (air rises, often wetter); diverging means they spread apart (often drier); neutral is neither.",
  },
  {
    term: "Rainfall",
    help: "Yearly precipitation, with the solstice split showing which season brings the rain.",
  },
  {
    term: "Humidity",
    help: "Remaining moisture in the air versus local saturation, as a percent. It is not a weather-station humidity reading.",
  },
  {
    term: "Aridity",
    help: "How much evaporative demand goes unmet by rain: humid, sub-humid, semi-arid, or arid land.",
  },
  {
    term: "Surface current",
    help: "Year-round surface-ocean drift near this coast: direction and strength of the major gyres, not a shipping chart.",
  },
  {
    term: "Biome",
    help: "A derived reading of temperature, rain, humidity, aridity, and elevation — an interpretation for worldbuilding, not painted decoration.",
  },
  {
    term: "Storms",
    help: "Storm climatology: where formation and tracks are plausible, and how exposed this spot is. It is not a forecast, and only materialized storms become world history.",
  },
  {
    term: "Elevation and surface",
    help: "Height relative to the water surface, and whether this spot reads as ocean, lake, or land.",
  },
];
let placeDialog = $state<HTMLDivElement | null>(null);
let placeOpener: HTMLElement | null = null;
let linking = $state(false);
let linkArming = $state(false);
let linkAnchor = $state<MapAnchor | null>(null);
let pickPanel = $state<HTMLDivElement | null>(null);
let placeHeading = $state<HTMLElement | null>(null);
let confirmCache = $state(false);
let showHelp = $state(false);
type SidebarPane = "layers" | "find" | "draw" | "inspect";
type FindTopic = "places" | "routes";
let sidebarPane = $state<SidebarPane>("layers");
let findTopic = $state<FindTopic>("places");
const sidebarPanes: Array<{ id: SidebarPane; label: string }> = [
  { id: "layers", label: "Layers" },
  { id: "find", label: "Find" },
  { id: "draw", label: "Draw" },
  { id: "inspect", label: "Inspect" },
];
let viewZoom = $state(1);
let worldMinZoom = $state(0);
let findPlaceResult = $state<FindPlaceResult | null>(null);
let findPlaceSelectedId = $state<number | null>(null);
let findPlaceSearching = $state(false);
let findPlaceError = $state("");
let routeResult = $state<RouteSuggestResult | null>(null);
let routeSelectedId = $state<number | null>(null);
let routeSearching = $state(false);
let routeError = $state("");
let routingArming = $state(false);
let routingStart = $state<[number, number] | null>(null);
let routeRequest = 0;
let unlisten: UnlistenFn | undefined;
let map: Map | null = null;
let tileSource: XYZ | null = null;
let findPlaceSource: VectorSource | null = null;
let findPlaceLayer: VectorLayer | null = null;
let namedWaterSource: VectorSource | null = null;
let namedWaterLayer: VectorLayer | null = null;
let namedWaterPins = $state<ProjectMapPin[]>([]);
let namedPlacePins = $state<ProjectMapPin[]>([]);
const PHYSICAL_LAKE_FEATURE_KIND = "physical-lake";
const PHYSICAL_RIVER_FEATURE_KIND = "physical-river";
const PHYSICAL_LANDMASS_FEATURE_KIND = "physical-landmass";
let routeSource: VectorSource | null = null;
let routeLayer: VectorLayer | null = null;
let overlaySource: VectorSource | null = null;
let overlayLayer: VectorLayer | null = null;
let landmassSource: VectorSource | null = null;
let landmassLayer: VectorLayer | null = null;
let overlayDraw: Draw | null = null;
let overlayTool = $state<OverlayDrawTool>("select");
let overlaySelectedId = $state<string | null>(null);
let overlayLinkSeedName = $state("");
let overlayLinkSeedRole = $state("");
let overlayDetectHint = $state("");
let overlayDetectBusy = false;
let landmassRequest = 0;
let overlaySyncing = false;
let overlayCreateName = $state("");
let landmassSelection = $state<LandmassSelection | null>(null);
let includeOccupied = $state(false);
let landmassHover = $state<LandmassSelection | null>(null);
let landmassHoverTimer: ReturnType<typeof setTimeout> | undefined;
let landmassHoverRequest = 0;
let landGeometryCache: {
  mapId: string;
  epoch: number;
  generation: number;
  proposal: AtlasRegionProposal;
} | null = null;
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
const overlayGeoJson = new GeoJSON({
  dataProjection: "EPSG:4326",
  featureProjection: "EPSG:3857",
});
const overlayGeoJsonOptions = {
  dataProjection: "EPSG:4326",
  featureProjection: "EPSG:3857",
} as const;

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
  return wrapLongitude(value);
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
  if (hit.kind === PHYSICAL_LAKE_FEATURE_KIND || hit.kind === PHYSICAL_RIVER_FEATURE_KIND) {
    return "Generated water at this epoch. Naming creates a Place; the geometry stays derived.";
  }
  if (hit.kind === PHYSICAL_LANDMASS_FEATURE_KIND) {
    return "Generated land at this epoch. Naming creates a Place; the geometry stays derived.";
  }
  if (hit.kind === "derived-tributary") {
    return "Atlas-only derived drainage. It is not canonical Physical Map data and cannot be edited or promoted from Studio.";
  }
  if (hit.derived) {
    return "Presentation overlay from the captured Atlas snapshot. It is not a Physical Map edit.";
  }
  return "Authored or semantic map feature from the captured project snapshot.";
}

function claimableHit(hit: AtlasStudioInspectHit) {
  return (
    hit.kind === PHYSICAL_LAKE_FEATURE_KIND ||
    hit.kind === PHYSICAL_RIVER_FEATURE_KIND ||
    hit.kind === PHYSICAL_LANDMASS_FEATURE_KIND
  );
}

function namedPlaceLabel(hit: AtlasStudioInspectHit) {
  const pin = namedPlacePins.find(
    (entry) => entry.featureId === hit.id || (entry.featureKind === hit.kind && entry.featureId === hit.id),
  );
  return pin?.label?.trim() || hit.label || hit.id;
}

function styleLabel(id: string) {
  return atlasStyleLabel(id);
}

const studioLayerBook = $derived([
  {
    id: "overlays",
    label: "Overlays",
    layers:
      overlayAuthoring?.layers.map((layer) => ({
        id: layer.id,
        name: layer.name,
        enabled: layer.defaultVisible,
        meta: overlayFamilyLabel(overlayLayerFamily(layer)),
        overlay: true,
      })) ?? [],
  },
  {
    id: "physical",
    label: "Physical",
    layers: layers.map((layer) => ({
      id: layer.id,
      name: layer.name,
      enabled: layer.enabled,
    })),
  },
]);

function atlasLayerIds() {
  return [
    ...layers.filter((layer) => layer.enabled).map((layer) => layer.id),
    ...(overlayAuthoring?.layers.filter((layer) => layer.defaultVisible).map((layer) => layer.id) ?? []),
  ];
}

function studioRequest() {
  return {
    schemaVersion: 1,
    mapEntityId: mapId,
    offsetYears,
    algorithmVersion: ATLAS_DETAIL_ALGORITHM_VERSION,
    level: "standard" as const,
    variant: 0,
    styleId,
    activeLayerIds: atlasLayerIds(),
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
    .filter((layer) => layer.id !== "frame" && layer.role !== "vector")
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
      findPlaceLayer = null;
      findPlaceSource = null;
      namedWaterLayer = null;
      namedWaterSource = null;
      routeLayer = null;
      routeSource = null;
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
      layers: [
        new TileLayer({ source: tileSource, preload: 1 }),
        authoredOverlayLayer(),
        landmassSelectionLayer(),
        routeOverlayLayer(),
        findPlaceOverlayLayer(),
        namedWaterOverlayLayer(),
      ],
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
    if (overlayTool === "landmass") {
      if (!event.dragging) scheduleLandmassHover(longitude, latitude);
    } else {
      clearLandmassHover();
      if (!picked) scheduleInspect(longitude, latitude);
    }
  });
  map.getViewport().addEventListener("pointerleave", () => clearLandmassHover());
  map.on("singleclick", (event) => {
    if (inspectHover) clearTimeout(inspectHover);
    const suggestionId = map?.forEachFeatureAtPixel(event.pixel, (feature) => feature.get("suggestionId"));
    if (typeof suggestionId === "number" && !routingArming) {
      const suggestion = routeResult?.suggestions.find((item: RouteSuggestion) => item.id === suggestionId);
      if (suggestion) selectRoute(suggestion);
      return;
    }
    const candidateId = map?.forEachFeatureAtPixel(event.pixel, (feature) => feature.get("candidateId"));
    if (typeof candidateId === "number") {
      const candidate = findPlaceResult?.candidates.find((item: FindPlaceCandidate) => item.id === candidateId);
      if (candidate) selectFindPlace(candidate);
      return;
    }
    const [longitude, latitude] = toLonLat(event.coordinate);
    if (routingArming) {
      pickRoutingPoint(longitude, latitude);
      return;
    }
    if (linkArming) {
      openLinkPanel(longitude, latitude);
      inspectAt(longitude, latitude);
      return;
    }
    if (overlayTool === "landmass") {
      clearLandmassHover();
      const origin = event.originalEvent;
      void selectDetectedRegion(
        longitude,
        latitude,
        Boolean(origin && "shiftKey" in origin && origin.shiftKey),
        Boolean(origin && "altKey" in origin && origin.altKey),
      );
      return;
    }
    if (overlayTool === "select") {
      const regionId = map?.forEachFeatureAtPixel(event.pixel, (feature) => {
        const id = feature.getId();
        return typeof id === "string" ? id : null;
      });
      if (typeof regionId === "string") {
        overlaySelectedId = regionId;
        return;
      }
    }
    if (overlayTool === "freehand" || overlayTool === "polygon") return;
    pickedSample = null;
    pickedHits = [];
    setPicked(longitude, latitude);
    inspectAt(longitude, latitude, true);
  });
  map.on("moveend", () => {
    if (!map) return;
    const zoom = mapZoom();
    if (viewZoom !== zoom) viewZoom = zoom;
    if (picked) setPicked(picked.lng, picked.lat, false);
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
    activeLayerIds: atlasLayerIds(),
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
    activeLayerIds: atlasLayerIds(),
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

function formatHumidity(ppm: number) {
  return `${Math.round(ppm / 10_000)}% of saturation`;
}

function formatAridity(ppm: number) {
  if (ppm < 200_000) return "humid";
  if (ppm < 500_000) return "sub-humid";
  if (ppm < 800_000) return "semi-arid";
  return "arid";
}

function formatDivergence(ppm: number) {
  if (ppm < -20_000) return "Converging";
  if (ppm > 20_000) return "Diverging";
  return "Neutral";
}

function formatFreeze(value: AtlasStudioSurfaceSample["freeze"]) {
  if (value === "permanent") return "Permanent";
  if (value === "seasonal") return "Seasonal";
  return "None";
}

function formatShare(ppm: number) {
  return `${Math.round(ppm / 10_000)}%`;
}

const SIDEBAR_MIN = 260;
const SIDEBAR_MAX = 560;
const SIDEBAR_DEFAULT = 312;
const SIDEBAR_STORAGE_KEY = "daena:atlas-sidebar-width";

function readSidebarWidth() {
  try {
    const raw = Number(localStorage.getItem(SIDEBAR_STORAGE_KEY));
    if (!Number.isFinite(raw)) return SIDEBAR_DEFAULT;
    return Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, Math.round(raw)));
  } catch {
    return SIDEBAR_DEFAULT;
  }
}

let sidebarWidth = $state(readSidebarWidth());

function setSidebarWidth(next: number) {
  sidebarWidth = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, Math.round(next)));
  try {
    localStorage.setItem(SIDEBAR_STORAGE_KEY, String(sidebarWidth));
  } catch {
    /* ignore quota / private-mode failures */
  }
}

function startSidebarResize(event: PointerEvent) {
  const handle = event.currentTarget;
  if (!(handle instanceof HTMLElement) || event.button !== 0) return;
  event.preventDefault();
  handle.setPointerCapture(event.pointerId);
  const originX = event.clientX;
  const originWidth = sidebarWidth;
  const onMove = (move: PointerEvent) => {
    setSidebarWidth(originWidth + (move.clientX - originX));
  };
  const onUp = () => {
    handle.removeEventListener("pointermove", onMove);
    handle.removeEventListener("pointerup", onUp);
  };
  handle.addEventListener("pointermove", onMove);
  handle.addEventListener("pointerup", onUp);
}

function onSidebarResizeKey(event: KeyboardEvent) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  event.preventDefault();
  setSidebarWidth(sidebarWidth + (event.key === "ArrowRight" ? 16 : -16));
}

function inspectAt(lng: number, lat: number, pin = false) {
  const token = session?.sessionToken;
  if (!token || !map) return;
  const seq = ++inspectSeq;
  if (pin) pinSeq = seq;
  void project
    .atlasStudioInspect(token, toMicro(lng), toMicro(lat), Math.floor(mapZoom()))
    .then((next) => {
      if (seq !== inspectSeq) return;
      hits = next.hits;
      surface = next.surface;
      sampledPoint = { lng, lat };
      if (seq === pinSeq) {
        pickedSample = { lng, lat, surface: next.surface };
        pickedHits = next.hits;
      }
    })
    .catch(() => {
      if (seq !== inspectSeq) return;
      hits = [];
      if (seq === pinSeq) pickedHits = [];
    });
}

function scheduleInspect(lng: number, lat: number) {
  if (inspectHover) clearTimeout(inspectHover);
  inspectHover = setTimeout(() => inspectAt(lng, lat), 240);
}

function setPicked(lng: number, lat: number, focus = true) {
  if (!map) return;
  const pixel = map.getPixelFromCoordinate(fromLonLat([lng, lat]));
  const size = map.getSize() ?? [0, 0];
  const width = size[0] ?? 0;
  const height = size[1] ?? 0;
  const x = Math.min(Math.max(pixel[0], 8), Math.max(8, width - 8));
  const y = Math.min(Math.max(pixel[1], 70), Math.max(70, height - 70));
  picked = { lng, lat, x, y, flip: pixel[0] > width * 0.6 };
  if (focus) void tick().then(() => pickPanel?.focus());
}

function clearPicked(focusMap = false) {
  picked = null;
  if (focusMap) host?.focus();
}

function openPlaceDetails(viaOverlay: boolean) {
  placeOpener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  if (viaOverlay) {
    modalSample = pickedSample;
  } else if (surface && sampledPoint) {
    modalSample = { ...sampledPoint, surface };
  } else {
    modalSample = null;
  }
  placeModal = true;
  guideOpen = false;
  clearPicked(false);
  void tick().then(() => placeDialog?.focus());
}

function closePlaceDetails() {
  placeModal = false;
  guideOpen = false;
  const opener = placeOpener;
  placeOpener = null;
  if (opener?.isConnected) opener.focus();
}

function trapPlaceFocus(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    closePlaceDetails();
    return;
  }
  if (event.key !== "Tab" || !placeDialog) return;
  const focusable = [
    ...placeDialog.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), [href], select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  ];
  if (focusable.length === 0) return;
  const index = focusable.indexOf(document.activeElement as HTMLElement);
  const next = event.shiftKey
    ? index <= 0
      ? focusable.length - 1
      : index - 1
    : index === focusable.length - 1
      ? 0
      : index + 1;
  event.preventDefault();
  focusable[next].focus();
}

function pointAnchorFor(lng: number, lat: number): MapAnchor {
  const [nx, ny] = authoredToNormalized(wrapLon(lng), lat, PHYSICAL_COORDINATE_SPACE);
  return { kind: "point", point: [nx, ny] };
}

function openLinkPanel(lng: number, lat: number) {
  linkAnchor = pointAnchorFor(lng, lat);
  linkArming = false;
  linking = true;
  clearPicked(false);
}

function nameClaimHit(hit: AtlasStudioInspectHit) {
  const point = sampledPoint ?? picked;
  if (!point) return;
  overlayLinkSeedName = "";
  overlayLinkSeedRole = "";
  const [nx, ny] = authoredToNormalized(wrapLon(point.lng), point.lat, PHYSICAL_COORDINATE_SPACE);
  linkAnchor = {
    kind: "provider-feature",
    provider: PHYSICAL_PROVIDER,
    featureKind: hit.kind,
    featureId: hit.id,
    fallbackPoint: [nx, ny],
  };
  linkArming = false;
  linking = true;
  clearPicked(false);
}

function pickNameActions() {
  return pickedHits.flatMap((hit) => {
    if (claimableHit(hit)) {
      const named = namedWaterPins.some((pin) => pin.featureKind === hit.kind && pin.featureId === hit.id);
      return [
        {
          label: named ? `Rename ${namedPlaceLabel(hit)}` : `Name ${namedPlaceLabel(hit)}`,
          run: () => nameClaimHit(hit),
        },
      ];
    }
    if (overlayFeatureForHit(hit.id)) {
      const named = namedPlacePins.some((pin) => pin.featureId === hit.id);
      return [
        {
          label: named ? `Rename ${namedPlaceLabel(hit)}` : `Place ${namedPlaceLabel(hit)}`,
          run: () => void nameOverlayFeature(hit.id),
        },
      ];
    }
    return [];
  });
}

function overlayFeatureForHit(id: string) {
  return overlayAuthoring?.features.find((feature) => feature.id === id) ?? null;
}

async function nameOverlayFeature(featureId: string) {
  const authoring = overlayAuthoring;
  const feature = overlayFeatureForHit(featureId);
  if (!authoring || !feature) return;
  if (authoring.dirty) {
    overlayDetectHint = "Saving overlay…";
    try {
      await authoring.save();
    } catch {
      overlayDetectHint = "Save the overlay before creating a Place.";
      return;
    }
  }
  const positions = feature.geometry.coordinates.flat(Infinity) as number[];
  if (positions.length < 2) {
    overlayDetectHint = "That region has no geometry to bind.";
    return;
  }
  const layer = authoring.layers.find((item) => item.id === feature.properties.daena.layerId);
  overlayLinkSeedName = featureName(feature) ?? "";
  overlayLinkSeedRole = overlayPlaceRole(layer ? overlayLayerFamily(layer) : "custom");
  const [nx, ny] = authoredToNormalized(wrapLon(positions[0]), positions[1], PHYSICAL_COORDINATE_SPACE);
  linkAnchor = {
    kind: "provider-feature",
    provider: PHYSICAL_PROVIDER,
    featureKind: "geojson-feature",
    featureId: feature.id,
    fallbackPoint: [nx, ny],
  };
  overlaySelectedId = feature.id;
  overlayTool = "select";
  overlayDetectHint = "";
  linkArming = false;
  linking = true;
  clearPicked(false);
}

async function onPlaceLinked() {
  await syncNamedWater();
  const anchor = linkAnchor;
  if (anchor?.kind !== "provider-feature" || anchor.featureKind !== "geojson-feature") return;
  const pin = namedPlacePins.find((entry) => entry.featureId === anchor.featureId);
  const label = pin?.label?.trim();
  if (label) overlayAuthoring?.renameFeature(anchor.featureId, label);
}

function namedWaterOverlayLayer() {
  namedWaterSource = new VectorSource();
  namedWaterLayer = new VectorLayer({
    source: namedWaterSource,
    zIndex: 21,
    style: (feature) =>
      new Style({
        image: new CircleStyle({
          radius: 5,
          fill: new Fill({
            color: feature.get("kind") === PHYSICAL_LANDMASS_FEATURE_KIND ? "#d5ab6c" : "#7ec8e3",
          }),
          stroke: new Stroke({ color: "#1b2822", width: 1.2 }),
        }),
        text: new Text({
          text: String(feature.get("label") ?? ""),
          fill: new Fill({ color: "#edf2ec" }),
          stroke: new Stroke({ color: "#1b2822", width: 3 }),
          font: "700 12px system-ui",
          offsetY: -12,
        }),
      }),
  });
  void syncNamedWater();
  return namedWaterLayer;
}

async function syncNamedWater() {
  const pins = await project.listMapPins(mapId).catch(() => []);
  namedPlacePins = pins;
  namedWaterPins = pins.filter(
    (pin) =>
      pin.featureKind === PHYSICAL_LAKE_FEATURE_KIND ||
      pin.featureKind === PHYSICAL_RIVER_FEATURE_KIND ||
      pin.featureKind === PHYSICAL_LANDMASS_FEATURE_KIND,
  );
  namedWaterSource?.clear();
  if (!namedWaterSource) return;
  for (const pin of namedWaterPins) {
    const anchor = pin.anchor as MapAnchor | undefined;
    const fallback =
      anchor?.kind === "provider-feature"
        ? anchor.fallbackPoint
        : pin.bounds[0] != null && pin.bounds[1] != null
          ? ([pin.bounds[0], pin.bounds[1]] as [number, number])
          : null;
    if (!fallback) continue;
    const [lng, lat] = normalizedToAuthored(fallback[0], fallback[1], PHYSICAL_COORDINATE_SPACE);
    const feature = new Feature({
      geometry: new Point(fromLonLat([lng, lat])),
      label: pin.label || pin.role,
      kind: pin.featureKind,
    });
    namedWaterSource.addFeature(feature);
  }
}

function closeLinkPanel(focusMap = false) {
  linking = false;
  if (focusMap) host?.focus();
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
    pickedSample = null;
    setPicked(center[0], center[1]);
    inspectAt(center[0], center[1], true);
  } else if (event.key === "Escape") {
    event.preventDefault();
    if (landmassSelection) {
      clearLandmassSelection();
      return;
    }
    if (routingArming || routeSearching || routeResult || routeError) {
      clearRouting();
      return;
    }
    if (linking) {
      closeLinkPanel(true);
      return;
    }
    if (linkArming) {
      linkArming = false;
      host?.focus();
      return;
    }
    if (overlayTool !== "select") {
      overlayTool = "select";
      overlayDetectHint = "";
      return;
    }
    if (overlaySelectedId) {
      overlaySelectedId = null;
      return;
    }
    clearPicked();
    hits = [];
  } else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
    event.preventDefault();
    if (event.shiftKey) overlayAuthoring?.redo();
    else overlayAuthoring?.undo();
  } else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
    event.preventDefault();
    void overlayAuthoring?.save();
  } else if ((event.key === "Delete" || event.key === "Backspace") && overlaySelectedId) {
    event.preventDefault();
    overlayAuthoring?.deleteFeatures([overlaySelectedId]);
    overlaySelectedId = null;
  } else if (event.key === "?" || event.key.toLowerCase() === "h") {
    if (!event.metaKey && !event.ctrlKey) {
      event.preventDefault();
      toggleStudioHelp();
    }
  }
}

function toggleStudioHelp() {
  showHelp = !showHelp;
  if (showHelp) sidebarPane = "inspect";
}

function sidebarPaneBadge(id: SidebarPane) {
  if (id === "find") {
    return (findPlaceResult?.candidateCount ?? 0) + (routeResult?.suggestionCount ?? 0);
  }
  return 0;
}

function setOffsetYears(next: number) {
  offsetYears = clampEpoch(next, EPOCH_STEP);
  clearFindPlace();
  clearRouting();
  scheduleSession();
}

function hexToRgba(hex: string, alpha: number) {
  const raw = hex.replace("#", "");
  const value =
    raw.length === 3
      ? raw
          .split("")
          .map((part) => part + part)
          .join("")
      : raw;
  const n = Number.parseInt(value, 16);
  if (!Number.isFinite(n)) return `rgba(143, 111, 209, ${alpha})`;
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

function authoredOverlayStyle(feature: Feature) {
  const id = String(feature.getId() ?? "");
  const vector = overlayAuthoring?.features.find((item) => item.id === id);
  const layer = overlayAuthoring?.layers.find((item) => item.id === vector?.properties.daena.layerId);
  const override = vector?.properties.daena.style ?? {};
  const fill = override.fill ?? layer?.style.fill ?? "#8f6fd1";
  const stroke = override.stroke ?? layer?.style.stroke ?? "#5e4893";
  const fillOpacity = (layer?.opacity ?? 1) * (override.fillOpacity ?? layer?.style.fillOpacity ?? 0.35);
  const strokeWidth = override.strokeWidth ?? layer?.style.strokeWidth ?? 1.5;
  const selected = id === overlaySelectedId;
  const name = vector?.properties.daena.name;
  return new Style({
    fill: new Fill({ color: hexToRgba(fill, selected ? Math.min(1, fillOpacity + 0.18) : fillOpacity) }),
    stroke: new Stroke({
      color: selected ? "#f3d39a" : hexToRgba(stroke, override.strokeOpacity ?? layer?.style.strokeOpacity ?? 1),
      width: selected ? Math.max(2.4, strokeWidth) : strokeWidth,
    }),
    text: name
      ? new Text({
          text: name,
          font: "600 12px system-ui",
          fill: new Fill({ color: "#f7f0e5" }),
          stroke: new Stroke({ color: "#0d1b2a", width: 3 }),
        })
      : undefined,
  });
}

function readOlOverlayFeature(feature: VectorFeature): Feature | null {
  try {
    const read = overlayGeoJson.readFeature(
      { type: "Feature", id: feature.id, geometry: feature.geometry, properties: {} },
      overlayGeoJsonOptions,
    );
    const olFeature = (Array.isArray(read) ? read[0] : read) as Feature | undefined;
    if (!olFeature?.getGeometry()) return null;
    olFeature.setId(feature.id);
    return olFeature;
  } catch {
    return null;
  }
}

function paintOverlayFeature(feature: VectorFeature) {
  if (!overlaySource) return;
  const existing = overlaySource.getFeatureById(feature.id);
  if (existing) overlaySource.removeFeature(existing);
  const olFeature = readOlOverlayFeature(feature);
  if (olFeature) overlaySource.addFeature(olFeature);
  overlayLayer?.changed();
}

function authoredOverlayLayer() {
  overlaySource = new VectorSource();
  overlayLayer = new VectorLayer({
    source: overlaySource,
    zIndex: 10,
    style: (feature) => authoredOverlayStyle(feature as Feature),
  });
  syncAuthoredOverlays();
  return overlayLayer;
}

function syncAuthoredOverlays() {
  if (!overlaySource || overlaySyncing) return;
  overlaySyncing = true;
  overlaySource.clear();
  const authoring = overlayAuthoring;
  if (authoring) {
    for (const layer of authoring.layers) {
      if (!layer.defaultVisible) continue;
      for (const feature of overlayFeaturesForLayer(authoring.features, layer.id)) {
        if (feature.geometry.type !== "Polygon" && feature.geometry.type !== "MultiPolygon") continue;
        const olFeature = readOlOverlayFeature(feature);
        if (olFeature) overlaySource.addFeature(olFeature);
      }
    }
  }
  overlaySyncing = false;
  overlayLayer?.changed();
}

function clearOverlayDraw() {
  if (overlayDraw && map) map.removeInteraction(overlayDraw);
  overlayDraw = null;
}

function overlayAuthoredPosition(position: number[]): number[] {
  let x = position[0];
  let y = position[1];
  if (Math.abs(x) > 540 || Math.abs(y) > 90) {
    [x, y] = toLonLat([x, y]);
  }
  return wrapGeographicPosition([x, y]);
}

function vectorGeometryFromOl(feature: Feature): VectorFeature["geometry"] | null {
  const written = overlayGeoJson.writeFeatureObject(feature, overlayGeoJsonOptions);
  const geometry = written.geometry;
  if (
    !geometry ||
    (geometry.type !== "Polygon" && geometry.type !== "MultiPolygon" && geometry.type !== "LineString")
  ) {
    return null;
  }
  return mapPositions(geometry as VectorFeature["geometry"], overlayAuthoredPosition);
}

function commitDrawnFeature(olFeature: Feature) {
  const authoring = overlayAuthoring;
  const layerId = authoring?.activeLayerId;
  if (!authoring || !layerId) {
    overlaySource?.removeFeature(olFeature);
    overlayDetectHint = "Create an overlay first.";
    return;
  }
  let geometry = vectorGeometryFromOl(olFeature);
  if (!geometry) {
    overlaySource?.removeFeature(olFeature);
    overlayDetectHint = "Could not keep that shape.";
    return;
  }
  if (overlayTool === "freehand") {
    const simplified = simplifyFreehandGeometry(geometry, mapZoom());
    if ("error" in simplified) {
      overlaySource?.removeFeature(olFeature);
      overlayDetectHint =
        simplified.error === "vector.limit.exceeded" ? "That drawing is too detailed." : "Could not close that shape.";
      return;
    }
    geometry = simplified;
  }
  if (geometry.type !== "Polygon" && geometry.type !== "MultiPolygon") {
    overlaySource?.removeFeature(olFeature);
    overlayDetectHint = "Could not close that shape.";
    return;
  }
  const feature: VectorFeature = {
    type: "Feature",
    id: crypto.randomUUID(),
    properties: daenaProperties(layerId, "region", null),
    geometry,
  };
  overlaySource?.removeFeature(olFeature);
  overlaySelectedId = feature.id;
  overlayTool = "select";
  overlayDetectHint = "Region added. Name it as a Place, then save.";
  authoring.addFeature(feature);
  paintOverlayFeature(feature);
}

function syncOverlayDraw() {
  clearOverlayDraw();
  if (!map || !overlaySource || (overlayTool !== "freehand" && overlayTool !== "polygon")) return;
  const layer = overlayAuthoring?.layers.find((item) => item.id === overlayAuthoring.activeLayerId);
  if (!layer || layer.locked || !layer.defaultVisible) return;
  overlayDraw = new Draw({
    source: overlaySource,
    type: "Polygon",
    freehand: overlayTool === "freehand",
  });
  overlayDraw.on("drawend", (event) => {
    commitDrawnFeature(event.feature);
  });
  map.addInteraction(overlayDraw);
}

function landmassPreviewStyle(feature: { getId: () => unknown }) {
  const hover = String(feature.getId() ?? "") === HOVER_PREVIEW_ID;
  return new Style({
    fill: new Fill({ color: hover ? "rgba(243, 211, 154, 0.12)" : "rgba(243, 211, 154, 0.28)" }),
    stroke: new Stroke({ color: "#f3d39a", width: hover ? 1.5 : 2, lineDash: hover ? [4, 4] : [6, 4] }),
  });
}

function landmassSelectionLayer() {
  landmassSource = new VectorSource();
  landmassLayer = new VectorLayer({
    source: landmassSource,
    zIndex: 11,
    style: (feature) => landmassPreviewStyle(feature),
  });
  paintLandmassSelection();
  return landmassLayer;
}

function activeOccupiedGeometries() {
  const authoring = overlayAuthoring;
  const layerId = authoring?.activeLayerId;
  if (!authoring || !layerId) return [];
  return occupiedGeometries(overlayFeaturesForLayer(authoring.features, layerId));
}

function paintLandmassItem(selection: LandmassSelection, hover: boolean) {
  if (!landmassSource) return;
  const geometry = displayLandmassGeometry(selection, activeOccupiedGeometries(), includeOccupied);
  if (!geometry) return;
  const olFeature = readOlOverlayFeature(
    hover ? hoverPreviewFeature(withGeometry(selection, geometry)) : previewFeature(withGeometry(selection, geometry)),
  );
  if (olFeature) landmassSource.addFeature(olFeature);
}

function paintLandmassSelection() {
  if (!landmassSource) return;
  landmassSource.clear();
  if (landmassHover && !hoverIsRedundant(landmassHover, landmassSelection)) {
    paintLandmassItem(landmassHover, true);
  }
  if (landmassSelection) paintLandmassItem(landmassSelection, false);
  landmassLayer?.changed();
}

function clearLandmassHover() {
  landmassHoverRequest += 1;
  if (landmassHoverTimer) {
    clearTimeout(landmassHoverTimer);
    landmassHoverTimer = undefined;
  }
  if (!landmassHover) return;
  landmassHover = null;
  paintLandmassSelection();
}

function scheduleLandmassHover(longitude: number, latitude: number) {
  if (overlayTool !== "landmass" || overlayDetectBusy || !landmassHydrologyReady()) return;
  if (hoverCoversPoint(landmassHover, longitude, latitude)) return;
  if (hoverCoversPoint(landmassSelection, longitude, latitude)) {
    clearLandmassHover();
    return;
  }
  if (landmassHoverTimer) clearTimeout(landmassHoverTimer);
  landmassHoverTimer = setTimeout(() => void hoverLandmassAt(longitude, latitude), LANDMASS_HOVER_DELAY_MS);
}

async function hoverLandmassAt(longitude: number, latitude: number) {
  const token = session?.sessionToken;
  const epoch = offsetYears;
  const generation = session?.capturedContentGeneration;
  if (!token || overlayTool !== "landmass" || overlayDetectBusy || !landmassHydrologyReady() || generation == null) {
    return;
  }
  if (hoverCoversPoint(landmassHover, longitude, latitude)) return;
  const request = ++landmassHoverRequest;
  try {
    const proposal = await project.atlasStudioProposeRegion(token, toMicro(longitude), toMicro(latitude), "landmass");
    if (
      request !== landmassHoverRequest ||
      overlayTool !== "landmass" ||
      session?.sessionToken !== token ||
      offsetYears !== epoch ||
      session?.capturedContentGeneration !== generation ||
      !landmassHydrologyReady()
    ) {
      return;
    }
    const next = selectionFromProposal(proposal, epoch);
    landmassHover = hoverIsRedundant(next, landmassSelection) ? null : next;
    paintLandmassSelection();
  } catch {
    if (request !== landmassHoverRequest) return;
    clearLandmassHover();
  }
}

function clearLandmassSelection(hint = "") {
  landmassRequest += 1;
  overlayDetectBusy = false;
  landmassSelection = null;
  landGeometryCache = null;
  overlayDetectHint = hint;
  clearLandmassHover();
  paintLandmassSelection();
}

function commitLandmassSelection(layerId: string, subtractOccupied = false) {
  const authoring = overlayAuthoring;
  const selection = landmassSelection;
  if (!authoring || !selection) return;
  const prepared = subtractOccupied
    ? selectionMinusOccupied(selection, activeOccupiedGeometries(), includeOccupied)
    : { ok: true as const, selection };
  if (!prepared.ok) {
    overlayDetectHint = prepared.detail;
    return;
  }
  const feature = featureFromSelection(prepared.selection, layerId);
  overlaySelectedId = feature.id;
  authoring.addFeature(feature);
  paintOverlayFeature(feature);
  overlayTool = "select";
  overlayDetectHint = `${selection.label} added. Set its color, then save.`;
  landmassSelection = null;
  clearLandmassHover();
  paintLandmassSelection();
}

function createFromLandmassSelection() {
  const authoring = overlayAuthoring;
  if (!authoring || !landmassSelection) return;
  const layerId = authoring.createLayer(overlayCreateName);
  overlayCreateName = "";
  if (!layerId) {
    overlayDetectHint = "Could not create overlay.";
    return;
  }
  commitLandmassSelection(layerId);
}

function addFromLandmassSelection() {
  const authoring = overlayAuthoring;
  const layerId = authoring?.activeLayerId;
  const layer = authoring?.layers.find((item) => item.id === layerId);
  if (!authoring || !layerId || !layer || layer.locked || !layer.defaultVisible) {
    overlayDetectHint = "Create or select an unlocked overlay first.";
    return;
  }
  commitLandmassSelection(layerId, true);
}

function landmassHydrologyReady() {
  return Boolean(session && session.mapEntityId === mapId && session.offsetYears === offsetYears);
}

async function invertLandmassSelection() {
  const selection = landmassSelection;
  const token = session?.sessionToken;
  const epoch = offsetYears;
  const generation = session?.capturedContentGeneration;
  if (!selection || !token || overlayDetectBusy) return;
  if (!landmassHydrologyReady() || generation == null) {
    overlayDetectHint = "Wait for the epoch to finish updating.";
    return;
  }
  const request = ++landmassRequest;
  overlayDetectBusy = true;
  overlayDetectHint = "Inverting selection…";
  try {
    let land =
      landGeometryCache?.mapId === mapId &&
      landGeometryCache.epoch === epoch &&
      landGeometryCache.generation === generation
        ? landGeometryCache.proposal
        : null;
    if (!land) {
      land = await project.atlasStudioProposeRegion(token, 0, 0, "land");
      if (
        request !== landmassRequest ||
        session?.sessionToken !== token ||
        offsetYears !== epoch ||
        session?.capturedContentGeneration !== generation ||
        !landmassHydrologyReady()
      ) {
        return;
      }
      landGeometryCache = { mapId, epoch, generation, proposal: land };
    }
    if (request !== landmassRequest) return;
    const next = invertSelection(selection, land);
    if (!next.ok) {
      overlayDetectHint = next.detail;
      return;
    }
    landmassSelection = next.selection;
    overlayDetectHint = next.selection ? "Selection inverted over all land." : "Selection cleared.";
    paintLandmassSelection();
  } catch (cause) {
    if (request !== landmassRequest) return;
    overlayDetectHint = cause instanceof Error ? cause.message : String(cause);
  } finally {
    if (request === landmassRequest) overlayDetectBusy = false;
  }
}

async function selectDetectedRegion(longitude: number, latitude: number, shiftKey: boolean, altKey: boolean) {
  const token = session?.sessionToken;
  const epoch = offsetYears;
  const generation = session?.capturedContentGeneration;
  if (!token || overlayDetectBusy) return;
  if (!landmassHydrologyReady() || generation == null) {
    overlayDetectHint = "Wait for the epoch to finish updating.";
    return;
  }
  const request = ++landmassRequest;
  overlayDetectBusy = true;
  overlayDetectHint = "Selecting landmass…";
  try {
    const proposal = await project.atlasStudioProposeRegion(token, toMicro(longitude), toMicro(latitude), "landmass");
    if (
      request !== landmassRequest ||
      session?.sessionToken !== token ||
      offsetYears !== epoch ||
      session?.capturedContentGeneration !== generation ||
      !landmassHydrologyReady()
    ) {
      return;
    }
    const next = combineSelection(landmassSelection, proposal, epoch, combineModeFromModifiers(shiftKey, altKey));
    if (!next.ok) {
      overlayDetectHint = next.detail;
      return;
    }
    landmassSelection = next.selection;
    overlayDetectHint = next.selection
      ? "Landmass selected. Shift-click adds, Alt-click subtracts."
      : "Selection cleared.";
    paintLandmassSelection();
  } catch (cause) {
    if (request !== landmassRequest) return;
    overlayDetectHint = cause instanceof Error ? cause.message : String(cause);
  } finally {
    if (request === landmassRequest) overlayDetectBusy = false;
  }
}

function candidatePoint(candidate: FindPlaceCandidate): [number, number] {
  return [candidate.longitudeMicrodegrees / 1_000_000, candidate.latitudeMicrodegrees / 1_000_000];
}

function findPlaceOverlayLayer() {
  findPlaceSource = new VectorSource();
  findPlaceLayer = new VectorLayer({
    source: findPlaceSource,
    zIndex: 20,
    style: (feature) => {
      const selected = feature.get("candidateId") === findPlaceSelectedId;
      return new Style({
        image: new CircleStyle({
          radius: selected ? 8 : 6,
          fill: new Fill({ color: selected ? "#e6b03c" : "#ec9c30" }),
          stroke: new Stroke({ color: "#1b2822", width: 1.5 }),
        }),
        text: new Text({
          text: String(feature.get("candidateId") ?? ""),
          fill: new Fill({ color: "#1b2822" }),
          font: "700 10px system-ui",
          offsetY: -12,
        }),
      });
    },
  });
  syncFindPlaceOverlay();
  return findPlaceLayer;
}

function syncFindPlaceOverlay() {
  findPlaceSource?.clear();
  if (!findPlaceSource || !findPlaceResult) return;
  for (const candidate of findPlaceResult.candidates) {
    const feature = new Feature({
      geometry: new Point(fromLonLat(candidatePoint(candidate))),
      candidateId: candidate.id,
    });
    findPlaceSource.addFeature(feature);
  }
  findPlaceLayer?.changed();
}

function clearFindPlace() {
  findPlaceResult = null;
  findPlaceSelectedId = null;
  findPlaceError = "";
  findPlaceSource?.clear();
}

function clearRouting() {
  routeRequest += 1;
  routingArming = false;
  routingStart = null;
  routeResult = null;
  routeSelectedId = null;
  routeError = "";
  routeSearching = false;
  syncRouteOverlay();
}

function armRouting() {
  if (routingArming) {
    clearRouting();
    return;
  }
  if (linkArming) linkArming = false;
  overlayTool = "select";
  overlayDetectHint = "";
  routeRequest += 1;
  routeSearching = false;
  routingArming = true;
  findTopic = "routes";
  sidebarPane = "find";
  routingStart = null;
  routeResult = null;
  routeSelectedId = null;
  routeError = "";
  clearPicked();
  host?.focus();
}

function pickRoutingPoint(longitude: number, latitude: number) {
  if (!routingStart) {
    routingStart = [longitude, latitude];
    syncRouteOverlay();
    return;
  }
  const start = routingStart;
  routingArming = false;
  void runSuggestRoutes(start, [longitude, latitude]);
}

function selectRoute(suggestion: RouteSuggestion) {
  routeSelectedId = suggestion.id;
  syncRouteOverlay();
}

function acceptRoute(suggestion: RouteSuggestion) {
  const authoring = overlayAuthoring;
  if (!authoring) {
    routeError = "Open Atlas from the physical map editor to keep a road.";
    return;
  }
  const feature = routeFeatureFromSuggestion(suggestion, offsetYears, "");
  if (!feature) return;
  const error = authoring.addRoute(feature);
  if (error) {
    routeError = error;
    return;
  }
  clearRouting();
}

function routeLineGeometry(geometry: VectorFeature["geometry"]) {
  if (geometry.type === "LineString") {
    return new LineString(geometry.coordinates.map((point) => fromLonLat([point[0], point[1]])));
  }
  if (geometry.type === "MultiLineString") {
    return new MultiLineString(
      geometry.coordinates.map((line) => line.map((point) => fromLonLat([point[0], point[1]]))),
    );
  }
  return null;
}

function routeOverlayLayer() {
  routeSource = new VectorSource();
  routeLayer = new VectorLayer({
    source: routeSource,
    zIndex: 15,
    style: (feature) => {
      const point = feature.get("routePoint");
      if (point) {
        const start = point === "start";
        return new Style({
          image: new CircleStyle({
            radius: start ? 7 : 6,
            fill: new Fill({ color: start ? "#58c4a8" : "#e6b03c" }),
            stroke: new Stroke({ color: "#1b2822", width: 1.5 }),
          }),
        });
      }
      const suggestionId = feature.get("suggestionId");
      const selected = suggestionId === routeSelectedId;
      const accepted = feature.get("accepted") === true;
      if (accepted) {
        return new Style({
          stroke: new Stroke({ color: "#c4a574", width: 2.4, lineCap: "round", lineJoin: "round" }),
        });
      }
      const width = selected ? 4 : 2.2;
      return [
        new Style({
          stroke: new Stroke({
            color: "#1b2822",
            width: width + 2.4,
            lineCap: "round",
            lineJoin: "round",
          }),
        }),
        new Style({
          stroke: new Stroke({
            color: selected ? "#e6b03c" : "#8f7a4a",
            width,
            lineCap: "round",
            lineJoin: "round",
          }),
        }),
      ];
    },
  });
  syncRouteOverlay();
  return routeLayer;
}

function syncRouteOverlay() {
  routeSource?.clear();
  if (!routeSource) return;
  for (const feature of overlayAuthoring?.routeFeatures ?? []) {
    const geometry = feature.geometry;
    const line = routeLineGeometry(geometry);
    if (!line) continue;
    const olFeature = new Feature({ geometry: line, accepted: true });
    olFeature.setId(feature.id);
    routeSource.addFeature(olFeature);
  }
  if (routeResult) {
    for (const suggestion of routeResult.suggestions) {
      const geometry = routeGeometryFromSuggestion(suggestion);
      if (!geometry) continue;
      const line = routeLineGeometry(geometry);
      if (!line) continue;
      const olFeature = new Feature({
        geometry: line,
        suggestionId: suggestion.id,
      });
      routeSource.addFeature(olFeature);
    }
    const selected =
      routeResult.suggestions.find((item: RouteSuggestion) => item.id === routeSelectedId) ??
      routeResult.suggestions[0];
    const ends = selected ? routeEndpoints(selected) : null;
    if (ends) {
      routeSource.addFeature(new Feature({ geometry: new Point(fromLonLat(ends.start)), routePoint: "start" }));
      routeSource.addFeature(new Feature({ geometry: new Point(fromLonLat(ends.end)), routePoint: "end" }));
    }
  } else if (routingStart) {
    routeSource.addFeature(new Feature({ geometry: new Point(fromLonLat(routingStart)), routePoint: "start" }));
  }
  routeLayer?.changed();
}

async function runSuggestRoutes(start: [number, number], end: [number, number]) {
  const generation = ++routeRequest;
  routeSearching = true;
  routeError = "";
  try {
    const [startLon, startLat] = toMicrodegrees(start[0], start[1]);
    const [endLon, endLat] = toMicrodegrees(end[0], end[1]);
    const next = await project.physicalSuggestRoutes(mapId, offsetYears, {
      startLongitudeMicrodegrees: startLon,
      startLatitudeMicrodegrees: startLat,
      endLongitudeMicrodegrees: endLon,
      endLatitudeMicrodegrees: endLat,
      existingRoutes: existingRoutePolylines(overlayAuthoring?.routeFeatures ?? []),
    });
    if (generation !== routeRequest) return;
    routeResult = next;
    routeSelectedId = next.suggestions[0]?.id ?? null;
    syncRouteOverlay();
  } catch (cause) {
    if (generation !== routeRequest) return;
    routeError = cause instanceof Error ? cause.message : String(cause);
    routeResult = null;
    routeSelectedId = null;
    syncRouteOverlay();
  } finally {
    if (generation === routeRequest) routeSearching = false;
  }
}

function selectFindPlace(candidate: FindPlaceCandidate) {
  findPlaceSelectedId = candidate.id;
  findPlaceLayer?.changed();
  const [lng, lat] = candidatePoint(candidate);
  if (map) {
    const view = map.getView();
    const zoom = Math.max(view.getZoom() ?? 3, 3);
    if (prefersReducedMotion()) {
      view.setCenter(fromLonLat([lng, lat]));
      view.setZoom(zoom);
    } else {
      view.animate({ center: fromLonLat([lng, lat]), zoom, duration: 280 });
    }
  }
  inspectAt(lng, lat, true);
}

function pinFindPlace(candidate: FindPlaceCandidate) {
  selectFindPlace(candidate);
  const [lng, lat] = candidatePoint(candidate);
  openLinkPanel(lng, lat);
}

async function runFindPlace(query: FindPlaceQuery) {
  findPlaceSearching = true;
  findPlaceError = "";
  try {
    const compact = Object.fromEntries(
      Object.entries(query).filter(([, value]) => value !== null && value !== undefined),
    ) as FindPlaceQuery;
    const next = await project.physicalFindPlace(mapId, offsetYears, compact);
    findPlaceResult = next;
    findPlaceSelectedId = next.candidates[0]?.id ?? null;
    syncFindPlaceOverlay();
    if (next.candidates[0]) selectFindPlace(next.candidates[0]);
  } catch (cause) {
    findPlaceError = cause instanceof Error ? cause.message : String(cause);
  } finally {
    findPlaceSearching = false;
  }
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
  clearFindPlace();
  clearRouting();
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
    toggleHelp: () => toggleStudioHelp(),
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
  overlayAuthoring?.layers.map((layer) => `${layer.id}:${layer.defaultVisible}`).join();
  overlayAuthoring?.features.map((feature) => feature.id).join();
  untrack(() => syncAuthoredOverlays());
});

$effect(() => {
  offsetYears;
  untrack(() => {
    if (landmassHover) clearLandmassHover();
    if (landmassSelection && landmassSelection.epochOffsetYears !== offsetYears) {
      clearLandmassSelection("Landmass selection cleared because the epoch changed.");
    }
  });
});

$effect(() => {
  mapId;
  session?.capturedContentGeneration;
  untrack(() => {
    if (landmassSelection || landGeometryCache || landmassHover) {
      clearLandmassSelection("Landmass selection cleared because the map changed.");
    }
    void syncNamedWater();
  });
});

$effect(() => {
  overlaySelectedId;
  overlayAuthoring?.layers;
  overlayAuthoring?.features;
  untrack(() => overlayLayer?.changed());
});

$effect(() => {
  overlayAuthoring?.routeFeatures.map((feature: VectorFeature) => feature.id).join();
  routeResult;
  routeSelectedId;
  routingStart;
  untrack(() => syncRouteOverlay());
});

$effect(() => {
  includeOccupied;
  overlayAuthoring?.activeLayerId;
  overlayAuthoring?.features;
  untrack(() => paintLandmassSelection());
});

$effect(() => {
  overlayTool;
  overlayAuthoring?.activeLayerId;
  untrack(() => {
    if (overlayTool !== "landmass") clearLandmassHover();
    if (overlayTool !== "select" && (routingArming || routeSearching || routeResult || routeError)) {
      clearRouting();
    }
    syncOverlayDraw();
  });
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
  clearOverlayDraw();
  map = null;
  tileSource = null;
  findPlaceLayer = null;
  findPlaceSource = null;
  namedWaterLayer = null;
  namedWaterSource = null;
  routeLayer = null;
  routeSource = null;
  overlayLayer = null;
  overlaySource = null;
  if (session) {
    void project.atlasStudioClose(session.sessionToken).catch(() => undefined);
  }
});
</script>

{#snippet surfaceFields(sample: AtlasStudioSurfaceSample)}
  <dl class="place-hero">
    <div>
      <dt>Temperature</dt>
      <dd>{formatTemperature(sample.temperatureCentiC)}</dd>
    </div>
    <div>
      <dt>Rainfall</dt>
      <dd>{sample.precipitationMm.toLocaleString("en-US")} mm</dd>
    </div>
    <div>
      <dt>Humidity</dt>
      <dd>{formatHumidity(sample.humidityPpm)}</dd>
    </div>
  </dl>
  <div class="place-sheet">
    <section aria-label="Temperature">
      <h3>Temperature</h3>
      <dl>
        <div>
          <dt>Northern-summer solstice</dt>
          <dd>{formatTemperature(sample.temperatureNhSummerCentiC)}</dd>
        </div>
        <div>
          <dt>Northern-winter solstice</dt>
          <dd>{formatTemperature(sample.temperatureNhWinterCentiC)}</dd>
        </div>
        <div>
          <dt>Annual range</dt>
          <dd>{formatTemperature(sample.seasonalRangeCentiC)}</dd>
        </div>
        <div>
          <dt>Freeze</dt>
          <dd>{formatFreeze(sample.freeze)}</dd>
        </div>
      </dl>
    </section>
    <section aria-label="Wind">
      <h3>Wind</h3>
      <dl>
        <div>
          <dt>Prevailing</dt>
          <dd>{formatWind(sample.windEastMilli, sample.windNorthMilli)}</dd>
        </div>
        <div>
          <dt>Northern-summer</dt>
          <dd>{formatWind(sample.windEastNhSummerMilli, sample.windNorthNhSummerMilli)}</dd>
        </div>
        <div>
          <dt>Northern-winter</dt>
          <dd>{formatWind(sample.windEastNhWinterMilli, sample.windNorthNhWinterMilli)}</dd>
        </div>
        <div>
          <dt>Circulation</dt>
          <dd>{titleCase(sample.windBand)}</dd>
        </div>
        <div>
          <dt>Northern-summer circulation</dt>
          <dd>{titleCase(sample.windBandNhSummer)}</dd>
        </div>
        <div>
          <dt>Northern-winter circulation</dt>
          <dd>{titleCase(sample.windBandNhWinter)}</dd>
        </div>
        <div>
          <dt>Wind flow</dt>
          <dd>{formatDivergence(sample.windDivergencePpm)}</dd>
        </div>
        <div>
          <dt>Northern-summer flow</dt>
          <dd>{formatDivergence(sample.windDivergenceNhSummerPpm)}</dd>
        </div>
        <div>
          <dt>Northern-winter flow</dt>
          <dd>{formatDivergence(sample.windDivergenceNhWinterPpm)}</dd>
        </div>
      </dl>
    </section>
    <section aria-label="Water">
      <h3>Water</h3>
      <dl>
        <div>
          <dt>Northern-summer rainfall</dt>
          <dd>{sample.precipitationNhSummerMm.toLocaleString("en-US")} mm</dd>
        </div>
        <div>
          <dt>Northern-winter rainfall</dt>
          <dd>{sample.precipitationNhWinterMm.toLocaleString("en-US")} mm</dd>
        </div>
        <div>
          <dt>Aridity</dt>
          <dd>{formatAridity(sample.aridityPpm)}</dd>
        </div>
        <div>
          <dt>Surface current</dt>
          <dd>{formatCurrent(sample.currentEastMilli, sample.currentNorthMilli)}</dd>
        </div>
      </dl>
    </section>
    <section aria-label="Land and climate">
      <h3>Land and climate</h3>
      <dl>
        <div>
          <dt>Surface</dt>
          <dd>{titleCase(sample.surface)}</dd>
        </div>
        {#if sample.iceThicknessMm > 0}
          <div>
            <dt>Ice cover</dt>
            <dd>{formatMetres(sample.iceThicknessMm)}</dd>
          </div>
        {/if}
        <div>
          <dt>Why this biome</dt>
          <dd>{sample.biomeReason}</dd>
        </div>
        <div>
          <dt>Storms</dt>
          <dd>{sample.stormReason}</dd>
        </div>
        <div>
          <dt>Storm suitability</dt>
          <dd>{formatShare(sample.stormSuitabilityPpm)}</dd>
        </div>
        <div>
          <dt>Storm tracks</dt>
          <dd>{formatShare(sample.stormTrackPpm)}</dd>
        </div>
        <div>
          <dt>Storm intensity</dt>
          <dd>{formatShare(sample.stormIntensityPpm)}</dd>
        </div>
      </dl>
    </section>
  </div>
{/snippet}

<section class="studio" aria-label="Atlas Studio">
  <div class="body" style={`--atlas-sidebar-width: ${sidebarWidth}px`}>
    <aside class="atlas-sidebar" aria-label="Atlas Studio controls">
      <header class="sidebar-head">
        <div class="sidebar-brand">
          <span>Atlas</span>
          <strong>{styleLabel(styleId)}</strong>
        </div>
        <label class="style-field">
          Style
          <select bind:value={styleId} onchange={() => scheduleSession()}>
            {#each capabilities?.styles ?? [] as id}
              <option value={id}>{styleLabel(id)}</option>
            {/each}
          </select>
        </label>
        {#if (capabilities?.presets.length ?? 0) > 0}
          <label class="style-field">
            Preset
            <select onchange={(event) => void applyPreset(event.currentTarget.value)}>
              <option value="">Saved presets</option>
              {#each capabilities?.presets ?? [] as preset}
                <option value={preset.id}>{preset.name}</option>
              {/each}
            </select>
          </label>
        {/if}
        <div class="epoch-card" aria-label="World epoch">
          <div class="epoch-copy">
            <span>Epoch</span>
            <strong>{formatEpoch(offsetYears)}</strong>
          </div>
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
        </div>
        {#if capabilities?.timeModes.includes("calendar-year")}
          <div class="time-row">
            <label>
              Time
              <select bind:value={timeKind} onchange={() => scheduleSession()}>
                <option value="physical-offset-year">Physical offset</option>
                <option value="calendar-year">Authored year</option>
              </select>
            </label>
            {#if timeKind === "calendar-year"}
              <label>
                Year
                <input type="number" bind:value={authoredYear} oninput={() => scheduleSession()} />
              </label>
            {/if}
          </div>
        {/if}
      </header>
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
      <div class="sidebar-tabs" role="tablist" aria-label="Atlas tools">
        {#each sidebarPanes as pane (pane.id)}
          {@const badge = sidebarPaneBadge(pane.id)}
          <button
            type="button"
            role="tab"
            id={`atlas-tab-${pane.id}`}
            aria-selected={sidebarPane === pane.id}
            aria-controls={`atlas-pane-${pane.id}`}
            class:active={sidebarPane === pane.id}
            class:dirty={pane.id === "draw" && Boolean(overlayAuthoring?.dirty)}
            onclick={() => (sidebarPane = pane.id)}>
            {#if pane.id === "layers"}<Layers size={14} strokeWidth={1.8} />
            {:else if pane.id === "find"}<Search size={14} strokeWidth={1.8} />
            {:else if pane.id === "draw"}<Pencil size={14} strokeWidth={1.8} />
            {:else}<Info size={14} strokeWidth={1.8} />{/if}
            <span>{pane.label}</span>
            {#if badge > 0}<em>{badge}</em>{/if}
          </button>
        {/each}
      </div>
      <div class="sidebar-body">
        {#if sidebarPane === "layers"}
          <div class="sidebar-pane" id="atlas-pane-layers" role="tabpanel" aria-labelledby="atlas-tab-layers">
            <MapLayerVisibilityList
              variant="studio"
              groups={studioLayerBook}
              activeId={overlayAuthoring?.activeLayerId ?? null}
              onSelect={(id) => {
                overlayAuthoring?.setActiveLayer(id);
                sidebarPane = "draw";
              }}
              onToggle={(id) => {
                const overlay = overlayAuthoring?.layers.find((layer) => layer.id === id);
                if (overlay && overlayAuthoring) {
                  overlayAuthoring.setVisible(id, !overlay.defaultVisible);
                  return;
                }
                layers = layers.map((layer) => (layer.id === id ? { ...layer, enabled: !layer.enabled } : layer));
                scheduleSession();
              }} />
          </div>
        {:else if sidebarPane === "find"}
          <div class="sidebar-pane find-pane" id="atlas-pane-find" role="tabpanel" aria-labelledby="atlas-tab-find">
            <div class="find-topics" role="tablist" aria-label="Search">
              <button
                type="button"
                role="tab"
                id="atlas-find-places"
                aria-selected={findTopic === "places"}
                aria-controls="atlas-find-places-panel"
                class:active={findTopic === "places"}
                onclick={() => (findTopic = "places")}>
                <MapPin size={13} strokeWidth={1.8} />
                Places
                {#if findPlaceResult}<em>{findPlaceResult.candidateCount}</em>{/if}
              </button>
              <button
                type="button"
                role="tab"
                id="atlas-find-routes"
                aria-selected={findTopic === "routes"}
                aria-controls="atlas-find-routes-panel"
                class:active={findTopic === "routes"}
                onclick={() => (findTopic = "routes")}>
                <Route size={13} strokeWidth={1.8} />
                Routes
                {#if routeResult}<em>{routeResult.suggestionCount}</em>{/if}
              </button>
            </div>
            {#if findTopic === "places"}
              <div id="atlas-find-places-panel" role="tabpanel" aria-labelledby="atlas-find-places">
                <FindPlacePanel
                  variant="studio"
                  disabled={loading}
                  searching={findPlaceSearching}
                  error={findPlaceError}
                  result={findPlaceResult}
                  selectedId={findPlaceSelectedId}
                  onsearch={(query) => void runFindPlace(query)}
                  onselect={selectFindPlace}
                  onpin={pinFindPlace}
                  onclear={clearFindPlace} />
              </div>
            {:else}
              <div id="atlas-find-routes-panel" role="tabpanel" aria-labelledby="atlas-find-routes">
                <RouteSuggestPanel
                  variant="studio"
                  disabled={loading}
                  searching={routeSearching}
                  error={routeError}
                  arming={routingArming}
                  startPicked={Boolean(routingStart)}
                  result={routeResult}
                  selectedId={routeSelectedId}
                  onarm={armRouting}
                  onselect={selectRoute}
                  onaccept={acceptRoute}
                  oncancel={clearRouting} />
              </div>
            {/if}
          </div>
        {:else if sidebarPane === "draw"}
          <div class="sidebar-pane" id="atlas-pane-draw" role="tabpanel" aria-labelledby="atlas-tab-draw">
            {#if overlayAuthoring}
              <AtlasOverlayPanel
                authoring={overlayAuthoring}
                bind:tool={overlayTool}
                bind:selectedFeatureId={overlaySelectedId}
                bind:detectHint={overlayDetectHint}
                bind:createName={overlayCreateName}
                selection={landmassSelection}
                bind:includeOccupied
                showIncludeOccupied={activeOccupiedGeometries().length > 0}
                covered={Boolean(
                  landmassSelection &&
                  !includeOccupied &&
                  activeOccupiedGeometries().length > 0 &&
                  !displayLandmassGeometry(landmassSelection, activeOccupiedGeometries(), false),
                )}
                onCreateFromSelection={createFromLandmassSelection}
                onAddFromSelection={addFromLandmassSelection}
                onInvertSelection={() => void invertLandmassSelection()}
                onClearSelection={() => clearLandmassSelection()}
                onNamePlace={(id) => void nameOverlayFeature(id)} />
            {:else}
              <p class="pane-empty">This map has no overlay authoring session.</p>
            {/if}
          </div>
        {:else}
          <div
            class="sidebar-pane inspect-pane"
            id="atlas-pane-inspect"
            role="tabpanel"
            aria-labelledby="atlas-tab-inspect">
            <section class="place" aria-label="Place" aria-live="polite">
              <strong>Place</strong>
              {#if surface}
                <dl class="place-summary">
                  <div>
                    <dt>Coordinates</dt>
                    <dd>{cursor}</dd>
                  </div>
                  <div>
                    <dt>Biome</dt>
                    <dd>{titleCase(surface.climate)}</dd>
                  </div>
                  <div>
                    <dt>Surface</dt>
                    <dd>{formatElevation(surface.elevationMm, surface.waterSurfaceMm, surface.surface)}</dd>
                  </div>
                  <div>
                    <dt>Temperature</dt>
                    <dd>{formatTemperature(surface.temperatureCentiC)}</dd>
                  </div>
                  <div>
                    <dt>Rainfall</dt>
                    <dd>{surface.precipitationMm.toLocaleString("en-US")} mm/year</dd>
                  </div>
                  <div>
                    <dt>Humidity</dt>
                    <dd>{formatHumidity(surface.humidityPpm)}</dd>
                  </div>
                </dl>
              {:else}
                <p>Move or click the map to sample this point.</p>
              {/if}
            </section>
            {#if hits.length > 0}
              <div class="inspect" role="region" aria-label="Feature inspection">
                <strong>Features</strong>
                {#each hits as hit}
                  <p>
                    <span>{namedPlaceLabel(hit)}</span>
                    <small>{hit.kind}{hit.derived ? " · derived" : ""}</small>
                    <small>{derivedExplanation(hit)}</small>
                  </p>
                {/each}
              </div>
            {:else}
              <p class="pane-empty">Click the map or press Enter to inspect the center.</p>
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
                  <li>Escape clears inspection, drawing, or suggested routing</li>
                  <li>⌘/Ctrl+Z undo overlay edits</li>
                  <li>⌘/Ctrl+S save overlay edits</li>
                </ul>
              </section>
            {/if}
          </div>
        {/if}
      </div>
      <button
        type="button"
        class="sidebar-foot"
        onclick={() => (sidebarPane = "inspect")}
        aria-label="Open place inspection">
        <span>{cursor}</span>
        {#if surface}
          <span>{titleCase(surface.climate)} · {formatTemperature(surface.temperatureCentiC)}</span>
        {:else}
          <span>Sample a point</span>
        {/if}
      </button>
    </aside>
    <button
      type="button"
      class="sidebar-resizer"
      role="slider"
      aria-orientation="vertical"
      aria-label="Resize Atlas sidebar"
      aria-valuemin={SIDEBAR_MIN}
      aria-valuemax={SIDEBAR_MAX}
      aria-valuenow={sidebarWidth}
      title="Drag to resize · Double-click to reset"
      onpointerdown={startSidebarResize}
      ondblclick={() => setSidebarWidth(SIDEBAR_DEFAULT)}
      onkeydown={onSidebarResizeKey}></button>
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
      {#if linking && linkAnchor}
        <MapLocationLinkPanel
          {mapId}
          bind:anchor={linkAnchor}
          arming={linkArming}
          onclose={() => closeLinkPanel(true)}
          seedName={overlayLinkSeedName}
          seedRole={overlayLinkSeedRole}
          onlinked={() => void onPlaceLinked()}
          onresnap={() => {
            linking = false;
            linkArming = true;
            host?.focus();
          }} />
      {/if}
      {#if routingArming}
        <p class="pick-arming" role="status">
          {routingStart ? "Click the end on land · Escape to cancel" : "Click the start on land · Escape to cancel"}
        </p>
      {:else if routeSearching}
        <p class="pick-arming" role="status">Finding land routes…</p>
      {:else if linkArming}
        <p class="pick-arming" role="status">Click the map to choose a link location · Escape to cancel</p>
      {/if}
      {#if picked}
        <div
          class="pick-overlay"
          class:flip={picked.flip}
          style={`left: ${picked.x}px; top: ${picked.y}px;`}
          role="dialog"
          aria-label="Map spot actions"
          tabindex="-1"
          bind:this={pickPanel}>
          <button type="button" class="pick-close" aria-label="Close spot actions" onclick={() => clearPicked(true)}>
            <X size={13} strokeWidth={2} aria-hidden="true" />
          </button>
          <strong>{picked.lng.toFixed(4)}°, {picked.lat.toFixed(4)}°</strong>
          {#if pickedSample}
            <span
              >{titleCase(pickedSample.surface.climate)} · {formatTemperature(pickedSample.surface.temperatureCentiC)} · {pickedSample.surface.precipitationMm.toLocaleString(
                "en-US",
              )} mm/year</span>
          {:else}
            <span>Sampling this point…</span>
          {/if}
          <div class="actions">
            {#each pickNameActions() as action (action.label)}
              <button type="button" onclick={action.run}>{action.label}</button>
            {/each}
            <button type="button" onclick={() => picked && openLinkPanel(picked.lng, picked.lat)}>
              Link to entity
            </button>
            <button type="button" onclick={() => openPlaceDetails(true)}>Place details</button>
          </div>
        </div>
      {/if}
    </div>
  </div>
  {#if placeModal}
    <div
      class="place-backdrop"
      role="presentation"
      onclick={(event) => {
        if (event.target === event.currentTarget) closePlaceDetails();
      }}>
      <div
        bind:this={placeDialog}
        class="place-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="place-modal-title"
        tabindex="-1"
        onkeydown={trapPlaceFocus}>
        <div class="place-modal-head">
          <button type="button" class="place-close" aria-label="Close place details" onclick={closePlaceDetails}>
            <X size={15} strokeWidth={2} aria-hidden="true" />
          </button>
          {#if modalSurface}
            <span class="place-kicker">Place details</span>
            <strong id="place-modal-title">{titleCase(modalSurface.climate)}</strong>
            <span class="place-sub"
              >{modalCoords} · {formatElevation(
                modalSurface.elevationMm,
                modalSurface.waterSurfaceMm,
                modalSurface.surface,
              )}</span>
          {:else}
            <span class="place-kicker">Place details</span>
            <strong id="place-modal-title">No sample yet</strong>
          {/if}
        </div>
        {#if modalSurface}
          {@render surfaceFields(modalSurface)}
        {:else}
          <p>Move or click the map to sample this point.</p>
        {/if}
        <div class="place-guide">
          <button
            type="button"
            onclick={() => (guideOpen = !guideOpen)}
            aria-expanded={guideOpen}
            aria-controls="place-guide-list">
            {guideOpen ? "Hide field guide" : "What do these terms mean?"}
          </button>
          {#if guideOpen}
            <dl id="place-guide-list">
              {#each FIELD_GUIDE as entry}
                <div>
                  <dt>{entry.term}</dt>
                  <dd>{entry.help}</dd>
                </div>
              {/each}
            </dl>
          {/if}
        </div>
        <div class="place-modal-foot">
          {#if modalSample}
            <button
              type="button"
              onclick={() => {
                const spot = modalSample;
                closePlaceDetails();
                if (spot) openLinkPanel(spot.lng, spot.lat);
              }}>
              Link to entity
            </button>
          {/if}
          <button type="button" onclick={closePlaceDetails}>Close</button>
        </div>
      </div>
    </div>
  {/if}
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
  grid-template-columns: var(--atlas-sidebar-width, 312px) 6px minmax(0, 1fr);
  min-height: 0;
  height: 100%;
  width: 100%;
}
.sidebar-resizer {
  width: 6px;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: #405047;
  cursor: col-resize;
  touch-action: none;
}
.sidebar-resizer:hover,
.sidebar-resizer:focus-visible {
  background: #d5ab6c;
}
.atlas-sidebar {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  background: #15211d;
  font: 12px/1.4 system-ui;
}
.sidebar-head {
  display: grid;
  gap: 10px;
  padding: 12px 12px 10px;
  border-bottom: 1px solid rgb(255 255 255 / 8%);
  background: #1b2822;
}
.sidebar-brand {
  display: grid;
  gap: 1px;
}
.sidebar-brand span {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #d5ab6c;
}
.sidebar-brand strong {
  font-size: 14px;
  font-weight: 700;
  letter-spacing: -0.01em;
}
.style-field,
.atlas-sidebar label,
.time-row label {
  display: grid;
  gap: 4px;
  color: #b8c8bc;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.time-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
}
.atlas-sidebar :global(.find-place .find-check),
.atlas-sidebar :global(.find-place .find-prefer label) {
  display: flex;
  align-items: center;
  gap: 6px;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 500;
  font-size: 11px;
  color: #edf2ec;
}
.atlas-sidebar :global(.find-place .detail-grid label) {
  display: grid;
}
.atlas-sidebar select,
.atlas-sidebar input[type="number"] {
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 7px;
  padding: 7px 8px;
  background: #0f1a16;
  color: #edf2ec;
  font: 12px/1.3 system-ui;
  text-transform: none;
  letter-spacing: 0;
  font-weight: 500;
}
.epoch-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 5.6em;
  gap: 6px 8px;
  align-items: center;
  padding: 8px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 10px;
  background: rgb(0 0 0 / 18%);
}
.epoch-copy {
  grid-column: 1 / -1;
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}
.epoch-copy span {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #b8c8bc;
}
.epoch-copy strong {
  font-size: 11px;
  font-weight: 600;
  color: #edf2ec;
}
.epoch-card input[type="range"] {
  width: 100%;
  accent-color: #d5ab6c;
}
.epoch-year {
  width: 100%;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  padding: 4px 5px;
  background: #0f1a16;
  color: #edf2ec;
  font: 12px system-ui;
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.sidebar-tabs {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 2px;
  padding: 8px 8px 0;
  background: #1b2822;
}
.sidebar-tabs button {
  position: relative;
  display: grid;
  justify-items: center;
  gap: 3px;
  min-height: 44px;
  padding: 6px 4px 7px;
  border: 0;
  border-radius: 9px 9px 0 0;
  background: transparent;
  color: #aebdb1;
  font: 650 10px/1.1 system-ui;
  letter-spacing: 0.02em;
}
.sidebar-tabs button:hover {
  color: #edf2ec;
  background: rgb(255 255 255 / 5%);
}
.sidebar-tabs button.active {
  background: #15211d;
  color: #edf2ec;
  box-shadow: inset 0 2px 0 #d5ab6c;
}
.sidebar-tabs button.dirty::after {
  content: "";
  position: absolute;
  top: 6px;
  right: 8px;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #d5ab6c;
}
.sidebar-tabs em {
  position: absolute;
  top: 4px;
  right: 6px;
  min-width: 14px;
  height: 14px;
  padding: 0 4px;
  border-radius: 999px;
  background: #d5ab6c;
  color: #1b2822;
  font: 700 9px/14px system-ui;
  font-style: normal;
  font-variant-numeric: tabular-nums;
}
.sidebar-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 10px 12px 12px;
  background: #15211d;
}
.sidebar-pane,
.find-pane,
.inspect-pane {
  display: grid;
  gap: 12px;
  align-content: start;
}
.find-topics {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
}
.find-topics button {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 32px;
  padding: 4px 8px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 8px;
  background: #0f1a16;
  color: #aebdb1;
  font: 650 11px/1.1 system-ui;
  cursor: pointer;
  transition:
    background 180ms ease,
    border-color 180ms ease,
    color 180ms ease;
}
.find-topics button.active {
  border-color: #d5ab6c;
  background: rgb(213 171 108 / 16%);
  color: #edf2ec;
}
.find-topics em {
  min-width: 14px;
  height: 14px;
  padding: 0 4px;
  border-radius: 999px;
  background: #d5ab6c;
  color: #1b2822;
  font: 700 9px/14px system-ui;
  font-style: normal;
  font-variant-numeric: tabular-nums;
}
.pane-empty {
  margin: 0;
  padding: 12px;
  border: 1px dashed rgb(255 255 255 / 14%);
  border-radius: 8px;
  color: #aebdb1;
}
.sidebar-foot {
  display: grid;
  gap: 2px;
  width: 100%;
  padding: 9px 12px;
  border: 0;
  border-top: 1px solid rgb(255 255 255 / 8%);
  border-radius: 0;
  background: #1b2822;
  color: #edf2ec;
  text-align: left;
  font: 11px/1.35 system-ui;
  cursor: pointer;
}
.sidebar-foot span:first-child {
  font-variant-numeric: tabular-nums;
}
.sidebar-foot span:last-child {
  color: #aebdb1;
}
.place,
.inspect {
  display: grid;
  gap: 6px;
  padding: 10px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 10px;
  background: rgb(255 255 255 / 3%);
}
.place dl,
.place p {
  margin: 0;
}
.place-summary > div {
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
.pick-overlay {
  position: absolute;
  z-index: 2;
  display: grid;
  gap: 4px;
  max-width: min(280px, calc(100% - 24px));
  padding: 8px 30px 8px 10px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 8px;
  background: #1b2822f2;
  color: #edf2ec;
  font: 12px/1.4 system-ui;
  box-shadow: 0 8px 24px rgb(0 0 0 / 45%);
  transform: translate(12px, -50%);
}
.pick-close {
  position: absolute;
  top: 5px;
  right: 5px;
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  padding: 0;
  line-height: 1;
}
.pick-overlay.flip {
  transform: translate(calc(-100% - 12px), -50%);
}
.pick-overlay span {
  color: #b8c8bc;
}
.pick-overlay:focus {
  outline: none;
}
.pick-overlay:focus-visible {
  outline: 2px solid var(--theme-success-border, #edf2ec);
  outline-offset: 2px;
}
.pick-arming {
  position: absolute;
  z-index: 2;
  bottom: 12px;
  left: 50%;
  transform: translateX(-50%);
  margin: 0;
  padding: 6px 12px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 999px;
  background: #1b2822f2;
  color: #edf2ec;
  font: 12px/1.4 system-ui;
  white-space: nowrap;
}
.place-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgb(13 27 42 / 55%);
}
.place-modal {
  position: relative;
  display: grid;
  gap: 0;
  width: min(620px, 100%);
  max-height: min(84vh, 700px);
  overflow: auto;
  padding: 0;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 14px;
  background: #1b2822;
  color: #edf2ec;
  box-shadow: 0 22px 70px rgb(0 0 0 / 50%);
  font: 13px/1.6 system-ui;
  outline: none;
}
.place-modal-head {
  position: relative;
  display: grid;
  gap: 4px;
  padding: 22px 24px 18px;
  border-bottom: 1px solid rgb(255 255 255 / 10%);
}
.place-modal-head strong {
  font-size: 22px;
  font-weight: 700;
  letter-spacing: -0.01em;
}
.place-kicker {
  display: block;
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  opacity: 0.7;
}
.place-sub {
  color: #b8c8bc;
  font-variant-numeric: tabular-nums;
}
.place-close {
  position: absolute;
  top: 14px;
  right: 14px;
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  line-height: 1;
}
.place-hero {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin: 14px 0 0;
  padding: 0;
}
.place-hero div {
  padding: 10px 12px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 10px;
  background: rgb(0 0 0 / 20%);
}
.place-hero dt {
  color: var(--theme-neutral-text-muted, #aebdb1);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}
.place-hero dd {
  margin: 2px 0 0;
  font-size: 17px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.place-sheet {
  display: grid;
  padding: 6px 24px;
}
.place-sheet section {
  min-width: 0;
  padding: 14px 0;
  border-bottom: 1px solid rgb(255 255 255 / 8%);
}
.place-sheet section:last-child {
  border-bottom: 0;
}
.place-sheet h3 {
  margin: 0 0 4px;
  font-size: 12px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #d5ab6c;
}
.place-sheet dl {
  margin: 0;
  display: grid;
}
.place-sheet dl div {
  display: grid;
  grid-template-columns: 13em minmax(0, 1fr);
  gap: 12px;
  align-items: baseline;
  min-width: 0;
  padding: 7px 0;
  border-bottom: 1px dotted rgb(255 255 255 / 7%);
}
.place-sheet dl div:last-child {
  border-bottom: 0;
}
.place-sheet dt {
  color: var(--theme-neutral-text-muted, #aebdb1);
}
.place-sheet dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
  overflow-wrap: break-word;
}
.place-modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 14px 24px 18px;
  border-top: 1px solid rgb(255 255 255 / 10%);
}
.place-guide {
  display: grid;
  gap: 10px;
  padding: 14px 24px;
  border-top: 1px solid rgb(255 255 255 / 10%);
  background: rgb(0 0 0 / 15%);
}
.place-guide > button {
  justify-self: start;
}
.place-guide dl {
  margin: 0;
  display: grid;
  gap: 10px;
}
.place-guide dl div {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.place-guide dt {
  color: #d5ab6c;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}
.place-guide dd {
  margin: 0;
  color: #d9d0c3;
  overflow-wrap: break-word;
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
  margin: 0 8px 8px;
  padding: 8px 10px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 8px;
}
.help {
  margin: 0;
}
.confirm {
  background: #1b2822;
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
.atlas-sidebar small {
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
