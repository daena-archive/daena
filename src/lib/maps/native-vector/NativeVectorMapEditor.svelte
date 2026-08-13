<script lang="ts">
import { onMount } from "svelte";
import type { Entity } from "$lib/project/client";
import type { MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import { PHASE0_VECTOR_LAYERS, phase0Fixture } from "./fixture";
import {
  createNativeVectorEditor,
  liveNativeVectorEditorCount,
  RENDERER_UNAVAILABLE,
  type NativeVectorEditor,
} from "./runtime";
import { registerNativeVectorSession } from "./session";
import type { VectorDrawMode, VectorFeatureCollection } from "./types";

let {
  mapId,
  picking = false,
  onstate,
}: {
  mapId?: string;
  picking?: boolean;
  focusLinkId?: string;
  oncreated?: (map: Entity) => void;
  onpick?: (anchor: MapAnchor) => void;
  onopen?: (entityId: string) => void;
  onstate?: (status: string, detail: unknown) => void;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let editor: NativeVectorEditor | null = null;
let draft = $state<VectorFeatureCollection>(phase0Fixture());
let layers = $state(PHASE0_VECTOR_LAYERS);
let activeLayerId = $state(PHASE0_VECTOR_LAYERS[0].id);
let tool = $state<VectorDrawMode>("select");
let dirty = $state(false);
let diagnostic = $state("");
let diagnosticCode = $state("");

const listedLayers = $derived(
  [...layers].sort((left, right) => left.order - right.order || left.id.localeCompare(right.id)),
);

function publish(status: string, detail: unknown = null) {
  onstate?.(status, detail);
}

function setDirty(next: boolean) {
  if (dirty === next) return;
  dirty = next;
  publish(next ? "dirty" : "clean");
}

async function save() {
  setDirty(false);
  publish("clean", { spike: true, persisted: false });
}

function isDirty() {
  return dirty;
}

function setTool(next: VectorDrawMode) {
  tool = next;
  editor?.setMode(next);
}

function switchLayer(layerId: string) {
  if (layerId === activeLayerId) return;
  editor?.switchLayer(layerId);
  activeLayerId = layerId;
  tool = "select";
}

onMount(() => {
  registerNativeVectorSession({ save, isDirty, teardown: () => editor?.dispose() });
  if (!host) return;
  const created = createNativeVectorEditor(host, {
    get draft() {
      return draft;
    },
    get activeLayerId() {
      return activeLayerId;
    },
    setDraft(next) {
      draft = next;
    },
    setActiveLayerId(id) {
      activeLayerId = id;
    },
    onDirty() {
      setDirty(true);
    },
    onDiagnostic(code, detail) {
      diagnosticCode = code;
      diagnostic = detail;
      if (code === RENDERER_UNAVAILABLE) publish("error", { code, detail });
    },
  });
  if ("error" in created) {
    diagnosticCode = created.error;
    diagnostic = created.detail;
    publish("error", created);
    return () => registerNativeVectorSession(null);
  }
  editor = created;
  publish("ready", { liveEditors: liveNativeVectorEditorCount(), workerUrl: created.workerUrl });
  return () => {
    created.dispose();
    editor = null;
    registerNativeVectorSession(null);
  };
});
</script>

<section class="native-vector-editor" aria-label="Native vector map editor">
  <header>
    <div>
      <span>NATIVE VECTOR MAP</span>
      <strong>{mapId ? "Local fixture" : "Phase 0 fixture"}</strong>
    </div>
    <div class="header-actions" role="toolbar" aria-label="Vector drawing tools">
      <button type="button" class:active={tool === "static"} onclick={() => setTool("static")}>Pan</button>
      <button type="button" class:active={tool === "select"} onclick={() => setTool("select")}>Select</button>
      <button type="button" class:active={tool === "point"} onclick={() => setTool("point")}>Point</button>
      <button type="button" class:active={tool === "linestring"} onclick={() => setTool("linestring")}>Line</button>
      <button type="button" class:active={tool === "polygon"} onclick={() => setTool("polygon")}>Polygon</button>
      <button type="button" class:active={tool === "freehand"} onclick={() => setTool("freehand")}>Freehand</button>
      <button type="button" onclick={() => editor?.undo()}>Undo</button>
      <button type="button" onclick={() => editor?.redo()}>Redo</button>
    </div>
  </header>
  {#if diagnostic}
    <p class="error" role="alert" data-code={diagnosticCode}>{diagnostic}</p>
  {/if}
  <div class="editor-body">
    <aside>
      <strong>Vector layers</strong>
      {#each listedLayers as layer}
        <button
          type="button"
          class="layer"
          class:active={layer.id === activeLayerId}
          onclick={() => switchLayer(layer.id)}>{layer.name}</button>
      {/each}
      <p class="hint">
        Base geography is read-only. This spike loads a local GeoJSON fixture and does not create a map entity.
      </p>
    </aside>
    <div class="canvas" class:picking bind:this={host}></div>
  </div>
</section>

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
.header-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
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
button.active {
  background: #d5ab6c;
  color: #243126;
}
.editor-body {
  display: grid;
  min-height: 0;
  flex: 1;
  grid-template-columns: 220px 1fr;
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
.layer {
  text-align: left;
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
