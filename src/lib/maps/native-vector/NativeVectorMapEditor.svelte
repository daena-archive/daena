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
  Image as ImageIcon,
  Link2,
  Lock,
  LockOpen,
  Magnet,
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
  Ruler,
  Save,
  Scissors,
  Slash,
  Square,
  SquarePlus,
  SquareStack,
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
import {
  IMAGE_MAX_RASTER_LAYERS,
  VECTOR_MAX_LAYERS,
  type MapAnchor,
  type MapBackgroundRef,
  type MapCoordinateSpace,
} from "../../../../packages/plugin-sdk/src/maps";
import NativeVectorImporter from "./NativeVectorImporter.svelte";
import MapLocationLinkPanel from "./MapLocationLinkPanel.svelte";
import {
  createMapAdapter,
  liveMapAdapterCount,
  RENDERER_UNAVAILABLE,
  type MapAdapter,
} from "../openlayers/MapAdapter";
import type { RuntimeBackground } from "../openlayers/background-registry";
import type { RasterLayerSource } from "../openlayers/layer-registry";
import { maxZoomForCoordinateSpace } from "../openlayers/projection";
import { registerNativeVectorSession } from "./session";
import {
  collectionBytes,
  featureCountForLayer,
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
  VECTOR_PROVIDER,
  featureLayerId,
  featureName,
  featureSemanticType,
  isRasterLayer,
  isVectorLayer,
  type MapLayerDefinition,
  type VectorDrawMode,
  type VectorFeature,
  type VectorFeatureCollection,
  type VectorLayerDefinition,
} from "./types";
import { paintPhysicalSurface } from "../physical/raster";
import PhysicalWorldView from "../physical/PhysicalWorldView.svelte";
import AtlasRenderPanel from "../atlas/AtlasRenderPanel.svelte";
import AtlasStudioView from "../atlas/AtlasStudioView.svelte";
import MapViewControls from "./MapViewControls.svelte";
import {
  CommandStack,
  PHYSICAL_COORDINATE_SPACE,
  addBackgroundCommand,
  authoredToNormalized,
  backgroundsFromDescriptor,
  buildCreateLayer,
  buildCreateRasterLayer,
  buildDuplicateLayer,
  buildRecoveryPackage,
  calibrateImageToWorld,
  calibrateWorldUnits,
  captureDeleteFeatures,
  captureReplaceCollection,
  coordinateSpaceFromDescriptor,
  coordinateSpaceKey,
  createMapDocument,
  defaultViewFromDescriptor,
  deleteLayerCommand,
  duplicateFeaturesCommand,
  duplicateOffset,
  encodeLayersField,
  extentOf,
  listedBackgrounds,
  measurementSummary,
  moveFeaturesToLayerCommand,
  nextBackgroundOrder,
  normalizedToAuthored,
  recoveryPackageBytes,
  removeBackgroundCommand,
  renameLayerCommand,
  reorderBackgroundCommand,
  reorderLayerCommand,
  reorderLayersByIdsCommand,
  replaceBackgroundCommand,
  setBackgroundOpacityCommand,
  setBackgroundVisibilityCommand,
  setDefaultViewCommand,
  setFeatureMetadataCommand,
  setLayerLockedCommand,
  setLayerOpacityCommand,
  setLayerStyleCommand,
  setLayerVisibilityCommand,
  applyGeometryOperationCommand,
  setSnapSettingsCommand,
  snapEnabledFromDescriptor,
  buildPreview,
  commitSelectionIds,
  canRunOperation,
  formatMeasurement,
  measureFeature,
  unitsForCoordinateSpace,
  type GeometryPreview,
  type GeometryOperationKind,
  type MapCommand,
} from "../editor";

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
let editor = $state.raw<MapAdapter | null>(null);
let commandStack = $state.raw<CommandStack | null>(null);
let canUndo = $state(false);
let canRedo = $state(false);
let draft = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let loaded = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
let layers = $state<MapLayerDefinition[]>([]);
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
let selectedFeatureIds = $state<string[]>([]);
let draggingLayerId = $state<string | null>(null);
let defaultView = $state({ center: [0, 0] as [number, number], zoom: 1, rotation: 0 });
let coordinateSpace = $state<MapCoordinateSpace>(PHYSICAL_COORDINATE_SPACE);
let rasterAssets = $state(new Map<string, { url: string; width: number; height: number; canvas: HTMLCanvasElement }>());
let rastersCollapsed = $state(false);
let calibrateMetres = $state("");
let mountedSpaceKey = $state("");
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
let geometryPreview = $state<GeometryPreview | null>(null);
let measureReadout = $state("");
let snapVertex = $state(true);
let snapEdge = $state(true);
let snapIntersection = $state(true);
let snapConfigOpen = $state(false);
let snapTargetLayerIds = $state(new Set<string>());
let bufferDistance = $state("10");
let simplifyTolerance = $state("0.5");
let operationNotice = $state("");
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
let linkPanelOpen = $state(false);
let linkArming = $state(false);
let linkAnchor = $state<MapAnchor | null>(null);
let pinsReady = $state(false);
let physicalEditor = $state<MapAdapter | null>(null);

const SIDEBAR_MIN = 180;
const SIDEBAR_MAX = 520;

const EPOCH_MIN = -100_000;
const EPOCH_MAX = 100_000;
const EPOCH_STEP = 10;

const listedLayers = $derived(
  [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id)),
);
const listedRasters = $derived(
  commandStack ? listedBackgrounds(commandStack.document) : backgroundsFromDescriptor(mapField?.value),
);
const unitsLabel = $derived(measurementSummary(coordinateSpace));
const snapEnabled = $derived(commandStack ? snapEnabledFromDescriptor(commandStack.document.descriptor) : true);
const selectedOpFeatures = $derived(
  commandStack?.document.collection.features.filter((feature) => selectedFeatureIds.includes(feature.id)) ?? [],
);
const viewMaxZoom = $derived(maxZoomForCoordinateSpace(coordinateSpace));
const brandIcon = $derived((physicalMap ? Mountain : MapIcon) as Component);
const iconProps = { size: 15, strokeWidth: 1.8, "aria-hidden": true } as const;

const activeLayer = $derived(layers.find((layer) => layer.id === activeLayerId) ?? null);
const canDraw = $derived(
  Boolean(activeLayer && isVectorLayer(activeLayer) && !activeLayer.locked && activeLayer.defaultVisible) &&
    !picking &&
    !linkArming &&
    !immutablePhysicalLayerIds.has(activeLayer?.id ?? ""),
);
const rasterLayerCount = $derived(layers.filter(isRasterLayer).length);
const pickArmed = $derived(Boolean(picking || linkArming));
const dirty = $derived(editorState.dirty);
const diagnostic = $derived(editorState.diagnostic);
const diagnosticCode = $derived(editorState.diagnosticCode);
const conflict = $derived(editorState.conflict);

