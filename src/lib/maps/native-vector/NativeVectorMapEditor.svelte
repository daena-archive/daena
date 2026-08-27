<script lang="ts">
import { onMount, tick, type Component } from "svelte";
import { listen } from "@tauri-apps/api/event";
import {
  ChevronDown,
  ChevronUp,
  ChevronRight,
  Circle,
  CircleHelp,
  Copy,
  Download,
  Ellipsis,
  Eye,
  EyeOff,
  GripVertical,
  Hexagon,
  Image as ImageIcon,
  Layers,
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
  Search,
  Settings2,
  Slash,
  SlidersHorizontal,
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
  type MapLabelV2,
  type MapStyleV2,
} from "../../../../packages/plugin-sdk/src/maps";
import NativeVectorImporter from "./NativeVectorImporter.svelte";
import MapLocationLinkPanel from "./MapLocationLinkPanel.svelte";
import { createMapAdapter, liveMapAdapterCount, RENDERER_UNAVAILABLE, type MapAdapter } from "../openlayers/MapAdapter";
import type { RuntimeBackground } from "../openlayers/background-registry";
import type { RasterLayerSource } from "../openlayers/layer-registry";
import { maxZoomForCoordinateSpace } from "../openlayers/projection";
import { registerNativeVectorSession } from "./session";
import { collectionBytes, featureCountForLayer, parseVectorCollection, parseVectorLayers, sha256Hex } from "./source";
import {
  initialVectorEditorState,
  parseVectorDiagnostic,
  reduceVectorEditor,
  type VectorEditorState,
} from "./editor-state";
import {
  VECTOR_PROVIDER,
  DEFAULT_VECTOR_LAYER_STYLE,
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
import AtlasRenderPanel from "../atlas/AtlasRenderPanel.svelte";
import AtlasStudioView from "../atlas/AtlasStudioView.svelte";
import DetachPhysicalLayerDialog from "../physical/DetachPhysicalLayerDialog.svelte";
import {
  buildPhysicalDetachPlan,
  isPhysicalDerivedLayerId,
  physicalFeaturesForLayer,
  selectedPhysicalFeatures,
  type PhysicalDetachScope,
} from "../physical/detach";
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
  setFeaturesMetadataByIdCommand,
  setLayerLockedCommand,
  setLayerOpacityCommand,
  setLayerStyleCommand,
  setLayerVisibilityCommand,
  detachPhysicalFeaturesCommand,
  applyGeometryOperationCommand,
  setSnapSettingsCommand,
  snapEnabledFromDescriptor,
  buildPreview,
  commitSelectionIds,
  canRunOperation,
  formatMeasurement,
  measureFeature,
  unitsForCoordinateSpace,
  buildMapSearchIndex,
  searchMapFeatures,
  type GeometryPreview,
  type GeometryOperationKind,
  type MapCommand,
} from "../editor";

let {
  mapId,
  picking = false,
  start = "geojson",
  focusLinkId,
  focusFeatureId,
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
  focusFeatureId?: string;
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
let derivedPhysical = $state<VectorFeatureCollection>({ type: "FeatureCollection", features: [] });
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
let featureSearch = $state("");
let searchOpen = $state(false);
let customKey = $state("");
let customValue = $state("");
let draggingLayerId = $state<string | null>(null);
let openMenuLayerId = $state<string | null>(null);
let openCustomizeLayerId = $state<string | null>(null);
let openRasterMenuId = $state<string | null>(null);
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
let sidebarWidth = $state(300);
let detachLayerId = $state<string | null>(null);
let detachError = $state("");

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
const linkedEntityNames = $derived(
  new Map([...featureLinks].map(([featureId, link]) => [featureId, link.label || link.entityId])),
);
const mapSearchIndex = $derived(buildMapSearchIndex(draft, layers, linkedEntityNames));
const mapSearchResults = $derived(searchMapFeatures(mapSearchIndex, featureSearch));
const selectedFeatures = $derived(
  commandStack?.document.collection.features.filter((feature) => selectedFeatureIds.includes(feature.id)) ?? [],
);
const detachLayer = $derived(
  detachLayerId ? (layers.find((layer) => layer.id === detachLayerId && isVectorLayer(layer)) ?? null) : null,
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

function renderedCollection(authored: VectorFeatureCollection): VectorFeatureCollection {
  if (!physicalMap) return authored;
  return {
    type: "FeatureCollection",
    features: [...derivedPhysical.features, ...authored.features],
  };
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
    label?: string | null;
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
        label: typeof pin.label === "string" && pin.label.trim() ? pin.label.trim() : null,
      });
    }
  }
  featureLinks = nextFeatureLinks;
  linkAnchors = nextAnchors;
  pinsReady = true;
}

