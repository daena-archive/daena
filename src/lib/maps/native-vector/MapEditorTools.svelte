<script lang="ts">
import {
  Circle,
  CircleHelp,
  Hexagon,
  Magnet,
  Mountain,
  MousePointer2,
  Move,
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

const iconProps = { size: 15, strokeWidth: 1.8, "aria-hidden": true } as const;
</script>

<div class="map-editor-tools">
  <button
    type="button"
    class="icon-button"
    class:active={tool === "static"}
    aria-pressed={tool === "static"}
    aria-label="View"
    title="View (V)"
    onclick={() => onset("static")}><Move {...iconProps} /></button>
  <button
    type="button"
    class="icon-button"
    class:active={tool === "select"}
    aria-pressed={tool === "select"}
    aria-label="Select"
    title="Select (S)"
    onclick={() => onset("select")}><MousePointer2 {...iconProps} /></button>
  <button
    type="button"
    class="icon-button"
    class:active={tool === "point"}
    aria-pressed={tool === "point"}
    aria-label="Point"
    title="Point"
    disabled={!canDraw}
    onclick={() => onset("point")}><Circle {...iconProps} /></button>
  <button
    type="button"
    class="icon-button"
    class:active={tool === "linestring"}
    aria-pressed={tool === "linestring"}
    aria-label="Line"
    title="Line"
    disabled={!canDraw}
    onclick={() => onset("linestring")}><Slash {...iconProps} /></button>
  <button
    type="button"
    class="icon-button"
    class:active={tool === "polygon"}
    aria-pressed={tool === "polygon"}
    aria-label="Polygon"
    title="Polygon"
    disabled={!canDraw}
    onclick={() => onset("polygon")}><Hexagon {...iconProps} /></button>
  <button
    type="button"
    class="icon-button"
    class:active={tool === "rectangle"}
    aria-pressed={tool === "rectangle"}
    aria-label="Rectangle"
    title="Rectangle"
    disabled={!canDraw}
    onclick={() => onset("rectangle")}><Square {...iconProps} /></button>
  <button
    type="button"
    class="icon-button"
    class:active={tool === "freehand"}
    aria-pressed={tool === "freehand"}
    aria-label="Freehand"
    title="Freehand"
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
  <button type="button" class="icon-button" aria-label="Undo" title="Undo" disabled={!canUndo} onclick={() => onundo()}
    ><Undo2 {...iconProps} /></button>
  <button type="button" class="icon-button" aria-label="Redo" title="Redo" disabled={!canRedo} onclick={() => onredo()}
    ><Redo2 {...iconProps} /></button>
</div>

<style>
.map-editor-tools {
  display: contents;
}
</style>
