<script lang="ts">
import {
  Circle,
  CircleHelp,
  Hand,
  Hexagon,
  Magnet,
  Mountain,
  MousePointer2,
  Pencil,
  Redo2,
  Ruler,
  Slash,
  Spline,
  Square,
  SquareStack,
  Undo2,
} from "@lucide/svelte";
import type { VectorDrawMode } from "./types";

let {
  tool,
  canDraw,
  physicalMap,
  mapId = null,
  snapEnabled,
  snapConfigOpen,
  canUndo,
  canRedo,
  editorReady,
  onset,
  ontogglesnap,
  ontogglesnapconfig,
  onundo,
  onredo,
}: {
  tool: VectorDrawMode;
  canDraw: boolean;
  physicalMap: boolean;
  mapId?: string | null;
  snapEnabled: boolean;
  snapConfigOpen: boolean;
  canUndo: boolean;
  canRedo: boolean;
  editorReady: boolean;
  onset: (tool: VectorDrawMode) => void;
  ontogglesnap: () => void;
  ontogglesnapconfig: () => void;
  onundo: () => void;
  onredo: () => void;
} = $props();

const iconProps = { size: 16, strokeWidth: 1.8, "aria-hidden": true } as const;
</script>

<nav class="map-tool-rail" aria-label="Editing tools">
  <div class="tool-group" role="group" aria-label="Navigate">
    <button
      type="button"
      class="icon-button"
      class:active={tool === "static"}
      aria-pressed={tool === "static"}
      aria-label="Hand"
      title="Hand (H), hold Space to pan"
      onclick={() => onset("static")}><Hand {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={tool === "select"}
      aria-pressed={tool === "select"}
      aria-label="Select"
      title="Select (V)"
      onclick={() => onset("select")}><MousePointer2 {...iconProps} /></button>
  </div>
  <div class="tool-group" role="group" aria-label="Draw">
    <button
      type="button"
      class="icon-button"
      class:active={tool === "point"}
      aria-pressed={tool === "point"}
      aria-label="Point"
      title="Point (P)"
      disabled={!canDraw}
      onclick={() => onset("point")}><Circle {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={tool === "linestring"}
      aria-pressed={tool === "linestring"}
      aria-label="Line"
      title="Line (L)"
      disabled={!canDraw}
      onclick={() => onset("linestring")}><Slash {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={tool === "polygon"}
      aria-pressed={tool === "polygon"}
      aria-label="Polygon"
      title="Polygon (G)"
      disabled={!canDraw}
      onclick={() => onset("polygon")}><Hexagon {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={tool === "rectangle"}
      aria-pressed={tool === "rectangle"}
      aria-label="Rectangle"
      title="Rectangle (R)"
      disabled={!canDraw}
      onclick={() => onset("rectangle")}><Square {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={tool === "freehand"}
      aria-pressed={tool === "freehand"}
      aria-label="Freehand"
      title="Freehand (F)"
      disabled={!canDraw}
      onclick={() => onset("freehand")}><Pencil {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={tool === "trace"}
      aria-pressed={tool === "trace"}
      aria-label="Trace"
      title="Trace (T)"
      disabled={!canDraw}
      onclick={() => onset("trace")}><Spline {...iconProps} /></button>
    {#if physicalMap}
      <button
        type="button"
        class="icon-button"
        class:active={tool === "landmass"}
        aria-pressed={tool === "landmass"}
        aria-label="Landmass"
        title="Select landmass. Shift adds, Alt subtracts."
        disabled={!mapId}
        onclick={() => onset("landmass")}><Mountain {...iconProps} /></button>
    {/if}
  </div>
  <div class="tool-group" role="group" aria-label="Snap">
    <button
      type="button"
      class="icon-button"
      class:active={snapEnabled}
      aria-pressed={snapEnabled}
      aria-label={snapEnabled ? "Snap on" : "Snap off"}
      title={snapEnabled ? "Snap on (\\)" : "Snap off (\\)"}
      disabled={!editorReady}
      onclick={() => ontogglesnap()}><Magnet {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={snapConfigOpen}
      aria-pressed={snapConfigOpen}
      aria-label="Snap settings"
      title="Snap settings"
      disabled={!editorReady}
      onclick={() => ontogglesnapconfig()}><CircleHelp {...iconProps} /></button>
  </div>
  <div class="tool-group" role="group" aria-label="Measure">
    <button
      type="button"
      class="icon-button"
      class:active={tool === "measure-distance"}
      aria-pressed={tool === "measure-distance"}
      aria-label="Measure distance"
      title="Measure distance (D)"
      disabled={!editorReady}
      onclick={() => onset("measure-distance")}><Ruler {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={tool === "measure-length"}
      aria-pressed={tool === "measure-length"}
      aria-label="Measure length"
      title="Measure length (Shift+M)"
      disabled={!editorReady}
      onclick={() => onset("measure-length")}><Slash {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      class:active={tool === "measure-area"}
      aria-pressed={tool === "measure-area"}
      aria-label="Measure area"
      title="Measure area (Shift+A)"
      disabled={!editorReady}
      onclick={() => onset("measure-area")}><SquareStack {...iconProps} /></button>
  </div>
  <div class="tool-group history" role="group" aria-label="History">
    <button
      type="button"
      class="icon-button"
      aria-label="Undo"
      title="Undo (⌘Z)"
      disabled={!canUndo}
      onclick={() => onundo()}><Undo2 {...iconProps} /></button>
    <button
      type="button"
      class="icon-button"
      aria-label="Redo"
      title="Redo (⇧⌘Z)"
      disabled={!canRedo}
      onclick={() => onredo()}><Redo2 {...iconProps} /></button>
  </div>
</nav>

<style>
.map-tool-rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  width: 44px;
  min-height: 0;
  padding: 8px 5px;
  overflow: auto;
  border-right: 1px solid var(--line, #e4e1d8);
  background: var(--surface, #fffefa);
}
.tool-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--line, #e4e1d8);
}
.tool-group.history {
  margin-top: auto;
  padding-bottom: 0;
  border-bottom: 0;
}
.icon-button {
  display: grid;
  width: 34px;
  height: 34px;
  min-width: 34px;
  place-items: center;
  margin: 0 auto;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--ink-soft, #77766d);
  cursor: pointer;
}
.icon-button:hover:not(:disabled),
.icon-button:focus-visible {
  border-color: var(--line, #e4e1d8);
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink);
  outline: 0;
}
.icon-button.active,
.icon-button[aria-pressed="true"] {
  border-color: var(--line-strong, #d9cdbd);
  background: var(--theme-success-bg, #e4ece4);
  color: var(--theme-success-text, #2f4e35);
}
.icon-button:disabled {
  opacity: 0.38;
  cursor: not-allowed;
}
@media (max-width: 900px) {
  .map-tool-rail {
    flex-direction: row;
    flex-wrap: wrap;
    align-items: center;
    width: 100%;
    height: auto;
    padding: 6px 8px;
    border-right: 0;
    border-bottom: 1px solid var(--line, #e4e1d8);
  }
  .tool-group {
    flex-direction: row;
    width: auto;
    padding: 0 8px 0 0;
    border-bottom: 0;
    border-right: 1px solid var(--line, #e4e1d8);
  }
  .tool-group.history {
    margin-top: 0;
    margin-left: auto;
    padding-right: 0;
    border-right: 0;
  }
}
</style>
