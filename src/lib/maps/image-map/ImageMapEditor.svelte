<script lang="ts">
import { onMount, tick } from "svelte";
  import { project, type Asset, type Entity, type FieldValue, type MapPin } from "$lib/project/client";
  import {
    IMAGE_MAX_RASTER_LAYERS,
    IMAGE_MAX_UNDO_BYTES,
  } from "../../../../packages/plugin-sdk/src/maps";
  import { ImageMapStage, type ImageMapTool } from "./engine";
  import { registerImageMapSession } from "./session";

type RasterLayer = {
  id: string;
  name: string;
  order: number;
  visible: boolean;
  opacity: number;
  locked: boolean;
    rasterAssetId: string;
    assetRevision: string;
    canvas: HTMLCanvasElement | null;
    painted: boolean;
    mapWidth: number;
    mapHeight: number;
  };

type UndoEntry = { layerId: string; pixels: ImageData };

let {
  mapId,
  picking = false,
  focusLinkId,
  oncreated,
  onpick,
  onopen,
  onstate,
}: {
  mapId?: string;
  picking?: boolean;
  focusLinkId?: string;
  oncreated?: (map: Entity) => void;
  onpick?: (anchor: { kind: "point"; point: [number, number] }) => void;
  onopen?: (entityId: string) => void;
  onstate?: (status: string, detail: unknown) => void;
} = $props();

let mode = $state<"view" | "edit">("view");
let tool = $state<ImageMapTool>("pan");
let brushColor = $state("#d5ab6c");
let brushSize = $state(16);
let busy = $state(false);
let message = $state("");
let conflict = $state("");
let dirty = $state(false);
let layers = $state<RasterLayer[]>([]);
let activeLayerId = $state<string | null>(null);
let layersField = $state<FieldValue | null>(null);
let pins = $state<MapPin[]>([]);
let renamingId = $state<string | null>(null);
let stageHost = $state<HTMLDivElement | null>(null);
let stage: ImageMapStage | null = null;
let baseUrl: string | null = null;
let undo = $state<UndoEntry[]>([]);
let redo = $state<UndoEntry[]>([]);
let undoBytes = 0;
let defaultView = { center: [0.5, 0.5] as [number, number], zoom: 1 };

const listedLayers = $derived(
  [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id)),
);
const activeLayer = $derived(layers.find((layer) => layer.id === activeLayerId) ?? null);
  const canPaint = $derived(
    mode === "edit" &&
      !picking &&
      Boolean(activeLayer?.canvas) &&
      !activeLayer?.locked &&
      activeLayer?.visible !== false,
  );
  let mapSize = $state({ width: 1, height: 1 });

function publish(status: string, detail: unknown = null) {
  if (mapId) onstate?.(status, detail);
}

function setDirty(next: boolean) {
  if (dirty === next) return;
  dirty = next;
  publish(next ? "dirty" : "clean");
}

function objectUrl(bytes: number[], mime: string) {
  return URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: mime }));
}

function loadHtmlImage(url: string) {
  return new Promise<HTMLImageElement>((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("The image source could not be decoded"));
    image.src = url;
  });
}

async function canvasFromPng(bytes: number[], width: number, height: number) {
  const url = objectUrl(bytes, "image/png");
  try {
    const image = await loadHtmlImage(url);
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Could not create a raster layer canvas");
    ctx.drawImage(image, 0, 0, width, height);
    return canvas;
  } finally {
    URL.revokeObjectURL(url);
  }
}

  function snapshot(layer: RasterLayer): UndoEntry | null {
    if (!layer.canvas) return null;
    const ctx = layer.canvas.getContext("2d");
    if (!ctx) return null;
    return { layerId: layer.id, pixels: ctx.getImageData(0, 0, layer.canvas.width, layer.canvas.height) };
  }

  function restore(entry: UndoEntry) {
    const layer = layers.find((item) => item.id === entry.layerId);
    if (!layer?.canvas) return;
    layer.canvas.getContext("2d")?.putImageData(entry.pixels, 0, 0);
  layer.painted = true;
  stage?.refreshLayer(layer.id);
}

