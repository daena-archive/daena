<script lang="ts">
import { onMount, tick, type Component } from "svelte";
import { listen } from "@tauri-apps/api/event";
import {
  ChevronDown,
  ChevronUp,
  Circle,
  CircleHelp,
  Copy,
  Download,
  Eye,
  EyeOff,
  Hexagon,
  Link2,
  Lock,
  LockOpen,
  Map as MapIcon,
  Maximize2,
  Minimize2,
  Mountain,
  MousePointer2,
  Move,
  Pencil,
  Redo2,
  RefreshCw,
  RotateCcw,
  Save,
  Slash,
  Square,
  SquarePlus,
  Trash2,
  Undo2,
} from "@lucide/svelte";
import WorkspaceTopbar from "$lib/layout/WorkspaceTopbar.svelte";
import {
  project,
  PHYSICAL_HISTORICAL_PROGRESS_EVENT,
  type Asset,
  type Entity,
  type FieldValue,
  type PhysicalHistoricalProgress,
  type PhysicalHistoricalProducts,
  type AtlasRenderRequest,
} from "$lib/project/client";
import { VECTOR_MAX_LAYERS, type MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import NativeVectorImporter from "./NativeVectorImporter.svelte";
import MapLocationLinkPanel from "./MapLocationLinkPanel.svelte";
import {
  createNativeVectorEditor,
  liveNativeVectorEditorCount,
  RENDERER_UNAVAILABLE,
  type NativeVectorEditor,
} from "./openlayers-runtime";
import { registerNativeVectorSession } from "./session";
import {
  collectionBytes,
  featureCountForLayer,
  layerFromField,
  parseVectorCollection,
  parseVectorLayers,
  sha256Hex,
} from "./source";
import {
  initialVectorEditorState,
  parseVectorDiagnostic,
  reduceVectorEditor,
  type VectorEditorState,
} from "./editor-state";
import {
  DEFAULT_VECTOR_LAYER_STYLE,
  VECTOR_PROVIDER,
  featureLayerId,
  featureName,
  featureSemanticType,
  type VectorDrawMode,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "./types";
import { lonLatToNormalized, physicalWorldOverlayCoordinates, type ImageOverlayCoordinates } from "./coordinates";
import { paintPhysicalSurface } from "../physical/raster";
import PhysicalWorldView from "../physical/PhysicalWorldView.svelte";
import AtlasRenderPanel from "../atlas/AtlasRenderPanel.svelte";
import AtlasStudioView from "../atlas/AtlasStudioView.svelte";
import MapViewControls from "./MapViewControls.svelte";

let {
  mapId,
  picking = false,
  start = "geojson",
  focusLinkId,
  oncreated,
  oncancel,
  onpick,
  onopen,
  onstate,
}: {
  mapId?: string;
  picking?: boolean;
  start?: "import" | "geojson";
  focusLinkId?: string;
  oncreated?: (map: Entity) => void;
  oncancel?: () => void;
  onpick?: (anchor: MapAnchor) => void;
  onopen?: (entityId: string) => void;
  onstate?: (status: string, detail: unknown) => void;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let editor = $state.raw<NativeVectorEditor | null>(null);
let draft = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let loaded = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let layers = $state<VectorLayerDefinition[]>([]);
let layersField = $state<FieldValue | null>(null);
let mapField = $state<FieldValue | null>(null);
let sourceAsset = $state<Asset | null>(null);
let activeLayerId = $state<string | null>(null);
let tool = $state<VectorDrawMode>("select");
let editorState = $state<VectorEditorState>(initialVectorEditorState());
let busy = $state(false);
let recoveryPath = $state("");
let notice = $state("");
let renamingId = $state<string | null>(null);
let selectedFeature = $state<VectorFeature | null>(null);
let defaultView = $state({ center: [0.5, 0.5] as [number, number], zoom: 1 });
let background = $state<{
  url: string;
  width: number;
  height: number;
  canvas: HTMLCanvasElement;
  coordinates?: ImageOverlayCoordinates;
} | null>(null);
let fullscreen = $state(false);
let physicalMap = $state(false);
let immutablePhysicalLayerIds = $state<Set<string>>(new Set());
let epochOffsetYears = $state(0);
let appliedEpochOffsetYears = $state(0);
let epochBusy = $state(false);
let epochNotice = $state("");
let epochPhase = $state("");
let epochProgress = $state<{ completed: number; total: number } | null>(null);
let activeEpochRequestId = "";
let epochRequest = 0;
let epochTimer: ReturnType<typeof setTimeout> | undefined;
let eventKind = $state<"earthquake" | "eruption">("earthquake");
let eventStartYears = $state(-10_000);
let eventEndYears = $state(10_000);
let eventMaxEvents = $state(8);
let eventHazardSeed = $state(7_331);
let eventBusy = $state(false);
let eventNotice = $state("");
let eventRequestId = $state<string | null>(null);
let eventRequestSignature = $state("");
let atlasOpen = $state(false);
let atlasSupported = $state(false);
let studioOpen = $state(false);
let studioSupported = $state(false);
let studioExport = $state<AtlasRenderRequest | null>(null);
let studioStage = $state("");
let studioApi = $state<{
  refresh: () => void;
  requestRegenerate: () => void;
  toggleHelp: () => void;
  exportView: () => AtlasRenderRequest | null;
} | null>(null);
let layersCollapsed = $state(false);
let historyCollapsed = $state(true);
let epochEra = $state<"past" | "future">("past");
let epochYearsAbs = $state(0);
let physicalLayerVisibility = $state<Map<string, boolean>>(new Map());
let sidebarWidth = $state(260);

let loadGeneration = 0;
let saveGeneration = 0;
const objectUrls: string[] = [];
let featureLinks = $state(new Map<string, { entityId: string; locationId: string; label: string | null }>());
let linkAnchors = $state(new Map<string, MapAnchor>());
let layersFieldRevision = "";
let layerMutationChain: Promise<void> = Promise.resolve();
let linkPanelOpen = $state(false);
let linkArming = $state(false);
let linkAnchor = $state<MapAnchor | null>(null);
let pinsReady = $state(false);
let physicalEditor = $state<NativeVectorEditor | null>(null);

const SIDEBAR_MIN = 180;
const SIDEBAR_MAX = 520;

const EPOCH_MIN = -100_000;
const EPOCH_MAX = 100_000;
const EPOCH_STEP = 10;

const listedLayers = $derived(
  [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id)),
);
const brandIcon = $derived((physicalMap ? Mountain : MapIcon) as Component);
const iconProps = { size: 15, strokeWidth: 1.8, "aria-hidden": true } as const;

const activeLayer = $derived(layers.find((layer) => layer.id === activeLayerId) ?? null);
const canDraw = $derived(
  Boolean(activeLayer) &&
    !activeLayer?.locked &&
    !picking &&
    !linkArming &&
    !immutablePhysicalLayerIds.has(activeLayer?.id ?? ""),
);
const pickArmed = $derived(Boolean(picking || linkArming));
const dirty = $derived(editorState.dirty);
const diagnostic = $derived(editorState.diagnostic);
const diagnosticCode = $derived(editorState.diagnosticCode);
const conflict = $derived(editorState.conflict);

function publish(status: string, detail: unknown = null) {
  onstate?.(status, detail);
}

function featureFallbackPoint(feature: VectorFeature | null): [number, number] {
  if (!feature) return [0.5, 0.5];
  const positions = feature.geometry.coordinates.flat(Infinity) as number[];
  if (positions.length < 2) return [0.5, 0.5];
  return lonLatToNormalized(positions[0], positions[1]);
}

function pickAnchorFor(feature: VectorFeature): MapAnchor {
  return {
    kind: "provider-feature",
    provider: VECTOR_PROVIDER,
    featureKind: "geojson-feature",
    featureId: feature.id,
    fallbackPoint: featureFallbackPoint(feature),
  };
}

function closeLinkPanel() {
  linkPanelOpen = false;
  linkArming = false;
  linkAnchor = null;
  if (!picking && canDraw) editor?.setMode(tool);
  else if (!picking) editor?.setMode("static");
}

function openLinkPanel(anchor: MapAnchor) {
  linkAnchor = anchor;
  linkArming = false;
  linkPanelOpen = true;
  editor?.setMode("static");
}

function requestLinkFromToolbar() {
  if (!mapId || studioOpen) return;
  if (linkArming) {
    linkArming = false;
    if (!linkAnchor) linkPanelOpen = false;
    if (!picking && canDraw) editor?.setMode(tool);
    return;
  }
  if (selectedFeature) {
    openLinkPanel(pickAnchorFor(selectedFeature));
    return;
  }
  if (linkAnchor && linkPanelOpen) {
    linkArming = true;
    editor?.setMode("static");
    return;
  }
  linkPanelOpen = true;
  linkArming = true;
  linkAnchor = null;
  editor?.setMode("static");
}

async function refreshFeatureLinks() {
  if (!mapId) return;
  const pins = await project.listMapPins(mapId).catch(() => []);
  applyFeatureLinks(pins);
}

function applyFeatureLinks(
  pins: Array<{
    id: string;
    entityId: string;
    anchor?: unknown;
    anchorKind?: string;
    provider?: string | null;
    featureKind?: string | null;
    featureId?: string | null;
    bounds?: [number | null, number | null, number | null, number | null];
  }>,
) {
  const nextFeatureLinks = new Map<string, { entityId: string; locationId: string; label: string | null }>();
  const nextAnchors = new Map<string, MapAnchor>();
  for (const pin of pins) {
    let anchor: MapAnchor | null = null;
    const raw = pin.anchor;
    if (raw && typeof raw === "object" && "kind" in raw) {
      const candidate = raw as MapAnchor;
      if (candidate.kind === "provider-feature" && typeof candidate.featureId === "string") {
        anchor = candidate;
      } else if (candidate.kind === "point" && Array.isArray(candidate.point) && candidate.point.length >= 2) {
        anchor = {
          kind: "point",
          point: [Number(candidate.point[0]), Number(candidate.point[1])],
        };
      }
    }
    if (!anchor && pin.anchorKind === "provider-feature" && typeof pin.featureId === "string") {
      const minX = pin.bounds?.[0];
      const minY = pin.bounds?.[1];
      const maxX = pin.bounds?.[2];
      const maxY = pin.bounds?.[3];
      const fallbackPoint: [number, number] =
        typeof minX === "number" && typeof minY === "number" && typeof maxX === "number" && typeof maxY === "number"
          ? [(minX + maxX) / 2, (minY + maxY) / 2]
          : [0.5, 0.5];
      anchor = {
        kind: "provider-feature",
        provider: pin.provider || VECTOR_PROVIDER,
        featureKind: pin.featureKind || "feature",
        featureId: pin.featureId,
        fallbackPoint,
      };
    }
    if (
      !anchor &&
      pin.anchorKind === "point" &&
      typeof pin.bounds?.[0] === "number" &&
      typeof pin.bounds?.[1] === "number"
    ) {
      anchor = {
        kind: "point",
        point: [pin.bounds[0] as number, pin.bounds[1] as number],
      };
    }
    if (!anchor) continue;
    nextAnchors.set(pin.id, anchor);
    if (anchor.kind === "provider-feature") {
      nextFeatureLinks.set(anchor.featureId, {
        entityId: pin.entityId,
        locationId: pin.id,
        label: null,
      });
    }
  }
  featureLinks = nextFeatureLinks;
  linkAnchors = nextAnchors;
  pinsReady = true;
}

function focusLinkedLocation(linkId: string | null | undefined) {
  if (!linkId || picking || linkArming || !pinsReady) return false;
  const target = editor ?? physicalEditor;
  if (!target) return false;
  const anchor = linkAnchors.get(linkId);
  if (!anchor) return false;
  if (anchor.kind === "provider-feature") return target.focusFeature(anchor.featureId);
  if (anchor.kind === "point") {
    target.focusPoint([anchor.point[0], anchor.point[1]]);
    return true;
  }
  return false;
}

async function requestBack() {
  publish("back");
}

async function setFullscreen(enabled: boolean) {
  if (fullscreen === enabled) return;
  fullscreen = enabled;
  publish("fullscreen", { enabled });
  await tick();
  editor?.resize();
}

function toggleFullscreen() {
  void setFullscreen(!fullscreen);
}

function applyEditorEvent(event: Parameters<typeof reduceVectorEditor>[1]) {
  editorState = reduceVectorEditor(editorState, event);
  publish(editorState.status, {
    code: editorState.diagnosticCode || null,
    detail: editorState.diagnostic || null,
  });
}

function parseDerivedCollection(text: string): VectorFeatureCollection {
  let skipped = 0;
  const collection = parseVectorCollection(new TextEncoder().encode(text), {
    lenient: true,
    onSkipped: () => {
      skipped += 1;
    },
  });
  if (skipped > 0) {
    notice = `${skipped} derived feature${skipped === 1 ? "" : "s"} skipped because of degenerate geometry.`;
  }
  return collection;
}

function cloneCollection(collection: VectorFeatureCollection): VectorFeatureCollection {
  // `draft` is Svelte state and may be a reactive Proxy after an edit. The
  // browser structured-clone algorithm rejects that proxy, while GeoJSON is
  // intentionally JSON-shaped and can be copied safely at this boundary.
  return JSON.parse(JSON.stringify(collection)) as VectorFeatureCollection;
}

function persistedCollection(collection: VectorFeatureCollection): VectorFeatureCollection {
  if (!physicalMap) return collection;
  return {
    type: "FeatureCollection",
    features: collection.features.filter((feature) => !immutablePhysicalLayerIds.has(featureLayerId(feature))),
  };
}

function physicalHillshadeCanvas(products: {
  width: number;
  height: number;
  seaLevelMm: number;
  waterLevelMm: number[];
  hillshadePpm: number[];
  bathymetryMm: number[];
  lakeCells?: boolean[];
}): HTMLCanvasElement {
  return paintPhysicalSurface(products);
}

function applyLayersField(field: FieldValue) {
  layersField = field;
  layersFieldRevision = field.revision;
  layers = withPhysicalVisibility(parseVectorLayers(field.value));
}

function withPhysicalVisibility(next: VectorLayerDefinition[]) {
  if (physicalLayerVisibility.size === 0) return next;
  return next.map((layer) => {
    const visible = physicalLayerVisibility.get(layer.id);
    return visible === undefined ? layer : { ...layer, defaultVisible: visible };
  });
}

function destroyEditor() {
  editor?.dispose();
  editor = null;
}

function formatEpoch(offset: number): string {
  if (offset === 0) return "Reference epoch";
  return `${offset > 0 ? "+" : ""}${offset.toLocaleString()} years`;
}

function clampEpoch(offset: number, step = 1) {
  const snapped = step > 1 ? Math.round(offset / step) * step : Math.round(offset);
  return Math.min(EPOCH_MAX, Math.max(EPOCH_MIN, snapped));
}

function syncEpochFields(offset: number) {
  epochOffsetYears = offset;
  epochYearsAbs = Math.abs(offset);
  if (offset < 0) epochEra = "past";
  else if (offset > 0) epochEra = "future";
}

function commitEpoch(offset: number) {
  const next = clampEpoch(offset);
  syncEpochFields(next);
  scheduleEpoch(next);
}

function commitEpochFromExact(absYears: number, era: "past" | "future") {
  const magnitude = Math.max(0, Math.round(Number.isFinite(absYears) ? absYears : 0));
  epochYearsAbs = magnitude;
  epochEra = era;
  commitEpoch(era === "past" ? -magnitude : magnitude);
}

function parseEpochYears(raw: string) {
  const digits = raw.replace(/[^\d]/g, "");
  const value = digits ? Number(digits) : 0;
  return Math.min(EPOCH_MAX, value);
}

function startSidebarResize(event: PointerEvent) {
  const handle = event.currentTarget;
  if (!(handle instanceof HTMLElement)) return;
  event.preventDefault();
  handle.setPointerCapture(event.pointerId);
  const originX = event.clientX;
  const originWidth = sidebarWidth;
  const onMove = (move: PointerEvent) => {
    sidebarWidth = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, originWidth + (move.clientX - originX)));
  };
  const onUp = () => {
    handle.removeEventListener("pointermove", onMove);
    handle.removeEventListener("pointerup", onUp);
    editor?.resize();
  };
  handle.addEventListener("pointermove", onMove);
  handle.addEventListener("pointerup", onUp);
}

