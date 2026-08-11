<script lang="ts">
import type { Entity } from "$lib/project/client";
import type { FieldDefinition } from "../../packages/module-api/src/index";

let {
  field,
  entities,
  selectedIds,
  onChange,
  placeholder = "Search and select entities…",
}: {
  field: FieldDefinition;
  entities: Entity[];
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  placeholder?: string;
} = $props();

let query = $state("");
let open = $state(false);

function candidates() {
  const normalizedQuery = query.trim().toLowerCase();
  return entities
    .filter((entity) => !entity.deleted)
    .filter((entity) => !field.targetEntityTypes || field.targetEntityTypes.includes(entity.entity_type ?? ""))
    .filter(
      (entity) =>
        !normalizedQuery || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(normalizedQuery),
    );
}

function isSelected(id: string) {
  return selectedIds.includes(id);
}

function toggle(id: string) {
  const next = new Set(selectedIds);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  onChange([...next]);
  query = "";
  open = true;
}

function remove(id: string) {
  onChange(selectedIds.filter((selectedId) => selectedId !== id));
}

function entityFor(id: string) {
  return entities.find((entity) => entity.id === id);
}
</script>

<div
  class="relationship-picker"
  onfocusout={(event) => {
    const next = event.relatedTarget as Node | null;
    const picker = event.currentTarget as HTMLElement;
    if (next && picker.contains(next)) return;
    window.setTimeout(() => {
      if (!picker.contains(document.activeElement)) open = false;
    }, 0);
  }}>
  {#if selectedIds.length > 0}
    <div class="relationship-selection" aria-label={`Selected ${field.label}`}>
      {#each selectedIds as id}
        {@const entity = entityFor(id)}
        {#if entity}
          <button
            type="button"
            class="relationship-chip"
            data-entity-id={id}
            onclick={() => remove(id)}
            title={`Remove ${entity.name}`}>
            <span>{entity.name}</span><b aria-hidden="true">×</b>
          </button>
        {/if}
      {/each}
    </div>
  {/if}
  <div class="relationship-search">
    <span aria-hidden="true">⌕</span>
    <input
      aria-label={field.label}
      value={query}
      {placeholder}
      onfocus={() => (open = true)}
      oninput={(event) => {
        query = (event.currentTarget as HTMLInputElement).value;
        open = true;
      }}
      onkeydown={(event) => {
        if (event.key === "Escape") open = false;
      }} />
  </div>
  {#if open}
    <div class="relationship-menu" role="listbox" aria-label={`${field.label} options`}>
      {#each candidates() as entity}
        <button
          type="button"
          role="option"
          aria-selected={isSelected(entity.id)}
          class:selected={isSelected(entity.id)}
          onpointerdown={(event) => {
            event.preventDefault();
            toggle(entity.id);
          }}
          onkeydown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              toggle(entity.id);
            }
          }}>
          <span><strong>{entity.name}</strong><small>{entity.entity_type ?? "Uncategorized"}</small></span>
          {#if isSelected(entity.id)}<b aria-hidden="true">✓</b>{/if}
        </button>
      {:else}
        <small class="relationship-empty">No matching entities.</small>
      {/each}
    </div>
  {/if}
</div>

<style>
.relationship-picker {
  position: relative;
  margin-top: 10px;
}
.relationship-selection {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-bottom: 6px;
}
.relationship-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: auto;
  margin: 0;
  padding: 5px 7px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 10px;
  cursor: pointer;
}
.relationship-chip:hover {
  border-color: #c99965;
  color: var(--ink);
}
.relationship-chip b {
  color: var(--accent);
  font-size: 13px;
  line-height: 1;
}
.relationship-search {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 8px 9px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink-faint);
}
.relationship-search:focus-within {
  border-color: #c99965;
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.relationship-search input {
  min-width: 0;
  width: 100%;
  padding: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--ink);
  font-size: 11px;
}
.relationship-menu {
  position: absolute;
  inset-inline: 0;
  top: calc(100% + 4px);
  z-index: 8;
  max-height: 190px;
  overflow-y: auto;
  padding: 4px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.relationship-menu button {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  padding: 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
}
.relationship-menu button:hover,
.relationship-menu button.selected {
  background: var(--surface-muted);
  color: var(--ink);
}
.relationship-menu strong,
.relationship-menu small {
  display: block;
}
.relationship-menu strong {
  font-size: 11px;
}
.relationship-menu small {
  margin-top: 2px;
  color: var(--ink-faint);
  font-size: 9px;
}
.relationship-menu button > b {
  color: var(--accent);
}
.relationship-empty {
  display: block;
  padding: 10px 8px;
  color: var(--ink-faint);
  font-size: 10px;
}
</style>