function pushUndo(entry: UndoEntry) {
  const size = entry.pixels.data.byteLength;
  const next = [...undo];
  let bytes = undoBytes;
    while (next.length && bytes + size > IMAGE_MAX_UNDO_BYTES) {
    const dropped = next.shift();
    if (dropped) bytes -= dropped.pixels.data.byteLength;
  }
  next.push(entry);
  undo = next;
  undoBytes = bytes + size;
  redo = [];
}

function undoStroke() {
  const entry = undo.at(-1);
  if (!entry) return;
  undo = undo.slice(0, -1);
  const layer = layers.find((item) => item.id === entry.layerId);
  const current = layer ? snapshot(layer) : null;
  restore(entry);
  undoBytes -= entry.pixels.data.byteLength;
  if (current) redo = [...redo, current];
  setDirty(layers.some((item) => item.painted));
}

function redoStroke() {
  const entry = redo.at(-1);
  if (!entry) return;
  redo = redo.slice(0, -1);
  const layer = layers.find((item) => item.id === entry.layerId);
  const current = layer ? snapshot(layer) : null;
  restore(entry);
  if (current) {
    undo = [...undo, current];
    undoBytes += current.pixels.data.byteLength;
  }
  setDirty(true);
}

function pinPoint(pin: MapPin): [number, number] | null {
  const [minX, minY, maxX, maxY] = pin.bounds;
  if (![minX, minY, maxX, maxY].every((value) => typeof value === "number" && Number.isFinite(value))) return null;
  return [((minX as number) + (maxX as number)) / 2, ((minY as number) + (maxY as number)) / 2];
}

function syncPins() {
  stage?.setPins(
    pins.flatMap((pin) => {
      const point = pinPoint(pin);
      if (!point) return [];
      return [
        {
          id: pin.id,
          entityId: pin.entityId,
          label: pin.label || pin.role,
          x: point[0],
          y: point[1],
          focused: pin.id === focusLinkId,
        },
      ];
    }),
  );
}

  function attachLayerNodes() {
    if (!stage) return;
    for (const [index, layer] of [...layers].sort((left, right) => left.order - right.order).entries()) {
      if (!layer.canvas) {
        stage.removeRasterLayer(layer.id);
        continue;
      }
      stage.setRasterLayer(layer.id, layer.canvas, {
        visible: layer.visible,
        opacity: layer.opacity,
        order: index + 1,
      });
    }
  }

  async function ensureLayerCanvas(layer: RasterLayer) {
    if (layer.canvas) return layer.canvas;
    const bytes = await project.readAssetBytes(layer.rasterAssetId);
    const canvas = await canvasFromPng(bytes, layer.mapWidth, layer.mapHeight);
    layer.canvas = canvas;
    layers = layers.map((item) => (item.id === layer.id ? layer : item));
    return canvas;
  }

  function releaseHiddenLayer(layer: RasterLayer) {
    if (layer.visible || layer.painted || !layer.canvas) return;
    stage?.removeRasterLayer(layer.id);
    layer.canvas = null;
  }

function syncTool() {
  const next = picking || mode === "view" ? "pan" : tool;
  stage?.setPicking(picking);
  stage?.setTool(next);
  stage?.setBrush(brushColor, brushSize);
  stage?.setActiveCanvas(canPaint ? (activeLayer?.canvas ?? null) : null);
}

async function ensureStage(image: HTMLImageElement) {
  await tick();
  if (!stageHost) return;
  stage?.destroy();
  stage = new ImageMapStage(stageHost);
  stage.onPick = (point) => onpick?.({ kind: "point", point });
  stage.onOpenPin = (entityId) => onopen?.(entityId);
  stage.onStrokeStart = () => {
    if (!activeLayer) return;
    const entry = snapshot(activeLayer);
    if (entry) pushUndo(entry);
  };
  stage.onPaint = () => {
    if (!activeLayer) return;
    activeLayer.painted = true;
    setDirty(true);
  };
  stage.setBase(image);
  stage.applyView(defaultView.center, defaultView.zoom);
  attachLayerNodes();
  syncPins();
  syncTool();
  if (focusLinkId) {
    const focused = pins.find((pin) => pin.id === focusLinkId);
    const point = focused ? pinPoint(focused) : null;
    if (point) stage.focusNormalized(point);
  }
}

