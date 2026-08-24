<script lang="ts">
import { RotateCcw } from "@lucide/svelte";
import type { EntityTypeColor, IconRef } from "../../../packages/plugin-sdk/src/generated";
import IconPicker from "$lib/entity-icons/IconPicker.svelte";
import TypeColorPicker from "$lib/entity-colors/TypeColorPicker.svelte";

export interface TypeAppearanceValue {
  icon: IconRef;
  iconColor: EntityTypeColor;
}

let {
  value,
  onChange,
  compact = false,
  showReset = false,
  onReset,
}: {
  value: TypeAppearanceValue;
  onChange: (value: TypeAppearanceValue) => void;
  compact?: boolean;
  showReset?: boolean;
  onReset?: () => void;
} = $props();
</script>

<div class="type-appearance-picker" class:compact>
  {#if compact && showReset && onReset}
    <button type="button" class="reset-button" aria-label="Reset to package default" onclick={onReset}>
      <RotateCcw size={12} strokeWidth={1.8} aria-hidden="true" />
    </button>
  {/if}
  <div class="appearance-controls">
    <IconPicker label="Icon" value={value.icon} onChange={(icon) => onChange({ ...value, icon })} />
    <TypeColorPicker
      label="Color"
      value={value.iconColor}
      onChange={(iconColor) => onChange({ ...value, iconColor })} />
  </div>
  {#if !compact && showReset && onReset}
    <button type="button" class="reset-button" aria-label="Reset to package default" onclick={onReset}>
      <RotateCcw size={12} strokeWidth={1.8} aria-hidden="true" />
      Reset
    </button>
  {/if}
</div>

<style>
.type-appearance-picker {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: end;
}
.reset-button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  cursor: pointer;
}
.type-appearance-picker.compact {
  flex: 1;
  flex-wrap: nowrap;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.appearance-controls {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: end;
}
.type-appearance-picker.compact .appearance-controls {
  flex-wrap: nowrap;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}
.type-appearance-picker.compact :global(.icon-picker),
.type-appearance-picker.compact :global(.color-picker) {
  display: block;
}
.type-appearance-picker.compact :global(.picker-label) {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
.type-appearance-picker.compact .reset-button {
  padding: 7px;
}
.reset-button:hover,
.reset-button:focus-visible {
  border-color: var(--line-strong);
  color: var(--ink);
  background: var(--surface-muted);
}
</style>