function handleHistoricalProgress(progress: PhysicalHistoricalProgress) {
  if (progress.mapEntityId !== mapId || progress.requestId !== activeEpochRequestId) return;
  epochPhase = progress.phase;
  epochProgress = { completed: progress.completed, total: progress.total };
}

function applyHistoricalProducts(products: PhysicalHistoricalProducts) {
  const authoredDraft = draft.features.filter(
    (feature) => !immutablePhysicalLayerIds.has(featureLayerId(feature)),
  );
  const authoredLoaded = loaded.features.filter(
    (feature) => !immutablePhysicalLayerIds.has(featureLayerId(feature)),
  );
  const physical = parseDerivedCollection(products.geojson);
  draft = cloneCollection({ type: "FeatureCollection", features: [...physical.features, ...authoredDraft] });
  loaded = cloneCollection({ type: "FeatureCollection", features: [...physical.features, ...authoredLoaded] });
  if (background?.url) URL.revokeObjectURL(background.url);
  background = {
    url: "",
    canvas: physicalHillshadeCanvas(products.hydrology),
    width: products.hydrology.width,
    height: products.hydrology.height,
    coordinates: physicalWorldOverlayCoordinates(),
  };
  epochOffsetYears = products.epochOffsetYears;
  appliedEpochOffsetYears = products.epochOffsetYears;
  syncEpochFields(products.epochOffsetYears);
}

