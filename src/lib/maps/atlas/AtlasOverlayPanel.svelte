<script lang="ts">
import { Hexagon, MousePointer2, Pencil, Plus, Redo2, Save, Trash2, Undo2 } from "@lucide/svelte";
import { featureName } from "../native-vector/types";
import LandmassSelectionBar from "../physical/LandmassSelectionBar.svelte";
import type { LandmassSelection } from "../physical/landmass-selection.ts";
import {
  overlayFamilyFromName,
  overlayFamilyLabel,
  overlayFeaturesForLayer,
  overlayNameSuggestions,
  type AtlasOverlayAuthoring,
  type OverlayDrawTool,
} from "./overlay-family.ts";

let {
  authoring,
  tool = $bindable("select"),
  selectedFeatureId = $bindable(null),
  detectHint = $bindable(""),
  createName = $bindable(""),
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
  createName?: string;
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
const listboxId = "atlas-overlay-name-list";
let suggestOpen = $state(false);
let suggestIndex = $state(-1);
let nameField = $state<HTMLInputElement | null>(null);

const activeLayer = $derived(authoring.layers.find((layer) => layer.id === authoring.activeLayerId) ?? null);
const regions = $derived(
  authoring.activeLayerId ? overlayFeaturesForLayer(authoring.features, authoring.activeLayerId) : [],
);
const selected = $derived(regions.find((feature) => feature.id === selectedFeatureId) ?? null);
const suggestions = $derived(overlayNameSuggestions(createName));
const inferredFamily = $derived(overlayFamilyFromName(createName));
const canDraw = $derived(Boolean(activeLayer && !activeLayer.locked && activeLayer.defaultVisible));
const regionStyle = $derived({
  fill: selected?.properties.daena.style?.fill ?? activeLayer?.style.fill ?? "#8f6fd1",
  fillOpacity: selected?.properties.daena.style?.fillOpacity ?? activeLayer?.style.fillOpacity ?? 0.35,
  stroke: selected?.properties.daena.style?.stroke ?? activeLayer?.style.stroke ?? "#5e4893",
});

function closeSuggestions() {
  suggestOpen = false;
  suggestIndex = -1;
}

function applySuggestion(label: string) {
  createName = label;
  closeSuggestions();
  nameField?.focus();
}

function createOverlay() {
  const id = authoring.createLayer(createName);
  if (!id) return;
  createName = "";
  closeSuggestions();
}

function onNameKey(event: KeyboardEvent) {
  if (event.key === "ArrowDown") {
    event.preventDefault();
    suggestOpen = true;
    suggestIndex = Math.min(suggestIndex + 1, suggestions.length - 1);
    return;
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    suggestIndex = Math.max(suggestIndex - 1, -1);
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    closeSuggestions();
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    const pick = suggestIndex >= 0 ? suggestions[suggestIndex] : null;
    if (pick) applySuggestion(pick.label);
    else createOverlay();
  }
}

function regionFill(featureId: string) {
  const feature = regions.find((item) => item.id === featureId);
  return feature?.properties.daena.style?.fill ?? activeLayer?.style.fill ?? "#8f6fd1";
}
</script>

<section class="overlays" aria-label="Overlay authoring">
  <div class="block">
    <span class="kicker">Layer</span>
    <p class="note">Name it anything. Suggestions only set a starting color.</p>
    <div class="create-row">
      <div
        class="suggest"
        onfocusout={(event) => {
          const next = event.relatedTarget as Node | null;
          if (next && event.currentTarget.contains(next)) return;
          closeSuggestions();
        }}>
        <label class="name-label" for="atlas-overlay-name">Name</label>
        <input
          id="atlas-overlay-name"
          bind:this={nameField}
          bind:value={createName}
          type="text"
          role="combobox"
          aria-autocomplete="list"
          aria-expanded={suggestOpen}
          aria-controls={listboxId}
          aria-activedescendant={suggestIndex >= 0 ? `${listboxId}-${suggestIndex}` : undefined}
          placeholder="Political, kingdoms, trade…"
          autocomplete="off"
          spellcheck="false"
          disabled={authoring.busy}
          onfocus={() => (suggestOpen = true)}
          oninput={() => {
            suggestOpen = true;
            suggestIndex = -1;
          }}
          onkeydown={onNameKey} />
        {#if suggestOpen && suggestions.length > 0}
          <ul class="suggest-list" id={listboxId} role="listbox" aria-label="Layer name suggestions">
            {#each suggestions as item, index (item.family)}
              <li>
                <button
                  type="button"
                  id={`${listboxId}-${index}`}
                  role="option"
                  aria-selected={index === suggestIndex}
                  class:active={index === suggestIndex}
                  onpointerdown={(event) => {
                    event.preventDefault();
                    applySuggestion(item.label);
                  }}>
                  <span>{item.label}</span>
                  <small>Preset</small>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
      <button type="button" class="primary" disabled={authoring.busy} onclick={createOverlay}>
        <Plus {...iconProps} /> Create
      </button>
    </div>
    {#if createName.trim()}
      <p class="hint">
        {inferredFamily === "custom"
          ? `Creates “${createName.trim()}” as a custom layer.`
          : `“${createName.trim()}” will use ${overlayFamilyLabel(inferredFamily)} colors. Rename anytime.`}
      </p>
    {/if}
    {#if authoring.layers.length > 0}
      <div class="layer-switch" role="group" aria-label="Overlays">
        {#each authoring.layers as layer (layer.id)}
          <button
            type="button"
            aria-pressed={layer.id === authoring.activeLayerId}
            class:active={layer.id === authoring.activeLayerId}
            disabled={authoring.busy}
            onclick={() => authoring.setActiveLayer(layer.id)}>
            <span class="swatch" style:background={layer.style.fill}></span>
            {layer.name}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if !activeLayer}
    <p class="note empty">Create a layer to draw and manage regions.</p>
  {:else}
    <div class="block">
      <span class="kicker">Active layer</span>
      <label>
        Rename
        <input
          value={activeLayer.name}
          disabled={authoring.busy || activeLayer.locked}
          onchange={(event) => authoring.renameLayer(activeLayer.id, event.currentTarget.value)} />
      </label>
      <div class="style-grid" aria-label="Layer appearance">
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
          Layer opacity
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
      <span class="kicker">Draw</span>
      <div class="tools" role="toolbar" aria-label="Region tools">
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
          disabled={!canDraw}
          onclick={() => setTool("freehand", "Draw a freehand region on the map.")}>
          <Pencil {...iconProps} /> Freehand
        </button>
        <button
          type="button"
          class:active={tool === "polygon"}
          aria-pressed={tool === "polygon"}
          disabled={!canDraw}
          onclick={() => setTool("polygon", "Click to place polygon vertices.")}>
          <Hexagon {...iconProps} /> Polygon
        </button>
        <button
          type="button"
          class:active={tool === "landmass"}
          aria-pressed={tool === "landmass"}
          onclick={() =>
            setTool(
              "landmass",
              "Click land to select the connected landmass at this epoch. Shift adds, Alt subtracts.",
            )}>
          Landmass
        </button>
      </div>
      <LandmassSelectionBar
        {selection}
        canCreate={!authoring.busy}
        canAdd={canDraw && !(covered && !includeOccupied)}
        busy={authoring.busy}
        hint={detectHint}
        bind:includeOccupied
        {showIncludeOccupied}
        {covered}
        oncreate={() => onCreateFromSelection?.()}
        onadd={() => onAddFromSelection?.()}
        oninvert={() => onInvertSelection?.()}
        onclear={() => onClearSelection?.()} />
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
        <p class="note">Draw, or add a landmass, then name and restyle each region here.</p>
      {:else}
        <ul class="region-list">
          {#each regions as feature (feature.id)}
            <li class:active={feature.id === selectedFeatureId}>
              <input
                type="color"
                value={regionFill(feature.id)}
                aria-label={`${featureName(feature) || "Unnamed region"} fill`}
                disabled={authoring.busy || activeLayer.locked}
                oninput={(event) => authoring.updateFeatureStyle(feature.id, { fill: event.currentTarget.value })}
                onfocus={() => (selectedFeatureId = feature.id)} />
              <input
                class="region-name"
                value={featureName(feature) ?? ""}
                placeholder="Unnamed region"
                aria-label="Region name"
                disabled={authoring.busy || activeLayer.locked}
                onfocus={() => (selectedFeatureId = feature.id)}
                onchange={(event) => authoring.renameFeature(feature.id, event.currentTarget.value.trim() || null)} />
              <button
                type="button"
                class="icon-danger"
                aria-label={`Delete ${featureName(feature) || "unnamed region"}`}
                disabled={authoring.busy || activeLayer.locked}
                onclick={() => {
                  if (selectedFeatureId === feature.id) selectedFeatureId = null;
                  authoring.deleteFeatures([feature.id]);
                }}>
                <Trash2 {...iconProps} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if selected}
        <div class="style-grid" aria-label="Selected region appearance">
          <label>
            Stroke
            <input
              type="color"
              value={regionStyle.stroke}
              disabled={authoring.busy || activeLayer.locked}
              oninput={(event) => authoring.updateFeatureStyle(selected.id, { stroke: event.currentTarget.value })} />
          </label>
          <label>
            Fill opacity
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={regionStyle.fillOpacity}
              disabled={authoring.busy || activeLayer.locked}
              oninput={(event) =>
                authoring.updateFeatureStyle(selected.id, { fillOpacity: Number(event.currentTarget.value) })} />
            <em>{Math.round(regionStyle.fillOpacity * 100)}%</em>
          </label>
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="danger"
      disabled={activeLayer.locked || authoring.busy}
      onclick={() => authoring.deleteLayer(activeLayer.id)}>
      Delete layer
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
.note,
.hint {
  margin: 0;
  color: var(--theme-neutral-text-muted, #aebdb1);
}
.hint {
  font-size: 11px;
}
.empty {
  padding: 10px;
  border: 1px dashed rgb(255 255 255 / 14%);
  border-radius: 8px;
}
.create-row,
.history,
.tools {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.create-row {
  align-items: end;
}
.suggest {
  position: relative;
  flex: 1;
  min-width: 0;
  display: grid;
  gap: 4px;
}
.name-label,
.overlays label {
  display: grid;
  gap: 4px;
  color: #d9d0c3;
  font-size: 11px;
}
.create-row input,
.overlays label input:not([type="color"]):not([type="range"]),
.region-name {
  min-width: 0;
  width: 100%;
  flex: 1;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 7px;
  padding: 6px 8px;
  background: #0f1a16;
  color: #edf2ec;
}
.create-row input:focus,
.overlays label input:focus,
.region-name:focus {
  outline: 2px solid #d5ab6c;
  outline-offset: 1px;
}
.suggest-list {
  position: absolute;
  z-index: 20;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  margin: 0;
  padding: 4px;
  list-style: none;
  display: grid;
  gap: 2px;
  border: 1px solid rgb(255 255 255 / 12%);
  border-radius: 8px;
  background: #15211d;
  box-shadow: 0 8px 24px rgb(0 0 0 / 35%);
}
.suggest-list button {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  min-height: 28px;
  padding: 4px 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: #edf2ec;
  cursor: pointer;
}
.suggest-list button.active,
.suggest-list button:hover {
  background: rgb(213 171 108 / 18%);
}
.suggest-list small {
  color: #aebdb1;
  font-size: 10px;
}
.layer-switch {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.layer-switch button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 28px;
  padding: 4px 8px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 999px;
  background: #0f1a16;
  color: #edf2ec;
  cursor: pointer;
  transition:
    background 180ms ease,
    border-color 180ms ease;
}
.layer-switch button.active {
  border-color: #d5ab6c;
  background: rgb(213 171 108 / 16%);
}
.swatch {
  width: 10px;
  height: 10px;
  border-radius: 99px;
  box-shadow: 0 0 0 1px rgb(255 255 255 / 20%);
}
.region-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 4px;
}
.region-list li {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) 28px;
  align-items: center;
  gap: 4px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 8px;
  padding: 4px;
  background: rgb(0 0 0 / 16%);
}
.region-list li.active {
  border-color: #d5ab6c;
  background: rgb(213 171 108 / 16%);
}
.region-list input[type="color"] {
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  background: #0f1a16;
  cursor: pointer;
}
.icon-danger,
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
  justify-content: center;
  gap: 4px;
  cursor: pointer;
  transition:
    background 180ms ease,
    color 180ms ease,
    opacity 180ms ease;
}
.create-row .primary {
  background: #d5ab6c;
  color: #1b2822;
  border-color: #d5ab6c;
}
.tools button.active {
  background: #d5ab6c;
  color: #1b2822;
}
.icon-danger {
  padding: 0;
  width: 28px;
}
.danger {
  justify-self: start;
}
.tools button:disabled,
.history button:disabled,
.danger:disabled,
.create-row button:disabled,
.icon-danger:disabled {
  opacity: 0.45;
  cursor: default;
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