function runtimeBackgrounds(): RuntimeBackground[] {
  if (physicalMap) {
    const physical = rasterAssets.get("physical");
    if (!physical) return [];
    return [
      {
        id: "physical",
        url: physical.url,
        canvas: physical.canvas,
        width: physical.width,
        height: physical.height,
        extent: extentOf(PHYSICAL_COORDINATE_SPACE),
        visible: true,
        locked: true,
        opacity: 1,
        order: 0,
      },
    ];
  }
  return listedRasters.flatMap((ref) => {
    const loaded = rasterAssets.get(ref.assetId);
    if (!loaded) return [];
    return [
      {
        id: ref.id,
        url: loaded.url,
        canvas: loaded.canvas,
        width: loaded.width,
        height: loaded.height,
        extent: [ref.extent[0], ref.extent[1], ref.extent[2], ref.extent[3]] as [number, number, number, number],
        visible: ref.visible,
        locked: ref.locked,
        opacity: ref.opacity,
        order: ref.order,
      },
    ];
  });
}

function runtimeLayerRasters(): Map<string, RasterLayerSource> {
  const next = new Map<string, RasterLayerSource>();
  for (const layer of layers) {
    if (!isRasterLayer(layer)) continue;
    const loaded = rasterAssets.get(layer.rasterAssetId);
    if (!loaded) continue;
    next.set(layer.rasterAssetId, { url: loaded.url, canvas: loaded.canvas });
  }
  return next;
}

function applyCoordinateSpaceFromDescriptor(descriptor: unknown, options?: { restoreView?: boolean }) {
  coordinateSpace = physicalMap ? PHYSICAL_COORDINATE_SPACE : coordinateSpaceFromDescriptor(descriptor);
  if (options?.restoreView !== false) {
    defaultView = defaultViewFromDescriptor(descriptor, coordinateSpace);
  }
}

async function decodeRasterBytes(
  bytes: number[] | Uint8Array,
  mimeType: string,
  generation: number,
): Promise<{ url: string; width: number; height: number; canvas: HTMLCanvasElement } | null> {
  const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: mimeType }));
  objectUrls.push(url);
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => {
      if (generation !== loadGeneration) {
        resolve(null);
        return;
      }
      const canvas = document.createElement("canvas");
      canvas.width = image.naturalWidth;
      canvas.height = image.naturalHeight;
      const context = canvas.getContext("2d");
      if (!context) {
        reject(new Error("Could not decode the imported map image"));
        return;
      }
      context.drawImage(image, 0, 0);
      resolve({ url, width: image.naturalWidth, height: image.naturalHeight, canvas });
    };
    image.onerror = () => reject(new Error("Could not decode the imported map image"));
    image.src = url;
  });
}

function clearRasterAssets() {
  for (const url of objectUrls) URL.revokeObjectURL(url);
  objectUrls.length = 0;
  rasterAssets = new Map();
}

function featureFallbackPoint(feature: VectorFeature | null): [number, number] {
  if (!feature) return authoredToNormalized(defaultView.center[0], defaultView.center[1], coordinateSpace);
  const positions = feature.geometry.coordinates.flat(Infinity) as number[];
  if (positions.length < 2) return authoredToNormalized(defaultView.center[0], defaultView.center[1], coordinateSpace);
  return authoredToNormalized(positions[0], positions[1], coordinateSpace);
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
    target.focusPoint(normalizedToAuthored(anchor.point[0], anchor.point[1], coordinateSpace));
    return true;
  }
  return false;
}

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

function syncUiFromStack(restoreView = false) {
  if (!commandStack) return;
  const snap = commandStack.snapshot();
  draft = snap.document.collection;
  layers = withPhysicalVisibility(snap.document.layers);
  if (mapField) {
    mapField = { ...mapField, value: snap.document.descriptor as FieldValue["value"] };
  }
  applyCoordinateSpaceFromDescriptor(snap.document.descriptor, { restoreView });
  canUndo = snap.canUndo;
  canRedo = snap.canRedo;
  const nextKey = coordinateSpaceKey(coordinateSpace);
  if (editor && nextKey !== mountedSpaceKey) {
    mountEditor();
  } else {
    editor?.syncDocument(draft, layers, runtimeLayerRasters());
    editor?.syncBackgrounds(runtimeBackgrounds());
    if (restoreView) editor?.applyView(defaultView.center, defaultView.zoom, defaultView.rotation);
  }
  if (snap.dirty) applyEditorEvent({ type: "document-changed" });
  else applyEditorEvent({ type: "loaded" });
}

function dispatchCommand(command: MapCommand) {
  if (!commandStack) return;
  commandStack.apply(command);
  syncUiFromStack(command.kind === "SetCoordinateSpace");
}

function resetCommandStack(documentInput: {
  descriptor: unknown;
  layers: MapLayerDefinition[];
  collection: VectorFeatureCollection;
}) {
  const document = createMapDocument(documentInput);
  commandStack = new CommandStack(document);
  draft = document.collection;
  layers = withPhysicalVisibility(document.layers);
  loaded = cloneCollection(document.collection);
  canUndo = false;
  canRedo = false;
  applyEditorEvent({ type: "loaded" });
}

function undoEdit() {
  if (!commandStack?.canUndo()) return;
  commandStack.undo();
  syncUiFromStack(true);
}

function redoEdit() {
  if (!commandStack?.canRedo()) return;
  commandStack.redo();
  syncUiFromStack(true);
}

function deleteSelectedFeatures() {
  if (!commandStack) return;
  const ids = editor?.selectedFeatureIds() ?? (selectedFeature ? [selectedFeature.id] : []);
  const command = captureDeleteFeatures(commandStack.document, ids);
  if (!command) return;
  dispatchCommand(command);
  selectedFeature = null;
  selectedFeatureIds = [];
  editor?.clearSelection();
}

function offsetGeometry(
  geometry: VectorFeature["geometry"],
  dx: number,
  dy: number,
): VectorFeature["geometry"] {
  const shift = (coords: number[]): number[] => [coords[0] + dx, coords[1] + dy, ...coords.slice(2)];
  const walk = (value: unknown, depth: number): unknown => {
    if (depth === 0) return shift(value as number[]);
    return (value as unknown[]).map((item) => walk(item, depth - 1));
  };
  switch (geometry.type) {
    case "Point":
      return { type: "Point", coordinates: shift(geometry.coordinates) };
    case "MultiPoint":
      return { type: "MultiPoint", coordinates: walk(geometry.coordinates, 1) as number[][] };
    case "LineString":
      return { type: "LineString", coordinates: walk(geometry.coordinates, 1) as number[][] };
    case "MultiLineString":
      return { type: "MultiLineString", coordinates: walk(geometry.coordinates, 2) as number[][][] };
    case "Polygon":
      return { type: "Polygon", coordinates: walk(geometry.coordinates, 2) as number[][][] };
    case "MultiPolygon":
      return { type: "MultiPolygon", coordinates: walk(geometry.coordinates, 3) as number[][][][] };
  }
}