async function loadPhysicalEpoch(offset: number) {
  if (!mapId || !physicalMap) return;
  const request = ++epochRequest;
  const requestId = crypto.randomUUID();
  activeEpochRequestId = requestId;
  epochBusy = true;
  epochPhase = "Starting historical derivation";
  epochProgress = { completed: 0, total: 1 };
  epochNotice = "Deriving climate, water, and geography…";
  try {
    const products = await project.physicalMapDerivedEpoch(mapId, offset, requestId);
    if (request !== epochRequest) return;
    applyHistoricalProducts(products);
    epochNotice = `Showing ${formatEpoch(products.epochOffsetYears)} · deterministic derived playback`;
    epochPhase = "";
    epochProgress = null;
  } catch (cause) {
    if (request !== epochRequest) return;
    epochOffsetYears = appliedEpochOffsetYears;
    syncEpochFields(appliedEpochOffsetYears);
    epochPhase = "";
    epochProgress = null;
    epochNotice = cause instanceof Error ? cause.message : String(cause);
  } finally {
    if (request === epochRequest) epochBusy = false;
  }
}

async function materializePhysicalEvents() {
  if (!mapId || !physicalMap || eventBusy) return;
  const requestSignature = JSON.stringify([
    mapId,
    eventKind,
    eventStartYears,
    eventEndYears,
    eventMaxEvents,
    eventHazardSeed,
  ]);
  if (eventRequestSignature !== requestSignature) {
    eventRequestId = crypto.randomUUID();
    eventRequestSignature = requestSignature;
  }
  const requestId = eventRequestId ?? crypto.randomUUID();
  eventRequestId = requestId;
  eventBusy = true;
  eventNotice = "Sampling and committing natural events…";
  try {
    const result = await project.physicalMaterializeEvents(
      mapId,
      {
        eventKind: eventKind,
        intervalStartYears: eventStartYears,
        intervalEndYears: eventEndYears,
        maxEvents: eventMaxEvents,
        hazardSeed: eventHazardSeed,
      },
      { requestId },
    );
    eventNotice = result.events.length
      ? `Committed ${result.events.length} ${eventKind} event${result.events.length === 1 ? "" : "s"} as durable history.`
      : "No events sampled for this bounded interval and hazard seed.";
    eventRequestId = null;
    eventRequestSignature = "";
  } catch (cause) {
    eventNotice = cause instanceof Error ? cause.message : String(cause);
  } finally {
    eventBusy = false;
  }
}

function scheduleEpoch(offset: number) {
  epochOffsetYears = offset;
  if (epochTimer) clearTimeout(epochTimer);
  epochTimer = setTimeout(() => {
    epochTimer = undefined;
    void loadPhysicalEpoch(offset);
  }, 180);
}

function mountEditor() {
  if (physicalMap) {
    destroyEditor();
    publish("ready", { liveEditors: liveNativeVectorEditorCount(), renderer: "openlayers" });
    return;
  }
  if (!host) return;
  destroyEditor();
  const created = createNativeVectorEditor(host, {
    get draft() {
      return draft;
    },
    get layers() {
      return layers;
    },
    get activeLayerId() {
      return activeLayerId;
    },
    get center() {
      return defaultView.center;
    },
    get zoom() {
      return defaultView.zoom;
    },
    setDraft(next) {
      draft = next;
    },
    setActiveLayerId(id) {
      activeLayerId = id;
    },
    onDirty() {
      applyEditorEvent({ type: "geometry-changed" });
    },
    onDiagnostic(code, detail) {
      applyEditorEvent({ type: "save-failed", message: `${code}: ${detail}` });
      if (code === RENDERER_UNAVAILABLE) publish("error", { code, detail });
    },
    onSelect(feature) {
      selectedFeature = feature;
      if (picking) return;
      if (feature) {
        const linked = featureLinks.get(feature.id);
        if (linked && onopen) onopen(linked.entityId);
      }
    },
    onDoubleClick(featureId) {
      const linked = featureLinks.get(featureId);
      if (linked && onopen) onopen(linked.entityId);
    },
    get pickArmed() {
      return pickArmed;
    },
    onMapPick(anchor) {
      if (picking && onpick) {
        onpick(anchor);
        return;
      }
      if (linkArming || linkPanelOpen) openLinkPanel(anchor);
    },
    get background() {
      return background;
    },
    onViewChange(next) {
      defaultView = {
        center: lonLatToNormalized(next.center[0], next.center[1]),
        zoom: next.zoom,
      };
    },
  });
  if ("error" in created) {
    applyEditorEvent({ type: "save-failed", message: `${created.error}: ${created.detail}` });
    publish("error", created);
    return;
  }
  editor = created;
  if (!canDraw) editor.setMode("static");
  else editor.setMode(tool);
  publish("ready", { liveEditors: liveNativeVectorEditorCount(), renderer: "openlayers" });
}

