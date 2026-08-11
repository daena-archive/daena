<script lang="ts">
import { tick } from "svelte";
import type { Entity } from "$lib/project/client";

export let open = false;
export let entities: Entity[] = [];
export let initialQuery = "";
export let initialSelectedId = "";
export let initialLabel = "";
export let onInsert: (entity: Entity, label: string) => void = () => {};
export let onCancel: () => void = () => {};

let query = "";
let selectedId = "";
let label = "";
let searchInput: HTMLInputElement;
let labelInput: HTMLInputElement;
let wasOpen = false;
let filteredEntities: Entity[] = [];

$: filteredEntities = entities
  .filter((entity) => !entity.deleted)
  .filter(
    (entity) =>
      !query.trim() || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(query.trim().toLowerCase()),
  );

function selectedEntity(): Entity | null {
  return (
    filteredEntities.find((entity) => entity.id === selectedId) ??
    entities.find((entity) => entity.id === selectedId) ??
    null
  );
}

function select(entity: Entity) {
  selectedId = entity.id;
  label = entity.name;
}

function submit() {
  const entity = selectedEntity();
  if (!entity || !label.trim()) return;
  onInsert(entity, label.trim());
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    onCancel();
  }
  if (event.key === "Enter" && !event.shiftKey && selectedEntity() && label.trim()) {
    event.preventDefault();
    submit();
  }
}

$: {
  if (!open) {
    wasOpen = false;
  } else if (!wasOpen) {
    query = initialQuery;
    selectedId = initialSelectedId;
    label = initialLabel;
    wasOpen = true;
    void tick().then(() => (initialLabel ? labelInput?.focus() : searchInput?.focus()));
  }
}
</script>

{#if open}
  <div class="entity-reference-backdrop" role="presentation" onclick={onCancel}>
    <div
      class="entity-reference-dialog"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="entity-reference-title"
      onclick={(event) => event.stopPropagation()}
      onkeydown={handleKeydown}>
      <header>
        <div>
          <span>ENTITY REFERENCE</span>
          <h2 id="entity-reference-title">Link to another entity</h2>
        </div>
        <button type="button" aria-label="Close entity reference dialog" onclick={onCancel}>×</button>
      </header>
      <label class="entity-reference-search">
        <span>Search entities</span>
        <input bind:this={searchInput} bind:value={query} placeholder="Search by name or type…" />
      </label>
      <div class="entity-reference-results" role="listbox" aria-label="Entity results">
        {#each filteredEntities as entity}
          <button
            type="button"
            role="option"
            aria-selected={selectedId === entity.id}
            class:selected={selectedId === entity.id}
            onclick={() => select(entity)}>
            <strong>{entity.name}</strong><small>{entity.entity_type ?? "Uncategorized"}</small>
          </button>
        {:else}
          <p>No matching entities.</p>
        {/each}
      </div>
      <label class="entity-reference-label">
        <span>Display text</span>
        <input bind:this={labelInput} bind:value={label} placeholder="How this reference appears in the document" />
      </label>
      <footer>
        <button type="button" class="quiet" onclick={onCancel}>Cancel</button>
        <button type="button" class="primary" disabled={!selectedEntity() || !label.trim()} onclick={submit}
          >Insert reference</button>
      </footer>
    </div>
  </div>
{/if}

<style>
.entity-reference-backdrop {
  position: fixed;
  z-index: 80;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 18px;
  background: rgba(37, 37, 31, 0.28);
}
.entity-reference-dialog {
  width: min(540px, 100%);
  display: grid;
  gap: 14px;
  max-height: min(700px, calc(100vh - 36px));
  padding: 20px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 12px;
  background: var(--surface, #fffefa);
  box-shadow: 0 24px 64px rgba(38, 42, 33, 0.24);
}
header,
footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
header span,
label > span {
  display: block;
  color: var(--accent, #b4773f);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
h2 {
  margin: 3px 0 0;
  color: var(--ink, #25251f);
  font: 700 20px/1.2 var(--font-display, Georgia, serif);
}
header button,
footer button {
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft, #77766d);
  cursor: pointer;
}
header button {
  width: 30px;
  height: 30px;
  font-size: 20px;
}
header button:hover,
header button:focus-visible {
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink, #25251f);
  outline: 0;
}
.entity-reference-search,
.entity-reference-label {
  display: grid;
  gap: 6px;
}
.entity-reference-search input,
.entity-reference-label input {
  width: 100%;
  height: 38px;
  padding: 0 10px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 7px;
  background: var(--canvas, #f7f6f2);
  color: var(--ink, #25251f);
  font: 500 13px/1 var(--font-body, system-ui, sans-serif);
  outline: 0;
}
.entity-reference-search input:focus,
.entity-reference-label input:focus {
  border-color: #c99965;
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.entity-reference-results {
  display: grid;
  align-content: start;
  grid-auto-rows: min-content;
  min-height: 116px;
  max-height: 260px;
  overflow-y: auto;
  padding: 4px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--canvas, #f7f6f2);
}
.entity-reference-results button {
  display: grid;
  gap: 2px;
  width: 100%;
  padding: 9px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink, #25251f);
  text-align: left;
  cursor: pointer;
}
.entity-reference-results button:hover,
.entity-reference-results button.selected {
  background: var(--surface, #fffefa);
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.05));
}
.entity-reference-results strong {
  font-size: 13px;
}
.entity-reference-results small,
.entity-reference-results p {
  color: var(--ink-soft, #77766d);
  font-size: 11px;
}
.entity-reference-results p {
  align-self: center;
  margin: 0;
  padding: 12px;
  text-align: center;
}
footer {
  justify-content: flex-end;
}
footer button {
  min-height: 34px;
  padding: 0 11px;
  font: 700 12px/1 var(--font-body, system-ui, sans-serif);
}
footer .quiet:hover,
footer .quiet:focus-visible {
  background: var(--surface-muted, #f4f2ec);
  color: var(--ink, #25251f);
  outline: 0;
}
footer .primary {
  background: var(--accent-dark, #365342);
  color: white;
}
footer .primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