async function load() {
  if (!mapId) return;
  busy = true;
  try {
    const fields = await project.listFields(mapId);
    const descriptorField = fields.find((field) => field.namespace === "maps" && field.key === "map");
    const descriptor = descriptorField?.value as {
      sourceAssetId?: string;
      defaultView?: { center?: [number, number]; zoom?: number };
    };
    if (descriptor?.defaultView?.center) defaultView.center = descriptor.defaultView.center;
    if (typeof descriptor?.defaultView?.zoom === "number") defaultView.zoom = descriptor.defaultView.zoom;
    layersField = fields.find((item) => item.namespace === "maps" && item.key === "layers") ?? null;
    const rawLayers = Array.isArray((layersField?.value as { layers?: unknown[] } | undefined)?.layers)
      ? (layersField?.value as { layers: Array<Record<string, unknown>> }).layers
      : [];
    const assets = await project.listAssets(mapId);
    const source = assets.find((asset) => asset.id === descriptor?.sourceAssetId);
    if (!source) throw new Error("The image source asset is missing");
    if (baseUrl) URL.revokeObjectURL(baseUrl);
    const sourceBytes = await project.readAssetBytes(source.id);
    baseUrl = objectUrl(sourceBytes, source.mime_type);
    const image = await loadHtmlImage(baseUrl);
    mapSize = { width: image.naturalWidth, height: image.naturalHeight };
    const nextLayers: RasterLayer[] = [];
    for (const layer of rawLayers) {
      if (layer.kind !== "raster" || typeof layer.id !== "string" || typeof layer.rasterAssetId !== "string") continue;
      const asset = assets.find((item) => item.id === layer.rasterAssetId);
      if (!asset) throw new Error(`Raster layer ${String(layer.name)} is missing its asset`);
      const next: RasterLayer = {
        id: layer.id,
        name: String(layer.name ?? "Layer"),
        order: Number(layer.order ?? 0),
        visible: layer.defaultVisible !== false,
        opacity: typeof layer.opacity === "number" ? layer.opacity : 1,
        locked: layer.locked === true,
        rasterAssetId: layer.rasterAssetId,
        assetRevision: asset.revision,
        canvas: null,
        painted: false,
        mapWidth: image.naturalWidth,
        mapHeight: image.naturalHeight,
      };
      if (next.visible) await ensureLayerCanvas(next);
      nextLayers.push(next);
    }
    layers = nextLayers;
    activeLayerId = nextLayers.some((layer) => layer.id === activeLayerId)
      ? activeLayerId
      : ([...nextLayers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id))[0]?.id ??
        null);
    pins = await project.listMapPins(mapId);
    undo = [];
    redo = [];
    undoBytes = 0;
    setDirty(false);
    conflict = "";
    message = "";
    await ensureStage(image);
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

async function importImage() {
  const source = await project.pickFile();
  if (typeof source !== "string") return;
  busy = true;
  try {
    const imported = await project.importImageMapFile(source);
    await oncreated?.(imported.entity);
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

function applyLayerField(change: { layers: FieldValue; asset?: Asset | null; layer_id: string }) {
  layersField = change.layers;
  if (change.asset) {
    layers = layers.map((layer) =>
      layer.id === change.layer_id
        ? { ...layer, rasterAssetId: change.asset!.id, assetRevision: change.asset!.revision }
        : layer,
    );
  }
}

async function addLayer() {
  if (!mapId || !layersField || layers.length >= IMAGE_MAX_RASTER_LAYERS) return;
  busy = true;
  try {
    const change = await project.createRasterLayer(mapId, `Layer ${layers.length + 1}`, layersField.revision);
    applyLayerField(change);
    const asset = change.asset;
    if (!asset) throw new Error("Layer creation did not return an asset");
    const created = ((change.layers.value as { layers?: Array<Record<string, unknown>> }).layers ?? []).find(
      (item) => item.id === change.layer_id,
    );
    const bytes = await project.readAssetBytes(asset.id);
    const base = await loadHtmlImage(baseUrl ?? "");
      const layer: RasterLayer = {
        id: change.layer_id,
        name: String(created?.name ?? `Layer ${layers.length + 1}`),
        order: Number(created?.order ?? layers.reduce((max, item) => Math.max(max, item.order), -1) + 1),
        visible: created?.defaultVisible !== false,
        opacity: typeof created?.opacity === "number" ? created.opacity : 1,
        locked: created?.locked === true,
        rasterAssetId: asset.id,
        assetRevision: asset.revision,
        canvas: await canvasFromPng(bytes, base.naturalWidth, base.naturalHeight),
        painted: false,
        mapWidth: base.naturalWidth,
        mapHeight: base.naturalHeight,
      };
    layers = [...layers, layer];
    activeLayerId = layer.id;
    mode = "edit";
    if (tool === "pan") tool = "brush";
    attachLayerNodes();
    syncTool();
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

async function mutateLayer(layer: RasterLayer, update: Parameters<typeof project.updateMapLayer>[3]) {
  if (!mapId || !layersField) return;
  try {
    const change = await project.updateMapLayer(mapId, layer.id, layersField.revision, update);
    applyLayerField(change);
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
    await load();
  }
}

  async function toggleVisible(layer: RasterLayer) {
    layer.visible = !layer.visible;
    if (layer.visible) await ensureLayerCanvas(layer);
    else releaseHiddenLayer(layer);
    layers = [...layers];
    attachLayerNodes();
    syncTool();
    await mutateLayer(layer, { defaultVisible: layer.visible });
  }

async function setOpacity(layer: RasterLayer, opacity: number) {
  layer.opacity = opacity;
  layers = [...layers];
  attachLayerNodes();
  await mutateLayer(layer, { opacity });
}

async function toggleLock(layer: RasterLayer) {
  layer.locked = !layer.locked;
  layers = [...layers];
  syncTool();
  await mutateLayer(layer, { locked: layer.locked });
}

async function renameLayer(layer: RasterLayer, name: string) {
  const trimmed = name.trim();
  renamingId = null;
  if (!trimmed || trimmed === layer.name) return;
  layer.name = trimmed;
  layers = [...layers];
  await mutateLayer(layer, { name: trimmed });
}

async function moveLayer(layer: RasterLayer, direction: -1 | 1) {
  if (!mapId || !layersField) return;
  const ordered = [...layers].sort((left, right) => left.order - right.order);
  const index = ordered.findIndex((item) => item.id === layer.id);
  const swap = ordered[index + direction];
  if (!swap) return;
  const order = swap.order;
  swap.order = layer.order;
  layer.order = order;
  layers = [...layers];
  attachLayerNodes();
  try {
    const first = await project.updateMapLayer(mapId, layer.id, layersField.revision, { order: layer.order });
    applyLayerField(first);
    const second = await project.updateMapLayer(mapId, swap.id, first.layers.revision, { order: swap.order });
    applyLayerField(second);
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
    await load();
  }
}

async function removeLayer(layer: RasterLayer) {
  if (!mapId || !layersField) return;
  if (!confirm(`Delete ${layer.name}? This cannot be undone after it is saved.`)) return;
  busy = true;
  try {
    const change = await project.deleteRasterLayer(mapId, layer.id, layersField.revision);
    applyLayerField(change);
    stage?.removeRasterLayer(layer.id);
    const remaining = layers.filter((item) => item.id !== layer.id);
    layers = remaining;
    if (activeLayerId === layer.id) {
      activeLayerId =
        [...remaining].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id))[0]?.id ??
        null;
    }
    undo = undo.filter((entry) => entry.layerId !== layer.id);
    redo = redo.filter((entry) => entry.layerId !== layer.id);
    setDirty(layers.some((item) => item.painted));
    attachLayerNodes();
    syncTool();
  } catch (cause) {
    message = cause instanceof Error ? cause.message : String(cause);
  } finally {
    busy = false;
  }
}

async function sha256(bytes: Uint8Array) {
  const copy = bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength ? bytes : bytes.slice();
  const hash = await crypto.subtle.digest("SHA-256", copy.buffer as ArrayBuffer);
  return `sha256:${Array.from(new Uint8Array(hash), (value) => value.toString(16).padStart(2, "0")).join("")}`;
}

function encodePng(canvas: HTMLCanvasElement) {
  return new Promise<Uint8Array>((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error("Could not encode the raster layer"));
        return;
      }
      void blob.arrayBuffer().then((buffer) => resolve(new Uint8Array(buffer)), reject);
    }, "image/png");
  });
}