async function load() {
  if (!mapId) return;
  const generation = ++loadGeneration;
  epochRequest += 1;
  epochBusy = false;
  epochPhase = "";
  epochProgress = null;
  busy = true;
  pinsReady = false;
  linkAnchors = new Map();
  featureLinks = new Map();
  try {
    const fields = await project.listFields(mapId);
    if (generation !== loadGeneration) return;
    const descriptorField = fields.find((field) => field.namespace === "maps" && field.key === "map");
    mapField = descriptorField ?? null;
    const descriptor = descriptorField?.value as {
      provider?: { id?: string };
      sourceAssetId?: string;
      authoredSourceAssetId?: string;
      defaultView?: { center?: [number, number]; zoom?: number };
    };
    physicalMap = descriptor?.provider?.id === "daena-physical";
    atlasSupported = false;
    atlasOpen = false;
    studioSupported = false;
    try {
      const capabilities = await project.atlasCapabilities(mapId);
      atlasSupported = capabilities.supported;
      studioSupported = capabilities.supportsStudio;
    } catch {
      atlasSupported = false;
      studioSupported = false;
    }
    if (generation !== loadGeneration) return;
    studioOpen = studioSupported;
    if (descriptor?.defaultView?.center) defaultView = { ...defaultView, center: descriptor.defaultView.center };
    if (typeof descriptor?.defaultView?.zoom === "number")
      defaultView = { ...defaultView, zoom: descriptor.defaultView.zoom };
    const nextLayersField = fields.find((item) => item.namespace === "maps" && item.key === "layers") ?? null;
    if (!nextLayersField) throw new Error("maps:layers is missing");
    applyLayersField(nextLayersField);
    const assets = await project.listAssets(mapId);
    if (generation !== loadGeneration) return;
    const sourceId = physicalMap ? descriptor?.authoredSourceAssetId : descriptor?.sourceAssetId;
    const source = assets.find((asset) => asset.id === sourceId);
    if (!source) throw new Error("The vector source asset is missing");
    sourceAsset = source;
    if (background?.url) URL.revokeObjectURL(background.url);
    background = null;
    const bytes = await project.readAssetBytes(source.id);
    if (generation !== loadGeneration) return;
    const collection = parseVectorCollection(bytes);
    if (physicalMap) {
      epochOffsetYears = 0;
      appliedEpochOffsetYears = 0;
      immutablePhysicalLayerIds = new Set([
        "base",
        "ocean",
        "land",
        "shelves",
        "bathymetric-contours",
        "tectonic-plates",
        "tectonic-boundaries",
        "bathymetry",
        "volcanic-centers",
        "earthquake-hazard",
        "volcanic-hazard",
        "lakes",
        "rivers",
        "watersheds",
        "islands",
        "ice",
      ]);
      physicalLayerVisibility = new Map([...immutablePhysicalLayerIds].map((id) => [id, false]));
      layers = withPhysicalVisibility(layers);
      syncEpochFields(0);
      const requestId = crypto.randomUUID();
      activeEpochRequestId = requestId;
      epochBusy = true;
      epochPhase = "Starting historical derivation";
      epochProgress = { completed: 0, total: 1 };
      const historical = await project.physicalMapDerivedEpoch(mapId, 0, requestId);
      if (generation !== loadGeneration) return;
      const physical = parseDerivedCollection(historical.geojson);
      const combined = { type: "FeatureCollection" as const, features: [...physical.features, ...collection.features] };
      draft = cloneCollection(combined);
      loaded = cloneCollection(combined);
      epochNotice = `Showing ${formatEpoch(historical.epochOffsetYears)} · deterministic derived playback`;
      background = {
        url: "",
        canvas: physicalHillshadeCanvas(historical.hydrology),
        width: historical.hydrology.width,
        height: historical.hydrology.height,
        coordinates: physicalWorldOverlayCoordinates(),
      };
      epochBusy = false;
      epochPhase = "";
      epochProgress = null;
    } else {
      immutablePhysicalLayerIds = new Set();
      physicalLayerVisibility = new Map();
      epochNotice = "";
      draft = cloneCollection(collection);
      loaded = cloneCollection(collection);
    }
    const previewId = (descriptorField?.value as { previewAssetId?: string | null } | undefined)?.previewAssetId;
    const preview = previewId ? assets.find((asset) => asset.id === previewId) : null;
    if (preview) {
      const previewBytes = await project.readAssetBytes(preview.id);
      if (generation !== loadGeneration) return;
      const url = URL.createObjectURL(new Blob([new Uint8Array(previewBytes)], { type: preview.mime_type }));
      objectUrls.push(url);
      background = await new Promise((resolve, reject) => {
        const image = new Image();
        image.onload = () => {
          if (generation !== loadGeneration) {
            URL.revokeObjectURL(url);
            resolve(null);
            return;
          }
          const canvas = document.createElement("canvas");
          canvas.width = image.naturalWidth;
          canvas.height = image.naturalHeight;
          const context = canvas.getContext("2d");
          if (!context) {
            URL.revokeObjectURL(url);
            reject(new Error("Could not decode the imported map image"));
            return;
          }
          context.drawImage(image, 0, 0);
          resolve({ url, width: image.naturalWidth, height: image.naturalHeight, canvas });
        };
        image.onerror = () => {
          URL.revokeObjectURL(url);
          reject(new Error("Could not decode the imported map image"));
        };
        image.src = url;
      });
      if (generation !== loadGeneration) return;
    }
    {
      const pins = await project.listMapPins(mapId).catch(() => []);
      if (generation !== loadGeneration) return;
      applyFeatureLinks(pins);
    }
    const ordered = [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id));
    activeLayerId = ordered.some((layer) => layer.id === activeLayerId) ? activeLayerId : (ordered[0]?.id ?? null);
    tool = "select";
    selectedFeature = null;
    applyEditorEvent({ type: "loaded" });
    recoveryPath = "";
    notice = "";
    await tick();
    if (generation !== loadGeneration) return;
    mountEditor();
    await tick();
    if (generation !== loadGeneration) return;
    focusLinkedLocation(focusLinkId);
  } catch (cause) {
    if (generation !== loadGeneration) return;
    applyEditorEvent({
      type: "save-failed",
      message: cause instanceof Error ? cause.message : String(cause),
    });
    publish("error", { message: editorState.diagnostic });
  } finally {
    if (generation === loadGeneration) {
      busy = false;
      epochBusy = false;
      epochPhase = "";
      epochProgress = null;
    }
  }
}

async function save() {
  if (!mapId || !sourceAsset || !mapField || !layersField || busy) return;
  if (!dirty) {
    applyEditorEvent({ type: "save-succeeded" });
    return;
  }
  const generation = ++saveGeneration;
  busy = true;
  applyEditorEvent({ type: "save-started" });
  try {
    editor?.flush();
    const snapshot = cloneCollection(persistedCollection(draft));
    const bytes = collectionBytes(snapshot);
    const hash = await sha256Hex(bytes);
    const applied = await project.applyMapEdit({
      mapEntityId: mapId,
      descriptor: mapField.value,
      layers: layersField.value,
      bytes,
      uploadContentHash: hash,
      expectedMapRevision: mapField.revision,
      expectedLayersRevision: layersField.revision,
      expectedSourceRevision: sourceAsset.revision,
      linkMutations: [],
    });
    if (generation !== saveGeneration) return;
    mapField = applied.map;
    layersField = applied.layers;
    layersFieldRevision = applied.layers.revision;
    sourceAsset = applied.source;
    loaded = cloneCollection(snapshot);
    recoveryPath = "";
    if (JSON.stringify(persistedCollection(draft)) === JSON.stringify(snapshot)) {
      applyEditorEvent({ type: "save-succeeded" });
    } else {
      applyEditorEvent({ type: "geometry-changed" });
    }
  } catch (cause) {
    if (generation !== saveGeneration) return;
    const text = cause instanceof Error ? cause.message : String(cause);
    const parsed = parseVectorDiagnostic(text);
    if (parsed.code === "asset.revision-conflict" || text.toLowerCase().includes("revision conflict")) {
      applyEditorEvent({ type: "save-conflict", message: text });
    } else {
      applyEditorEvent({ type: "save-failed", message: text });
    }
  } finally {
    if (generation === saveGeneration) busy = false;
  }
}

async function exportDraft() {
  if (!mapId || !mapField || !layersField) return;
  try {
    const packageBytes = new TextEncoder().encode(
      JSON.stringify({
        schemaVersion: 1,
        kind: "daena-map-edit-draft",
        mapEntityId: mapId,
        descriptor: mapField.value,
        layers: layersField.value,
        geojson: new TextDecoder().decode(collectionBytes(persistedCollection(draft))),
        linkMutations: [],
      }),
    );
    recoveryPath = await project.mapsRecoveryExport(mapId, packageBytes);
    notice = `Draft exported to ${recoveryPath}`;
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
  }
}

function isDirty() {
  return editorState.dirty;
}

function setTool(next: VectorDrawMode) {
  if (!canDraw && next !== "static" && next !== "select") return;
  tool = next;
  editor?.setMode(!canDraw ? "static" : next);
}

function switchLayer(layerId: string) {
  if (layerId === activeLayerId) return;
  editor?.switchLayer(layerId);
  activeLayerId = layerId;
  const layer = layers.find((item) => item.id === layerId);
  tool = layer?.locked ? "static" : "select";
  editor?.setMode(tool);
}

async function addLayer() {
  if (!mapId || !layersField || layers.length >= VECTOR_MAX_LAYERS) return;
  layerMutationChain = layerMutationChain.then(() => runAddLayer()).catch(() => {});
  await layerMutationChain;
}

async function runAddLayer() {
  if (!mapId || !layersField || layers.length >= VECTOR_MAX_LAYERS) return;
  busy = true;
  try {
    const change = await project.createVectorLayer(mapId, `Layer ${layers.length + 1}`, layersFieldRevision, {
      style: { ...DEFAULT_VECTOR_LAYER_STYLE },
    });
    applyLayersField(change.layers);
    const created = layerFromField(change.layers.value as { layers?: Array<Record<string, unknown>> }, change.layer_id);
    if (created) switchLayer(created.id);
    else activeLayerId = change.layer_id;
    editor?.syncLayers(layers);
    tool = "select";
    editor?.setMode("select");
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
  } finally {
    busy = false;
  }
}