function duplicateSelectedFeatures() {
  if (!commandStack || !editor) return;
  const ids = editor.selectedFeatureIds();
  const selected = commandStack.document.collection.features.filter((feature) => ids.includes(feature.id));
  if (selected.length === 0) return;
  const offset = duplicateOffset(coordinateSpace);
  const copies = selected.map((feature) => {
    const clone = cloneCollection({ type: "FeatureCollection", features: [feature] }).features[0];
    clone.id = crypto.randomUUID();
    clone.geometry = offsetGeometry(clone.geometry, offset, -offset);
    return clone;
  });
  dispatchCommand(duplicateFeaturesCommand(copies));
}

function renameSelectedFeature(name: string | null) {
  if (!commandStack || !selectedFeature) return;
  const previous = featureName(selectedFeature);
  if (previous === name) return;
  dispatchCommand(setFeatureMetadataCommand(selectedFeature.id, name, previous));
  selectedFeature = {
    ...selectedFeature,
    properties: { daena: { ...selectedFeature.properties.daena, name } },
  };
}

function moveSelectedToLayer(layerId: string) {
  if (!commandStack || !editor) return;
  const ids = editor.selectedFeatureIds();
  if (ids.length === 0) return;
  const previousLayerIds: Record<string, string> = {};
  for (const feature of commandStack.document.collection.features) {
    if (ids.includes(feature.id)) previousLayerIds[feature.id] = featureLayerId(feature);
  }
  dispatchCommand(moveFeaturesToLayerCommand(ids, layerId, previousLayerIds));
  activeLayerId = layerId;
  editor.switchLayer(layerId);
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
  const parsed = withPhysicalVisibility(parseVectorLayers(field.value));
  layers = parsed;
  if (commandStack) {
    commandStack.replaceDocument({
      ...commandStack.document,
      layers: parsed,
      descriptor: mapField?.value ?? commandStack.document.descriptor,
    });
  }
}

