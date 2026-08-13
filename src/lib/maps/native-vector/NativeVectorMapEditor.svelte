<script lang="ts">
import { onMount, tick } from "svelte";
import { project, type Asset, type Entity, type FieldValue } from "$lib/project/client";
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
import { DEFAULT_VECTOR_LAYER_STYLE, type VectorDrawMode, type VectorFeature, type VectorFeatureCollection, type VectorLayerDefinition } from "./types";

let {
  mapId,
  picking = false,
  oncreated,
  oncancel,
  onstate,
}: {
  mapId?: string;
  picking?: boolean;
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

const listedLayers = $derived(
  [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id)),
);
const activeLayer = $derived(layers.find((layer) => layer.id === activeLayerId) ?? null);
const canDraw = $derived(Boolean(activeLayer) && !activeLayer?.locked && !picking);
const dirty = $derived(editorState.dirty);
const diagnostic = $derived(editorState.diagnostic);
const diagnosticCode = $derived(editorState.diagnosticCode);
const conflict = $derived(editorState.conflict);

function publish(status: string, detail: unknown = null) {
  onstate?.(status, detail);
}

function applyEditorEvent(event: Parameters<typeof reduceVectorEditor>[1]) {
  editorState = reduceVectorEditor(editorState, event);
  publish(editorState.status, {
    code: editorState.diagnosticCode || null,
    detail: editorState.diagnostic || null,
  });
}

function cloneCollection(collection: VectorFeatureCollection): VectorFeatureCollection {
  return structuredClone(collection);
}

function applyLayersField(field: FieldValue) {
  layersField = field;
  layers = parseVectorLayers(field.value);
}

function destroyEditor() {
  editor?.dispose();
  editor = null;
}

function mountEditor() {
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
  busy = true;
  try {
    const fields = await project.listFields(mapId);
    const descriptorField = fields.find((field) => field.namespace === "maps" && field.key === "map");
    const descriptor = descriptorField?.value as {
      sourceAssetId?: string;
      defaultView?: { center?: [number, number]; zoom?: number };
    };
    if (descriptor?.defaultView?.center) defaultView = { ...defaultView, center: descriptor.defaultView.center };
    if (typeof descriptor?.defaultView?.zoom === "number") defaultView = { ...defaultView, zoom: descriptor.defaultView.zoom };
    const nextLayersField = fields.find((item) => item.namespace === "maps" && item.key === "layers") ?? null;
    if (!nextLayersField) throw new Error("maps:layers is missing");
    applyLayersField(nextLayersField);
    const assets = await project.listAssets(mapId);
    const source = assets.find((asset) => asset.id === descriptor?.sourceAssetId);
    if (!source) throw new Error("The vector source asset is missing");
    sourceAsset = source;
    const bytes = await project.readAssetBytes(source.id);
    const collection = parseVectorCollection(bytes);
    draft = cloneCollection(collection);
    loaded = cloneCollection(collection);
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
    const bytes = collectionBytes(draft);
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
    recoveryPath = await project.mapsRecoveryExport(mapId, collectionBytes(draft));
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
  layer.defaultVisible = !layer.defaultVisible;
  layers = [...layers];
  editor?.syncLayers(layers);
  await mutateLayer(layer, { defaultVisible: layer.defaultVisible });
}

async function toggleLock(layer: VectorLayerDefinition) {
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
  const trimmed = name.trim();
  renamingId = null;
  if (!trimmed || trimmed === layer.name) return;
  layer.name = trimmed;
  layers = [...layers];
  await mutateLayer(layer, { name: trimmed });
}

async function moveLayer(layer: VectorLayerDefinition, direction: -1 | 1) {
  const index = listedLayers.findIndex((item) => item.id === layer.id);
  const neighbor = listedLayers[index + direction];
  if (!neighbor) return;
  const layerOrder = layer.order;
  await mutateLayer(layer, { order: neighbor.order });
  await mutateLayer(neighbor, { order: layerOrder });
}

async function updateStyle(layer: VectorLayerDefinition, patch: Partial<VectorLayerDefinition["style"]>) {
  const style = { ...layer.style, ...patch };
  layer.style = style;
  layers = [...layers];
  editor?.syncLayers(layers);
  await mutateLayer(layer, { style });
}

async function removeLayer(layer: VectorLayerDefinition) {
  if (!mapId || !layersField || !sourceAsset) return;
  const savedCount = featureCountForLayer(loaded, layer.id);
  const draftCount = featureCountForLayer(draft, layer.id);
  const extra = draftCount === savedCount ? "" : ` Unsaved draft features on this layer (${draftCount}) will be discarded.`;
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
  registerNativeVectorSession({ save, isDirty, teardown: () => editor?.dispose() });
  window.addEventListener("keydown", onKey);
  void load();
  return () => {
    window.removeEventListener("keydown", onKey);
    destroyEditor();
    registerNativeVectorSession(null);
  };
});
</script>

