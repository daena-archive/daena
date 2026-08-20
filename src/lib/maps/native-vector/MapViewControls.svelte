<script lang="ts">
import { ChevronDown } from "@lucide/svelte";

let {
  zoom,
  min = 0,
  max = 8,
  onzoom,
  onpan,
}: {
  zoom: number;
  min?: number;
  max?: number;
  onzoom: (zoom: number) => void;
  onpan?: (longitudeDegrees: number, latitudeDegrees: number) => void;
} = $props();

const ZOOM_STEP = 1;

function zoomIn() {
  onzoom(Math.min(max, zoom + ZOOM_STEP));
}

function zoomOut() {
  onzoom(Math.max(min, zoom - ZOOM_STEP));
}
</script>

<div class="map-view-controls" aria-label="Map view">
  {#if onpan}
    <div class="pan-pad" role="group" aria-label="Pan map">
      <button type="button" class="north" aria-label="Pan north" title="Pan north" onclick={() => onpan(0, 15)}
        >⌃</button>
      <button type="button" class="west" aria-label="Pan west" title="Pan west" onclick={() => onpan(-30, 0)}>‹</button>
      <button type="button" class="east" aria-label="Pan east" title="Pan east" onclick={() => onpan(30, 0)}>›</button>
      <button type="button" class="south" aria-label="Pan south" title="Pan south" onclick={() => onpan(0, -15)}
        ><ChevronDown size={12} strokeWidth={1.8} aria-hidden="true" /></button>
    </div>
  {/if}
  <div class="zoom-stack" role="group" aria-label="Zoom">
    <button type="button" aria-label="Zoom in" title="Zoom in" disabled={zoom >= max} onclick={zoomIn}>+</button>
    <button type="button" aria-label="Zoom out" title="Zoom out" disabled={zoom <= min} onclick={zoomOut}>−</button>
  </div>
</div>

<style>
.map-view-controls {
  position: absolute;
  z-index: 2;
  right: 10px;
  bottom: 10px;
  display: flex;
  flex-direction: row;
  align-items: flex-end;
  gap: 8px;
  color: #d8e3d9;
  font-size: 12px;
}
.pan-pad {
  display: grid;
  grid-template-columns: 1.45rem 1.45rem 1.45rem;
  grid-template-rows: 1.45rem 1.45rem 1.45rem;
  gap: 1px;
  padding: 3px;
  border: 1px solid #405047;
  border-radius: 8px;
  background: rgb(27 40 34 / 92%);
}
.pan-pad button {
  display: grid;
  place-items: center;
  padding: 0;
  font-size: 11px;
  line-height: 1;
}
.north {
  grid-column: 2;
  grid-row: 1;
}
.west {
  grid-column: 1;
  grid-row: 2;
}
.east {
  grid-column: 3;
  grid-row: 2;
}
.south {
  grid-column: 2;
  grid-row: 3;
}
.zoom-stack {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid #405047;
  border-radius: 8px;
  background: rgb(27 40 34 / 92%);
}
.zoom-stack button {
  width: 2.25rem;
  height: 2.25rem;
  padding: 0;
  border-radius: 0;
  font-size: 1.15rem;
  line-height: 1;
}
.zoom-stack button + button {
  border-top: 1px solid #405047;
}
button {
  border: 0;
  border-radius: 6px;
  padding: 4px 8px;
  background: transparent;
  color: #edf2ec;
  font: 700 12px system-ui;
  cursor: pointer;
}
button:hover:not(:disabled) {
  background: rgb(255 255 255 / 10%);
}
button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
button:focus-visible {
  outline: 2px solid #f3d39a;
  outline-offset: 2px;
}
</style>