function withPhysicalVisibility(next: MapLayerDefinition[]) {
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
  const physical = parseDerivedCollection(products.geojson);
  const combined = {
    type: "FeatureCollection" as const,
    features: [...physical.features, ...authoredDraft],
  };
  resetCommandStack({
    descriptor: mapField?.value ?? commandStack?.document.descriptor ?? {},
    layers,
    collection: combined,
  });
  const canvas = physicalHillshadeCanvas(products.hydrology);
  rasterAssets = new Map([
    ["physical", { url: "", width: products.hydrology.width, height: products.hydrology.height, canvas }],
  ]);
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
    publish("ready", { liveEditors: liveMapAdapterCount(), renderer: "openlayers" });
    return;
  }
  if (!host) return;
  destroyEditor();
  mountedSpaceKey = coordinateSpaceKey(coordinateSpace);
  const created = createMapAdapter(host, {
    get draft() {
      return draft;
    },
    get layers() {
      return layers;
    },
    get rasters() {
      return runtimeLayerRasters();
    },
    get activeLayerId() {
      return activeLayerId;
    },
    coordinateSpace,
    setActiveLayerId(id) {
      activeLayerId = id;
    },
    onCommand(payload) {
      if (!commandStack) return;
      if (payload.type === "replace-collection") {
        const command = captureReplaceCollection(
          commandStack.document,
          payload.collection,
          payload.label,
          payload.coalesceKey,
        );
        if (command) dispatchCommand(command);
        return;
      }
      if (payload.type === "set-view") {
        const previous = defaultViewFromDescriptor(commandStack.document.descriptor, coordinateSpace);
        if (
          previous.center[0] === payload.center[0] &&
          previous.center[1] === payload.center[1] &&
          previous.zoom === payload.zoom &&
          previous.rotation === payload.rotation
        ) {
          return;
        }
        defaultView = payload;
        dispatchCommand(setDefaultViewCommand(payload, previous));
      }
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
    onSelectionChange(ids) {
      selectedFeatureIds = ids;
      if (ids.length === 0) selectedFeature = null;
      if (tool === "measure-length" || tool === "measure-area") updateMeasureFromSelection();
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
    get backgrounds() {
      return runtimeBackgrounds();
    },
    initialView: defaultView,
    onViewChange(next) {
      defaultView = next;
    },
    onMeasureReadout(readout) {
      measureReadout = readout?.label ?? "";
    },
  });
  if ("error" in created) {
    applyEditorEvent({ type: "save-failed", message: `${created.error}: ${created.detail}` });
    publish("error", created);
    return;
  }
  editor = created;
  syncSnapToEditor();
  if (!canDraw) editor.setMode("static");
  else editor.setMode(tool);
  requestAnimationFrame(() => editor?.resize());
  publish("ready", { liveEditors: liveMapAdapterCount(), renderer: "openlayers" });
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
    applyCoordinateSpaceFromDescriptor(descriptorField?.value);
    const nextLayersField = fields.find((item) => item.namespace === "maps" && item.key === "layers") ?? null;
    if (!nextLayersField) throw new Error("maps:layers is missing");
    applyLayersField(nextLayersField);
    const assets = await project.listAssets(mapId);
    if (generation !== loadGeneration) return;
    const sourceId = physicalMap ? descriptor?.authoredSourceAssetId : descriptor?.sourceAssetId;
    const source = assets.find((asset) => asset.id === sourceId);
    if (!source) throw new Error("The vector source asset is missing");
    sourceAsset = source;
    clearRasterAssets();
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
      resetCommandStack({
        descriptor: mapField?.value ?? {},
        layers,
        collection: combined,
      });
      epochNotice = `Showing ${formatEpoch(historical.epochOffsetYears)} · deterministic derived playback`;
      const canvas = physicalHillshadeCanvas(historical.hydrology);
      rasterAssets = new Map([
        [
          "physical",
          { url: "", width: historical.hydrology.width, height: historical.hydrology.height, canvas },
        ],
      ]);
      epochBusy = false;
      epochPhase = "";
      epochProgress = null;
    } else {
      immutablePhysicalLayerIds = new Set();
      physicalLayerVisibility = new Map();
      epochNotice = "";
      resetCommandStack({
        descriptor: mapField?.value ?? {},
        layers,
        collection,
      });
    }
    applyCoordinateSpaceFromDescriptor(mapField?.value);
    const backgroundRefs = backgroundsFromDescriptor(mapField?.value);
    const nextRasters = new Map(rasterAssets);
    for (const ref of backgroundRefs) {
      const asset = assets.find((item) => item.id === ref.assetId);
      if (!asset || nextRasters.has(asset.id)) continue;
      const previewBytes = await project.readAssetBytes(asset.id);
      if (generation !== loadGeneration) return;
      const decoded = await decodeRasterBytes(previewBytes, asset.mime_type, generation);
      if (generation !== loadGeneration) return;
      if (decoded) nextRasters.set(asset.id, decoded);
    }
    for (const layer of layers) {
      if (!isRasterLayer(layer) || nextRasters.has(layer.rasterAssetId)) continue;
      const asset = assets.find((item) => item.id === layer.rasterAssetId);
      if (!asset) continue;
      const previewBytes = await project.readAssetBytes(asset.id);
      if (generation !== loadGeneration) return;
      const decoded = await decodeRasterBytes(previewBytes, asset.mime_type, generation);
      if (generation !== loadGeneration) return;
      if (decoded) nextRasters.set(asset.id, decoded);
    }
    rasterAssets = nextRasters;
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
  if (!mapId || !sourceAsset || !mapField || !layersField || !commandStack || busy) return;
  if (!dirty) {
    applyEditorEvent({ type: "save-succeeded" });
    return;
  }
  const generation = ++saveGeneration;
  busy = true;
  applyEditorEvent({ type: "save-started" });
  try {
    editor?.flush();
    const document = commandStack.document;
    const snapshot = cloneCollection(persistedCollection(document.collection));
    const bytes = collectionBytes(snapshot);
    const hash = await sha256Hex(bytes);
    const layersValue = encodeLayersField({
      ...document,
      collection: snapshot,
      layers: document.layers,
    });
    const applied = await project.applyMapEdit({
      mapEntityId: mapId,
      descriptor: document.descriptor,
      layers: layersValue,
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
    commandStack.setBaseline(
      createMapDocument({
        descriptor: applied.map.value,
        layers: parseVectorLayers(applied.layers.value),
        collection: snapshot,
      }),
    );
    draft = snapshot;
    loaded = cloneCollection(snapshot);
    layers = withPhysicalVisibility(parseVectorLayers(applied.layers.value));
    canUndo = false;
    canRedo = false;
    recoveryPath = "";
    applyEditorEvent({ type: "save-succeeded" });
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
  if (!mapId || !commandStack) return;
  try {
    const packageBytes = recoveryPackageBytes(buildRecoveryPackage(mapId, commandStack.document));
    recoveryPath = await project.mapsRecoveryExport(mapId, packageBytes);
    notice = `Draft exported to ${recoveryPath}`;
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
  }
}

function isDirty() {
  return editorState.dirty;
}

function syncSnapToEditor() {
  editor?.setSnapOptions({
    enabled: snapEnabled,
    vertex: snapVertex,
    edge: snapEdge,
    intersection: snapIntersection,
  });
  editor?.setSnapTargetLayerIds(snapTargetLayerIds);
}

function toggleSnapEnabled() {
  if (!commandStack) return;
  dispatchCommand(setSnapSettingsCommand(!snapEnabled, snapEnabled));
  syncSnapToEditor();
}

function toggleSnapTargetLayer(layerId: string) {
  const next = new Set(snapTargetLayerIds);
  if (next.has(layerId)) next.delete(layerId);
  else next.add(layerId);
  snapTargetLayerIds = next;
  syncSnapToEditor();
}

function cancelGeometryPreview() {
  geometryPreview = null;
  operationNotice = "";
  editor?.setGeometryPreview(null);
}

function startGeometryOperation(operation: GeometryOperationKind) {
  if (!commandStack) return;
  operationNotice = "";
  const params =
    operation === "buffer"
      ? { bufferDistance: Number(bufferDistance) }
      : operation === "simplify"
        ? { simplifyTolerance: Number(simplifyTolerance) }
        : {};
  const built = buildPreview(commandStack.document, operation, selectedFeatureIds, params);
  if (built.error) {
    operationNotice = built.error.detail;
    return;
  }
  if (!built.preview) return;
  geometryPreview = built.preview;
  editor?.setGeometryPreview(built.preview.previewFeatures);
}

function commitGeometryPreview() {
  if (!geometryPreview || !commandStack) return;
  const removed = commandStack.document.collection.features.filter((feature) =>
    geometryPreview!.removedFeatureIds.includes(feature.id),
  );
  dispatchCommand(
    applyGeometryOperationCommand(removed, geometryPreview.previewFeatures, geometryPreview.label),
  );
  const ids = commitSelectionIds(geometryPreview);
  cancelGeometryPreview();
  queueMicrotask(() => {
    if (!editor || !commandStack) return;
    editor.syncDocument(commandStack.document.collection, layers, runtimeLayerRasters());
    editor.selectFeatureIds(ids);
  });
}

function updateMeasureFromSelection() {
  if (!commandStack || selectedOpFeatures.length === 0) return;
  const units = unitsForCoordinateSpace(coordinateSpace);
  if (tool === "measure-length") {
    const total = selectedOpFeatures.reduce((sum, feature) => sum + (measureFeature(feature, coordinateSpace).length ?? 0), 0);
    measureReadout = formatMeasurement(total, units.length);
    return;
  }
  if (tool === "measure-area") {
    const total = selectedOpFeatures.reduce((sum, feature) => sum + (measureFeature(feature, coordinateSpace).area ?? 0), 0);
    measureReadout = formatMeasurement(total, units.area);
  }
}

function setTool(next: VectorDrawMode) {
  if (!canDraw && next !== "static" && next !== "select" && !next.startsWith("measure-")) return;
  cancelGeometryPreview();
  tool = next;
  measureReadout = "";
  editor?.clearMeasure();
  editor?.setMode(!canDraw && !next.startsWith("measure-") ? "static" : next);
  if (next === "measure-length" || next === "measure-area") updateMeasureFromSelection();
}

function switchLayer(layerId: string) {
  if (layerId === activeLayerId) return;
  editor?.switchLayer(layerId);
  activeLayerId = layerId;
  const layer = layers.find((item) => item.id === layerId);
  tool = layer?.locked ? "static" : "select";
  editor?.setMode(tool);
}

function addLayer() {
  if (!commandStack || layers.filter(isVectorLayer).length >= VECTOR_MAX_LAYERS) return;
  const built = buildCreateLayer(commandStack.document, `Layer ${layers.filter(isVectorLayer).length + 1}`);
  dispatchCommand(built.command);
  switchLayer(built.layer.id);
  tool = "select";
  editor?.setMode("select");
}

async function addRasterLayer() {
  if (!mapId || !commandStack || physicalMap) return;
  if (rasterLayerCount >= IMAGE_MAX_RASTER_LAYERS) {
    notice = `Raster layer count exceeds the budget of ${IMAGE_MAX_RASTER_LAYERS}.`;
    return;
  }
  const source = await project.pickImageMapFile();
  if (typeof source !== "string") return;
  if (!source.toLowerCase().endsWith(".png")) {
    notice = "Raster layers require a PNG asset. Use Rasters for JPEG or SVG overlays.";
    return;
  }
  try {
    busy = true;
    const attached = await project.attachMapRasterAsset(mapId, source);
    if (attached.asset.mime_type !== "image/png") {
      notice = "Raster layers require a PNG asset.";
      return;
    }
    await rememberRasterAsset(attached.asset.id, attached.asset.mime_type);
    const built = buildCreateRasterLayer(
      commandStack.document,
      attached.asset.filename.replace(/\.[^.]+$/, "") || "Raster layer",
      attached.asset.id,
    );
    dispatchCommand(built.command);
    switchLayer(built.layer.id);
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
  } finally {
    busy = false;
  }
}

async function duplicateLayer(layer: MapLayerDefinition) {
  if (!commandStack || (physicalMap && immutablePhysicalLayerIds.has(layer.id))) return;
  if (isVectorLayer(layer) && layers.filter(isVectorLayer).length >= VECTOR_MAX_LAYERS) return;
  if (isRasterLayer(layer) && rasterLayerCount >= IMAGE_MAX_RASTER_LAYERS) return;
  if (isRasterLayer(layer)) {
    if (!mapId) return;
    try {
      busy = true;
      const attached = await project.duplicateMapRasterAsset(mapId, layer.rasterAssetId);
      await rememberRasterAsset(attached.asset.id, attached.asset.mime_type);
      const built = buildDuplicateLayer(commandStack.document, layer, attached.asset.id);
      if (!built) return;
      dispatchCommand(built.command);
      switchLayer(built.layer.id);
    } catch (cause) {
      applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
    } finally {
      busy = false;
    }
    return;
  }
  const built = buildDuplicateLayer(commandStack.document, layer);
  if (!built) return;
  dispatchCommand(built.command);
  switchLayer(built.layer.id);
}

function toggleVisible(layer: MapLayerDefinition) {
  const nextVisible = !layer.defaultVisible;
  if (physicalLayerVisibility.has(layer.id)) {
    physicalLayerVisibility.set(layer.id, nextVisible);
    physicalLayerVisibility = new Map(physicalLayerVisibility);
    layers = layers.map((item) => (item.id === layer.id ? { ...item, defaultVisible: nextVisible } : item));
    editor?.syncLayers(layers, runtimeLayerRasters());
    physicalEditor?.syncLayers(layers);
    return;
  }
  if (!commandStack) return;
  dispatchCommand(setLayerVisibilityCommand(layer.id, nextVisible, layer.defaultVisible));
}

function toggleLock(layer: MapLayerDefinition) {
  if (physicalMap && immutablePhysicalLayerIds.has(layer.id)) return;
  if (!commandStack) return;
  const nextLocked = !layer.locked;
  dispatchCommand(setLayerLockedCommand(layer.id, nextLocked, layer.locked));
  if (layer.id === activeLayerId) {
    tool = nextLocked || !isVectorLayer(layer) ? "static" : "select";
    editor?.switchLayer(layer.id);
    editor?.setMode(tool);
  }
}

function renameLayer(layer: MapLayerDefinition, name: string) {
  if (physicalMap && immutablePhysicalLayerIds.has(layer.id)) return;
  const trimmed = name.trim();
  renamingId = null;
  if (!trimmed || trimmed === layer.name || !commandStack) return;
  dispatchCommand(renameLayerCommand(layer.id, trimmed, layer.name));
}

function moveLayer(layer: MapLayerDefinition, direction: -1 | 1) {
  if ((physicalMap && immutablePhysicalLayerIds.has(layer.id)) || !commandStack) return;
  const index = listedLayers.findIndex((item) => item.id === layer.id);
  const neighbor = listedLayers[index + direction];
  if (!neighbor) return;
  dispatchCommand(
    reorderLayerCommand(layer.id, neighbor.order, layer.order, neighbor.id, layer.order, neighbor.order),
  );
}

function dropLayer(sourceId: string, targetId: string) {
  if (!commandStack || sourceId === targetId) return;
  const display = [...listedLayers];
  const from = display.findIndex((item) => item.id === sourceId);
  const to = display.findIndex((item) => item.id === targetId);
  if (from < 0 || to < 0) return;
  const [moved] = display.splice(from, 1);
  display.splice(to, 0, moved);
  const nextIds = [...display].reverse().map((item) => item.id);
  const previousIds = [...layers].sort((left, right) => left.order - right.order || left.id.localeCompare(right.id)).map((item) => item.id);
  dispatchCommand(reorderLayersByIdsCommand(nextIds, previousIds));
}

function onLayerKey(event: KeyboardEvent, layer: MapLayerDefinition) {
  if (!event.altKey) return;
  if (event.key === "ArrowUp") {
    event.preventDefault();
    moveLayer(layer, -1);
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    moveLayer(layer, 1);
  }
}

function updateStyle(layer: MapLayerDefinition, patch: Partial<VectorLayerDefinition["style"]>) {
  if ((physicalMap && immutablePhysicalLayerIds.has(layer.id)) || !commandStack || !isVectorLayer(layer)) return;
  const style = { ...layer.style, ...patch };
  dispatchCommand(setLayerStyleCommand(layer.id, style, { ...layer.style }));
}

function setLayerOpacity(layer: MapLayerDefinition, opacity: number) {
  if ((physicalMap && immutablePhysicalLayerIds.has(layer.id)) || !commandStack) return;
  dispatchCommand(setLayerOpacityCommand(layer.id, opacity, layer.opacity));
}

function removeLayer(layer: MapLayerDefinition) {
  if ((physicalMap && immutablePhysicalLayerIds.has(layer.id)) || !commandStack || layer.locked) return;
  const savedCount = featureCountForLayer(loaded, layer.id);
  const draftCount = featureCountForLayer(draft, layer.id);
  const extra =
    isRasterLayer(layer)
      ? " The raster asset stays in the project."
      : draftCount === savedCount
        ? ""
        : ` Unsaved draft features on this layer (${draftCount}) will be discarded.`;
  const countLabel = isRasterLayer(layer)
    ? "this raster layer"
    : `${savedCount} saved feature${savedCount === 1 ? "" : "s"} from the map`;
  if (!confirm(`Delete ${layer.name}? This removes ${countLabel}.${extra}`)) return;
  const removedFeatures = commandStack.document.collection.features.filter(
    (feature) => featureLayerId(feature) === layer.id,
  );
  dispatchCommand(deleteLayerCommand(layer.id, layer, removedFeatures));
  const remaining = [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id));
  activeLayerId = remaining[0]?.id ?? null;
  if (activeLayerId) editor?.switchLayer(activeLayerId);
}

async function rememberRasterAsset(assetId: string, mimeType: string) {
  const bytes = await project.readAssetBytes(assetId);
  const decoded = await decodeRasterBytes(bytes, mimeType, loadGeneration);
  if (!decoded) return;
  const next = new Map(rasterAssets);
  next.set(assetId, decoded);
  rasterAssets = next;
}

function backgroundExtentForRaster(width: number, height: number): [number, number, number, number] {
  if (coordinateSpace.kind === "image") return extentOf(coordinateSpace);
  if (coordinateSpace.kind === "world" || coordinateSpace.kind === "geographic") return extentOf(coordinateSpace);
  return [0, 0, width, height];
}

async function addRaster() {
  if (!mapId || !commandStack || physicalMap) return;
  if (listedRasters.length >= IMAGE_MAX_RASTER_LAYERS) {
    notice = `Raster count exceeds the budget of ${IMAGE_MAX_RASTER_LAYERS}.`;
    return;
  }
  const source = await project.pickImageMapFile();
  if (typeof source !== "string") return;
  try {
    busy = true;
    const attached = await project.attachMapRasterAsset(mapId, source);
    await rememberRasterAsset(attached.asset.id, attached.asset.mime_type);
    const background: MapBackgroundRef = {
      id: crypto.randomUUID(),
      assetId: attached.asset.id,
      name: attached.asset.filename.replace(/\.[^.]+$/, "") || "Raster",
      visible: true,
      locked: false,
      opacity: 1,
      order: nextBackgroundOrder(listedRasters),
      extent: backgroundExtentForRaster(attached.width, attached.height),
    };
    dispatchCommand(addBackgroundCommand(background));
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
  } finally {
    busy = false;
  }
}

async function replaceRaster(current: MapBackgroundRef) {
  if (!mapId || !commandStack || physicalMap) return;
  const source = await project.pickImageMapFile();
  if (typeof source !== "string") return;
  try {
    busy = true;
    const attached = await project.attachMapRasterAsset(mapId, source);
    await rememberRasterAsset(attached.asset.id, attached.asset.mime_type);
    dispatchCommand(
      replaceBackgroundCommand(
        current.id,
        {
          ...current,
          assetId: attached.asset.id,
          name: attached.asset.filename.replace(/\.[^.]+$/, "") || current.name,
          extent: backgroundExtentForRaster(attached.width, attached.height),
        },
        current,
      ),
    );
  } catch (cause) {
    applyEditorEvent({ type: "save-failed", message: cause instanceof Error ? cause.message : String(cause) });
  } finally {
    busy = false;
  }
}

function removeRaster(current: MapBackgroundRef) {
  if (!commandStack) return;
  if (!confirm(`Remove ${current.name}? The raster overlay is removed from this map.`)) return;
  dispatchCommand(removeBackgroundCommand(current.id, current));
}

function moveRaster(current: MapBackgroundRef, direction: -1 | 1) {
  if (!commandStack) return;
  const index = listedRasters.findIndex((item) => item.id === current.id);
  const neighbor = listedRasters[index + direction];
  if (!neighbor) return;
  dispatchCommand(
    reorderBackgroundCommand(current.id, neighbor.order, current.order, neighbor.id, current.order, neighbor.order),
  );
}

function applyCalibration() {
  if (!commandStack) return;
  const raw = calibrateMetres.trim();
  const metres = raw === "" ? null : Number(raw);
  if (raw !== "" && (!Number.isFinite(metres) || (metres ?? 0) <= 0)) {
    notice = "Calibration requires a positive metres-per-unit value, or leave it empty for arbitrary units.";
    return;
  }
  const command =
    coordinateSpace.kind === "image"
      ? calibrateImageToWorld(commandStack.document, metres)
      : calibrateWorldUnits(commandStack.document, metres);
  if (!command) {
    notice = "Geographic maps are not calibrated this way.";
    return;
  }
  dispatchCommand(command);
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
  if (event.key === "Escape" && geometryPreview) {
    event.preventDefault();
    cancelGeometryPreview();
    return;
  }
  if (event.key === "Escape" && measureReadout) {
    event.preventDefault();
    measureReadout = "";
    editor?.clearMeasure();
    if (tool.startsWith("measure-")) setTool("select");
    return;
  }
  if (event.key === "Escape" && selectedFeatureIds.length > 0) {
    event.preventDefault();
    editor?.clearSelection();
    selectedFeature = null;
    selectedFeatureIds = [];
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
    if (event.shiftKey) redoEdit();
    else undoEdit();
  } else if (!meta && !renamingId && !picking) {
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      deleteSelectedFeatures();
    } else if (event.key === "v" || event.key === "h") setTool("static");
    if (event.key === "s") setTool("select");
    if (event.key === "p") setTool("point");
    if (event.key === "l") setTool("linestring");
    if (event.key === "g") setTool("polygon");
    if (event.key === "r") setTool("rectangle");
    if (event.key === "f") setTool("freehand");
    if (event.key === "\\") toggleSnapEnabled();
    if (event.key === "d") setTool("measure-distance");
    if (event.key === "M") setTool("measure-length");
    if (event.key === "A") setTool("measure-area");
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
    for (const url of objectUrls) URL.revokeObjectURL(url);
    objectUrls.length = 0;
    rasterAssets = new Map();
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
          ? `Unsaved changes · ${unitsLabel}`
          : physicalMap
            ? "Generated world map"
            : unitsLabel}
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
          <button
            type="button"
            class="icon-button"
            class:active={snapEnabled}
            aria-pressed={snapEnabled}
            aria-label={snapEnabled ? "Snap on" : "Snap off"}
            title={snapEnabled ? "Snap on (\\)" : "Snap off (\\)"}
            disabled={!editor}
            onclick={() => toggleSnapEnabled()}><Magnet {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={snapConfigOpen}
            aria-pressed={snapConfigOpen}
            aria-label="Snap settings"
            title="Snap settings"
            disabled={!editor}
            onclick={() => (snapConfigOpen = !snapConfigOpen)}><CircleHelp {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "measure-distance"}
            aria-pressed={tool === "measure-distance"}
            aria-label="Measure distance"
            title="Measure distance (D)"
            disabled={!editor}
            onclick={() => setTool("measure-distance")}><Ruler {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "measure-length"}
            aria-pressed={tool === "measure-length"}
            aria-label="Measure length"
            title="Measure length (Shift+M)"
            disabled={!editor}
            onclick={() => setTool("measure-length")}><Slash {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            class:active={tool === "measure-area"}
            aria-pressed={tool === "measure-area"}
            aria-label="Measure area"
            title="Measure area (Shift+A)"
            disabled={!editor}
            onclick={() => setTool("measure-area")}><SquareStack {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            aria-label="Undo"
            title="Undo"
            disabled={!canUndo}
            onclick={() => undoEdit()}><Undo2 {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            aria-label="Redo"
            title="Redo"
            disabled={!canRedo}
            onclick={() => redoEdit()}><Redo2 {...iconProps} /></button>
          <button
            type="button"
            class="icon-button"
            aria-label="Add layer"
            title="Add layer"
            disabled={busy || layers.length >= VECTOR_MAX_LAYERS}
            onclick={() => addLayer()}><SquarePlus {...iconProps} /></button>
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
            <strong id="vector-layers-heading">Layers</strong>
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
            {#if !physicalMap}
              <div class="layer-row">
                <button type="button" class="text-button" disabled={busy || layers.filter(isVectorLayer).length >= VECTOR_MAX_LAYERS} onclick={() => addLayer()}>
                  Add vector
                </button>
                <button type="button" class="text-button" disabled={busy || rasterLayerCount >= IMAGE_MAX_RASTER_LAYERS} onclick={() => void addRasterLayer()}>
                  Add raster layer
                </button>
              </div>
            {/if}
            {#if snapConfigOpen && !physicalMap}
              <div class="snap-config" aria-label="Snap settings">
                <label><input type="checkbox" bind:checked={snapVertex} onchange={syncSnapToEditor} /> Vertex</label>
                <label><input type="checkbox" bind:checked={snapEdge} onchange={syncSnapToEditor} /> Edge</label>
                <label><input type="checkbox" bind:checked={snapIntersection} onchange={syncSnapToEditor} /> Intersection</label>
                <small>Locked layers can opt into snap targets from the layer row.</small>
              </div>
            {/if}
            <div class="layer-list" role="list" aria-labelledby="vector-layers-heading">
              {#each listedLayers as layer (layer.id)}
                <div
                  class="layer"
                  class:active={layer.id === activeLayerId}
                  role="listitem"
                  tabindex="0"
                  draggable={!immutablePhysicalLayerIds.has(layer.id)}
                  ondragstart={() => (draggingLayerId = layer.id)}
                  ondragover={(event) => event.preventDefault()}
                  ondrop={(event) => {
                    event.preventDefault();
                    if (draggingLayerId) dropLayer(draggingLayerId, layer.id);
                    draggingLayerId = null;
                  }}
                  onkeydown={(event) => onLayerKey(event, layer)}>
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
                    <small class="layer-meta">{layer.kind} · {isRasterLayer(layer) ? "raster" : featureCountForLayer(draft, layer.id)}</small>
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
                      {#if layer.locked && layer.defaultVisible && isVectorLayer(layer)}
                        <button
                          type="button"
                          class="icon-button"
                          class:active={snapTargetLayerIds.has(layer.id)}
                          aria-pressed={snapTargetLayerIds.has(layer.id)}
                          aria-label={`Snap to ${layer.name}`}
                          title={`Snap to ${layer.name}`}
                          onclick={() => toggleSnapTargetLayer(layer.id)}><Magnet {...iconProps} /></button>
                      {/if}
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
                        disabled={busy || (isRasterLayer(layer) ? rasterLayerCount >= IMAGE_MAX_RASTER_LAYERS : layers.filter(isVectorLayer).length >= VECTOR_MAX_LAYERS)}
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
                        Layer opacity
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.05"
                          value={layer.opacity}
                          aria-label={`${layer.name} opacity`}
                          oninput={(event) => void setLayerOpacity(layer, Number(event.currentTarget.value))} />
                      </label>
                      {#if isVectorLayer(layer)}
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
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
          {#if !physicalMap}
            <button
              type="button"
              class="aside-toggle"
              aria-expanded={!rastersCollapsed}
              onclick={() => (rastersCollapsed = !rastersCollapsed)}>
              <strong id="raster-layers-heading">Rasters</strong>
              <span class="aside-chevron" class:collapsed={rastersCollapsed}><ChevronDown {...iconProps} /></span>
            </button>
            {#if !rastersCollapsed}
              <p class="hint" role="status">{unitsLabel}</p>
              <div class="layer-row">
                <button type="button" class="text-button" disabled={busy || listedRasters.length >= IMAGE_MAX_RASTER_LAYERS} onclick={() => void addRaster()}>
                  Add raster
                </button>
                <button type="button" class="text-button" onclick={() => editor?.fitExtent()}>Fit</button>
                <button type="button" class="text-button" onclick={() => editor?.actualPixels()}>Actual pixels</button>
              </div>
              <label class="calibrate">
                Metres per unit
                <input
                  type="number"
                  min="0"
                  step="any"
                  bind:value={calibrateMetres}
                  placeholder={coordinateSpace.kind === "image" ? "pixels until calibrated" : "optional"}
                  aria-label="Metres per unit" />
                <button type="button" onclick={() => applyCalibration()}>Calibrate</button>
              </label>
              {#if listedRasters.length === 0}
                <p class="hint">Add PNG, JPEG, or SVG overlays. Image maps open at exact pixel extent.</p>
              {/if}
              <div class="layer-list" role="list" aria-labelledby="raster-layers-heading">
                {#each listedRasters as raster (raster.id)}
                  <div class="layer" role="listitem">
                    <span class="layer-name">{raster.name}</span>
                    <div class="layer-row">
                      <button
                        type="button"
                        class="icon-button"
                        aria-pressed={raster.visible}
                        aria-label={raster.visible ? `Hide ${raster.name}` : `Show ${raster.name}`}
                        onclick={() =>
                          dispatchCommand(setBackgroundVisibilityCommand(raster.id, !raster.visible, raster.visible))}
                        >{#if raster.visible}<Eye {...iconProps} />{:else}<EyeOff {...iconProps} />{/if}</button>
                      <button type="button" class="icon-button" aria-label={`Move ${raster.name} up`} onclick={() => moveRaster(raster, -1)}
                        ><ChevronUp {...iconProps} /></button>
                      <button type="button" class="icon-button" aria-label={`Move ${raster.name} down`} onclick={() => moveRaster(raster, 1)}
                        ><ChevronDown {...iconProps} /></button>
                      <button type="button" class="icon-button" aria-label={`Replace ${raster.name}`} onclick={() => void replaceRaster(raster)}
                        ><ImageIcon {...iconProps} /></button>
                      <button type="button" class="icon-button" aria-label={`Remove ${raster.name}`} onclick={() => removeRaster(raster)}
                        ><Trash2 {...iconProps} /></button>
                    </div>
                    <label>
                      Opacity
                      <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.05"
                        value={raster.opacity}
                        aria-label={`${raster.name} opacity`}
                        oninput={(event) =>
                          dispatchCommand(
                            setBackgroundOpacityCommand(raster.id, Number(event.currentTarget.value), raster.opacity),
                          )} />
                    </label>
                  </div>
                {/each}
              </div>
            {/if}
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
          {#if selectedFeatureIds.length > 0 && !physicalMap}
            <div class="geometry-ops" aria-label="Geometry operations">
              <strong>Geometry</strong>
              {#if geometryPreview}
                <p class="hint">Preview: {geometryPreview.label}. Commit or cancel to finish.</p>
                <div class="layer-row">
                  <button type="button" onclick={() => commitGeometryPreview()}>Apply</button>
                  <button type="button" onclick={() => cancelGeometryPreview()}>Cancel</button>
                </div>
              {:else}
                <div class="layer-row">
                  <button type="button" disabled={!canRunOperation("union", selectedOpFeatures)} onclick={() => startGeometryOperation("union")}>Union</button>
                  <button type="button" disabled={!canRunOperation("difference", selectedOpFeatures)} onclick={() => startGeometryOperation("difference")}>Diff</button>
                  <button type="button" disabled={!canRunOperation("intersection", selectedOpFeatures)} onclick={() => startGeometryOperation("intersection")}>Intersect</button>
                </div>
                <div class="layer-row">
                  <button type="button" disabled={!canRunOperation("split", selectedOpFeatures)} onclick={() => startGeometryOperation("split")}><Scissors {...iconProps} /> Split</button>
                  <label>
                    Buffer
                    <input type="number" min="0" step="any" bind:value={bufferDistance} aria-label="Buffer distance" />
                  </label>
                  <button type="button" disabled={!canRunOperation("buffer", selectedOpFeatures)} onclick={() => startGeometryOperation("buffer")}>Run</button>
                </div>
                <div class="layer-row">
                  <label>
                    Simplify
                    <input type="number" min="0" step="any" bind:value={simplifyTolerance} aria-label="Simplify tolerance" />
                  </label>
                  <button type="button" disabled={!canRunOperation("simplify", selectedOpFeatures)} onclick={() => startGeometryOperation("simplify")}>Run</button>
                </div>
                {#if operationNotice}<small role="status">{operationNotice}</small>{/if}
              {/if}
            </div>
          {/if}
          {#if selectedFeatureIds.length > 1 && !physicalMap}
            <div class="inspector" aria-label="Selected features">
              <strong>{selectedFeatureIds.length} features selected</strong>
              <p class="hint">Shift-click adds to the selection. Modifier-drag boxes select across visible unlocked layers.</p>
              <label>
                Move to layer
                <select
                  aria-label="Feature layer"
                  onchange={(event) => moveSelectedToLayer(event.currentTarget.value)}>
                  {#each listedLayers.filter((layer) => isVectorLayer(layer) && layer.id !== "base" && layer.defaultVisible && !layer.locked) as layer}
                    <option value={layer.id}>{layer.name}</option>
                  {/each}
                </select>
              </label>
              <button type="button" onclick={() => duplicateSelectedFeatures()}>Duplicate features</button>
              <button type="button" onclick={() => editor?.fitSelection(selectedFeatureIds)}>Fit selection</button>
              <button type="button" onclick={() => deleteSelectedFeatures()}>Delete</button>
            </div>
          {:else if selectedFeature && !physicalMap}
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
                    renameSelectedFeature(next);
                  }} />
              </label>
              <label>
                Layer
                <select
                  value={featureLayerId(selectedFeature)}
                  aria-label="Feature layer"
                  disabled={featureLayerId(selectedFeature) === "base" || Boolean(activeLayer?.locked)}
                  onchange={(event) => moveSelectedToLayer(event.currentTarget.value)}>
                  {#each listedLayers.filter((layer) => isVectorLayer(layer) && layer.id !== "base" && layer.defaultVisible && !layer.locked) as layer}
                    <option value={layer.id}>{layer.name}</option>
                  {/each}
                </select>
              </label>
              <button
                type="button"
                disabled={featureLayerId(selectedFeature) === "base" || Boolean(activeLayer?.locked)}
                onclick={() => duplicateSelectedFeatures()}>Duplicate feature</button>
              <button type="button" onclick={() => editor?.fitSelection([selectedFeature.id])}>Fit selection</button>
              <p class="hint">Alt-click a vertex to delete it.</p>
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
              raster={rasterAssets.get("physical")?.canvas ?? null}
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
            tabindex="0"
            role="application"
            aria-label="Native vector map canvas">
            <div class="map-host" bind:this={host}></div>
            {#if editor}
              <MapViewControls
                zoom={defaultView.zoom}
                min={0}
                max={viewMaxZoom}
                onzoom={(zoom) => {
                  editor?.setZoom(zoom);
                }}
                onpan={(x, y) => editor?.panCardinal(x > 0 ? 1 : x < 0 ? -1 : 0, y > 0 ? 1 : y < 0 ? -1 : 0)} />
            {/if}
            {#if measureReadout}
              <div class="measure-readout" role="status">{measureReadout}</div>
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
.calibrate {
  display: grid;
  gap: 6px;
  padding: 8px;
  font: 12px system-ui;
}
.calibrate input {
  width: 100%;
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
.snap-config,
.geometry-ops {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 8px;
  background: rgb(27 40 34 / 72%);
  font-size: 12px;
}
.snap-config label,
.geometry-ops label {
  display: flex;
  align-items: center;
  gap: 6px;
}
.measure-readout {
  position: absolute;
  z-index: 2;
  top: 10px;
  left: 50%;
  transform: translateX(-50%);
  padding: 6px 10px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 8px;
  background: rgb(27 40 34 / 92%);
  color: #f3d39a;
  font-size: 13px;
  font-weight: 700;
  pointer-events: none;
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
  display: flex;
  flex-direction: column;
  gap: 2px;
  align-items: flex-start;
  text-align: left;
  width: 100%;
  padding: 4px 6px;
  background: transparent;
  font-weight: 600;
}
.layer-meta {
  color: var(--theme-neutral-text-muted, #aebdb1);
  font-size: 10px;
  font-weight: 500;
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
.map-host {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}
.map-host :global(.ol-viewport) {
  width: 100%;
  height: 100%;
}
.stage {
  position: relative;
  display: flex;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
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