async function duplicateLayer(layer: VectorLayerDefinition) {
  if (!mapId || !layersField || layers.length >= VECTOR_MAX_LAYERS) return;
  layerMutationChain = layerMutationChain.then(() => runDuplicateLayer(layer)).catch(() => {});
  await layerMutationChain;
}

async function runDuplicateLayer(layer: VectorLayerDefinition) {
  if (!mapId || !layersField || layers.length >= VECTOR_MAX_LAYERS) return;
  busy = true;
  try {
    const change = await project.createVectorLayer(mapId, `${layer.name} copy`, layersFieldRevision, {
      style: { ...layer.style },
    });
    applyLayersField(change.layers);
    editor?.syncLayers(layers);
    editor?.duplicateLayerFeatures(layer.id, change.layer_id);
    switchLayer(change.layer_id);
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
  } finally {
    busy = false;
  }
}

function mutateLayer(layer: VectorLayerDefinition, update: Parameters<typeof project.updateMapLayer>[3]) {
  layerMutationChain = layerMutationChain.then(() => runLayerMutation(layer, update)).catch(() => {});
  return layerMutationChain;
}

async function runLayerMutation(layer: VectorLayerDefinition, update: Parameters<typeof project.updateMapLayer>[3]) {
  if (!mapId || !layersField) return;
  try {
    const change = await project.updateMapLayer(mapId, layer.id, layersFieldRevision, update);
    applyLayersField(change.layers);
    editor?.syncLayers(layers);
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
    await refreshLayersField();
  }
}

async function refreshLayersField() {
  if (!mapId) return;
  try {
    const fields = await project.listFields(mapId);
    const next = fields.find((item) => item.namespace === "maps" && item.key === "layers") ?? null;
    if (next) applyLayersField(next);
  } catch {
    // Keep the current field; the next mutation retries with the same revision.
  }
}

async function toggleVisible(layer: VectorLayerDefinition) {
  const nextVisible = !layer.defaultVisible;
  if (physicalLayerVisibility.has(layer.id)) {
    physicalLayerVisibility.set(layer.id, nextVisible);
    physicalLayerVisibility = new Map(physicalLayerVisibility);
    layers = layers.map((item) => (item.id === layer.id ? { ...item, defaultVisible: nextVisible } : item));
    editor?.syncLayers(layers);
    return;
  }
  layer.defaultVisible = nextVisible;
  layers = [...layers];
  editor?.syncLayers(layers);
  await mutateLayer(layer, { defaultVisible: nextVisible });
}

async function toggleLock(layer: VectorLayerDefinition) {
  if (physicalMap && immutablePhysicalLayerIds.has(layer.id)) return;
  layer.locked = !layer.locked;
  layers = [...layers];
  if (layer.id === activeLayerId) {
    tool = layer.locked ? "static" : "select";
    editor?.switchLayer(layer.id);
    editor?.setMode(tool);
  }
  await mutateLayer(layer, { locked: layer.locked });
}

async function renameLayer(layer: VectorLayerDefinition, name: string) {
  if (physicalMap && immutablePhysicalLayerIds.has(layer.id)) return;
  const trimmed = name.trim();
  renamingId = null;
  if (!trimmed || trimmed === layer.name) return;
  layer.name = trimmed;
  layers = [...layers];
  await mutateLayer(layer, { name: trimmed });
}

async function moveLayer(layer: VectorLayerDefinition, direction: -1 | 1) {
  if (physicalMap && immutablePhysicalLayerIds.has(layer.id)) return;
  const index = listedLayers.findIndex((item) => item.id === layer.id);
  const neighbor = listedLayers[index + direction];
  if (!neighbor) return;
  const layerOrder = layer.order;
  await mutateLayer(layer, { order: neighbor.order });
  await mutateLayer(neighbor, { order: layerOrder });
}

async function updateStyle(layer: VectorLayerDefinition, patch: Partial<VectorLayerDefinition["style"]>) {
  if (physicalMap && immutablePhysicalLayerIds.has(layer.id)) return;
  const style = { ...layer.style, ...patch };
  layer.style = style;
  layers = [...layers];
  editor?.syncLayers(layers);
  await mutateLayer(layer, { style });
}

async function removeLayer(layer: VectorLayerDefinition) {
  if (physicalMap && immutablePhysicalLayerIds.has(layer.id)) return;
  if (!mapId || !layersField || !sourceAsset) return;
  const savedCount = featureCountForLayer(loaded, layer.id);
  const draftCount = featureCountForLayer(draft, layer.id);
  const extra =
    draftCount === savedCount ? "" : ` Unsaved draft features on this layer (${draftCount}) will be discarded.`;
  if (
    !confirm(
      `Delete ${layer.name}? This removes ${savedCount} saved feature${savedCount === 1 ? "" : "s"} from the map.${extra}`,
    )
  ) {
    return;
  }
  layerMutationChain = layerMutationChain.then(() => runRemoveLayer(layer, savedCount)).catch(() => {});
  await layerMutationChain;
}

async function runRemoveLayer(layer: VectorLayerDefinition, savedCount: number) {
  if (!mapId || !layersField || !sourceAsset) return;
  busy = true;
  try {
    const change = await project.deleteVectorLayer(
      mapId,
      layer.id,
      layersFieldRevision,
      sourceAsset.revision,
      savedCount,
    );
    applyLayersField(change.layers);
    sourceAsset = change.source;
    draft = {
      type: "FeatureCollection",
      features: draft.features.filter((feature) => featureLayerId(feature) !== layer.id),
    };
    loaded = {
      type: "FeatureCollection",
      features: loaded.features.filter((feature) => featureLayerId(feature) !== layer.id),
    };
    const remaining = [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id));
    activeLayerId = remaining[0]?.id ?? null;
    await tick();
    mountEditor();
  } catch (cause) {
    const text = cause instanceof Error ? cause.message : String(cause);
    const parsed = parseVectorDiagnostic(text);
    if (parsed.code === "asset.revision-conflict" || parsed.code === "vector.layer.in-use") {
      applyEditorEvent({ type: "save-conflict", message: text });
    } else {
      applyEditorEvent({ type: "save-failed", message: text });
    }
    await refreshLayersField();
  } finally {
    busy = false;
  }
}

$effect(() => {
  focusLinkId;
  editor;
  physicalEditor;
  pinsReady;
  linkAnchors;
  picking;
  linkArming;
  focusLinkedLocation(focusLinkId);
});

$effect(() => {
  if ((picking || linkArming) && editor) editor.setMode("static");
});

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape" && linkPanelOpen) {
    event.preventDefault();
    closeLinkPanel();
    return;
  }
  if (event.key === "Escape" && fullscreen) {
    event.preventDefault();
    void setFullscreen(false);
    return;
  }
  if (event.target instanceof HTMLElement && event.target.closest("input, textarea, select, [contenteditable=true]")) {
    return;
  }
  const meta = event.metaKey || event.ctrlKey;
  if (meta && event.key.toLowerCase() === "s") {
    event.preventDefault();
    void save();
  } else if (meta && event.key.toLowerCase() === "z") {
    event.preventDefault();
    if (event.shiftKey) editor?.redo();
    else editor?.undo();
  } else if (!meta && !renamingId && !picking) {
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      editor?.deleteSelection();
    } else if (event.key === "v" || event.key === "h") setTool("static");
    if (event.key === "s") setTool("select");
    if (event.key === "p") setTool("point");
    if (event.key === "l") setTool("linestring");
    if (event.key === "g") setTool("polygon");
    if (event.key === "r") setTool("rectangle");
    if (event.key === "f") setTool("freehand");
  }
}

