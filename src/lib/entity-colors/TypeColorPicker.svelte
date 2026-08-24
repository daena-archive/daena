<script lang="ts">
import { tick } from "svelte";
import type { EntityTypeColor } from "../../../packages/plugin-sdk/src/generated";
import {
  DEFAULT_TYPE_COLOR,
  TYPE_COLOR_PRESET_OPTIONS,
  TYPE_COLOR_PRESETS,
  normalizeHexColor,
  type TypeColorPresetId,
} from "$lib/entity-colors/presets";

const DEFAULT_CUSTOM_LIGHT = "#4a667a";
const DEFAULT_CUSTOM_DARK = "#8eb0c4";

let {
  value = DEFAULT_TYPE_COLOR,
  label = "Color",
  onChange,
}: {
  value?: EntityTypeColor;
  label?: string;
  onChange: (value: EntityTypeColor) => void;
} = $props();

let open = $state(false);
let draft = $state<EntityTypeColor>(DEFAULT_TYPE_COLOR);
let draftCustomLight = $state(DEFAULT_CUSTOM_LIGHT);
let draftCustomDark = $state(DEFAULT_CUSTOM_DARK);
let trigger = $state<HTMLButtonElement>();
let popover = $state<HTMLDivElement>();
let pickerRoot = $state<HTMLDivElement>();
let popoverStyle = $state("");

const selectedLabel = $derived(
  value.kind === "preset" ? (TYPE_COLOR_PRESETS[value.id]?.label ?? value.id) : "Custom",
);

const draftLabel = $derived(
  draft.kind === "preset" ? (TYPE_COLOR_PRESETS[draft.id]?.label ?? draft.id) : "Custom",
);

const triggerStyle = $derived.by(() => {
  if (value.kind === "preset") {
    const preset = TYPE_COLOR_PRESETS[value.id];
    if (!preset) return "";
    return `--trigger-fg:${preset.light.fg};--trigger-bg:${preset.light.bg};`;
  }
  const light = normalizeHexColor(value.light) ?? DEFAULT_CUSTOM_LIGHT;
  return `--trigger-fg:${light};--trigger-bg:${light};`;
});

function cloneColor(color: EntityTypeColor): EntityTypeColor {
  return color.kind === "preset"
    ? { kind: "preset", id: color.id }
    : { kind: "custom", light: color.light, dark: color.dark };
}

function presetCustomHex(id: TypeColorPresetId): { light: string; dark: string } {
  const preset = TYPE_COLOR_PRESETS[id];
  return { light: preset.light.fg, dark: preset.dark.fg };
}

function resetDraftFromValue() {
  draft = cloneColor(value);
  if (draft.kind === "custom") {
    draftCustomLight = normalizeHexColor(draft.light) ?? DEFAULT_CUSTOM_LIGHT;
    draftCustomDark = normalizeHexColor(draft.dark) ?? DEFAULT_CUSTOM_DARK;
  } else {
    const hex = presetCustomHex(draft.id as TypeColorPresetId);
    draftCustomLight = hex.light;
    draftCustomDark = hex.dark;
  }
}

function placePopover() {
  if (!trigger || !popover) return;
  const margin = 12;
  const gap = 6;
  const anchor = trigger.getBoundingClientRect();
  const panel = popover.getBoundingClientRect();
  const left = Math.min(Math.max(margin, anchor.left), Math.max(margin, window.innerWidth - panel.width - margin));
  const below = anchor.bottom + gap;
  const top =
    below + panel.height <= window.innerHeight - margin ? below : Math.max(margin, anchor.top - gap - panel.height);
  popoverStyle = `left:${left}px;top:${top}px;max-height:${Math.max(260, window.innerHeight - top - margin)}px`;
}

async function togglePicker() {
  if (open) {
    open = false;
    return;
  }
  resetDraftFromValue();
  popoverStyle = "";
  open = true;
  await tick();
  placePopover();
}

function selectPreset(id: (typeof TYPE_COLOR_PRESET_OPTIONS)[number]["id"]) {
  const hex = presetCustomHex(id);
  draftCustomLight = hex.light;
  draftCustomDark = hex.dark;
  draft = { kind: "preset", id };
}

function syncDraftCustom() {
  const light = normalizeHexColor(draftCustomLight);
  const dark = normalizeHexColor(draftCustomDark);
  if (!light || !dark) return;
  draftCustomLight = light;
  draftCustomDark = dark;
  draft = { kind: "custom", light, dark };
}

function saveDraft() {
  if (draft.kind === "custom") {
    syncDraftCustom();
    onChange({ kind: "custom", light: draftCustomLight, dark: draftCustomDark });
  } else {
    onChange(draft);
  }
  open = false;
}

function cancelDraft() {
  open = false;
}

$effect(() => {
  if (!open) return;
  const reposition = () => placePopover();
  window.addEventListener("resize", reposition);
  window.addEventListener("scroll", reposition, true);
  return () => {
    window.removeEventListener("resize", reposition);
    window.removeEventListener("scroll", reposition, true);
  };
});

$effect(() => {
  if (!open) return;
  const onPointerDown = (event: PointerEvent) => {
    const target = event.target;
    if (!(target instanceof Node) || pickerRoot?.contains(target)) return;
    cancelDraft();
  };
  const onKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape") cancelDraft();
  };
  window.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("keydown", onKeydown, true);
  return () => {
    window.removeEventListener("pointerdown", onPointerDown, true);
    window.removeEventListener("keydown", onKeydown, true);
  };
});
</script>

