<script lang="ts">
import { Hexagon, MousePointer2, Pencil, Redo2, Save, Trash2, Undo2 } from "@lucide/svelte";
import { featureName } from "../native-vector/types";
import LandmassSelectionBar from "../physical/LandmassSelectionBar.svelte";
import type { LandmassSelection } from "../physical/landmass-selection.ts";
import {
  OVERLAY_FAMILIES,
  overlayFamilyLabel,
  overlayFeaturesForLayer,
  type AtlasOverlayAuthoring,
  type OverlayDrawTool,
  type OverlayFamily,
} from "./overlay-family.ts";

let {
  authoring,
  tool = $bindable("select"),
  selectedFeatureId = $bindable(null),
  detectHint = $bindable(""),
  createFamily = $bindable("political"),
  selection = null,
  includeOccupied = $bindable(false),
  showIncludeOccupied = false,
  covered = false,
  onCreateFromSelection,
  onAddFromSelection,
  onInvertSelection,
  onClearSelection,
}: {
  authoring: AtlasOverlayAuthoring;
  tool?: OverlayDrawTool;
  selectedFeatureId?: string | null;
  detectHint?: string;
  createFamily?: OverlayFamily;
  selection?: LandmassSelection | null;
  includeOccupied?: boolean;
  showIncludeOccupied?: boolean;
  covered?: boolean;
  onCreateFromSelection?: () => void;
  onAddFromSelection?: () => void;
  onInvertSelection?: () => void;
  onClearSelection?: () => void;
} = $props();

function setTool(next: OverlayDrawTool, hint: string) {
  tool = next;
  detectHint = hint;
}

const iconProps = { size: 13, strokeWidth: 1.8 };
const activeLayer = $derived(authoring.layers.find((layer) => layer.id === authoring.activeLayerId) ?? null);
const regions = $derived(
  authoring.activeLayerId ? overlayFeaturesForLayer(authoring.features, authoring.activeLayerId) : [],
);
const selected = $derived(regions.find((feature) => feature.id === selectedFeatureId) ?? null);
const regionStyle = $derived({
  fill: selected?.properties.daena.style?.fill ?? activeLayer?.style.fill ?? "#8f6fd1",
  fillOpacity: selected?.properties.daena.style?.fillOpacity ?? activeLayer?.style.fillOpacity ?? 0.35,
  stroke: selected?.properties.daena.style?.stroke ?? activeLayer?.style.stroke ?? "#5e4893",
  strokeWidth: selected?.properties.daena.style?.strokeWidth ?? activeLayer?.style.strokeWidth ?? 1.5,
});
function onRenameSelected(value: string) {
  if (!selected) return;
  authoring.renameFeature(selected.id, value.trim() || null);
}
</script>