async function save() {
  if (!mapId || busy) return;
  const dirtyLayers = layers.filter((layer) => layer.painted);
  if (dirtyLayers.length === 0) {
    setDirty(false);
    return;
  }
  busy = true;
  publish("saving");
  try {
    for (const layer of dirtyLayers) {
      if (!layer.canvas) continue;
      const bytes = await encodePng(layer.canvas);
      const hash = await sha256(bytes);
      const asset = await project.replaceAssetBytes(layer.rasterAssetId, bytes, hash, "image/png", layer.assetRevision);
      layer.assetRevision = asset.revision;
      layer.rasterAssetId = asset.id;
      layer.painted = false;
    }
    layers = [...layers];
    undo = [];
    redo = [];
    undoBytes = 0;
    conflict = "";
    message = "";
    setDirty(false);
    publish("saved");
  } catch (cause) {
    const text = cause instanceof Error ? cause.message : String(cause);
    if (text.includes("revision conflict")) {
      conflict = "This layer changed elsewhere. Reload the map to continue, or keep editing locally.";
      publish("conflict", { message: text });
    } else {
      message = text;
      publish("error", { message: text });
    }
  } finally {
    busy = false;
  }
}

function fit() {
  stage?.fit();
}

function resetView() {
  stage?.applyView(defaultView.center, defaultView.zoom);
}

