<script lang="ts">
import { onMount, tick } from "svelte";
import { listen } from "@tauri-apps/api/event";
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
import NativeVectorGenerator from "./NativeVectorGenerator.svelte";
import {
  createNativeVectorEditor,
  liveNativeVectorEditorCount,
  RENDERER_UNAVAILABLE,
  type NativeVectorEditor,
} from "./runtime";
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
  type VectorDrawMode,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "./types";
import { physicalWorldOverlayCoordinates, type ImageOverlayCoordinates } from "./coordinates";
import { paintPhysicalSurface } from "../physical/raster";
import PhysicalWorldView from "../physical/PhysicalWorldView.svelte";
import AtlasRenderPanel from "../atlas/AtlasRenderPanel.svelte";
import AtlasStudioView from "../atlas/AtlasStudioView.svelte";
import MapViewControls from "./MapViewControls.svelte";

let {
  mapId,
  picking = false,
  start = "generate",
  focusLinkId: _focusLinkId,
  oncreated,
  oncancel,
  onpick: _onpick,
  onopen: _onopen,
  onstate,
}: {
  mapId?: string;
  picking?: boolean;
  start?: "generate" | "import";
  focusLinkId?: string;
  oncreated?: (map: Entity) => void;
  oncancel?: () => void;
  onpick?: (anchor: MapAnchor) => void;
  onopen?: (entityId: string) => void;
  onstate?: (status: string, detail: unknown) => void;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let editor: NativeVectorEditor | null = null;
let draft = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let loaded = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let layers = $state<VectorLayerDefinition[]>([]);
let layersField = $state<FieldValue | null>(null);
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
let layersCollapsed = $state(false);
let historyCollapsed = $state(true);
let epochEra = $state<"past" | "future">("past");
let epochYearsAbs = $state(0);
let physicalLayerVisibility = $state<Map<string, boolean>>(new Map());
let sidebarWidth = $state(260);

const SIDEBAR_MIN = 180;
const SIDEBAR_MAX = 520;

const EPOCH_MIN = -100_000;
const EPOCH_MAX = 100_000;
const EPOCH_STEP = 10;

const listedLayers = $derived(
  [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id)),
);

const icons = {
  back: '<path d="M19 12H5"/><path d="m12 19-7-7 7-7"/>',
  pan: '<path d="M12 2v20"/><path d="m15 19-3 3-3-3"/><path d="m19 9 3 3-3 3"/><path d="M2 12h20"/><path d="m5 9-3 3 3 3"/><path d="m9 5 3-3 3 3"/>',
  select: '<path d="M4 4 16 9.5 10.5 11 9.5 16Z"/>',
  point: '<circle cx="12" cy="12" r="3.5"/>',
  line: '<path d="M5 19 19 5"/>',
  polygon: '<path d="M12 3 20 8.5v7L12 21 4 15.5v-7Z"/>',
  freehand: '<path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/>',
  undo: '<path d="M3 7v6h6"/><path d="M3 13a9 9 0 1 0 3-7.3L3 13"/>',
  redo: '<path d="M21 7v6h-6"/><path d="M21 13a9 9 0 1 1-3-7.3L21 13"/>',
  addLayer: '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M8 12h8"/><path d="M12 8v8"/>',
  save: '<path d="M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M17 21v-7H7v7"/><path d="M7 3v4h8"/>',
  fullscreen:
    '<path d="M8 3H5a2 2 0 0 0-2 2v3"/><path d="M21 8V5a2 2 0 0 0-2-2h-3"/><path d="M3 16v3a2 2 0 0 0 2 2h3"/><path d="M16 21h3a2 2 0 0 0 2-2v-3"/>',
  exitFullscreen:
    '<path d="M8 3v3a2 2 0 0 1-2 2H3"/><path d="M21 8h-3a2 2 0 0 1-2-2V3"/><path d="M3 16h3a2 2 0 0 1 2 2v3"/><path d="M16 21v-3a2 2 0 0 1 2-2h3"/>',
  show: '<path d="M2.06 12a10.94 10.94 0 0 1 20 0"/><path d="M2.06 12a10.94 10.94 0 0 0 20 0"/><circle cx="12" cy="12" r="3"/>',
  hide: '<path d="M10.7 5.1A11 11 0 0 1 12 5c5 0 9.3 3.1 11 7-.5 1.2-1.2 2.3-2.1 3.2"/><path d="M17.9 17.9A11 11 0 0 1 12 19c-5 0-9.3-3.1-11-7 1-2.3 2.6-4.2 4.6-5.5"/><path d="m2 2 20 20"/><path d="M9.9 9.9a3 3 0 0 0 4.2 4.2"/>',
  lock: '<rect width="14" height="10" x="5" y="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
  unlock: '<rect width="14" height="10" x="5" y="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/>',
  rename: '<path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/>',
  up: '<path d="m18 15-6-6-6 6"/>',
  down: '<path d="m6 9 6 6 6-6"/>',
  remove:
    '<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M10 11v6"/><path d="M14 11v6"/>',
  exportAtlas: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/><path d="M12 15V3"/>',
  chevron: '<path d="m6 9 6 6 6-6"/>',
} as const;
const activeLayer = $derived(layers.find((layer) => layer.id === activeLayerId) ?? null);
const canDraw = $derived(
  Boolean(activeLayer) && !activeLayer?.locked && !picking && !immutablePhysicalLayerIds.has(activeLayer?.id ?? ""),
);
const dirty = $derived(editorState.dirty);
const diagnostic = $derived(editorState.diagnostic);
const diagnosticCode = $derived(editorState.diagnosticCode);
const conflict = $derived(editorState.conflict);