function focusLinkedLocation(linkId: string | null | undefined) {
  if (!linkId || picking || linkArming || !pinsReady) return false;
  const target = editor;
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
  draft = renderedCollection(snap.document.collection);
  layers = snap.document.layers;
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
  draft = renderedCollection(document.collection);
  layers = document.layers;
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

function offsetGeometry(geometry: VectorFeature["geometry"], dx: number, dy: number): VectorFeature["geometry"] {
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
  const [offsetX, offsetY] = duplicateOffset(coordinateSpace);
  const copies = selected.map((feature) => {
    const clone = cloneCollection({ type: "FeatureCollection", features: [feature] }).features[0];
    clone.id = crypto.randomUUID();
    clone.geometry = offsetGeometry(clone.geometry, offsetX, offsetY);
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

function selectedMetadataSnapshot(feature: VectorFeature) {
  return {
    name: feature.properties.daena.name,
    semanticType: feature.properties.daena.semanticType,
    style: feature.properties.daena.style,
    label: feature.properties.daena.label,
    custom: feature.properties.daena.custom,
  };
}

function updateSelectedMetadata(
  build: (feature: VectorFeature) => Partial<VectorFeature["properties"]["daena"]>,
  label: string,
  coalesceKey?: string,
) {
  if (!commandStack || selectedFeatures.length === 0) return;
  const next: Record<string, ReturnType<typeof selectedMetadataSnapshot>> = {};
  const previous: Record<string, ReturnType<typeof selectedMetadataSnapshot>> = {};
  for (const feature of selectedFeatures) {
    previous[feature.id] = selectedMetadataSnapshot(feature);
    next[feature.id] = { ...previous[feature.id], ...build(feature) };
  }
  dispatchCommand(setFeaturesMetadataByIdCommand(next, previous, label, coalesceKey));
}

function setSelectedSemanticType(semanticType: VectorFeature["properties"]["daena"]["semanticType"]) {
  updateSelectedMetadata(() => ({ semanticType }), "Change semantic type", "feature-semantic-type");
}

function updateSelectedStyle(patch: Partial<MapStyleV2>) {
  updateSelectedMetadata(
    (feature) => ({ style: { ...(feature.properties.daena.style ?? {}), ...patch } }),
    "Edit feature style",
    `feature-style:${Object.keys(patch).join(",")}`,
  );
}

function defaultFeatureLabel(feature: VectorFeature): MapLabelV2 {
  return {
    source: "name",
    text: null,
    size: 12,
    color: "#f7f0e5",
    haloColor: "#0d1b2a",
    haloWidth: 3,
    placement: feature.geometry.type === "LineString" || feature.geometry.type === "MultiLineString" ? "line" : "point",
    offset: [0, -14],
    rotation: 0,
    minZoom: null,
    maxZoom: null,
  };
}

function updateSelectedLabel(patch: Partial<MapLabelV2>) {
  updateSelectedMetadata(
    (feature) => ({ label: { ...(feature.properties.daena.label ?? defaultFeatureLabel(feature)), ...patch } }),
    "Edit feature label",
    `feature-label:${Object.keys(patch).join(",")}`,
  );
}

function clearSelectedOverrides() {
  updateSelectedMetadata(() => ({ style: null, label: null }), "Use layer style");
}

function addCustomProperty() {
  const key = customKey.trim();
  if (!key || !/^[A-Za-z][A-Za-z0-9_.-]{0,63}$/.test(key)) {
    notice =
      "Custom property keys must start with a letter and contain only letters, numbers, dot, dash, or underscore.";
    return;
  }
  const value = customValue.trim();
  updateSelectedMetadata(
    (feature) => ({ custom: { ...feature.properties.daena.custom, [key]: value } }),
    "Set custom property",
    `feature-custom:${key}`,
  );
  customKey = "";
  customValue = "";
  notice = "";
}

function removeCustomProperty(key: string) {
  updateSelectedMetadata((feature) => {
    const custom = { ...feature.properties.daena.custom };
    delete custom[key];
    return { custom };
  }, "Remove custom property");
}

function featureVertexCount(feature: VectorFeature) {
  return (feature.geometry.coordinates.flat(Infinity) as number[]).length / 2;
}

function focusSearchResult(featureId: string, layerId: string) {
  if (!commandStack || !editor) return;
  const layer = layers.find((item) => item.id === layerId);
  if (layer && !layer.defaultVisible) {
    dispatchCommand(setLayerVisibilityCommand(layer.id, true, false));
  }
  activeLayerId = layerId;
  editor.switchLayer(layerId);
  editor.focusFeature(featureId);
  searchOpen = false;
}

function focusFeatureFromNavigation(featureId: string | null | undefined) {
  if (!featureId || !commandStack || !editor) return false;
  const feature = commandStack.document.collection.features.find((item) => item.id === featureId);
  if (!feature) return false;
  focusSearchResult(feature.id, featureLayerId(feature));
  return true;
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
  const parsed = parseVectorLayers(field.value);
  layers = parsed;
  if (commandStack) {
    commandStack.replaceDocument({
      ...commandStack.document,
      layers: parsed,
      descriptor:
        mapField?.value && typeof mapField.value === "object"
          ? (mapField.value as typeof commandStack.document.descriptor)
          : commandStack.document.descriptor,
    });
  }
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
  if (busy || epochBusy || dirty) return;
  const next = clampEpoch(offset);
  syncEpochFields(next);
  scheduleEpoch(next);
}

function commitEpochFromExact(absYears: number, era: "past" | "future") {
  if (busy || epochBusy || dirty) return;
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
  if (!commandStack || commandStack.isDirty()) return;
  const physical = parseDerivedCollection(products.geojson);
  derivedPhysical = physical;
  const canvas = physicalHillshadeCanvas(products.hydrology);
  rasterAssets = new Map([
    ["physical", { url: "", width: products.hydrology.width, height: products.hydrology.height, canvas }],
  ]);
  epochOffsetYears = products.epochOffsetYears;
  appliedEpochOffsetYears = products.epochOffsetYears;
  syncEpochFields(products.epochOffsetYears);
  draft = renderedCollection(commandStack.document.collection);
  editor?.syncDocument(draft, layers, runtimeLayerRasters());
  editor?.syncBackgrounds(runtimeBackgrounds());
}

async function loadPhysicalEpoch(offset: number) {
  if (!mapId || !physicalMap || commandStack?.isDirty()) return;
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
    labelsVisible: physicalMap ? (layerId) => !isPhysicalDerivedLayerId(layerId) : true,
    allowLockedBoxSelection: physicalMap,
  });
  if ("error" in created) {
    applyEditorEvent({ type: "save-failed", message: `${created.error}: ${created.detail}` });
    publish("error", created);
    return;
  }
  editor = created;
  syncSnapToEditor();
  editor.setMode(canDraw || tool === "select" || tool === "static" || tool.startsWith("measure-") ? tool : "static");
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
      syncEpochFields(0);
      const requestId = crypto.randomUUID();
      activeEpochRequestId = requestId;
      epochBusy = true;
      epochPhase = "Starting historical derivation";
      epochProgress = { completed: 0, total: 1 };
      const historical = await project.physicalMapDerivedEpoch(mapId, 0, requestId);
      if (generation !== loadGeneration) return;
      const physical = parseDerivedCollection(historical.geojson);
      derivedPhysical = physical;
      resetCommandStack({
        descriptor: mapField?.value ?? {},
        layers,
        collection,
      });
      epochNotice = `Showing ${formatEpoch(historical.epochOffsetYears)} · deterministic derived playback`;
      const canvas = physicalHillshadeCanvas(historical.hydrology);
      rasterAssets = new Map([
        ["physical", { url: "", width: historical.hydrology.width, height: historical.hydrology.height, canvas }],
      ]);
      epochBusy = false;
      epochPhase = "";
      epochProgress = null;
    } else {
      immutablePhysicalLayerIds = new Set();
      derivedPhysical = { type: "FeatureCollection", features: [] };
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
    const snapshot = cloneCollection(document.collection);
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
    draft = renderedCollection(snapshot);
    loaded = cloneCollection(snapshot);
    layers = parseVectorLayers(applied.layers.value);
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
  dispatchCommand(applyGeometryOperationCommand(removed, geometryPreview.previewFeatures, geometryPreview.label));
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
    const total = selectedOpFeatures.reduce(
      (sum, feature) => sum + (measureFeature(feature, coordinateSpace).length ?? 0),
      0,
    );
    measureReadout = formatMeasurement(total, units.length);
    return;
  }
  if (tool === "measure-area") {
    const total = selectedOpFeatures.reduce(
      (sum, feature) => sum + (measureFeature(feature, coordinateSpace).area ?? 0),
      0,
    );
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
  if (!commandStack) return;
  dispatchCommand(setLayerVisibilityCommand(layer.id, nextVisible, layer.defaultVisible));
}

function openDetachDialog(layer: MapLayerDefinition) {
  if (!physicalMap || !isVectorLayer(layer) || !isPhysicalDerivedLayerId(layer.id)) return;
  if (physicalFeaturesForLayer(derivedPhysical, layer.id).length === 0) return;
  detachLayerId = layer.id;
  detachError = "";
}

async function confirmDetach(scope: PhysicalDetachScope) {
  if (!commandStack || !detachLayer || !isPhysicalDerivedLayerId(detachLayer.id)) return;
  const plan = buildPhysicalDetachPlan({
    collection: derivedPhysical,
    document: commandStack.document,
    sourceLayer: detachLayer,
    epochOffsetYears: appliedEpochOffsetYears,
    scope,
    selectedIds: selectedFeatureIds,
  });
  if ("code" in plan) {
    detachError = plan.message;
    return;
  }
  dispatchCommand(
    detachPhysicalFeaturesCommand({
      sourceLayerId: plan.sourceLayerId,
      sourceLayerName: plan.sourceLayerName,
      sourceWasVisible: detachLayer.defaultVisible,
      targetLayer: plan.targetLayer,
      copies: plan.copies,
    }),
  );
  detachLayerId = null;
  activeLayerId = plan.targetLayer.id;
  tool = "select";
  editor?.switchLayer(plan.targetLayer.id);
  editor?.setMode("select");
  await tick();
  editor?.selectFeatureIds(plan.copies.map((feature) => feature.id));
  notice = `Detached ${plan.copies.length} features from ${plan.sourceLayerName} at ${formatEpoch(plan.epochOffsetYears)}. Save to commit the snapshot.`;
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

function focusOnMount(node: HTMLInputElement) {
  node.focus();
}

function moveLayer(layer: MapLayerDefinition, direction: -1 | 1) {
  if ((physicalMap && immutablePhysicalLayerIds.has(layer.id)) || !commandStack) return;
  const index = listedLayers.findIndex((item) => item.id === layer.id);
  const neighbor = listedLayers[index + direction];
  if (!neighbor) return;
  dispatchCommand(reorderLayerCommand(layer.id, neighbor.order, layer.order, neighbor.id, layer.order, neighbor.order));
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
  const previousIds = [...layers]
    .sort((left, right) => left.order - right.order || left.id.localeCompare(right.id))
    .map((item) => item.id);
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
  const extra = isRasterLayer(layer)
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
  pinsReady;
  linkAnchors;
  picking;
  linkArming;
  focusLinkedLocation(focusLinkId);
});

$effect(() => {
  focusFeatureId;
  editor;
  commandStack;
  focusFeatureFromNavigation(focusFeatureId);
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
  const handleLayerMenuOutside = (event: MouseEvent) => {
    const target = event.target as HTMLElement | null;
    if (!target?.closest(".layer-card") && !target?.closest(".raster-card")) {
      openMenuLayerId = null;
      openCustomizeLayerId = null;
      openRasterMenuId = null;
    }
  };
  window.addEventListener("click", handleLayerMenuOutside);
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
    window.removeEventListener("click", handleLayerMenuOutside);
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
        : physicalMap
          ? dirty
            ? `Unsaved authored changes · ${unitsLabel}`
            : "Generated world map"
          : dirty
            ? `Unsaved changes · ${unitsLabel}`
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
        {#if !studioOpen}
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
            disabled={dirty}
            onclick={() => {
              if (dirty) return;
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
    {#if detachLayer && isPhysicalDerivedLayerId(detachLayer.id)}
      {@const detachAll = physicalFeaturesForLayer(derivedPhysical, detachLayer.id)}
      {@const detachSelected = selectedPhysicalFeatures(derivedPhysical, detachLayer.id, selectedFeatureIds)}
      <DetachPhysicalLayerDialog
        sourceLayerName={detachLayer.name}
        epochOffsetYears={appliedEpochOffsetYears}
        selectedFeatureCount={detachSelected.length}
        totalSourceLayerFeatureCount={detachAll.length}
        initialScope={detachSelected.length > 0 && detachSelected.length < detachAll.length ? "selected" : "layer"}
        {busy}
        error={detachError}
        onconfirm={(scope) => void confirmDetach(scope)}
        oncancel={() => {
          detachLayerId = null;
          detachError = "";
        }} />
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
        <aside class="map-layers-panel" aria-label="Map layers">
          <div class="map-panel-head">
            <div class="map-panel-head-copy">
              <span class="panel-kicker">Map layers</span>
              <strong>{listedLayers.length} layers · {draft.features.length} features</strong>
              <small class="map-panel-subtitle"
                >{unitsLabel}{listedRasters.length ? ` · ${listedRasters.length} rasters` : ""}</small>
            </div>
            <button
              type="button"
              class="panel-icon-btn"
              aria-label="Add vector layer"
              title="Add vector layer"
              disabled={busy || layers.filter(isVectorLayer).length >= VECTOR_MAX_LAYERS}
              onclick={() => addLayer()}>
              <SquarePlus size={15} strokeWidth={1.8} aria-hidden="true" />
            </button>
          </div>

          <div class="map-search-wrap">
            <div class="map-search" class:open={searchOpen}>
              <label>
                <Search size={14} strokeWidth={1.8} aria-hidden="true" />
                <input
                  type="search"
                  placeholder="Search features…"
                  aria-label="Search map features"
                  bind:value={featureSearch}
                  onfocus={() => (searchOpen = true)}
                  onblur={() => setTimeout(() => (searchOpen = false), 180)} />
                {#if featureSearch.trim()}
                  <button
                    type="button"
                    class="search-clear"
                    aria-label="Clear search"
                    onclick={() => (featureSearch = "")}>×</button>
                {/if}
              </label>
              {#if searchOpen && featureSearch.trim()}
                <div class="map-search-results" role="listbox" aria-label="Map search results">
                  {#if mapSearchResults.length === 0}
                    <p class="search-empty">No matching features.</p>
                  {:else}
                    {#each mapSearchResults as result (result.featureId)}
                      <button
                        type="button"
                        role="option"
                        aria-selected={selectedFeatureIds.includes(result.featureId)}
                        onclick={() => focusSearchResult(result.featureId, result.layerId)}>
                        <span class="result-name">{result.name}</span>
                        <span class="result-meta"
                          >{result.semanticType} · {result.layerName}{result.linkedEntityName
                            ? ` · ${result.linkedEntityName}`
                            : ""}</span>
                      </button>
                    {/each}
                  {/if}
                </div>
              {/if}
            </div>
          </div>

          {#if studioSupported}
            <div class="studio-callout">
              <button
                type="button"
                class="studio-open-btn"
                class:active={studioOpen}
                aria-pressed={studioOpen}
                disabled={dirty}
                onclick={() => {
                  if (!dirty) studioOpen = !studioOpen;
                }}>
                <span class="studio-open-label">Atlas Studio</span>
                <small>{studioOpen ? "Close" : dirty ? "Save to open" : "Open"}</small>
              </button>
              {#if dirty}<span class="studio-hint">Save authored changes before opening Atlas.</span>{/if}
            </div>
          {/if}

          <div class="map-panel-body">
            <!-- Layers -->
            <details class="map-section-group" open={!layersCollapsed}>
              <summary
                onclick={(event) => {
                  event.preventDefault();
                  layersCollapsed = !layersCollapsed;
                }}>
                <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
                <strong>Layers</strong>
                <span class="section-count">{listedLayers.length}</span>
              </summary>
              <div class="section-body">
                {#if physicalMap}
                  <p class="section-note">
                    Hazard layers show relative generated rates; they are not real-world predictions.
                  </p>
                {/if}
                {#if listedLayers.length === 0}
                  <p class="empty-note">
                    Add a vector layer to draw points, lines, and regions. Base geography stays read-only.
                  </p>
                {/if}
                <div class="quick-add-row">
                  <button
                    type="button"
                    class="quiet-button small"
                    disabled={busy || layers.filter(isVectorLayer).length >= VECTOR_MAX_LAYERS}
                    onclick={() => addLayer()}>
                    Add vector
                  </button>
                  <button
                    type="button"
                    class="quiet-button small"
                    disabled={physicalMap || busy || rasterLayerCount >= IMAGE_MAX_RASTER_LAYERS}
                    onclick={() => void addRasterLayer()}>
                    Add raster layer
                  </button>
                  <button
                    type="button"
                    class="ghost-btn small"
                    class:active={snapConfigOpen}
                    aria-pressed={snapConfigOpen}
                    onclick={() => (snapConfigOpen = !snapConfigOpen)}
                    title="Snap settings">
                    <Settings2 size={13} strokeWidth={1.8} aria-hidden="true" /> Snap
                  </button>
                </div>
                {#if snapConfigOpen}
                  <div class="snap-config" aria-label="Snap settings">
                    <label
                      ><input type="checkbox" bind:checked={snapVertex} onchange={syncSnapToEditor} /> Vertex</label>
                    <label><input type="checkbox" bind:checked={snapEdge} onchange={syncSnapToEditor} /> Edge</label>
                    <label
                      ><input type="checkbox" bind:checked={snapIntersection} onchange={syncSnapToEditor} /> Intersection</label>
                    <small>Locked layers can opt into snap targets from the layer row.</small>
                  </div>
                {/if}

                <div class="layer-list" role="list">
                  {#each listedLayers as layer (layer.id)}
                    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                    <div
                      class="layer-card"
                      class:active={layer.id === activeLayerId}
                      class:locked={layer.locked}
                      class:immutable={immutablePhysicalLayerIds.has(layer.id)}
                      role="listitem"
                      draggable={!immutablePhysicalLayerIds.has(layer.id)}
                      ondragstart={() => (draggingLayerId = layer.id)}
                      ondragover={(event) => event.preventDefault()}
                      ondrop={(event) => {
                        event.preventDefault();
                        if (draggingLayerId) dropLayer(draggingLayerId, layer.id);
                        draggingLayerId = null;
                      }}>
                      <div class="layer-card-main">
                        <span class="drag-handle" aria-hidden="true" title="Drag to reorder">
                          <GripVertical size={12} strokeWidth={1.8} />
                        </span>
                        <span class="layer-kind-icon" aria-hidden="true">
                          {#if isRasterLayer(layer)}<ImageIcon size={13} strokeWidth={1.8} />{:else}<Layers
                              size={13}
                              strokeWidth={1.8} />{/if}
                        </span>
                        <button
                          class="layer-name"
                          type="button"
                          aria-pressed={layer.id === activeLayerId}
                          onkeydown={(event) => {
                            if (event.target === event.currentTarget) onLayerKey(event, layer);
                          }}
                          onclick={() => switchLayer(layer.id)}>
                          {#if renamingId === layer.id}
                            <input
                              value={layer.name}
                              aria-label="Layer name"
                              use:focusOnMount
                              onblur={(event) => void renameLayer(layer, event.currentTarget.value)}
                              onkeydown={(event) => {
                                if (event.key === "Enter") void renameLayer(layer, event.currentTarget.value);
                                if (event.key === "Escape") renamingId = null;
                              }} />
                          {:else}
                            <span class="layer-name-text">{layer.name}</span>
                            <span class="layer-meta"
                              >{layer.kind} · {isRasterLayer(layer)
                                ? "raster"
                                : `${featureCountForLayer(draft, layer.id)} feats`}{layer.locked
                                ? " · locked"
                                : ""}{!layer.defaultVisible ? " · hidden" : ""}</span>
                          {/if}
                        </button>
                        <div class="layer-card-actions">
                          <button
                            type="button"
                            class="mini-icon"
                            class:off={!layer.defaultVisible}
                            aria-pressed={layer.defaultVisible}
                            aria-label={layer.defaultVisible ? `Hide ${layer.name}` : `Show ${layer.name}`}
                            title={layer.defaultVisible ? "Hide" : "Show"}
                            onclick={() => void toggleVisible(layer)}
                            >{#if layer.defaultVisible}<Eye size={14} strokeWidth={1.8} />{:else}<EyeOff
                                size={14}
                                strokeWidth={1.8} />{/if}</button>
                          {#if !immutablePhysicalLayerIds.has(layer.id)}
                            <button
                              type="button"
                              class="mini-icon"
                              class:off={!layer.locked}
                              aria-pressed={layer.locked}
                              aria-label={layer.locked ? `Unlock ${layer.name}` : `Lock ${layer.name}`}
                              title={layer.locked ? "Unlock" : "Lock"}
                              onclick={() => void toggleLock(layer)}
                              >{#if layer.locked}<Lock size={14} strokeWidth={1.8} />{:else}<LockOpen
                                  size={14}
                                  strokeWidth={1.8} />{/if}</button>
                          {/if}
                          {#if physicalMap && isVectorLayer(layer) && isPhysicalDerivedLayerId(layer.id)}
                            <button
                              type="button"
                              class="mini-icon"
                              aria-label={`Detach ${layer.name} for editing`}
                              title="Detach for editing"
                              disabled={busy ||
                                epochBusy ||
                                physicalFeaturesForLayer(derivedPhysical, layer.id).length === 0}
                              onclick={() => openDetachDialog(layer)}><Scissors size={14} strokeWidth={1.8} /></button>
                          {/if}
                          <button
                            type="button"
                            class="mini-icon customize-btn"
                            class:active={openCustomizeLayerId === layer.id}
                            aria-label={`Customize ${layer.name}`}
                            aria-expanded={openCustomizeLayerId === layer.id}
                            title="Customize"
                            onclick={(event) => {
                              event.stopPropagation();
                              openCustomizeLayerId = openCustomizeLayerId === layer.id ? null : layer.id;
                            }}><SlidersHorizontal size={14} strokeWidth={1.8} /></button>
                        </div>
                      </div>

                      {#if !immutablePhysicalLayerIds.has(layer.id)}
                        <div class="layer-card-toolbar">
                          <button
                            type="button"
                            class="toolbar-btn"
                            aria-label={`Rename ${layer.name}`}
                            title="Rename"
                            onclick={() => (renamingId = layer.id)}><Pencil size={12} strokeWidth={1.8} /></button>
                          <button
                            type="button"
                            class="toolbar-btn"
                            aria-label={`Move ${layer.name} up`}
                            title="Move up"
                            disabled={busy}
                            onclick={() => void moveLayer(layer, -1)}><ChevronUp size={12} strokeWidth={1.8} /></button>
                          <button
                            type="button"
                            class="toolbar-btn"
                            aria-label={`Move ${layer.name} down`}
                            title="Move down"
                            disabled={busy}
                            onclick={() => void moveLayer(layer, 1)}
                            ><ChevronDown size={12} strokeWidth={1.8} /></button>
                          <button
                            type="button"
                            class="toolbar-btn"
                            aria-label={`Duplicate ${layer.name}`}
                            title="Duplicate"
                            disabled={busy ||
                              (isRasterLayer(layer)
                                ? rasterLayerCount >= IMAGE_MAX_RASTER_LAYERS
                                : layers.filter(isVectorLayer).length >= VECTOR_MAX_LAYERS)}
                            onclick={() => void duplicateLayer(layer)}><Copy size={12} strokeWidth={1.8} /></button>
                          <button
                            type="button"
                            class="toolbar-btn danger"
                            aria-label={`Remove ${layer.name}`}
                            title="Remove"
                            onclick={() => void removeLayer(layer)}><Trash2 size={12} strokeWidth={1.8} /></button>
                        </div>
                      {/if}

                      {#if openCustomizeLayerId === layer.id}
                        <div class="layer-customize">
                          <div class="opacity-group">
                            <span class="customize-label">Opacity</span>
                            <label class="detail-range">
                              <span>Layer</span>
                              <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.05"
                                value={layer.opacity}
                                aria-label={`${layer.name} layer opacity`}
                                oninput={(event) => void setLayerOpacity(layer, Number(event.currentTarget.value))} />
                              <em>{Math.round(layer.opacity * 100)}%</em>
                            </label>
                            <label class="detail-range">
                              <span>Fill</span>
                              <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.05"
                                value={layer.style.fillOpacity}
                                aria-label={`${layer.name} fill opacity`}
                                oninput={(event) =>
                                  void updateStyle(layer, { fillOpacity: Number(event.currentTarget.value) })} />
                              <em>{Math.round(layer.style.fillOpacity * 100)}%</em>
                            </label>
                            <label class="detail-range">
                              <span>Stroke</span>
                              <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.05"
                                value={layer.style.strokeOpacity ?? 1}
                                aria-label={`${layer.name} stroke opacity`}
                                oninput={(event) =>
                                  void updateStyle(layer, { strokeOpacity: Number(event.currentTarget.value) })} />
                              <em>{Math.round((layer.style.strokeOpacity ?? 1) * 100)}%</em>
                            </label>
                          </div>
                          {#if isVectorLayer(layer)}
                            <details class="layer-advanced" open>
                              <summary>
                                <span>Advanced style</span>
                                <ChevronDown size={12} strokeWidth={1.8} aria-hidden="true" />
                              </summary>
                              <div class="detail-grid">
                                <label
                                  ><span>Fill</span><input
                                    type="color"
                                    value={layer.style.fill}
                                    aria-label={`${layer.name} fill`}
                                    onchange={(event) =>
                                      void updateStyle(layer, { fill: event.currentTarget.value })} /></label>
                                <label
                                  ><span>Stroke</span><input
                                    type="color"
                                    value={layer.style.stroke}
                                    aria-label={`${layer.name} stroke`}
                                    onchange={(event) =>
                                      void updateStyle(layer, { stroke: event.currentTarget.value })} /></label>
                                <label
                                  ><span>Stroke width</span><input
                                    type="number"
                                    min="0"
                                    max="32"
                                    step="0.25"
                                    value={layer.style.strokeWidth}
                                    aria-label={`${layer.name} stroke width`}
                                    onchange={(event) =>
                                      void updateStyle(layer, {
                                        strokeWidth: Number(event.currentTarget.value),
                                      })} /></label>
                                <label
                                  ><span>Dash</span><input
                                    type="text"
                                    value={(layer.style.strokeDash ?? []).join(", ")}
                                    placeholder="solid"
                                    aria-label={`${layer.name} stroke dash`}
                                    onchange={(event) => {
                                      const values = event.currentTarget.value
                                        .split(",")
                                        .map((v) => Number(v.trim()))
                                        .filter(Number.isFinite);
                                      void updateStyle(layer, { strokeDash: values.slice(0, 16) });
                                    }} /></label>
                                <label
                                  ><span>Point radius</span><input
                                    type="number"
                                    min="1"
                                    max="64"
                                    step="1"
                                    value={layer.style.pointRadius}
                                    aria-label={`${layer.name} point radius`}
                                    onchange={(event) =>
                                      void updateStyle(layer, {
                                        pointRadius: Number(event.currentTarget.value),
                                      })} /></label>
                                <label
                                  ><span>Marker</span>
                                  <select
                                    value={layer.style.icon ?? "circle"}
                                    aria-label={`${layer.name} marker icon`}
                                    onchange={(event) =>
                                      void updateStyle(layer, {
                                        icon: event.currentTarget.value === "circle" ? null : event.currentTarget.value,
                                      })}>
                                    <option value="circle">Circle</option><option value="square">Square</option><option
                                      value="diamond">Diamond</option
                                    ><option value="triangle">Triangle</option><option value="star">Star</option>
                                  </select>
                                </label>
                                <label
                                  ><span>Marker size</span><input
                                    type="number"
                                    min="4"
                                    max="256"
                                    step="1"
                                    value={layer.style.iconSize ?? 20}
                                    aria-label={`${layer.name} marker size`}
                                    onchange={(event) =>
                                      void updateStyle(layer, {
                                        iconSize: Number(event.currentTarget.value),
                                      })} /></label>
                                <label
                                  ><span>Label size</span><input
                                    type="number"
                                    min="6"
                                    max="96"
                                    step="1"
                                    value={layer.style.label?.size ?? 12}
                                    aria-label={`${layer.name} label size`}
                                    onchange={(event) =>
                                      void updateStyle(layer, {
                                        label: {
                                          ...(layer.style.label ?? DEFAULT_VECTOR_LAYER_STYLE.label!),
                                          size: Number(event.currentTarget.value),
                                        },
                                      })} /></label>
                              </div>
                            </details>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              </div>
            </details>

            {#if !physicalMap}
              <details class="map-section-group" open={!rastersCollapsed}>
                <summary
                  onclick={(event) => {
                    event.preventDefault();
                    rastersCollapsed = !rastersCollapsed;
                  }}>
                  <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
                  <strong>Rasters</strong>
                  <span class="section-count">{listedRasters.length}</span>
                </summary>
                <div class="section-body">
                  <p class="section-note">{unitsLabel}</p>
                  <div class="quick-add-row">
                    <button
                      type="button"
                      class="quiet-button small"
                      disabled={busy || listedRasters.length >= IMAGE_MAX_RASTER_LAYERS}
                      onclick={() => void addRaster()}>Add raster</button>
                    <button type="button" class="quiet-button small" onclick={() => editor?.fitExtent()}>Fit</button>
                    <button type="button" class="quiet-button small" onclick={() => editor?.actualPixels()}
                      >Actual pixels</button>
                  </div>
                  <label class="calibrate-field">
                    <span>Metres per unit</span>
                    <div class="calibrate-row">
                      <input
                        type="number"
                        min="0"
                        step="any"
                        bind:value={calibrateMetres}
                        placeholder={coordinateSpace.kind === "image" ? "pixels until calibrated" : "optional"}
                        aria-label="Metres per unit" />
                      <button type="button" class="quiet-button small" onclick={() => applyCalibration()}
                        >Calibrate</button>
                    </div>
                  </label>
                  {#if listedRasters.length === 0}
                    <p class="empty-note">
                      No rasters yet. Add PNG, JPEG, or SVG overlays. Image maps open at exact pixel extent.
                    </p>
                  {/if}
                  <div class="raster-list" role="list">
                    {#each listedRasters as raster (raster.id)}
                      <div class="raster-card" role="listitem">
                        <div class="raster-card-main">
                          <span class="raster-icon"><ImageIcon size={13} strokeWidth={1.8} aria-hidden="true" /></span>
                          <span class="raster-name">{raster.name}</span>
                          <div class="raster-actions">
                            <button
                              type="button"
                              class="mini-icon"
                              aria-pressed={raster.visible}
                              aria-label={raster.visible ? `Hide ${raster.name}` : `Show ${raster.name}`}
                              onclick={() =>
                                dispatchCommand(
                                  setBackgroundVisibilityCommand(raster.id, !raster.visible, raster.visible),
                                )}
                              >{#if raster.visible}<Eye size={14} strokeWidth={1.8} />{:else}<EyeOff
                                  size={14}
                                  strokeWidth={1.8} />{/if}</button>
                            <button
                              type="button"
                              class="mini-icon"
                              aria-label={`Replace ${raster.name}`}
                              onclick={() => void replaceRaster(raster)}
                              ><ImageIcon size={14} strokeWidth={1.8} /></button>
                            <button
                              type="button"
                              class="mini-icon danger"
                              aria-label={`Remove ${raster.name}`}
                              onclick={() => removeRaster(raster)}><Trash2 size={14} strokeWidth={1.8} /></button>
                            <button
                              type="button"
                              class="mini-icon more-btn"
                              class:active={openRasterMenuId === raster.id}
                              aria-label="More actions"
                              aria-expanded={openRasterMenuId === raster.id}
                              aria-haspopup="menu"
                              title="More"
                              onclick={(event) => {
                                event.stopPropagation();
                                openRasterMenuId = openRasterMenuId === raster.id ? null : raster.id;
                              }}><Ellipsis size={14} strokeWidth={1.8} /></button>
                          </div>
                        </div>
                        {#if openRasterMenuId === raster.id}
                          <div class="layer-menu" role="menu" tabindex="-1" aria-label={`Actions for ${raster.name}`}>
                            <button
                              type="button"
                              role="menuitem"
                              class="layer-menu-item"
                              onclick={() => {
                                openRasterMenuId = null;
                                moveRaster(raster, -1);
                              }}><ChevronUp size={12} strokeWidth={1.8} /> Move up</button>
                            <button
                              type="button"
                              role="menuitem"
                              class="layer-menu-item"
                              onclick={() => {
                                openRasterMenuId = null;
                                moveRaster(raster, 1);
                              }}><ChevronDown size={12} strokeWidth={1.8} /> Move down</button>
                            <div class="layer-menu-separator"></div>
                            <span class="layer-menu-meta"
                              >{raster.visible ? "Visible" : "Hidden"} · {Math.round(raster.opacity * 100)}%</span>
                          </div>
                        {/if}
                        <label class="detail-range small">
                          <span>Opacity</span>
                          <input
                            type="range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={raster.opacity}
                            aria-label={`${raster.name} opacity`}
                            oninput={(event) =>
                              dispatchCommand(
                                setBackgroundOpacityCommand(
                                  raster.id,
                                  Number(event.currentTarget.value),
                                  raster.opacity,
                                ),
                              )} />
                          <em>{Math.round(raster.opacity * 100)}%</em>
                        </label>
                      </div>
                    {/each}
                  </div>
                </div>
              </details>
            {/if}

            {#if physicalMap}
              <details class="map-section-group" open={!historyCollapsed}>
                <summary
                  onclick={(event) => {
                    event.preventDefault();
                    historyCollapsed = !historyCollapsed;
                  }}>
                  <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
                  <strong>Natural history</strong>
                </summary>
                <div class="section-body">
                  <div class="event-control" aria-label="Materialize natural history">
                    <label
                      ><span>Event</span>
                      <select bind:value={eventKind} disabled={eventBusy || busy}>
                        <option value="earthquake">Earthquake</option>
                        <option value="eruption">Eruption</option>
                      </select>
                    </label>
                    <div class="event-grid">
                      <label
                        ><span>From (years)</span><input
                          type="number"
                          min="-100000"
                          max="100000"
                          step="1"
                          bind:value={eventStartYears}
                          disabled={eventBusy || busy} /></label>
                      <label
                        ><span>To (years)</span><input
                          type="number"
                          min="-100000"
                          max="100000"
                          step="1"
                          bind:value={eventEndYears}
                          disabled={eventBusy || busy} /></label>
                    </div>
                    <div class="event-grid">
                      <label
                        ><span>Max events</span><input
                          type="number"
                          min="1"
                          max="128"
                          step="1"
                          bind:value={eventMaxEvents}
                          disabled={eventBusy || busy} /></label>
                      <label
                        ><span>Hazard seed</span><input
                          type="number"
                          min="0"
                          step="1"
                          bind:value={eventHazardSeed}
                          disabled={eventBusy || busy} /></label>
                    </div>
                    <button
                      type="button"
                      class="primary-button small"
                      disabled={eventBusy || busy}
                      onclick={() => void materializePhysicalEvents()}
                      >{eventBusy ? "Committing…" : "Commit events"}</button>
                    <p class="field-hint">
                      Creates revisioned entities and map links; generated hazards remain read-only and are not
                      predictions.
                    </p>
                    {#if eventNotice}<p class="field-hint" role="status">{eventNotice}</p>{/if}
                  </div>
                </div>
              </details>
            {/if}

            {#if selectedOpFeatures.length > 0}
              <details class="map-section-group" open>
                <summary>
                  <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
                  <strong>Geometry</strong>
                  <span class="section-count">{selectedOpFeatures.length}</span>
                </summary>
                <div class="section-body">
                  <div class="geometry-ops" aria-label="Geometry operations">
                    {#if geometryPreview}
                      <p class="section-note">Preview: {geometryPreview.label}. Commit or cancel to finish.</p>
                      <div class="quick-add-row">
                        <button type="button" class="primary-button small" onclick={() => commitGeometryPreview()}
                          >Apply</button>
                        <button type="button" class="quiet-button small" onclick={() => cancelGeometryPreview()}
                          >Cancel</button>
                      </div>
                    {:else}
                      <div class="quick-add-row">
                        <button
                          type="button"
                          class="quiet-button small"
                          disabled={!canRunOperation("union", selectedOpFeatures)}
                          onclick={() => startGeometryOperation("union")}>Union</button>
                        <button
                          type="button"
                          class="quiet-button small"
                          disabled={!canRunOperation("difference", selectedOpFeatures)}
                          onclick={() => startGeometryOperation("difference")}>Diff</button>
                        <button
                          type="button"
                          class="quiet-button small"
                          disabled={!canRunOperation("intersection", selectedOpFeatures)}
                          onclick={() => startGeometryOperation("intersection")}>Intersect</button>
                      </div>
                      <div class="quick-add-row">
                        <button
                          type="button"
                          class="quiet-button small"
                          disabled={!canRunOperation("split", selectedOpFeatures)}
                          onclick={() => startGeometryOperation("split")}
                          ><Scissors size={12} strokeWidth={1.8} /> Split</button>
                        <label class="inline-field"
                          ><span>Buffer</span><input
                            type="number"
                            min="0"
                            step="any"
                            bind:value={bufferDistance}
                            aria-label="Buffer distance" /></label>
                        <button
                          type="button"
                          class="quiet-button small"
                          disabled={!canRunOperation("buffer", selectedOpFeatures)}
                          onclick={() => startGeometryOperation("buffer")}>Run</button>
                      </div>
                      <div class="quick-add-row">
                        <label class="inline-field"
                          ><span>Simplify</span><input
                            type="number"
                            min="0"
                            step="any"
                            bind:value={simplifyTolerance}
                            aria-label="Simplify tolerance" /></label>
                        <button
                          type="button"
                          class="quiet-button small"
                          disabled={!canRunOperation("simplify", selectedOpFeatures)}
                          onclick={() => startGeometryOperation("simplify")}>Run</button>
                      </div>
                      {#if operationNotice}<p class="field-hint" role="status">{operationNotice}</p>{/if}
                    {/if}
                  </div>
                </div>
              </details>
            {/if}

            {#if selectedFeatures.length > 0}
              {@const primary = selectedFeatures[0]}
              {@const primaryLabel = primary.properties.daena.label ?? defaultFeatureLabel(primary)}
              {@const primaryStyle = primary.properties.daena.style ?? {}}
              {@const linkedEntity = selectedFeatures.length === 1 ? featureLinks.get(primary.id) : null}
              <details class="map-section-group" open>
                <summary>
                  <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
                  <strong
                    >{selectedFeatures.length === 1
                      ? "Selected feature"
                      : `${selectedFeatures.length} features`}</strong>
                </summary>
                <div class="section-body">
                  <div class="selection-head">
                    <strong class="selection-title"
                      >{selectedFeatures.length === 1
                        ? (featureName(primary) ?? "Untitled feature")
                        : `${selectedFeatures.length} features selected`}</strong>
                    <p class="section-note">
                      {primary.geometry.type} · {selectedFeatures.reduce((sum, f) => sum + featureVertexCount(f), 0)} vertices{#if selectedFeatures.length === 1}
                        · {featureSemanticType(primary)}{/if}
                    </p>
                  </div>

                  {#if selectedFeatures.length === 1}
                    <label class="field"
                      ><span>Name</span><input
                        value={featureName(primary) ?? ""}
                        maxlength="256"
                        aria-label="Feature name"
                        disabled={featureLayerId(primary) === "base"}
                        onchange={(event) => renameSelectedFeature(event.currentTarget.value.trim() || null)} /></label>
                  {/if}
                  <label class="field"
                    ><span>Semantic type</span>
                    <select
                      value={featureSemanticType(primary)}
                      aria-label="Feature semantic type"
                      onchange={(event) =>
                        setSelectedSemanticType(
                          event.currentTarget.value as VectorFeature["properties"]["daena"]["semanticType"],
                        )}>
                      <option value="land">Land</option><option value="lake">Lake</option><option value="region"
                        >Region</option
                      ><option value="route">Route</option><option value="marker">Marker</option><option value="custom"
                        >Custom</option>
                    </select>
                  </label>
                  <label class="field"
                    ><span>Layer</span>
                    <select
                      value={featureLayerId(primary)}
                      aria-label="Feature layer"
                      onchange={(event) => moveSelectedToLayer(event.currentTarget.value)}>
                      {#each listedLayers.filter((l) => isVectorLayer(l) && l.id !== "base" && l.defaultVisible && !l.locked) as layer}
                        <option value={layer.id}>{layer.name}</option>
                      {/each}
                    </select>
                  </label>

                  <details class="sub-section">
                    <summary>Style override</summary>
                    <div class="detail-grid compact">
                      <label
                        ><span>Fill</span><input
                          type="color"
                          value={primaryStyle.fill ?? "#8f6fd1"}
                          onchange={(event) => updateSelectedStyle({ fill: event.currentTarget.value })} /></label>
                      <label
                        ><span>Fill opacity</span>
                        <div class="range-with-value">
                          <input
                            type="range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={primaryStyle.fillOpacity ?? 0.35}
                            oninput={(event) =>
                              updateSelectedStyle({ fillOpacity: Number(event.currentTarget.value) })} /><small
                            >{Math.round((primaryStyle.fillOpacity ?? 0.35) * 100)}%</small>
                        </div>
                      </label>
                      <label
                        ><span>Stroke</span><input
                          type="color"
                          value={primaryStyle.stroke ?? "#5e4893"}
                          onchange={(event) => updateSelectedStyle({ stroke: event.currentTarget.value })} /></label>
                      <label
                        ><span>Stroke opacity</span>
                        <div class="range-with-value">
                          <input
                            type="range"
                            min="0"
                            max="1"
                            step="0.05"
                            value={primaryStyle.strokeOpacity ?? 1}
                            oninput={(event) =>
                              updateSelectedStyle({ strokeOpacity: Number(event.currentTarget.value) })} /><small
                            >{Math.round((primaryStyle.strokeOpacity ?? 1) * 100)}%</small>
                        </div>
                      </label>
                      <label
                        ><span>Width</span><input
                          type="number"
                          min="0"
                          max="32"
                          step="0.25"
                          value={primaryStyle.strokeWidth ?? 1.5}
                          onchange={(event) =>
                            updateSelectedStyle({ strokeWidth: Number(event.currentTarget.value) })} /></label>
                      <label
                        ><span>Point radius</span><input
                          type="number"
                          min="1"
                          max="64"
                          step="1"
                          value={primaryStyle.pointRadius ?? 5}
                          onchange={(event) =>
                            updateSelectedStyle({ pointRadius: Number(event.currentTarget.value) })} /></label>
                      <label
                        ><span>Marker</span>
                        <select
                          value={primaryStyle.icon ?? "circle"}
                          onchange={(event) =>
                            updateSelectedStyle({
                              icon: event.currentTarget.value === "circle" ? null : event.currentTarget.value,
                            })}>
                          <option value="circle">Circle</option><option value="square">Square</option><option
                            value="diamond">Diamond</option
                          ><option value="triangle">Triangle</option><option value="star">Star</option>
                        </select>
                      </label>
                    </div>
                    <button type="button" class="quiet-button small" onclick={clearSelectedOverrides}
                      >Use layer style</button>
                  </details>

                  <details class="sub-section">
                    <summary>Label</summary>
                    <div class="detail-grid compact">
                      <label
                        ><span>Source</span><select
                          value={primaryLabel.source}
                          onchange={(event) =>
                            updateSelectedLabel({ source: event.currentTarget.value as "name" | "explicit" })}
                          ><option value="name">Feature name</option><option value="explicit">Custom text</option
                          ></select
                        ></label>
                      {#if primaryLabel.source === "explicit"}
                        <label
                          ><span>Text</span><input
                            type="text"
                            maxlength="512"
                            value={primaryLabel.text ?? ""}
                            onchange={(event) => updateSelectedLabel({ text: event.currentTarget.value })} /></label>
                      {/if}
                      <label
                        ><span>Size</span><input
                          type="number"
                          min="6"
                          max="96"
                          value={primaryLabel.size}
                          onchange={(event) =>
                            updateSelectedLabel({ size: Number(event.currentTarget.value) })} /></label>
                      <label
                        ><span>Color</span><input
                          type="color"
                          value={primaryLabel.color}
                          onchange={(event) => updateSelectedLabel({ color: event.currentTarget.value })} /></label>
                      <label
                        ><span>Halo</span><input
                          type="color"
                          value={primaryLabel.haloColor}
                          onchange={(event) => updateSelectedLabel({ haloColor: event.currentTarget.value })} /></label>
                      <label
                        ><span>Halo width</span><input
                          type="number"
                          min="0"
                          max="16"
                          step="0.5"
                          value={primaryLabel.haloWidth}
                          onchange={(event) =>
                            updateSelectedLabel({ haloWidth: Number(event.currentTarget.value) })} /></label>
                      <label
                        ><span>Placement</span><select
                          value={primaryLabel.placement}
                          onchange={(event) =>
                            updateSelectedLabel({ placement: event.currentTarget.value as MapLabelV2["placement"] })}
                          ><option value="point">Point</option><option value="line">Line</option><option
                            value="interior">Interior</option
                          ></select
                        ></label>
                      <label
                        ><span>Rotation</span><input
                          type="number"
                          min="-360"
                          max="360"
                          value={primaryLabel.rotation}
                          onchange={(event) =>
                            updateSelectedLabel({ rotation: Number(event.currentTarget.value) })} /></label>
                      <label
                        ><span>Min zoom</span><input
                          type="number"
                          min="0"
                          max="24"
                          placeholder="none"
                          value={primaryLabel.minZoom ?? ""}
                          onchange={(event) =>
                            updateSelectedLabel({
                              minZoom: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                            })} /></label>
                      <label
                        ><span>Max zoom</span><input
                          type="number"
                          min="0"
                          max="24"
                          placeholder="none"
                          value={primaryLabel.maxZoom ?? ""}
                          onchange={(event) =>
                            updateSelectedLabel({
                              maxZoom: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                            })} /></label>
                    </div>
                  </details>

                  {#if selectedFeatures.length === 1}
                    <details class="sub-section">
                      <summary>Custom properties</summary>
                      {#each Object.entries(primary.properties.daena.custom) as [key, value] (key)}
                        <div class="property-row">
                          <span><strong>{key}</strong> {String(value ?? "null")}</span>
                          <button
                            type="button"
                            class="mini-icon danger"
                            aria-label={`Remove ${key}`}
                            onclick={() => removeCustomProperty(key)}><Trash2 size={12} strokeWidth={1.8} /></button>
                        </div>
                      {/each}
                      <div class="property-editor">
                        <input placeholder="Key" aria-label="Custom property key" bind:value={customKey} />
                        <input placeholder="Value" aria-label="Custom property value" bind:value={customValue} />
                        <button type="button" class="quiet-button small" onclick={addCustomProperty}>Add</button>
                      </div>
                    </details>
                    <div class="linked-entity-card">
                      <strong>Linked entity</strong>
                      {#if linkedEntity}
                        <span class="linked-name">{linkedEntity.label || "Linked entry"}</span>
                        <div class="quick-add-row">
                          <button
                            type="button"
                            class="quiet-button small"
                            onclick={() => onopen?.(linkedEntity.entityId)}>Open</button>
                          <button
                            type="button"
                            class="quiet-button small"
                            onclick={() => openLinkPanel(pickAnchorFor(primary))}>Manage link</button>
                        </div>
                      {:else}
                        <span class="section-note"
                          >No entity linked. Deleted targets remain visible as unresolved links.</span>
                        <button
                          type="button"
                          class="quiet-button small"
                          onclick={() => openLinkPanel(pickAnchorFor(primary))}>Link entity</button>
                      {/if}
                    </div>
                  {/if}

                  <div class="quick-add-row">
                    <button type="button" class="quiet-button small" onclick={() => duplicateSelectedFeatures()}
                      >Duplicate</button>
                    <button
                      type="button"
                      class="quiet-button small"
                      onclick={() => editor?.fitSelection(selectedFeatureIds)}>Fit</button>
                    <button type="button" class="quiet-button small danger" onclick={() => deleteSelectedFeatures()}
                      >Delete</button>
                  </div>
                  <p class="field-hint">Shift-click adds to selection. Alt-click a vertex to delete it.</p>
                </div>
              </details>
            {/if}

            {#if !physicalMap}
              <p class="section-note foot-note">
                Base geography is read-only. Point, line, polygon, rectangle, and freehand edits save through the
                canonical GeoJSON source. Delete removes the selected feature.
              </p>
            {/if}
          </div>
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
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <div
            class="canvas"
            class:picking={picking || linkArming}
            tabindex="0"
            role="application"
            aria-label="Physical world map">
            <div class="map-host" bind:this={host}></div>
            {#if editor}
              <MapViewControls
                zoom={defaultView.zoom}
                min={0}
                max={viewMaxZoom}
                onzoom={(zoom) => editor?.setZoom(zoom)}
                onpan={(x, y) => editor?.panCardinal(x > 0 ? 1 : x < 0 ? -1 : 0, y > 0 ? 1 : y < 0 ? -1 : 0)} />
            {/if}
            <div class="epoch-control" aria-label="World epoch">
              <input
                id="physical-epoch"
                type="range"
                min={EPOCH_MIN}
                max={EPOCH_MAX}
                step={EPOCH_STEP}
                value={epochOffsetYears}
                aria-label="Epoch offset"
                disabled={busy || epochBusy || dirty}
                oninput={(event) => commitEpoch(clampEpoch(Number(event.currentTarget.value), EPOCH_STEP))} />
              <input
                class="epoch-year"
                type="text"
                inputmode="numeric"
                autocomplete="off"
                spellcheck="false"
                value={epochYearsAbs.toLocaleString("en-US")}
                aria-label="Years from epoch"
                disabled={busy || epochBusy || dirty}
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
            {#if dirty}<p class="epoch-dirty-hint">
                Save or undo authored changes before changing the physical epoch.
              </p>{/if}
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
              }}
              {onopen} />
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
              }}
              {onopen} />
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
  background: var(--canvas, #f7f6f2);
  color: var(--ink);
}

/* keep header-actions for topbar */
.header-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}

.editor-body {
  display: grid;
  min-height: 0;
  flex: 1 1 auto;
  grid-template-columns: var(--sidebar-width, 300px) 6px minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr);
  background: var(--canvas, #f7f6f2);
}
.editor-body.studio {
  grid-template-columns: minmax(0, 1fr);
}
.sidebar-resizer {
  width: 6px;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: var(--line, #e4e1d8);
  cursor: col-resize;
}
.sidebar-resizer:hover,
.sidebar-resizer:focus-visible {
  background: var(--line-strong, #d9cdbd);
}

/* ───────── Map Layers Panel ───────── */
.map-layers-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  background: var(--surface, #fffefa);
  border-right: 1px solid var(--line, #e4e1d8);
  color: var(--ink);
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}

/* panel header - like CollectionPane */
.map-panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 14px 12px;
  border-bottom: 1px solid var(--line, #e4e1d8);
  background: var(--surface, #fffefa);
}
.map-panel-head-copy {
  display: grid;
  gap: 3px;
  min-width: 0;
}
.map-panel-head-copy .panel-kicker {
  color: var(--accent, #b4773f);
  font-weight: 800;
  font-size: 10px;
  line-height: 1;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.14em;
  text-transform: uppercase;
}
.map-panel-head-copy strong {
  color: var(--ink);
  font-weight: 650;
  font-size: 12px;
  line-height: 1.2;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.map-panel-subtitle {
  color: var(--ink-faint, #aaa79d);
  font-size: 11px;
  line-height: 1.3;
}
.panel-icon-btn {
  display: grid;
  width: 32px;
  height: 32px;
  place-items: center;
  flex: 0 0 32px;
  padding: 0;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--surface, #fffefa);
  color: var(--ink-soft, #77766d);
  cursor: pointer;
}
.panel-icon-btn:hover:not(:disabled) {
  border-color: var(--line-strong, #d9cdbd);
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink);
}
.panel-icon-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* search */
.map-search-wrap {
  padding: 10px 12px 8px;
  border-bottom: 1px solid var(--line, #e4e1d8);
  background: var(--surface, #fffefa);
}
.map-search {
  position: relative;
}
.map-search > label {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 9px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 9px;
  background: var(--surface, #fffefa);
  color: var(--ink-faint);
  box-shadow: 0 1px 2px rgba(34, 40, 34, 0.04);
  transition:
    border-color 0.15s,
    box-shadow 0.15s;
}
.map-search:focus-within > label {
  border-color: var(--line-strong, #d9cdbd);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-soft, #c99965) 18%, transparent);
}
.map-search input {
  min-width: 0;
  width: 100%;
  padding: 9px 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.map-search input::placeholder {
  color: var(--ink-faint);
}
.search-clear {
  display: grid;
  width: 20px;
  height: 20px;
  place-items: center;
  flex: 0 0 20px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-soft);
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
}
.search-clear:hover {
  background: var(--line, #e4e1d8);
  color: var(--ink);
}
.map-search-results {
  position: absolute;
  z-index: 10;
  top: calc(100% + 6px);
  left: 0;
  right: 0;
  display: grid;
  gap: 2px;
  max-height: 260px;
  overflow: auto;
  padding: 6px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 10px;
  background: var(--surface, #fffefa);
  box-shadow: var(--shadow-md, 0 8px 24px rgba(38, 42, 33, 0.12));
}
.map-search-results button {
  display: grid;
  gap: 2px;
  padding: 8px 9px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.map-search-results button:hover,
.map-search-results button[aria-selected="true"] {
  background: var(--surface-muted, #f4f2ec);
}
.result-name {
  color: var(--ink);
  font-weight: 600;
  font-size: 11px;
  line-height: 1.2;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.result-meta {
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.search-empty {
  margin: 0;
  padding: 10px 8px;
  color: var(--ink-faint);
  font-size: 11px;
  text-align: center;
}

/* studio callout */
.studio-callout {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--line, #e4e1d8);
  background: var(--surface-subtle, #f7f3ec);
}
.studio-open-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--surface, #fffefa);
  color: var(--ink-soft);
  font-weight: 650;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
}
.studio-open-btn:hover:not(:disabled) {
  border-color: var(--line-strong);
  background: var(--surface-muted);
  color: var(--ink);
}
.studio-open-btn.active {
  border-color: var(--accent-soft, #c99965);
  background: var(--accent-bg, #f2e4d2);
  color: var(--accent, #b4773f);
}
.studio-open-btn small {
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.studio-open-btn.active small {
  background: var(--surface);
  color: var(--accent);
}
.studio-hint {
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1.3;
}

/* body */
.map-panel-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 0;
  padding-bottom: 12px;
  background: var(--surface, #fffefa);
}

/* section groups - mimic InspectorSection */
.map-section-group {
  border-bottom: 1px solid var(--line, #e4e1d8);
}
.map-section-group summary {
  display: flex;
  align-items: center;
  gap: 7px;
  min-height: 38px;
  padding: 0 12px;
  list-style: none;
  cursor: pointer;
  user-select: none;
  color: var(--ink-soft);
}
.map-section-group summary::-webkit-details-marker {
  display: none;
}
.map-section-group summary:hover {
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink);
}
.map-section-group summary :global(svg) {
  flex: 0 0 14px;
  transition: transform 0.16s ease;
  color: var(--ink-faint);
}
.map-section-group[open] summary :global(svg) {
  transform: rotate(90deg);
}
.map-section-group summary strong {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 700;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--ink-soft);
}
.map-section-group summary:hover strong {
  color: var(--ink);
}
.section-count {
  min-width: 20px;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-faint);
  font-weight: 700;
  font-size: 9px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  text-align: center;
}
.map-section-group[open] .section-count {
  background: var(--accent-bg, #f2e4d2);
  color: var(--accent, #b4773f);
}
.section-body {
  padding: 10px 12px 14px;
  display: grid;
  gap: 10px;
}
.section-note {
  margin: 0;
  color: var(--ink-faint);
  font-size: 11px;
  line-height: 1.45;
}
.empty-note {
  margin: 0;
  padding: 10px 11px;
  border: 1px dashed var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--surface-subtle, #f7f3ec);
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.45;
  text-align: center;
}
.foot-note {
  margin: 8px 12px 0;
  padding-top: 10px;
  border-top: 1px solid var(--line, #e4e1d8);
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1.45;
}

/* quick add row */
.quick-add-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.quick-add-row .quiet-button.small,
.quick-add-row .ghost-btn.small,
.quick-add-row .primary-button.small {
  padding: 6px 9px;
  border-radius: 7px;
  font-weight: 650;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.quiet-button.small {
  border: 1px solid var(--line, #e4e1d8);
  background: var(--surface, #fffefa);
  color: var(--ink-soft);
  cursor: pointer;
}
.quiet-button.small:hover:not(:disabled) {
  border-color: var(--line-strong);
  background: var(--surface-muted);
  color: var(--ink);
}
.quiet-button.small:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.ghost-btn.small {
  border: 1px solid transparent;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.ghost-btn.small:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.ghost-btn.small.active {
  border-color: var(--line-strong);
  background: var(--accent-bg);
  color: var(--accent);
}
.primary-button.small {
  border: 1px solid var(--accent, #b4773f);
  background: var(--accent, #b4773f);
  color: var(--on-accent, #fffefa);
  cursor: pointer;
}
.primary-button.small:hover:not(:disabled) {
  filter: brightness(0.96);
}
.primary-button.small:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* snap config */
.snap-config {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 11px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 9px;
  background: var(--surface-subtle, #f7f3ec);
}
.snap-config label {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--ink-soft);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
}
.snap-config small {
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1.3;
}

/* layer list */
.layer-list,
.raster-list {
  display: grid;
  gap: 6px;
}
.layer-card,
.raster-card {
  display: grid;
  gap: 0;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 10px;
  background: var(--surface, #fffefa);
  overflow: hidden;
  transition:
    border-color 0.15s,
    box-shadow 0.15s;
}
.layer-card:hover {
  border-color: var(--line-strong, #d9cdbd);
  box-shadow: 0 1px 6px rgba(38, 42, 33, 0.06);
}
.layer-card.active {
  border-color: var(--accent-soft, #c99965);
  box-shadow:
    0 0 0 2px color-mix(in srgb, var(--accent-soft) 22%, transparent),
    0 2px 10px rgba(38, 42, 33, 0.07);
}
.layer-card.locked {
  opacity: 0.92;
}
.layer-card.immutable {
  background: var(--surface-muted, #f4f2ec);
  border-style: dashed;
}
.layer-card-main,
.raster-card-main {
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 36px;
  padding: 4px 6px 4px 4px;
}
.drag-handle {
  display: grid;
  place-items: center;
  width: 16px;
  height: 28px;
  flex: 0 0 16px;
  color: var(--ink-faint);
  cursor: grab;
  border-radius: 4px;
}
.drag-handle:active {
  cursor: grabbing;
}
.layer-kind-icon,
.raster-icon {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  flex: 0 0 26px;
  border-radius: 7px;
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink-soft);
}
.layer-card.active .layer-kind-icon {
  background: var(--accent-bg, #f2e4d2);
  color: var(--accent, #b4773f);
}
.layer-name {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
  padding: 2px 4px;
  border: 0;
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.layer-name-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--ink);
  font-weight: 600;
  font-size: 11px;
  line-height: 1.2;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.layer-card.active .layer-name-text {
  color: var(--ink);
}
.layer-meta {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--ink-faint);
  font-weight: 500;
  font-size: 10px;
  line-height: 1;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.layer-name input {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid var(--line-strong, #d9cdbd);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  outline: 0;
}
.layer-name input:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-soft) 18%, transparent);
}
.layer-card-actions,
.raster-actions {
  display: flex;
  align-items: center;
  gap: 3px;
  flex: 0 0 auto;
}
.mini-icon {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink-soft);
  cursor: pointer;
}
.mini-icon:hover:not(:disabled) {
  border-color: var(--line-strong);
  background: var(--surface-muted);
  color: var(--ink);
}
.mini-icon.off {
  background: var(--surface-muted);
  color: var(--ink-faint);
  border-color: transparent;
}
.mini-icon.danger {
  color: var(--danger, #a14f42);
}
.mini-icon.danger:hover:not(:disabled) {
  background: var(--danger-bg, #fdf2ef);
  border-color: var(--danger-line, #e7c4bc);
  color: var(--danger);
}
.mini-icon:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.mini-icon.more-btn.active {
  border-color: var(--line-strong, #d9cdbd);
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink);
}
.mini-icon.customize-btn.active {
  border-color: var(--accent-soft, #c99965);
  background: var(--accent-bg, #f2e4d2);
  color: var(--accent, #b4773f);
}
.layer-menu {
  display: grid;
  gap: 2px;
  padding: 6px;
  border-top: 1px solid var(--line, #e4e1d8);
  background: var(--surface-subtle, #f7f3ec);
  animation: layer-menu-in 0.12s ease;
}
@keyframes layer-menu-in {
  from {
    opacity: 0;
    transform: translateY(-2px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
.layer-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  text-align: left;
  cursor: pointer;
}
.layer-menu-item:hover:not(:disabled) {
  background: var(--surface, #fffefa);
  color: var(--ink);
}
.layer-menu-item:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.layer-menu-separator {
  height: 1px;
  margin: 2px 0;
  background: var(--line, #e4e1d8);
}
.layer-menu-meta {
  padding: 6px 8px;
  color: var(--ink-faint);
  font-weight: 500;
  font-size: 9px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  text-align: center;
}
.layer-advanced {
  margin-top: 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  overflow: hidden;
  background: var(--surface, #fffefa);
}
.layer-advanced summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 7px 9px;
  background: var(--surface-subtle, #f7f3ec);
  color: var(--ink-soft);
  font-weight: 700;
  font-size: 9px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.06em;
  text-transform: uppercase;
  cursor: pointer;
  list-style: none;
}
.layer-advanced summary::-webkit-details-marker {
  display: none;
}
.layer-advanced[open] summary {
  border-bottom: 1px solid var(--line, #e4e1d8);
}
.layer-advanced summary:hover {
  color: var(--ink);
}
.layer-advanced .detail-grid {
  padding: 8px;
}
.layer-card-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  border-top: 1px solid var(--line, #e4e1d8);
  background: var(--surface-subtle, #f7f3ec);
  overflow-x: auto;
}
.toolbar-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 5px 7px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 6px;
  background: var(--surface, #fffefa);
  color: var(--ink-soft);
  font-weight: 600;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  white-space: nowrap;
  cursor: pointer;
  flex: 0 0 auto;
}
.toolbar-btn:hover:not(:disabled) {
  border-color: var(--line-strong);
  background: var(--surface-muted);
  color: var(--ink);
}
.toolbar-btn.danger {
  color: var(--danger, #a14f42);
}
.toolbar-btn.danger:hover:not(:disabled) {
  background: var(--danger-bg);
  border-color: var(--danger-line);
}
.toolbar-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

/* layer detail */
.layer-customize {
  display: grid;
  gap: 10px;
  padding: 10px 11px 12px;
  border-top: 1px solid var(--accent-soft, #c99965);
  background: var(--accent-bg, #f2e4d2);
}
.opacity-group {
  display: grid;
  gap: 8px;
  padding-bottom: 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--accent-soft, #c99965) 40%, transparent);
}
.customize-label {
  color: var(--ink-soft);
  font-weight: 700;
  font-size: 9px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.detail-range {
  display: grid;
  grid-template-columns: 60px 1fr 36px;
  align-items: center;
  gap: 8px;
  color: var(--ink-soft);
  font-weight: 600;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.detail-range.small {
  grid-template-columns: 60px 1fr 36px;
}
.detail-range span {
  color: var(--ink-soft);
}
.detail-range em {
  color: var(--ink-faint);
  font-style: normal;
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.detail-range input[type="range"] {
  width: 100%;
  accent-color: var(--accent, #b4773f);
}
.detail-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.detail-grid.compact {
  gap: 8px;
}
.detail-grid label {
  display: grid;
  gap: 4px;
  color: var(--ink-soft);
  font-weight: 600;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.detail-grid label span {
  color: var(--ink-faint);
  font-size: 10px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.detail-grid input[type="color"] {
  width: 100%;
  height: 32px;
  padding: 2px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  cursor: pointer;
}
.detail-grid input[type="number"],
.detail-grid input[type="text"],
.detail-grid select {
  width: 100%;
  padding: 7px 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  outline: 0;
}
.detail-grid input:focus,
.detail-grid select:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-soft) 16%, transparent);
}
.range-with-value {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: 6px;
}
.range-with-value small {
  color: var(--ink-faint);
  font-weight: 600;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  font-variant-numeric: tabular-nums;
  min-width: 32px;
  text-align: right;
}

/* raster */
.raster-card-main .raster-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--ink);
  font-weight: 600;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.calibrate-field {
  display: grid;
  gap: 6px;
}
.calibrate-field span {
  color: var(--ink-soft);
  font-weight: 700;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.calibrate-row {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 6px;
}
.calibrate-row input {
  min-width: 0;
  padding: 7px 9px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.calibrate-row input:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-soft) 16%, transparent);
  outline: 0;
}

/* event / geometry / selection */
.event-control {
  display: grid;
  gap: 10px;
}
.event-control label {
  display: grid;
  gap: 4px;
}
.event-control label span {
  color: var(--ink-faint);
  font-weight: 700;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.event-control select,
.event-control input {
  width: 100%;
  padding: 7px 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.event-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.field-hint {
  margin: 0;
  color: var(--ink-faint);
  font-size: 10px;
  line-height: 1.4;
}
.inline-field {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--ink-soft);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.inline-field input {
  width: 84px;
  padding: 6px 7px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.geometry-ops {
  display: grid;
  gap: 8px;
}
.sub-section {
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  overflow: hidden;
  background: var(--surface, #fffefa);
}
.sub-section summary {
  padding: 8px 10px;
  background: var(--surface-subtle, #f7f3ec);
  color: var(--ink-soft);
  font-weight: 700;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.06em;
  text-transform: uppercase;
  cursor: pointer;
  list-style: none;
}
.sub-section summary::-webkit-details-marker {
  display: none;
}
.sub-section[open] summary {
  border-bottom: 1px solid var(--line, #e4e1d8);
}
.sub-section .detail-grid {
  padding: 10px;
}
.sub-section .quiet-button {
  margin: 0 10px 10px;
}
.field {
  display: grid;
  gap: 5px;
}
.field span {
  color: var(--ink-soft);
  font-weight: 700;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.field input,
.field select {
  width: 100%;
  padding: 8px 9px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.field input:focus,
.field select:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-soft) 16%, transparent);
  outline: 0;
}
.selection-head {
  display: grid;
  gap: 4px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--line, #e4e1d8);
}
.selection-title {
  color: var(--ink);
  font-weight: 600;
  font-size: 12px;
  line-height: 1.2;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  overflow-wrap: anywhere;
}
.property-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface-subtle, #f7f3ec);
  font-size: 12px;
}
.property-row strong {
  color: var(--ink);
}
.property-editor {
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: 6px;
  padding: 10px;
}
.property-editor input {
  min-width: 0;
  padding: 7px 8px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--surface, #fffefa);
  color: var(--ink);
  font-weight: 500;
  font-size: 11px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.linked-entity-card {
  display: grid;
  gap: 6px;
  padding: 10px 11px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 9px;
  background: var(--surface-subtle, #f7f3ec);
}
.linked-entity-card strong {
  color: var(--ink-soft);
  font-weight: 700;
  font-size: 10px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.linked-name {
  color: var(--ink);
  font-weight: 600;
  font-size: 12px;
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}

/* canvas / stage remain dark */
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
  background: #0d1b2a;
}
.canvas.picking {
  outline: 2px solid var(--accent-soft, #c99965);
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
.hint,
.error {
  color: var(--ink-faint);
  line-height: 1.45;
  font-size: 11px;
}
.error {
  margin: 0;
  padding: 8px 16px;
  color: var(--danger, #a14f42);
  background: var(--danger-bg, #fdf2ef);
  border-bottom: 1px solid var(--danger-line, #e7c4bc);
}
button:focus-visible {
  outline: 2px solid var(--accent-soft, #c99965);
  outline-offset: 2px;
}
@media (max-width: 900px) {
  .editor-body {
    grid-template-columns: 1fr;
    grid-template-rows: auto 6px minmax(320px, 1fr);
  }
  .map-layers-panel {
    max-height: 42vh;
    border-right: 0;
    border-bottom: 1px solid var(--line);
  }
  .sidebar-resizer {
    display: none;
  }
  .event-grid {
    grid-template-columns: 1fr;
  }
}
@media (prefers-reduced-motion: reduce) {
  .native-vector-editor,
  .native-vector-editor * {
    transition: none !important;
    animation: none !important;
  }
}
</style>