function selectLayer(layer: RasterLayer) {
  activeLayerId = layer.id;
  if (mode === "edit" && tool === "pan") tool = "brush";
  syncTool();
}

  function onKey(event: KeyboardEvent) {
    if (event.target instanceof HTMLElement && event.target.closest("input, textarea, select, [contenteditable=true]")) {
      return;
    }
    const meta = event.metaKey || event.ctrlKey;
    if (meta && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void save();
    } else if (meta && event.key.toLowerCase() === "z") {
      event.preventDefault();
      if (event.shiftKey) redoStroke();
      else undoStroke();
    } else if (!meta && mode === "edit" && !renamingId) {
      if (event.key === "b") tool = "brush";
      if (event.key === "e") tool = "eraser";
      if (event.key === "h" || event.key === "v") tool = "pan";
      if (event.key === "[") brushSize = Math.max(1, brushSize - 2);
      if (event.key === "]") brushSize = Math.min(128, brushSize + 2);
      syncTool();
    }
  }

  function onCanvasKey(event: KeyboardEvent) {
    const step = event.shiftKey ? 80 : 32;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      stage?.panBy(step, 0);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      stage?.panBy(-step, 0);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      stage?.panBy(0, step);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      stage?.panBy(0, -step);
    } else if (event.key === "+" || event.key === "=") {
      event.preventDefault();
      stage?.zoomAtCenter(1.1);
    } else if (event.key === "-" || event.key === "_") {
      event.preventDefault();
      stage?.zoomAtCenter(0.9);
    } else if (event.key === "Home") {
      event.preventDefault();
      fit();
    }
  }

function beforeUnload(event: BeforeUnloadEvent) {
  if (!dirty) return;
  event.preventDefault();
  event.returnValue = "";
}

$effect(() => {
  void tool;
  void mode;
  void picking;
  void brushColor;
  void brushSize;
  void activeLayerId;
  void canPaint;
  syncTool();
});

$effect(() => {
  void pins;
  void focusLinkId;
  syncPins();
  if (focusLinkId) {
    const focused = pins.find((pin) => pin.id === focusLinkId);
    const point = focused ? pinPoint(focused) : null;
    if (point) stage?.focusNormalized(point);
  }
});