{#if !mapId}
  <NativeVectorGenerator {oncreated} {oncancel} />
{:else}
<section class="native-vector-editor" aria-label="Native vector map editor">
  <header>
    <div>
      <span>NATIVE VECTOR MAP</span>
      <strong>{busy ? "Loading…" : dirty ? "Unsaved changes" : "Saved"}</strong>
    </div>
    <div class="header-actions" role="toolbar" aria-label="Vector drawing tools">
      <button type="button" class:active={tool === "static"} aria-pressed={tool === "static"} onclick={() => setTool("static")}
        >Pan</button>
      <button type="button" class:active={tool === "select"} aria-pressed={tool === "select"} onclick={() => setTool("select")}
        >Select</button>
      <button
        type="button"
        class:active={tool === "point"}
        aria-pressed={tool === "point"}
        disabled={!canDraw}
        onclick={() => setTool("point")}>Point</button>
      <button
        type="button"
        class:active={tool === "linestring"}
        aria-pressed={tool === "linestring"}
        disabled={!canDraw}
        onclick={() => setTool("linestring")}>Line</button>
      <button
        type="button"
        class:active={tool === "polygon"}
        aria-pressed={tool === "polygon"}
        disabled={!canDraw}
        onclick={() => setTool("polygon")}>Polygon</button>
      <button
        type="button"
        class:active={tool === "freehand"}
        aria-pressed={tool === "freehand"}
        disabled={!canDraw}
        onclick={() => setTool("freehand")}>Freehand</button>
      <button type="button" onclick={() => editor?.undo()}>Undo</button>
      <button type="button" onclick={() => editor?.redo()}>Redo</button>
      <button type="button" disabled={busy || layers.length >= VECTOR_MAX_LAYERS} onclick={() => void addLayer()}
        >Add layer</button>
      <button type="button" class="save" disabled={busy || !dirty} onclick={() => void save()}
        >{busy ? "Saving…" : dirty ? "Save" : "Saved"}</button>
    </div>
  </header>
  {#if conflict}
    <p class="error" role="alert">
      This map changed elsewhere. Reload the canonical source, export this draft, or keep editing without saving over it.
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
  <div class="editor-body">
    <aside aria-label="Vector layers">
      <strong id="vector-layers-heading">Vector layers</strong>
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
                aria-pressed={layer.defaultVisible}
                aria-label={layer.defaultVisible ? `Hide ${layer.name}` : `Show ${layer.name}`}
                onclick={() => void toggleVisible(layer)}>{layer.defaultVisible ? "Show" : "Hide"}</button>
              <button
                type="button"
                aria-pressed={layer.locked}
                aria-label={layer.locked ? `Unlock ${layer.name}` : `Lock ${layer.name}`}
                onclick={() => void toggleLock(layer)}>{layer.locked ? "Locked" : "Unlocked"}</button>
              <button type="button" onclick={() => (renamingId = layer.id)}>Rename</button>
              <button type="button" onclick={() => void moveLayer(layer, -1)}>Up</button>
              <button type="button" onclick={() => void moveLayer(layer, 1)}>Down</button>
              <button type="button" onclick={() => void removeLayer(layer)}>Delete</button>
            </div>
            {#if layer.id === activeLayerId}
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
                    oninput={(event) => void updateStyle(layer, { fillOpacity: Number(event.currentTarget.value) })} />
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
                    onchange={(event) => void updateStyle(layer, { strokeWidth: Number(event.currentTarget.value) })} />
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
                    onchange={(event) => void updateStyle(layer, { pointRadius: Number(event.currentTarget.value) })} />
                </label>
              </div>
            {/if}
          </div>
        {/each}
      </div>
      {#if selectedFeature}
        <div class="inspector" aria-label="Selected feature">
          <strong>Selected feature</strong>
          <p class="hint">{selectedFeature.properties.kind} · {selectedFeature.properties.daenaLayerId === "base" ? "base geography" : "authored"}</p>
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
      <p class="hint">Base geography is read-only. Point, line, polygon, and freehand edits save through the canonical GeoJSON source. Delete removes the selected feature.</p>
    </aside>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
    <div
      class="canvas"
      class:picking
      bind:this={host}
      tabindex="0"
      role="application"
      aria-label="Native vector map canvas"></div>
  </div>
</section>
{/if}

<style>
.native-vector-editor {
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
.layer-row,
.style-row {
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
  flex: 1;
  grid-template-columns: 260px 1fr;
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
.layer-list {
  display: grid;
  gap: 8px;
}
.layer {
  display: grid;
  gap: 6px;
  padding: 8px;
  border-radius: 8px;
  background: #18241f;
}
.layer.active {
  outline: 1px solid #d5ab6c;
}
.layer-name {
  text-align: left;
  width: 100%;
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
  min-height: 0;
  background: #0d1b2a;
}
.canvas.picking {
  outline: 2px solid #d5ab6c;
  outline-offset: -2px;
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
</style>