function publish(status: string, detail: unknown = null) {
  onstate?.(status, detail);
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
    features: collection.features.filter((feature) => !immutablePhysicalLayerIds.has(feature.properties.daenaLayerId)),
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
    (feature) => !immutablePhysicalLayerIds.has(feature.properties.daenaLayerId),
  );
  const authoredLoaded = loaded.features.filter(
    (feature) => !immutablePhysicalLayerIds.has(feature.properties.daenaLayerId),
  );
  const physical = parseVectorCollection(new TextEncoder().encode(products.geojson));
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
    publish("ready", { liveEditors: liveNativeVectorEditorCount(), workerUrl: "" });
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
    },
    get background() {
      return background;
    },
    onViewChange(next) {
      defaultView = { ...defaultView, zoom: next.zoom };
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
  publish("ready", { liveEditors: liveNativeVectorEditorCount(), workerUrl: created.workerUrl });
}

async function load() {
  if (!mapId) return;
  epochRequest += 1;
  epochBusy = false;
  epochPhase = "";
  epochProgress = null;
  busy = true;
  try {
    const fields = await project.listFields(mapId);
    const descriptorField = fields.find((field) => field.namespace === "maps" && field.key === "map");
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
    studioOpen = false;
    try {
      const capabilities = await project.atlasCapabilities(mapId);
      atlasSupported = capabilities.supported;
      studioSupported = capabilities.supportsStudio;
    } catch {
      atlasSupported = false;
      studioSupported = false;
    }
    if (descriptor?.defaultView?.center) defaultView = { ...defaultView, center: descriptor.defaultView.center };
    if (typeof descriptor?.defaultView?.zoom === "number")
      defaultView = { ...defaultView, zoom: descriptor.defaultView.zoom };
    const nextLayersField = fields.find((item) => item.namespace === "maps" && item.key === "layers") ?? null;
    if (!nextLayersField) throw new Error("maps:layers is missing");
    applyLayersField(nextLayersField);
    const assets = await project.listAssets(mapId);
    const sourceId = physicalMap ? descriptor?.authoredSourceAssetId : descriptor?.sourceAssetId;
    const source = assets.find((asset) => asset.id === sourceId);
    if (!source) throw new Error("The vector source asset is missing");
    sourceAsset = source;
    if (background?.url) URL.revokeObjectURL(background.url);
    background = null;
    const bytes = await project.readAssetBytes(source.id);
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
      const physical = parseVectorCollection(new TextEncoder().encode(historical.geojson));
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
      const url = URL.createObjectURL(new Blob([new Uint8Array(previewBytes)], { type: preview.mime_type }));
      background = await new Promise((resolve, reject) => {
        const image = new Image();
        image.onload = () => {
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
    }
    const ordered = [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id));
    activeLayerId = ordered.some((layer) => layer.id === activeLayerId) ? activeLayerId : (ordered[0]?.id ?? null);
    tool = "select";
    selectedFeature = null;
    applyEditorEvent({ type: "loaded" });
    recoveryPath = "";
    notice = "";
    await tick();
    mountEditor();
  } catch (cause) {
    applyEditorEvent({
      type: "save-failed",
      message: cause instanceof Error ? cause.message : String(cause),
    });
    publish("error", { message: editorState.diagnostic });
  } finally {
    busy = false;
    epochBusy = false;
    epochPhase = "";
    epochProgress = null;
  }
}

async function save() {
  if (!mapId || !sourceAsset || busy) return;
  if (!dirty) {
    applyEditorEvent({ type: "save-succeeded" });
    return;
  }
  busy = true;
  applyEditorEvent({ type: "save-started" });
  try {
    editor?.flush();
    const bytes = collectionBytes(persistedCollection(draft));
    const hash = await sha256Hex(bytes);
    const replaced = await project.replaceVectorSource(sourceAsset.id, bytes, hash, sourceAsset.revision);
    sourceAsset = replaced.source;
    loaded = cloneCollection(draft);
    recoveryPath = "";
    applyEditorEvent({ type: "save-succeeded" });
  } catch (cause) {
    const text = cause instanceof Error ? cause.message : String(cause);
    const parsed = parseVectorDiagnostic(text);
    if (parsed.code === "asset.revision-conflict") {
      applyEditorEvent({ type: "save-conflict", message: text });
    } else {
      applyEditorEvent({ type: "save-failed", message: text });
    }
  } finally {
    busy = false;
  }
}

async function exportDraft() {
  if (!mapId) return;
  try {
    recoveryPath = await project.mapsRecoveryExport(mapId, collectionBytes(persistedCollection(draft)));
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
  busy = true;
  try {
    const change = await project.createVectorLayer(mapId, `Layer ${layers.length + 1}`, layersField.revision, {
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

async function mutateLayer(layer: VectorLayerDefinition, update: Parameters<typeof project.updateMapLayer>[3]) {
  if (!mapId || !layersField) return;
  try {
    const change = await project.updateMapLayer(mapId, layer.id, layersField.revision, update);
    applyLayersField(change.layers);
    editor?.syncLayers(layers);
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
    await load();
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
  busy = true;
  try {
    const change = await project.deleteVectorLayer(
      mapId,
      layer.id,
      layersField.revision,
      sourceAsset.revision,
      savedCount,
    );
    applyLayersField(change.layers);
    sourceAsset = change.source;
    draft = {
      type: "FeatureCollection",
      features: draft.features.filter((feature) => feature.properties.daenaLayerId !== layer.id),
    };
    loaded = {
      type: "FeatureCollection",
      features: loaded.features.filter((feature) => feature.properties.daenaLayerId !== layer.id),
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
  } finally {
    busy = false;
  }
}

function onKey(event: KeyboardEvent) {
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
    unlistenHistoricalProgress?.();
    destroyEditor();
    epochRequest += 1;
    if (epochTimer) clearTimeout(epochTimer);
    if (background?.url) URL.revokeObjectURL(background.url);
    registerNativeVectorSession(null);
  };
});
</script>

{#snippet glyph(markup: string)}
  <svg
    aria-hidden="true"
    width="15"
    height="15"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round">{@html markup}</svg>
{/snippet}

{#if !mapId}
  <NativeVectorGenerator
    {oncreated}
    {oncancel}
    autostartImport={start === "import"}
    onfullscreen={(enabled) => void setFullscreen(enabled)}
    {fullscreen} />
{:else}
  <section class="native-vector-editor" aria-label="Native vector map editor">
    <header>
      <div>
        <span>{physicalMap ? "PHYSICAL WORLD" : "VECTOR MAP"}</span>
        {#if !physicalMap && dirty}<strong>Unsaved changes</strong>{/if}
      </div>
      <div
        class="header-actions"
        role="toolbar"
        aria-label={physicalMap ? "Physical map actions" : "Vector drawing tools"}>
        {#if !physicalMap}
          <button
            type="button"
            class="icon-button"
            class:active={tool === "static"}
            aria-pressed={tool === "static"}
            aria-label="Pan"
            title="Pan"
            onclick={() => setTool("static")}>{@render glyph(icons.pan)}</button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "select"}
            aria-pressed={tool === "select"}
            aria-label="Select"
            title="Select"
            onclick={() => setTool("select")}>{@render glyph(icons.select)}</button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "point"}
            aria-pressed={tool === "point"}
            aria-label="Point"
            title="Point"
            disabled={!canDraw}
            onclick={() => setTool("point")}>{@render glyph(icons.point)}</button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "linestring"}
            aria-pressed={tool === "linestring"}
            aria-label="Line"
            title="Line"
            disabled={!canDraw}
            onclick={() => setTool("linestring")}>{@render glyph(icons.line)}</button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "polygon"}
            aria-pressed={tool === "polygon"}
            aria-label="Polygon"
            title="Polygon"
            disabled={!canDraw}
            onclick={() => setTool("polygon")}>{@render glyph(icons.polygon)}</button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "freehand"}
            aria-pressed={tool === "freehand"}
            aria-label="Freehand"
            title="Freehand"
            disabled={!canDraw}
            onclick={() => setTool("freehand")}>{@render glyph(icons.freehand)}</button>
          <button type="button" class="icon-button" aria-label="Undo" title="Undo" onclick={() => editor?.undo()}
            >{@render glyph(icons.undo)}</button>
          <button type="button" class="icon-button" aria-label="Redo" title="Redo" onclick={() => editor?.redo()}
            >{@render glyph(icons.redo)}</button>
          <button
            type="button"
            class="icon-button"
            aria-label="Add layer"
            title="Add layer"
            disabled={busy || layers.length >= VECTOR_MAX_LAYERS}
            onclick={() => void addLayer()}>{@render glyph(icons.addLayer)}</button>
          <button
            type="button"
            class="icon-button save"
            aria-label={busy ? "Saving…" : dirty ? "Save" : "Saved"}
            title={busy ? "Saving…" : dirty ? "Save" : "Saved"}
            disabled={busy || !dirty}
            onclick={() => void save()}>{@render glyph(icons.save)}</button>
        {/if}
        {#if atlasSupported}
          <button
            type="button"
            class="icon-button"
            class:active={atlasOpen}
            aria-pressed={atlasOpen}
            aria-label="Export atlas"
            title="Export atlas"
            onclick={() => (atlasOpen = !atlasOpen)}>{@render glyph(icons.exportAtlas)}</button>
        {/if}
        <button
          type="button"
          class="icon-button"
          class:active={fullscreen}
          aria-label={fullscreen ? "Exit full screen" : "Full screen"}
          aria-pressed={fullscreen}
          title={fullscreen ? "Exit full screen (Esc)" : "Full screen"}
          onclick={toggleFullscreen}>{@render glyph(fullscreen ? icons.exitFullscreen : icons.fullscreen)}</button>
        <button type="button" class="text-button" aria-label="Close" title="Close" onclick={() => void requestBack()}
          >Close</button>
      </div>
    </header>
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
      <AtlasRenderPanel {mapId} {epochOffsetYears} seed={studioExport} onclose={() => (atlasOpen = false)} />
    {/if}
    <div class="editor-body" style={`--sidebar-width: ${sidebarWidth}px`}>
      <aside aria-label="Map layers">
        {#if studioSupported}
          <button
            type="button"
            class="studio-open"
            class:active={studioOpen}
            aria-pressed={studioOpen}
            onclick={() => (studioOpen = !studioOpen)}>{studioOpen ? "Close Atlas Studio" : "Atlas Studio"}</button>
        {/if}
        <button
          type="button"
          class="aside-toggle"
          aria-expanded={!layersCollapsed}
          onclick={() => (layersCollapsed = !layersCollapsed)}>
          <strong id="vector-layers-heading">Vector layers</strong>
          <span class="aside-chevron" class:collapsed={layersCollapsed}>{@render glyph(icons.chevron)}</span>
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
                    >{@render glyph(layer.defaultVisible ? icons.show : icons.hide)}</button>
                  {#if !immutablePhysicalLayerIds.has(layer.id)}
                    <button
                      type="button"
                      class="icon-button"
                      aria-pressed={layer.locked}
                      aria-label={layer.locked ? `Unlock ${layer.name}` : `Lock ${layer.name}`}
                      title={layer.locked ? `Unlock ${layer.name}` : `Lock ${layer.name}`}
                      onclick={() => void toggleLock(layer)}
                      >{@render glyph(layer.locked ? icons.lock : icons.unlock)}</button>
                    <button
                      type="button"
                      class="icon-button"
                      aria-label={`Rename ${layer.name}`}
                      title="Rename"
                      onclick={() => (renamingId = layer.id)}>{@render glyph(icons.rename)}</button>
                    <button
                      type="button"
                      class="icon-button"
                      aria-label={`Move ${layer.name} up`}
                      title="Up"
                      onclick={() => void moveLayer(layer, -1)}>{@render glyph(icons.up)}</button>
                    <button
                      type="button"
                      class="icon-button"
                      aria-label={`Move ${layer.name} down`}
                      title="Down"
                      onclick={() => void moveLayer(layer, 1)}>{@render glyph(icons.down)}</button>
                    <button
                      type="button"
                      class="icon-button"
                      aria-label={`Delete ${layer.name}`}
                      title="Delete"
                      onclick={() => void removeLayer(layer)}>{@render glyph(icons.remove)}</button>
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
                      Fill opacity
                      <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.05"
                        value={layer.style.fillOpacity}
                        aria-label={`${layer.name} fill opacity`}
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
            <span class="aside-chevron" class:collapsed={historyCollapsed}>{@render glyph(icons.chevron)}</span>
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
                >Creates revisioned entities and map links; generated hazards remain read-only and are not predictions.</small>
              {#if eventNotice}<small role="status">{eventNotice}</small>{/if}
            </div>
          {/if}
        {/if}
        {#if selectedFeature && !physicalMap}
          <div class="inspector" aria-label="Selected feature">
            <strong>Selected feature</strong>
            <p class="hint">
              {selectedFeature.properties.kind} · {selectedFeature.properties.daenaLayerId === "base"
                ? "base geography"
                : "authored"}
            </p>
            <label>
              Name
              <input
                value={selectedFeature.properties.name ?? ""}
                maxlength="256"
                aria-label="Feature name"
                disabled={selectedFeature.properties.daenaLayerId === "base" || activeLayer?.locked}
                onchange={(event) => {
                  const next = event.currentTarget.value.trim() || null;
                  editor?.updateSelectedName(next);
                }} />
            </label>
          </div>
        {/if}
        {#if !physicalMap}
          <p class="hint">
            Base geography is read-only. Point, line, polygon, and freehand edits save through the canonical GeoJSON
            source. Delete removes the selected feature.
          </p>
        {/if}
      </aside>
      <button
        type="button"
        class="sidebar-resizer"
        aria-label="Resize sidebar"
        title="Drag to resize"
        onpointerdown={startSidebarResize}></button>
      {#if physicalMap && !studioOpen}
        <div class="canvas" role="img" aria-label="Physical world map">
          <PhysicalWorldView collection={draft} {layers} raster={background?.canvas ?? null} />
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
      {:else if studioOpen && mapId}
        <div class="canvas" role="img" aria-label="Atlas Studio">
          <AtlasStudioView
            {mapId}
            onexport={(request) => {
              studioExport = request;
              atlasOpen = true;
            }} />
        </div>
      {:else}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
        <div
          class="canvas"
          class:picking
          bind:this={host}
          tabindex="0"
          role="application"
          aria-label="Native vector map canvas">
          {#if editor}
            <MapViewControls
              zoom={defaultView.zoom}
              onzoom={(zoom) => {
                defaultView = { ...defaultView, zoom };
                editor?.setZoom(zoom);
              }}
              onreset={() => editor?.resetView()} />
          {/if}
          {#if busy}
            <div class="map-busy" role="status"><strong>Loading…</strong></div>
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
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid #405047;
  background: #202c27;
}
header div:first-child {
  display: grid;
  gap: 2px;
}
header span {
  font-size: 10px;
  letter-spacing: 0.12em;
  color: #b8c8bc;
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
  color: #243126;
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
  border: 1px solid #405047;
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
  border: 1px solid #405047;
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
  color: #aebdb1;
  font-size: 11px;
}
.event-control input,
.event-control select {
  min-width: 0;
  border: 1px solid #405047;
  border-radius: 6px;
  padding: 6px 7px;
  background: #0f1a16;
  color: #edf2ec;
  font: 12px system-ui;
}
.event-control small {
  color: #aebdb1;
}
aside {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
  overflow: auto;
  border-right: 1px solid #405047;
  background: #202c27;
}
.hazard-legend {
  margin: 0;
  color: #aebdb1;
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
  outline: 1px solid #d5ab6c;
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
.canvas :global(.maplibregl-map) {
  width: 100%;
  height: 100%;
}
.canvas.picking {
  outline: 2px solid #d5ab6c;
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
  border: 1px solid #4d6358;
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
  outline: 2px solid #f3d39a;
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
