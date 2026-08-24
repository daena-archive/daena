<script lang="ts">
import { tick } from "svelte";
import type { IconRef } from "../../../packages/plugin-sdk/src/generated";
import EntityIcon from "./EntityIcon.svelte";
import { CATALOG_ICON_OPTIONS, FALLBACK_ICON, validateUserSvg } from "./catalog";

let {
  value = FALLBACK_ICON,
  label = "Icon",
  onChange,
}: {
  value?: IconRef;
  label?: string;
  onChange: (value: IconRef) => void;
} = $props();

let open = $state(false);
let query = $state("");
let fileInput = $state<HTMLInputElement>();
let trigger = $state<HTMLButtonElement>();
let popover = $state<HTMLDivElement>();
let pickerRoot = $state<HTMLDivElement>();
let popoverStyle = $state("");
let svgError = $state("");
const selectedLabel = $derived(
  value.kind === "catalog"
    ? (CATALOG_ICON_OPTIONS.find((option) => option.id === value.id)?.label ?? "Unknown")
    : value.kind === "plugin-svg"
      ? "Plugin SVG"
      : "Custom SVG",
);
const filtered = $derived(
  CATALOG_ICON_OPTIONS.filter((option) => option.label.toLowerCase().includes(query.trim().toLowerCase())),
);

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
  popoverStyle = `left:${left}px;top:${top}px;max-height:${Math.max(180, window.innerHeight - top - margin)}px`;
}

async function togglePicker() {
  if (open) {
    closePicker();
    return;
  }
  popoverStyle = "";
  open = true;
  await tick();
  placePopover();
}

function closePicker() {
  open = false;
  query = "";
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
    closePicker();
  };
  const onKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape") closePicker();
  };
  window.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("keydown", onKeydown, true);
  return () => {
    window.removeEventListener("pointerdown", onPointerDown, true);
    window.removeEventListener("keydown", onKeydown, true);
  };
});
</script>

<div class="icon-picker" bind:this={pickerRoot}>
  <span class="picker-label">{label}</span>
  <button
    type="button"
    bind:this={trigger}
    class="picker-trigger"
    aria-expanded={open}
    aria-label={`Choose icon: ${selectedLabel}`}
    title={selectedLabel}
    onclick={togglePicker}>
    <EntityIcon icon={value} size={17} />
  </button>
  {#if open}
    <div class="picker-popover" bind:this={popover} style={popoverStyle}>
      <input bind:value={query} aria-label="Search icons" placeholder="Search icons…" />
      <div class="icon-grid">
        {#each filtered as option}
          <button
            type="button"
            class:selected={value.kind === "catalog" && value.id === option.id}
            title={option.label}
            aria-label={option.label}
            onclick={() => {
              onChange({ kind: "catalog", id: option.id });
              closePicker();
            }}>
            <EntityIcon icon={{ kind: "catalog", id: option.id }} size={18} />
          </button>
        {/each}
      </div>
      {#if filtered.length === 0}<span class="empty">No matching icons</span>{/if}
      <div class="svg-upload">
        <button type="button" onclick={() => fileInput?.click()}>Use SVG file</button>
        <input
          bind:this={fileInput}
          type="file"
          accept="image/svg+xml,.svg"
          onchange={async (event) => {
            svgError = "";
            const input = event.currentTarget;
            const file = input.files?.[0];
            input.value = "";
            if (!file) return;
            const svg = await file.text();
            svgError = validateUserSvg(svg) ?? "";
            if (!svgError) {
              onChange({ kind: "user-svg", svg });
              closePicker();
            }
          }} />
        {#if svgError}<span>{svgError}</span>{/if}
      </div>
    </div>
  {/if}
</div>

<style>
.icon-picker {
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
  color: var(--ink);
  font: inherit;
  cursor: pointer;
}
.picker-popover {
  position: fixed;
  z-index: 90;
  width: min(300px, calc(100vw - 24px));
  overflow: auto;
  padding: 9px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.picker-popover[style=""] {
  visibility: hidden;
}
.picker-popover input {
  width: 100%;
  box-sizing: border-box;
  margin-bottom: 8px;
}
.icon-grid {
  display: grid;
  grid-template-columns: repeat(7, 34px);
  gap: 4px;
  max-height: 190px;
  overflow-y: auto;
}
.icon-grid button {
  display: grid;
  width: 34px;
  height: 34px;
  place-items: center;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
.icon-grid button:hover,
.icon-grid button:focus-visible {
  background: var(--surface-muted);
  color: var(--ink);
}
.icon-grid button.selected {
  border-color: var(--accent-soft);
  background: var(--accent-dark);
  color: var(--on-accent);
}
.empty {
  display: block;
  padding: 12px 4px 5px;
  color: var(--ink-faint);
  font-size: 11px;
}
.svg-upload {
  display: grid;
  gap: 5px;
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--line);
}
.svg-upload input {
  display: none;
}
.svg-upload button {
  justify-self: start;
  border: 0;
  background: transparent;
  color: var(--accent);
  font: inherit;
  font-weight: 700;
  cursor: pointer;
}
.svg-upload span {
  color: var(--danger);
  font-size: 10px;
}
</style>