<section class="overlays" aria-label="Overlay authoring">
  <div class="block">
    <span class="kicker">Create</span>
    <p class="note">Physical tiles stay read-only. Draw or select land on the active overlay.</p>
    <div class="create-row">
      <select bind:value={createFamily} aria-label="Overlay family" disabled={authoring.busy}>
        {#each OVERLAY_FAMILIES as family}
          <option value={family}>{overlayFamilyLabel(family)}</option>
        {/each}
      </select>
      <button type="button" disabled={authoring.busy} onclick={() => authoring.createLayer(createFamily)}>
        New overlay
      </button>
    </div>
  </div>
  <div class="block">
    <span class="kicker">Landmass</span>
    <div class="tools" role="toolbar" aria-label="Landmass tools">
      <button
        type="button"
        class:active={tool === "landmass"}
        aria-pressed={tool === "landmass"}
        onclick={() =>
          setTool("landmass", "Click land to select the connected landmass at this epoch. Shift adds, Alt subtracts.")}>
        Landmass
      </button>
    </div>
    <LandmassSelectionBar
      {selection}
      canCreate={!authoring.busy}
      canAdd={Boolean(activeLayer && !activeLayer.locked && activeLayer.defaultVisible) &&
        !(covered && !includeOccupied)}
      busy={authoring.busy}
      hint={detectHint}
      bind:includeOccupied
      {showIncludeOccupied}
      {covered}
      oncreate={() => onCreateFromSelection?.()}
      onadd={() => onAddFromSelection?.()}
      oninvert={() => onInvertSelection?.()}
      onclear={() => onClearSelection?.()} />
  </div>
  {#if !activeLayer}
    <p class="note">Create an overlay, or select one on the Layers tab.</p>
  {:else}
    <div class="block">
      <span class="kicker">Active overlay</span>
      <label>
        Name
        <input
          value={activeLayer.name}
          disabled={authoring.busy || activeLayer.locked}
          onchange={(event) => authoring.renameLayer(activeLayer.id, event.currentTarget.value)} />
      </label>
      <div class="style-grid" aria-label="Overlay appearance">
        <label>
          Fill
          <input
            type="color"
            value={activeLayer.style.fill}
            disabled={authoring.busy || activeLayer.locked}
            oninput={(event) => authoring.updateLayerStyle(activeLayer.id, { fill: event.currentTarget.value })} />
        </label>
        <label>
          Stroke
          <input
            type="color"
            value={activeLayer.style.stroke}
            disabled={authoring.busy || activeLayer.locked}
            oninput={(event) => authoring.updateLayerStyle(activeLayer.id, { stroke: event.currentTarget.value })} />
        </label>
        <label class="span">
          Fill opacity
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={activeLayer.style.fillOpacity}
            disabled={authoring.busy || activeLayer.locked}
            oninput={(event) =>
              authoring.updateLayerStyle(activeLayer.id, { fillOpacity: Number(event.currentTarget.value) })} />
          <em>{Math.round(activeLayer.style.fillOpacity * 100)}%</em>
        </label>
        <label class="span">
          Overlay opacity
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={activeLayer.opacity}
            disabled={authoring.busy || activeLayer.locked}
            oninput={(event) => authoring.updateLayerOpacity(activeLayer.id, Number(event.currentTarget.value))} />
          <em>{Math.round(activeLayer.opacity * 100)}%</em>
        </label>
      </div>
    </div>
    <div class="block">
      <span class="kicker">Tools</span>
      <div class="tools" role="toolbar" aria-label="Overlay tools">
        <button
          type="button"
          class:active={tool === "select"}
          aria-pressed={tool === "select"}
          onclick={() => setTool("select", "")}>
          <MousePointer2 {...iconProps} /> Select
        </button>
        <button
          type="button"
          class:active={tool === "freehand"}
          aria-pressed={tool === "freehand"}
          disabled={activeLayer.locked || !activeLayer.defaultVisible}
          onclick={() => setTool("freehand", "Draw a freehand region on the map.")}>
          <Pencil {...iconProps} /> Freehand
        </button>
        <button
          type="button"
          class:active={tool === "polygon"}
          aria-pressed={tool === "polygon"}
          disabled={activeLayer.locked || !activeLayer.defaultVisible}
          onclick={() => setTool("polygon", "Click to place polygon vertices.")}>
          <Hexagon {...iconProps} /> Polygon
        </button>
      </div>
      <div class="history">
        <button type="button" disabled={!authoring.canUndo || authoring.busy} onclick={() => authoring.undo()}>
          <Undo2 {...iconProps} /> Undo
        </button>
        <button type="button" disabled={!authoring.canRedo || authoring.busy} onclick={() => authoring.redo()}>
          <Redo2 {...iconProps} /> Redo
        </button>
        <button type="button" disabled={!authoring.dirty || authoring.busy} onclick={() => void authoring.save()}>
          <Save {...iconProps} />
          {authoring.busy ? "Saving…" : authoring.dirty ? "Save" : "Saved"}
        </button>
      </div>
    </div>
    <div class="block">
      <span class="kicker">Regions</span>
      {#if regions.length === 0}
        <p class="note">Draw freehand or select land with Landmass.</p>
      {:else}
        <ul class="region-list">
          {#each regions as feature (feature.id)}
            <li class:active={feature.id === selectedFeatureId}>
              <button type="button" onclick={() => (selectedFeatureId = feature.id)}>
                {featureName(feature) || "Unnamed region"}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if selected}
        <label>
          Region name
          <input
            value={featureName(selected) ?? ""}
            onchange={(event) => onRenameSelected(event.currentTarget.value)} />
        </label>
        <div class="style-grid" aria-label="Region appearance">
          <label>
            Fill
            <input
              type="color"
              value={regionStyle.fill}
              oninput={(event) => authoring.updateFeatureStyle(selected.id, { fill: event.currentTarget.value })} />
          </label>
          <label>
            Stroke
            <input
              type="color"
              value={regionStyle.stroke}
              oninput={(event) => authoring.updateFeatureStyle(selected.id, { stroke: event.currentTarget.value })} />
          </label>
          <label class="span">
            Fill opacity
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={regionStyle.fillOpacity}
              oninput={(event) =>
                authoring.updateFeatureStyle(selected.id, { fillOpacity: Number(event.currentTarget.value) })} />
            <em>{Math.round(regionStyle.fillOpacity * 100)}%</em>
          </label>
        </div>
        <button
          type="button"
          class="danger"
          onclick={() => {
            const id = selected.id;
            selectedFeatureId = null;
            authoring.deleteFeatures([id]);
          }}>
          <Trash2 {...iconProps} /> Delete region
        </button>
      {/if}
    </div>
    <button
      type="button"
      class="danger"
      disabled={activeLayer.locked}
      onclick={() => authoring.deleteLayer(activeLayer.id)}>
      Delete overlay
    </button>
  {/if}
</section>

<style>
.overlays {
  display: grid;
  gap: 10px;
}
.block {
  display: grid;
  gap: 8px;
  padding: 10px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 10px;
  background: rgb(255 255 255 / 3%);
}
.kicker {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #d5ab6c;
}
.note {
  margin: 0;
  color: var(--theme-neutral-text-muted, #aebdb1);
}
.create-row,
.history,
.tools {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.create-row select,
.overlays label input:not([type="color"]):not([type="range"]) {
  min-width: 0;
  flex: 1;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 7px;
  padding: 6px 8px;
  background: #0f1a16;
  color: #edf2ec;
}
.overlays label {
  display: grid;
  gap: 4px;
  color: #d9d0c3;
  font-size: 11px;
}
.region-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 4px;
}
.region-list li {
  display: flex;
  align-items: center;
  gap: 4px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 8px;
  padding: 2px;
  background: rgb(0 0 0 / 16%);
}
.region-list li.active {
  border-color: #d5ab6c;
  background: rgb(213 171 108 / 16%);
}
.region-list button {
  flex: 1;
  display: grid;
  min-height: 28px;
  text-align: left;
  background: none;
  border: 0;
  color: inherit;
  padding: 4px 8px;
  cursor: pointer;
}
.tools button,
.history button,
.danger,
.create-row button {
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 7px;
  background: #0f1a16;
  color: #edf2ec;
  min-height: 28px;
  padding: 4px 8px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}
.tools button.active {
  background: #d5ab6c;
  color: #1b2822;
}
.danger {
  justify-self: start;
}
.style-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px 8px;
}
.style-grid label {
  display: grid;
  gap: 3px;
  color: #d9d0c3;
  font-size: 11px;
}
.style-grid .span {
  grid-column: 1 / -1;
}
.style-grid input[type="color"] {
  width: 100%;
  height: 28px;
  padding: 0;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  background: #0f1a16;
}
.style-grid input[type="range"] {
  width: 100%;
  accent-color: #d5ab6c;
}
.style-grid em {
  font-style: normal;
  color: #aebdb1;
  font-size: 10px;
}
</style>