onMount(() => {
  registerImageMapSession({ save, isDirty: () => dirty });
  void load();
  window.addEventListener("keydown", onKey);
  window.addEventListener("beforeunload", beforeUnload);
  let observer: ResizeObserver | null = null;
  const frame = requestAnimationFrame(() => {
    if (!stageHost) return;
    observer = new ResizeObserver(() => stage?.resize());
    observer.observe(stageHost);
  });
  return () => {
    cancelAnimationFrame(frame);
    observer?.disconnect();
    window.removeEventListener("keydown", onKey);
    window.removeEventListener("beforeunload", beforeUnload);
    registerImageMapSession(null);
    stage?.destroy();
    if (baseUrl) URL.revokeObjectURL(baseUrl);
  };
});
</script>

<section class="image-map-editor" aria-label="Image Map editor">
  <header>
    <div>
      <span>IMAGE MAP</span>
      <strong>{mapId ? (mode === "edit" ? "Edit annotations" : "Map view") : "New image map"}</strong>
    </div>
    {#if mapId}
      <div class="header-actions" role="toolbar" aria-label="Map view controls">
        <button type="button" class:active={mode === "view"} aria-pressed={mode === "view"} onclick={() => (mode = "view")}
          >View</button>
        <button type="button" class:active={mode === "edit"} aria-pressed={mode === "edit"} onclick={() => (mode = "edit")}
          >Edit</button>
        <button type="button" onclick={fit}>Fit map</button>
        <button type="button" class="quiet" onclick={resetView}>Reset view</button>
        {#if mode === "edit"}
          <button
            type="button"
            disabled={busy || !layersField || layers.length >= IMAGE_MAX_RASTER_LAYERS}
            onclick={() => void addLayer()}>Add layer</button>
          <button type="button" class="save" disabled={busy || !dirty} onclick={() => void save()}
            >{busy ? "Saving…" : dirty ? "Save" : "Saved"}</button>
        {/if}
      </div>
    {/if}
  </header>
  {#if !mapId}
    <div class="import-empty">
      <strong>Import an image to begin</strong>
      <p>PNG, JPEG, or safe SVG. The source stays immutable; annotations live on separate layers.</p>
      <button type="button" disabled={busy} onclick={() => void importImage()}
        >{busy ? "Importing…" : "Import image"}</button>
    </div>
  {:else}
    <div class="editor-body">
      <aside aria-label="Map layers">
        <strong id="image-map-layers-heading">Layers</strong>
        {#if listedLayers.length === 0}<p class="hint">Add a raster layer to paint annotations.</p>{/if}
        <div class="layer-list" role="list" aria-labelledby="image-map-layers-heading">
          {#each listedLayers as layer (layer.id)}
            <div class="layer" class:active={layer.id === activeLayerId} role="listitem">
              <button
                class="layer-name"
                type="button"
                aria-pressed={layer.id === activeLayerId}
                onclick={() => selectLayer(layer)}>
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
                  aria-pressed={layer.visible}
                  aria-label={layer.visible ? `Hide ${layer.name}` : `Show ${layer.name}`}
                  onclick={() => void toggleVisible(layer)}>{layer.visible ? "Show" : "Hide"}</button>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={layer.opacity}
                  aria-label={`${layer.name} opacity`}
                  oninput={(event) => void setOpacity(layer, Number(event.currentTarget.value))} />
                {#if mode === "edit"}
                  <button
                    type="button"
                    aria-pressed={layer.locked}
                    aria-label={layer.locked ? `Unlock ${layer.name}` : `Lock ${layer.name}`}
                    onclick={() => void toggleLock(layer)}>{layer.locked ? "Locked" : "Unlocked"}</button>
                  <button type="button" aria-label={`Rename ${layer.name}`} onclick={() => (renamingId = layer.id)}
                    >Rename</button>
                  <button type="button" aria-label={`Move ${layer.name} up`} onclick={() => void moveLayer(layer, 1)}
                    >Up</button>
                  <button type="button" aria-label={`Move ${layer.name} down`} onclick={() => void moveLayer(layer, -1)}
                    >Down</button>
                  <button type="button" aria-label={`Delete ${layer.name}`} onclick={() => void removeLayer(layer)}
                    >Delete</button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
        {#if mode === "edit"}
          <div class="tools">
            <strong id="image-map-tools-heading">Tools</strong>
            <div class="tool-row" role="toolbar" aria-labelledby="image-map-tools-heading">
              <button type="button" class:active={tool === "pan"} aria-pressed={tool === "pan"} onclick={() => (tool = "pan")}
                >Pan</button>
              <button
                type="button"
                class:active={tool === "brush"}
                aria-pressed={tool === "brush"}
                disabled={!activeLayer || activeLayer.locked}
                onclick={() => (tool = "brush")}>Brush</button>
              <button
                type="button"
                class:active={tool === "eraser"}
                aria-pressed={tool === "eraser"}
                disabled={!activeLayer || activeLayer.locked}
                onclick={() => (tool = "eraser")}>Eraser</button>
            </div>
            <label>Color <input type="color" bind:value={brushColor} disabled={!canPaint} aria-label="Brush color" /></label>
            <label
              >Size
              <input type="range" min="1" max="64" bind:value={brushSize} disabled={!canPaint} aria-label="Brush size" />
              {brushSize}px</label>
            <div class="tool-row">
              <button type="button" disabled={!undo.length} onclick={undoStroke}>Undo</button>
              <button type="button" disabled={!redo.length} onclick={redoStroke}>Redo</button>
            </div>
          </div>
        {/if}
      </aside>
      <div
        class:picking
        class="canvas"
        bind:this={stageHost}
        tabindex="0"
        role="application"
        aria-label={picking ? "Click the map to place the location" : "Map canvas"}
        aria-describedby="image-map-canvas-help"
        onkeydown={onCanvasKey}>
      </div>
    </div>
  {/if}
  <p id="image-map-canvas-help" class="visually-hidden">
    Arrow keys pan the {mapSize.width} by {mapSize.height} map. Plus and minus zoom. Home fits the map.
  </p>
  {#if conflict}<p class="error" role="alert">
      {conflict} <button type="button" onclick={() => void load()}>Reload</button>
    </p>{/if}
  {#if message}<p class="error" role="alert">{message}</p>{/if}
  {#if busy && mapId && !message}<p class="status" aria-live="polite">Working…</p>{/if}
</section>

<style>
.image-map-editor {
  display: flex;
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
.tool-row,
.layer-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
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
button.save,
.import-empty button {
  background: #d5ab6c;
  color: #243126;
}
button.quiet {
  background: transparent;
  color: #d5ab6c;
}
button:disabled {
  opacity: 0.45;
  cursor: default;
}
.import-empty,
.status {
  margin: auto;
  max-width: 390px;
  text-align: center;
}
.import-empty p,
.hint,
.status {
  color: #bac7bd;
  line-height: 1.5;
}
.editor-body {
  display: grid;
  min-height: 0;
  flex: 1;
  grid-template-columns: 240px 1fr;
}
aside {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  overflow: auto;
  border-right: 1px solid #405047;
  background: #202c27;
}
.layer {
  padding: 8px;
  border: 1px solid #4b5a51;
  border-radius: 6px;
}
.layer.active {
  border-color: #d5ab6c;
}
.layer-name {
  width: 100%;
  text-align: left;
  background: transparent;
  padding: 4px 0;
}
.layer input[type="range"] {
  flex: 1;
}
.tools {
  display: grid;
  gap: 8px;
  margin-top: 8px;
}
.canvas {
  min-height: 0;
  overflow: hidden;
  background: #111;
}
.canvas.picking {
  cursor: crosshair;
  outline: 2px solid #d5ab6c;
  outline-offset: -2px;
}
.error {
  margin: 12px;
  color: #f5a49c;
}
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
button:focus-visible,
input:focus-visible,
.canvas:focus-visible {
  outline: 2px solid #f3d39a;
  outline-offset: 2px;
}
@media (prefers-reduced-motion: reduce) {
  .image-map-editor,
  .image-map-editor * {
    transition: none !important;
    animation: none !important;
    scroll-behavior: auto !important;
  }
}
</style>
