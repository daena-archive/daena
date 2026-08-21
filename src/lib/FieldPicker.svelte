<script lang="ts">
import { Check, Search, X } from "@lucide/svelte";

export type FieldPickerOption = {
  key: string;
  label: string;
  hint?: string;
};

let {
  options,
  selected = [],
  onChange,
  placeholder = "Search fields…",
  emptyLabel = "No matching fields.",
}: {
  options: FieldPickerOption[];
  selected?: string[];
  onChange: (keys: string[]) => void;
  placeholder?: string;
  emptyLabel?: string;
} = $props();

let query = $state("");
let open = $state(false);

function filtered(): FieldPickerOption[] {
  const q = query.trim().toLowerCase();
  return options.filter(
    (option) => !q || `${option.label} ${option.key} ${option.hint ?? ""}`.toLowerCase().includes(q),
  );
}

function isSelected(key: string): boolean {
  return selected.includes(key);
}

function toggle(key: string) {
  const next = new Set(selected);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  onChange([...next].sort());
}

function remove(key: string) {
  onChange(selected.filter((candidate) => candidate !== key));
}

function labelFor(key: string): string {
  return options.find((option) => option.key === key)?.label ?? key;
}
</script>

<div
  class="field-picker"
  onfocusout={(event) => {
    const next = event.relatedTarget as Node | null;
    const picker = event.currentTarget as HTMLElement;
    if (next && picker.contains(next)) return;
    window.setTimeout(() => {
      if (!picker.contains(document.activeElement)) open = false;
    }, 0);
  }}>
  <div class="picker-control" class:open>
    <span class="picker-search-icon" aria-hidden="true"><Search size={13} strokeWidth={1.8} /></span>
    <input
      type="text"
      aria-label={placeholder}
      bind:value={query}
      {placeholder}
      autocomplete="off"
      onfocus={() => (open = true)}
      oninput={() => (open = true)}
      onkeydown={(event) => {
        if (event.key === "Escape") open = false;
      }} />
    {#if selected.length > 0}
      <span class="picker-count">{selected.length}</span>
    {/if}
  </div>
  {#if open}
    <div class="picker-menu" role="listbox" aria-multiselectable="true" aria-label={placeholder}>
      {#each filtered() as option (option.key)}
        <button
          type="button"
          role="option"
          aria-selected={isSelected(option.key)}
          class:selected={isSelected(option.key)}
          onpointerdown={(event) => {
            event.preventDefault();
            toggle(option.key);
          }}
          onkeydown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              toggle(option.key);
            }
          }}>
          <span class="option-copy">
            <strong>{option.label}</strong>
            {#if option.hint}<small>{option.hint}</small>{/if}
          </span>
          {#if isSelected(option.key)}<Check size={13} strokeWidth={2.2} aria-hidden="true" />{/if}
        </button>
      {:else}
        <small class="picker-empty">{emptyLabel}</small>
      {/each}
    </div>
  {/if}
  {#if selected.length > 0}
    <div class="picker-chips">
      {#each selected as key (key)}
        <button type="button" class="picker-chip" title={`Remove ${labelFor(key)}`} onclick={() => remove(key)}>
          <span>{labelFor(key)}</span>
          <X size={11} strokeWidth={2} aria-hidden="true" />
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
.field-picker {
  position: relative;
  display: grid;
  gap: 8px;
}
.picker-control {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  padding: 0 11px;
  border: 1px solid #d9cdbd;
  border-radius: 9px;
  background: #fff;
  transition:
    border-color 0.14s ease,
    box-shadow 0.14s ease;
}
.picker-control:focus-within,
.picker-control.open {
  border-color: #b4773f;
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.12);
}
.picker-search-icon {
  display: grid;
  place-items: center;
  color: #b0a89c;
}
.picker-control input {
  flex: 1;
  min-width: 0;
  height: 34px;
  padding: 0;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--ink, #302c26);
  font:
    400 13px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.picker-control input::placeholder {
  color: #b0a89c;
}
.picker-count {
  display: inline-grid;
  place-items: center;
  min-width: 20px;
  height: 20px;
  padding: 0 6px;
  border-radius: 999px;
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #62594e;
  font:
    700 11px Inter,
    sans-serif;
}
.picker-menu {
  position: absolute;
  inset-inline: 0;
  top: calc(100% + 4px);
  z-index: 40;
  max-height: 224px;
  overflow-y: auto;
  padding: 4px;
  border: 1px solid #d9cdbd;
  border-radius: 10px;
  background: #fffefa;
  box-shadow: 0 14px 34px rgba(48, 44, 38, 0.16);
  display: grid;
  gap: 2px;
}
.picker-menu button {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: #62594e;
  text-align: left;
  cursor: pointer;
  font:
    500 12.5px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.picker-menu button:hover,
.picker-menu button.selected {
  background: #f4eee4;
  color: #3f3830;
}
.option-copy {
  display: grid;
  gap: 1px;
  min-width: 0;
}
.option-copy strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 600;
  font-size: 12.5px;
  color: var(--ink, #302c26);
}
.option-copy small {
  color: #b0a89c;
  font-size: 10.5px;
}
.picker-empty {
  display: block;
  padding: 10px;
  color: #b0a89c;
  font-size: 11.5px;
}
.picker-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.picker-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 9px;
  border: 1px solid #d9cdbd;
  border-radius: 999px;
  background: #f7f1e7;
  color: #62594e;
  cursor: pointer;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.picker-chip:hover {
  border-color: #b7a88f;
  background: #f4eee4;
  color: #3f3830;
}
</style>