<div class="color-picker" bind:this={pickerRoot}>
  <span class="picker-label">{label}</span>
  <button
    type="button"
    bind:this={trigger}
    class="picker-trigger"
    style={triggerStyle}
    aria-expanded={open}
    aria-label={`Choose color: ${selectedLabel}`}
    title={selectedLabel}
    onclick={togglePicker}>
    <span class="trigger-dot" aria-hidden="true"></span>
  </button>
  {#if open}
    <div class="picker-popover" bind:this={popover} style={popoverStyle}>
      <div class="popover-heading">
        <strong>Presets</strong>
        <span>{draftLabel}</span>
      </div>
      <div class="preset-grid" role="group" aria-label="Type color presets">
        {#each TYPE_COLOR_PRESET_OPTIONS as preset}
          <button
            type="button"
            class="preset-dot"
            class:selected={draft.kind === "preset" && draft.id === preset.id}
            title={preset.label}
            aria-label={preset.label}
            aria-pressed={draft.kind === "preset" && draft.id === preset.id}
            onclick={() => selectPreset(preset.id)}>
            <span class="dot" style={`--dot-fg:${preset.light.fg};--dot-bg:${preset.light.bg};`}></span>
          </button>
        {/each}
      </div>
      <div class="custom-section">
        <span class="custom-label">Custom</span>
        <div class="custom-colors">
          <label>
            <span>Light</span>
            <input
              type="color"
              bind:value={draftCustomLight}
              aria-label="Light theme color"
              oninput={syncDraftCustom} />
          </label>
          <label>
            <span>Dark</span>
            <input type="color" bind:value={draftCustomDark} aria-label="Dark theme color" oninput={syncDraftCustom} />
          </label>
        </div>
      </div>
      <div class="popover-actions">
        <button type="button" class="action quiet" onclick={cancelDraft}>Cancel</button>
        <button type="button" class="action primary" onclick={saveDraft}>Save</button>
      </div>
    </div>
  {/if}
</div>

<style>
.color-picker {
  position: relative;
  display: grid;
  gap: 5px;
}
.picker-label {
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.picker-trigger {
  display: flex;
  min-height: 36px;
  align-items: center;
  width: 36px;
  justify-content: center;
  padding: 7px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  cursor: pointer;
}
.trigger-dot {
  display: block;
  width: 18px;
  height: 18px;
  border-radius: 999px;
  background: var(--trigger-fg, var(--ink-soft));
  box-shadow:
    inset 0 0 0 1px color-mix(in srgb, white 28%, transparent),
    0 0 0 1px color-mix(in srgb, var(--trigger-fg, var(--ink-soft)) 24%, transparent);
}
.picker-popover {
  position: fixed;
  z-index: 90;
  width: min(280px, calc(100vw - 24px));
  overflow: auto;
  padding: 10px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.picker-popover[style=""] {
  visibility: hidden;
}
.popover-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 8px;
}
.popover-heading strong {
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.popover-heading span {
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 600;
}
.preset-grid {
  display: grid;
  grid-template-columns: repeat(8, 28px);
  gap: 5px;
}
.preset-dot {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 999px;
  background: transparent;
  cursor: pointer;
}
.preset-dot:hover .dot,
.preset-dot:focus-visible .dot {
  transform: scale(1.06);
}
.preset-dot.selected {
  border-color: var(--accent-soft);
  background: var(--surface-muted);
}
.dot {
  display: block;
  width: 20px;
  height: 20px;
  border-radius: 999px;
  background: var(--dot-fg);
  box-shadow:
    inset 0 0 0 1px color-mix(in srgb, white 28%, transparent),
    0 0 0 1px color-mix(in srgb, var(--dot-fg) 24%, transparent);
  transition: transform 0.12s ease;
}
.preset-dot.selected .dot {
  box-shadow:
    inset 0 0 0 1px color-mix(in srgb, white 28%, transparent),
    0 0 0 2px var(--surface),
    0 0 0 3px var(--accent-soft);
}
.custom-section {
  display: grid;
  gap: 8px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}
.custom-label {
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.custom-colors {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.custom-colors label {
  display: grid;
  gap: 5px;
}
.custom-colors label span {
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.custom-colors input[type="color"] {
  width: 100%;
  height: 36px;
  padding: 2px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--theme-surface-bg, #fff);
  cursor: pointer;
}
.custom-colors input[type="color"]::-webkit-color-swatch-wrapper {
  padding: 2px;
}
.custom-colors input[type="color"]::-webkit-color-swatch {
  border: 0;
  border-radius: 5px;
}
.custom-colors input[type="color"]::-moz-color-swatch {
  border: 0;
  border-radius: 5px;
}
.popover-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}
.action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 32px;
  padding: 7px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-muted);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  cursor: pointer;
}
.action.quiet {
  border-color: var(--line);
  background: transparent;
}
.action.primary {
  border-color: var(--accent-soft);
  background: var(--accent-dark);
  color: var(--on-accent);
}
.action:hover,
.action:focus-visible {
  filter: brightness(0.98);
}
</style>