onMount(() => {
  if (!mapId) return;
  let mounted = true;
  let unlistenHistoricalProgress: (() => void) | null = null;
  registerNativeVectorSession({ save, isDirty, teardown: () => editor?.dispose() });
  window.addEventListener("keydown", onKey);
  void listen<PhysicalHistoricalProgress>(PHYSICAL_HISTORICAL_PROGRESS_EVENT, (event) => {
    handleHistoricalProgress(event.payload);
  })
    .then((unlisten) => {
      if (!mounted) {
        unlisten();
        return;
      }
      unlistenHistoricalProgress = unlisten;
      void load();
    })
    .catch(() => {
      if (mounted) void load();
    });
  return () => {
    mounted = false;
    window.removeEventListener("keydown", onKey);
    loadGeneration += 1;
    unlistenHistoricalProgress?.();
    destroyEditor();
    epochRequest += 1;
    if (epochTimer) clearTimeout(epochTimer);
    if (background?.url) URL.revokeObjectURL(background.url);
    for (const url of objectUrls) URL.revokeObjectURL(url);
    objectUrls.length = 0;
    registerNativeVectorSession(null);
  };
});
</script>

{#if !mapId}
  <NativeVectorImporter
    mode={start === "import" ? "image" : "geojson"}
    {oncreated}
    {oncancel}
    onfullscreen={(enabled) => void setFullscreen(enabled)}
    {fullscreen} />
{:else}
  <section class="native-vector-editor" aria-label="Native vector map editor">
    <WorkspaceTopbar
      title={studioOpen ? "Atlas Studio" : physicalMap ? "Physical world" : "Vector map"}
      subtitle={studioOpen && studioStage
        ? studioStage
        : !physicalMap && dirty
          ? "Unsaved changes"
          : physicalMap
            ? "Generated world map"
            : "Map editor"}
      icon={brandIcon}
      onBack={() => void requestBack()}
      actionsLabel={physicalMap ? "Physical map actions" : "Vector drawing tools"}>
      <div class="header-actions" data-workspace-topbar-actions>
        {#if !studioOpen}
          <button
            type="button"
            class="icon-button"
            class:active={linkArming || linkPanelOpen}
            aria-pressed={linkArming || linkPanelOpen}
            aria-label={linkArming ? "Click map to choose a location" : "Link location"}
            title={linkArming ? "Click map to choose a location" : "Link location"}
            disabled={!mapId || picking}
            onclick={() => requestLinkFromToolbar()}><Link2 {...iconProps} /></button>
        {/if}
        {#if !physicalMap}
          <button
            type="button"
            class="icon-button"
            class:active={tool === "static"}
            aria-pressed={tool === "static"}
            aria-label="Pan"
            title="Pan"
            onclick={() => setTool("static")}><Move {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "select"}
            aria-pressed={tool === "select"}
            aria-label="Select"
            title="Select"
            onclick={() => setTool("select")}><MousePointer2 {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "point"}
            aria-pressed={tool === "point"}
            aria-label="Point"
            title="Point"
            disabled={!canDraw}
            onclick={() => setTool("point")}><Circle {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "linestring"}
            aria-pressed={tool === "linestring"}
            aria-label="Line"
            title="Line"
            disabled={!canDraw}
            onclick={() => setTool("linestring")}><Slash {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "polygon"}
            aria-pressed={tool === "polygon"}
            aria-label="Polygon"
            title="Polygon"
            disabled={!canDraw}
            onclick={() => setTool("polygon")}><Hexagon {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "rectangle"}
            aria-pressed={tool === "rectangle"}
            aria-label="Rectangle"
            title="Rectangle"
            disabled={!canDraw}
            onclick={() => setTool("rectangle")}><Square {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "freehand"}
            aria-pressed={tool === "freehand"}
            aria-label="Freehand"
            title="Freehand"
            disabled={!canDraw}
            onclick={() => setTool("freehand")}><Pencil {...iconProps} /></button>
          <button type="button" class="icon-button" aria-label="Undo" title="Undo" onclick={() => editor?.undo()}
            ><Undo2 {...iconProps} /></button>
          <button type="button" class="icon-button" aria-label="Redo" title="Redo" onclick={() => editor?.redo()}
            ><Redo2 {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            aria-label="Add layer"
            title="Add layer"
            disabled={busy || layers.length >= VECTOR_MAX_LAYERS}
            onclick={() => void addLayer()}><SquarePlus {...iconProps} /></button>
          <button
            type="button"
            class="icon-button save"
            aria-label={busy ? "Saving…" : dirty ? "Save" : "Saved"}
            title={busy ? "Saving…" : dirty ? "Save" : "Saved"}
            disabled={busy || !dirty}
            onclick={() => void save()}><Save {...iconProps} /></button>
        {/if}
        {#if studioOpen}
          <button type="button" class="text-button" onclick={() => (studioOpen = false)}>Open Physical Map</button>
          <button
            type="button"
            class="icon-button"
            aria-label="Refresh Atlas"
            title="Refresh Atlas"
            onclick={() => studioApi?.refresh()}><RefreshCw {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            aria-label="Regenerate cache"
            title="Regenerate cache"
            onclick={() => studioApi?.requestRegenerate()}><RotateCcw {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            aria-label="Keyboard help"
            title="Keyboard help"
            onclick={() => studioApi?.toggleHelp()}><CircleHelp {...iconProps} /></button>
        {/if}
        {#if atlasSupported}
          <button
            type="button"
            class="icon-button"
            class:active={atlasOpen}
            aria-pressed={atlasOpen}
            aria-label="Export atlas"
            title="Export atlas"
            onclick={() => {
              if (studioOpen) {
                const request = studioApi?.exportView();
                if (request) studioExport = request;
              }
              atlasOpen = !atlasOpen;
            }}><Download {...iconProps} /></button>
        {/if}
        <button
          type="button"
          class="icon-button"
          class:active={fullscreen}
          aria-label={fullscreen ? "Exit full screen" : "Full screen"}
          aria-pressed={fullscreen}
          title={fullscreen ? "Exit full screen (Esc)" : "Full screen"}
          onclick={toggleFullscreen}
          >{#if fullscreen}<Minimize2 {...iconProps} />{:else}<Maximize2 {...iconProps} />{/if}</button>
      </div>
    </WorkspaceTopbar>
    {#if conflict}
      <p class="error" role="alert">
        This map changed elsewhere. Reload the canonical source, export this draft, or keep editing without saving over
        it.
        <button type="button" onclick={() => void load()}>Reload canonical source</button>
        <button type="button" onclick={() => void exportDraft()}>Export draft</button>
        <button type="button" onclick={() => applyEditorEvent({ type: "keep-editing" })}>Keep editing</button>
      </p>
    {/if}
    {#if diagnostic && !conflict}
      <p class="error" role="alert" data-code={diagnosticCode}>{diagnostic}</p>
    {/if}
    {#if notice}
      <p class="hint" role="status">{notice}</p>
    {/if}
    {#if atlasOpen && mapId}
      <AtlasRenderPanel
        {mapId}
        {epochOffsetYears}
        viewerLayers={layers}
        seed={studioExport}
        onclose={() => (atlasOpen = false)} />
    {/if}
    <div class="editor-body" class:studio={studioOpen} style={`--sidebar-width: ${sidebarWidth}px`}>
      {#if !studioOpen}
        <aside aria-label="Map layers">
          {#if studioSupported}
            <button
              type="button"
              class="studio-open"
              class:active={studioOpen}
              aria-pressed={studioOpen}
              onclick={() => (studioOpen = !studioOpen)}>Atlas Studio</button>
          {/if}
          <button
            type="button"
            class="aside-toggle"
            aria-expanded={!layersCollapsed}
            onclick={() => (layersCollapsed = !layersCollapsed)}>
            <strong id="vector-layers-heading">Vector layers</strong>
            <span class="aside-chevron" class:collapsed={layersCollapsed}><ChevronDown {...iconProps} /></span>
          </button>
          {#if !layersCollapsed}
            {#if physicalMap}
              <p class="hazard-legend">
                Hazard layers show relative generated rates; they are not real-world predictions.
              </p>
            {/if}
            {#if listedLayers.length === 0}
              <p class="hint">Add a vector layer to draw points, lines, and regions. Base geography stays read-only.</p>
            {/if}
            <div class="layer-list" role="list" aria-labelledby="vector-layers-heading">
              {#each listedLayers as layer (layer.id)}
                <div class="layer" class:active={layer.id === activeLayerId} role="listitem">
                  <button
                    class="layer-name"
                    type="button"
                    aria-pressed={layer.id === activeLayerId}
                    onclick={() => switchLayer(layer.id)}>
                    {#if renamingId === layer.id}
                      <input
                        value={layer.name}
                        aria-label="Layer name"
                        onblur={(event) => void renameLayer(layer, event.currentTarget.value)}
                        onkeydown={(event) => {
                          if (event.key === "Enter") void renameLayer(layer, event.currentTarget.value);
                          if (event.key === "Escape") renamingId = null;
                        }} />
                    {:else}{layer.name}{/if}
                  </button>
                  <div class="layer-row">
                    <button
                      type="button"
                      class="icon-button"
                      aria-pressed={layer.defaultVisible}
                      aria-label={layer.defaultVisible ? `Hide ${layer.name}` : `Show ${layer.name}`}
                      title={layer.defaultVisible ? `Hide ${layer.name}` : `Show ${layer.name}`}
                      onclick={() => void toggleVisible(layer)}
                      >{#if layer.defaultVisible}<Eye {...iconProps} />{:else}<EyeOff {...iconProps} />{/if}</button>
                    {#if !immutablePhysicalLayerIds.has(layer.id)}
                      <button
                        type="button"
                        class="icon-button"
                        aria-pressed={layer.locked}
                        aria-label={layer.locked ? `Unlock ${layer.name}` : `Lock ${layer.name}`}
                        title={layer.locked ? `Unlock ${layer.name}` : `Lock ${layer.name}`}
                        onclick={() => void toggleLock(layer)}
                        >{#if layer.locked}<Lock {...iconProps} />{:else}<LockOpen {...iconProps} />{/if}</button>
                      <button
                        type="button"
                        class="icon-button"
                        aria-label={`Rename ${layer.name}`}
                        title="Rename"
                        onclick={() => (renamingId = layer.id)}><Pencil {...iconProps} /></button>
                      <button
                        type="button"
                        class="icon-button"
                        aria-label={`Duplicate ${layer.name}`}
                        title="Duplicate"
                        disabled={busy || layers.length >= VECTOR_MAX_LAYERS}
                        onclick={() => void duplicateLayer(layer)}><Copy {...iconProps} /></button>
                      <button
                        type="button"
                        class="icon-button"
                        aria-label={`Move ${layer.name} up`}
                        title="Up"
                        onclick={() => void moveLayer(layer, -1)}><ChevronUp {...iconProps} /></button>
                      <button
                        type="button"
                        class="icon-button"
                        aria-label={`Move ${layer.name} down`}
                        title="Down"
                        onclick={() => void moveLayer(layer, 1)}><ChevronDown {...iconProps} /></button>
                      <button
                        type="button"
                        class="icon-button"
                        aria-label={`Delete ${layer.name}`}
                        title="Delete"
                        onclick={() => void removeLayer(layer)}><Trash2 {...iconProps} /></button>
                    {/if}
                  </div>
                  {#if layer.id === activeLayerId && !immutablePhysicalLayerIds.has(layer.id)}
                    <div class="style-row">
                      <label>
                        Fill
                        <input
                          type="color"
                          value={layer.style.fill}
                          aria-label={`${layer.name} fill`}
                          onchange={(event) => void updateStyle(layer, { fill: event.currentTarget.value })} />
                      </label>
                      <label>
                        Stroke
                        <input
                          type="color"
                          value={layer.style.stroke}
                          aria-label={`${layer.name} stroke`}
                          onchange={(event) => void updateStyle(layer, { stroke: event.currentTarget.value })} />
                      </label>
                      <label>
                        Layer opacity
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.05"
                          value={layer.style.fillOpacity}
                          aria-label={`${layer.name} opacity`}
                          oninput={(event) =>
                            void updateStyle(layer, { fillOpacity: Number(event.currentTarget.value) })} />
                      </label>
                      <label>
                        Stroke width
                        <input
                          type="number"
                          min="0"
                          max="32"
                          step="0.25"
                          value={layer.style.strokeWidth}
                          aria-label={`${layer.name} stroke width`}
                          onchange={(event) =>
                            void updateStyle(layer, { strokeWidth: Number(event.currentTarget.value) })} />
                      </label>
                      <label>
                        Point radius
                        <input
                          type="number"
                          min="1"
                          max="64"
                          step="1"
                          value={layer.style.pointRadius}
                          aria-label={`${layer.name} point radius`}
                          onchange={(event) =>
                            void updateStyle(layer, { pointRadius: Number(event.currentTarget.value) })} />
                      </label>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
          {#if physicalMap}
            <button
              type="button"
              class="aside-toggle"
              aria-expanded={!historyCollapsed}
              onclick={() => (historyCollapsed = !historyCollapsed)}>
              <strong>Natural history</strong>
              <span class="aside-chevron" class:collapsed={historyCollapsed}><ChevronDown {...iconProps} /></span>
            </button>
            {#if !historyCollapsed}
              <div class="event-control" aria-label="Materialize natural history">
                <label>
                  Event
                  <select bind:value={eventKind} disabled={eventBusy || busy}>
                    <option value="earthquake">Earthquake</option>
                    <option value="eruption">Eruption</option>
                  </select>
                </label>
                <label>
                  From (years)
                  <input
                    type="number"
                    min="-100000"
                    max="100000"
                    step="1"
                    bind:value={eventStartYears}
                    disabled={eventBusy || busy} />
                </label>
                <label>
                  To (years)
                  <input
                    type="number"
                    min="-100000"
                    max="100000"
                    step="1"
                    bind:value={eventEndYears}
                    disabled={eventBusy || busy} />
                </label>
                <label>
                  Max events
                  <input
                    type="number"
                    min="1"
                    max="128"
                    step="1"
                    bind:value={eventMaxEvents}
                    disabled={eventBusy || busy} />
                </label>
                <label>
                  Hazard seed
                  <input type="number" min="0" step="1" bind:value={eventHazardSeed} disabled={eventBusy || busy} />
                </label>
                <button type="button" disabled={eventBusy || busy} onclick={() => void materializePhysicalEvents()}>
                  {eventBusy ? "Committing…" : "Commit events"}
                </button>
                <small
                  >Creates revisioned entities and map links; generated hazards remain read-only and are not
                  predictions.</small>
                {#if eventNotice}<small role="status">{eventNotice}</small>{/if}
              </div>
            {/if}
          {/if}
          {#if selectedFeature && !physicalMap}
            <div class="inspector" aria-label="Selected feature">
              <strong>Selected feature</strong>
              <p class="hint">
                {featureSemanticType(selectedFeature)} · {featureLayerId(selectedFeature) === "base"
                  ? "base geography"
                  : "authored"}
              </p>
              <label>
                Name
                <input
                  value={featureName(selectedFeature) ?? ""}
                  maxlength="256"
                  aria-label="Feature name"
                  disabled={featureLayerId(selectedFeature) === "base" || activeLayer?.locked}
                  onchange={(event) => {
                    const next = event.currentTarget.value.trim() || null;
                    editor?.updateSelectedName(next);
                  }} />
              </label>
              <label>
                Layer
                <select
                  value={featureLayerId(selectedFeature)}
                  aria-label="Feature layer"
                  disabled={featureLayerId(selectedFeature) === "base" || activeLayer?.locked}
                  onchange={(event) => editor?.moveSelectionToLayer(event.currentTarget.value)}>
                  {#each listedLayers.filter((layer) => layer.id !== "base" && layer.defaultVisible && !layer.locked) as layer}
                    <option value={layer.id}>{layer.name}</option>
                  {/each}
                </select>
              </label>
              <button
                type="button"
                disabled={featureLayerId(selectedFeature) === "base" || activeLayer?.locked}
                onclick={() => editor?.duplicateSelection()}>Duplicate feature</button>
            </div>
          {/if}
          {#if !physicalMap}
            <p class="hint">
              Base geography is read-only. Point, line, polygon, rectangle, and freehand edits save through the
              canonical GeoJSON source. Delete removes the selected feature.
            </p>
          {/if}
        </aside>
        <button
          type="button"
          class="sidebar-resizer"
          aria-label="Resize sidebar"
          title="Drag to resize"
          onpointerdown={startSidebarResize}></button>
      {/if}
      {#if physicalMap && !studioOpen}
        <div class="stage">
          <div class="canvas" class:picking={picking || linkArming} role="img" aria-label="Physical world map">
            <PhysicalWorldView
              collection={draft}
              {layers}
              raster={background?.canvas ?? null}
              {pickArmed}
              onready={(next) => {
                physicalEditor = next;
              }}
              onMapPick={(anchor) => {
                if (picking && onpick) {
                  onpick(anchor);
                  return;
                }
                if (linkArming || linkPanelOpen) openLinkPanel(anchor);
              }} />
            <div class="epoch-control" aria-label="World epoch">
              <input
                id="physical-epoch"
                type="range"
                min={EPOCH_MIN}
                max={EPOCH_MAX}
                step={EPOCH_STEP}
                value={epochOffsetYears}
                aria-label="Epoch offset"
                disabled={busy}
                oninput={(event) => commitEpoch(clampEpoch(Number(event.currentTarget.value), EPOCH_STEP))} />
              <input
                class="epoch-year"
                type="text"
                inputmode="numeric"
                autocomplete="off"
                spellcheck="false"
                value={epochYearsAbs.toLocaleString("en-US")}
                aria-label="Years from epoch"
                disabled={busy}
                onchange={(event) => commitEpochFromExact(parseEpochYears(event.currentTarget.value), epochEra)} />
              <span>
                {#if epochOffsetYears === 0}
                  at epoch
                {:else if epochOffsetYears < 0}
                  years before epoch
                {:else}
                  years after epoch
                {/if}
              </span>
            </div>
            {#if busy || epochBusy}
              <div class="map-busy" role="status">
                <strong>{epochPhase || (busy ? "Loading…" : "Working…")}</strong>
                {#if epochProgress}<span>{epochProgress.completed} / {epochProgress.total}</span>{/if}
              </div>
            {/if}
          </div>
          {#if linkPanelOpen && mapId}
            <MapLocationLinkPanel
              {mapId}
              bind:anchor={linkAnchor}
              arming={linkArming}
              onclose={closeLinkPanel}
              onresnap={() => {
                linkArming = true;
              }}
              onlinked={() => {
                void refreshFeatureLinks();
              }} />
          {/if}
        </div>
      {:else if studioOpen && mapId}
        <div class="canvas" role="img" aria-label="Atlas Studio">
          <AtlasStudioView
            {mapId}
            viewerLayers={layers}
            bind:stage={studioStage}
            onready={(api) => (studioApi = api)}
            onexport={(request) => {
              studioExport = request;
              atlasOpen = true;
            }} />
        </div>
      {:else}
        <div class="stage">
          <!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
          <div
            class="canvas"
            class:picking={picking || linkArming}
            bind:this={host}
            tabindex="0"
            role="application"
            aria-label="Native vector map canvas">
            {#if editor}
              <MapViewControls
                zoom={defaultView.zoom}
                min={0}
                max={8}
                onzoom={(zoom) => {
                  defaultView = { ...defaultView, zoom };
                  editor?.setZoom(zoom);
                }}
                onpan={(longitude, latitude) => editor?.panBy(longitude, latitude)} />
            {/if}
            {#if busy}
              <div class="map-busy" role="status"><strong>Loading…</strong></div>
            {/if}
          </div>
          {#if linkPanelOpen && mapId}
            <MapLocationLinkPanel
              {mapId}
              bind:anchor={linkAnchor}
              arming={linkArming}
              onclose={closeLinkPanel}
              onresnap={() => {
                linkArming = true;
                editor?.setMode("static");
              }}
              onlinked={() => {
                void refreshFeatureLinks();
              }} />
          {/if}
        </div>
      {/if}
    </div>
  </section>
{/if}

<style>
.native-vector-editor {
  display: flex;
  width: 100%;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  background: #17211d;
  color: #edf2ec;
}
.header-actions,
.layer-row,
.style-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.text-button {
  padding: 6px 12px;
}
.studio-open {
  width: 100%;
  padding: 10px 12px;
  font-size: 13px;
}
.aside-toggle {
  display: flex;
  width: 100%;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  background: transparent;
  color: inherit;
  text-align: left;
}
.aside-chevron {
  display: grid;
  place-items: center;
  transform: rotate(0deg);
}
.aside-chevron.collapsed {
  transform: rotate(-90deg);
}
button {
  border: 0;
  border-radius: 7px;
  padding: 8px 10px;
  background: #31443b;
  color: #edf2ec;
  font: 700 12px system-ui;
  cursor: pointer;
}
button.active,
button.save {
  background: #d5ab6c;
  color: var(--brass-ink);
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.editor-body {
  display: grid;
  min-height: 0;
  flex: 1 1 auto;
  grid-template-columns: var(--sidebar-width, 260px) 6px minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr);
}
.editor-body.studio {
  grid-template-columns: minmax(0, 1fr);
}
.sidebar-resizer {
  width: 6px;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: #405047;
  cursor: col-resize;
}
.sidebar-resizer:hover,
.sidebar-resizer:focus-visible {
  background: #d5ab6c;
}
.epoch-control {
  position: absolute;
  z-index: 2;
  top: 10px;
  left: 10px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 8px;
  max-width: min(28rem, calc(100% - 20px));
  padding: 6px 8px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 8px;
  background: rgb(27 40 34 / 92%);
  color: #d8e3d9;
  font-size: 12px;
}
.epoch-control input[type="range"] {
  width: 140px;
  min-width: 0;
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
.epoch-control span {
  color: #d8e3d9;
  font-size: 12px;
}
.event-control {
  display: grid;
  gap: 8px;
  padding: 4px 0 8px;
  color: #d8e3d9;
  font-size: 12px;
}
.event-control label {
  display: grid;
  gap: 4px;
  color: var(--theme-neutral-text-muted, #aebdb1);
  font-size: 11px;
}
.event-control input,
.event-control select {
  min-width: 0;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  padding: 6px 7px;
  background: #0f1a16;
  color: #edf2ec;
  font: 12px system-ui;
}
.event-control small {
  color: var(--theme-neutral-text-muted, #aebdb1);
}
aside {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
  overflow: auto;
  border-right: 1px solid var(--theme-neutral-border-strong, #405047);
  background: #1b2822;
}
.hazard-legend {
  margin: 0;
  color: var(--theme-neutral-text-muted, #aebdb1);
  font-size: 11px;
  line-height: 1.4;
}
.layer-list {
  display: grid;
  gap: 6px;
}
.layer {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 2px 4px;
  padding: 1px 4px;
  border-radius: 6px;
  background: #18241f;
}
.layer.active {
  outline: 1px solid var(--theme-warning-border, #d5ab6c);
}
.layer-name {
  text-align: left;
  width: 100%;
  padding: 4px 6px;
  background: transparent;
  font-weight: 600;
}
.style-row {
  grid-column: 1 / -1;
}
.inspector {
  display: grid;
  gap: 6px;
  padding: 8px;
  border-radius: 8px;
  background: #18241f;
}
.inspector input,
.inspector select,
.layer-name input,
.style-row input[type="number"] {
  width: 100%;
  border: 0;
  border-radius: 6px;
  padding: 6px 8px;
  background: #0f1a16;
  color: #edf2ec;
}
.style-row label {
  display: grid;
  gap: 4px;
  font-size: 11px;
  color: #b8c8bc;
}
.canvas {
  position: relative;
  display: flex;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  background: #0d1b2a;
}
.stage {
  position: relative;
  display: flex;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
}
.canvas :global(.ol-viewport) {
  width: 100%;
  height: 100%;
}
.canvas.picking {
  outline: 2px solid var(--theme-warning-border, #d5ab6c);
  outline-offset: -2px;
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
.map-busy span {
  color: #d9d0c3;
  font-size: 0.8rem;
}
.icon-button {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--theme-neutral-border-strong, #4d6358);
  background: transparent;
}
.layer-row {
  flex-wrap: nowrap;
  gap: 2px;
}
.layer-row .icon-button {
  width: 22px;
  height: 22px;
}
.hint,
.error {
  color: #bac7bd;
  line-height: 1.45;
}
.error {
  margin: 0;
  padding: 8px 16px;
  color: #f5a49c;
}
button:focus-visible {
  outline: 2px solid var(--theme-warning-border, #f3d39a);
  outline-offset: 2px;
}
@media (prefers-reduced-motion: reduce) {
  .native-vector-editor,
  .native-vector-editor * {
    transition: none !important;
    animation: none !important;
  }
}
@media (max-width: 900px) {
  .event-control {
    grid-template-columns: 1fr;
  }
}
</style>
