<script lang="ts">
import { tick } from "svelte";
import { X } from "@lucide/svelte";
import type { Entity } from "$lib/project/client";
import {
  createRequestGate,
  emptyAsyncEntityPage,
  runAsyncEntitySearch,
  type AsyncEntityOption,
  type AsyncEntitySearchFn,
  type AsyncEntitySearchPage,
} from "$lib/entity-lifecycle/asyncEntityQuery.ts";

export let open = false;
export let search: AsyncEntitySearchFn = async () => emptyAsyncEntityPage();
/** Optional warm cache for resolving the initially selected entity name when editing. */
export let entities: Entity[] = [];
export let initialQuery = "";
export let initialSelectedId = "";
export let initialLabel = "";
export let initialIsCustom = false;
export let onInsert: (entity: AsyncEntityOption, label: string, isCustom: boolean) => void = () => {};
export let onCancel: () => void = () => {};
export let pageSize = 20;

let query = "";
let selectedId = "";
let selectedEntity: AsyncEntityOption | null = null;
let label = "";
let isCustom = false;
let searchInput: HTMLInputElement;
let labelInput: HTMLInputElement;
let wasOpen = false;
let lastFocused: Element | null = null;
let page: AsyncEntitySearchPage = emptyAsyncEntityPage(pageSize);
let busy = false;
let error = "";
let searchTimer: ReturnType<typeof setTimeout> | null = null;
const gate = createRequestGate();

async function load(nextOffset = 0) {
  busy = true;
  error = "";
  const result = await runAsyncEntitySearch(gate, search, {
    text: query.trim(),
    offset: nextOffset,
    limit: pageSize,
    sortField: "name",
    sortDirection: "asc",
  });
  if ("stale" in result) return;
  if ("error" in result) {
    error = result.error instanceof Error ? result.error.message : String(result.error);
    page = emptyAsyncEntityPage(pageSize);
    busy = false;
    return;
  }
  page = result.page;
  if (selectedId) {
    const match = page.items.find((entity) => entity.id === selectedId);
    if (match) selectedEntity = match;
  }
  busy = false;
}

function scheduleLoad(immediate = false) {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => void load(0), immediate || !query.trim() ? 0 : 180);
}

function select(entity: AsyncEntityOption) {
  selectedId = entity.id;
  selectedEntity = entity;
  label = entity.name;
}

function submit() {
  if (!selectedEntity) return;
  if (isCustom) {
    if (!label.trim()) return;
    onInsert(selectedEntity, label.trim(), true);
  } else {
    onInsert(selectedEntity, selectedEntity.name, false);
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    onCancel();
    return;
  }
  if (event.key === "Tab") {
    trapFocus(event);
    return;
  }
  if (event.target === searchInput && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
    event.preventDefault();
    const current = page.items.findIndex((entity) => entity.id === selectedId);
    const offset = event.key === "ArrowDown" ? 1 : -1;
    const next = current < 0 ? (offset > 0 ? 0 : page.items.length - 1) : current + offset;
    const entity = page.items[Math.max(0, Math.min(page.items.length - 1, next))];
    if (entity) {
      select(entity);
      void tick().then(() =>
        document.querySelector(`[data-entity-result="${CSS.escape(entity.id)}"]`)?.scrollIntoView({ block: "nearest" }),
      );
    }
    return;
  }
  if (event.key === "Enter" && (event.target as HTMLElement | null)?.closest("button")) return;
  if (event.key === "Enter" && !selectedEntity && page.items[0]) select(page.items[0]);
  const canSubmit = selectedEntity && (isCustom ? label.trim() : true);
  if (event.key === "Enter" && !event.shiftKey && canSubmit) {
    event.preventDefault();
    submit();
  }
}

function trapFocus(event: KeyboardEvent) {
  const dialog = event.currentTarget as HTMLElement;
  const focusable = [
    ...dialog.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  ].filter((element) => !element.hasAttribute("hidden"));
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (!dialog.contains(document.activeElement)) {
    event.preventDefault();
    (event.shiftKey ? last : first).focus();
  } else if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

$: {
  if (!open) {
    wasOpen = false;
    if (searchTimer) {
      clearTimeout(searchTimer);
      searchTimer = null;
    }
  } else if (!wasOpen) {
    query = initialQuery;
    selectedId = initialSelectedId;
    label = initialLabel;
    isCustom = initialIsCustom;
    selectedEntity = null;
    page = emptyAsyncEntityPage(pageSize);
    if (initialSelectedId) {
      const cached = entities.find((entity) => entity.id === initialSelectedId && !entity.deleted);
      if (cached) {
        selectedEntity = {
          id: cached.id,
          name: cached.name,
          entityType: cached.entity_type,
          revision: cached.revision,
        };
        if (!isCustom) label = cached.name;
      }
    }
    wasOpen = true;
    lastFocused = document.activeElement;
    void tick().then(() => {
      if (isCustom) labelInput?.focus();
      else searchInput?.focus();
      if (isCustom && labelInput) labelInput.select();
    });
  }
}

$: if (open && wasOpen) {
  void query;
  scheduleLoad();
}

$: if (selectedEntity && !isCustom && label !== selectedEntity.name) {
  label = selectedEntity.name;
}

$: if (!open && lastFocused) {
  if (lastFocused instanceof HTMLElement && lastFocused.isConnected) lastFocused.focus();
  lastFocused = null;
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
        <button type="button" aria-label="Close entity reference dialog" onclick={onCancel}
          ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </header>
      <label class="entity-reference-search">
        <span>Search entities</span>
        <input bind:this={searchInput} bind:value={query} placeholder="Search by name or type…" />
      </label>
      <div class="entity-reference-results" role="listbox" aria-label="Entity results">
        {#if error}
          <p role="alert">{error}</p>
        {:else if busy && page.items.length === 0}
          <p>Searching…</p>
        {:else}
          {#each page.items as entity (entity.id)}
            <button
              type="button"
              role="option"
              data-entity-result={entity.id}
              aria-selected={selectedId === entity.id}
              class:selected={selectedId === entity.id}
              onclick={() => select(entity)}>
              <strong>{entity.name}</strong><small>{entity.entityType ?? "Uncategorized"}</small>
            </button>
          {:else}
            <p>No matching entities.</p>
          {/each}
          {#if page.total > page.items.length || page.hasMore || page.offset > 0}
            <div class="entity-reference-pager">
              <button
                type="button"
                class="quiet"
                disabled={busy || page.offset === 0}
                onclick={() => void load(Math.max(0, page.offset - pageSize))}>Previous</button>
              <button
                type="button"
                class="quiet"
                disabled={busy || (!page.hasMore && page.offset + page.items.length >= page.total)}
                onclick={() => void load(page.offset + pageSize)}>Next</button>
            </div>
          {/if}
        {/if}
      </div>
      <div class="entity-reference-custom-toggle">
        <label class="custom-checkbox">
          <input
            type="checkbox"
            bind:checked={isCustom}
            onchange={() => {
              if (!isCustom && selectedEntity) label = selectedEntity.name;
              if (isCustom) void tick().then(() => labelInput?.focus());
            }} />
          <span>Use custom display text</span>
        </label>
        <div class="hint-wrapper">
          <button
            type="button"
            class="hint-button"
            aria-label="About custom display text"
            aria-describedby="custom-hint-tooltip">?</button>
          <div id="custom-hint-tooltip" class="hint-tooltip" role="tooltip">
            When using the entity name, the link updates automatically when the entity is renamed. Custom labels stay as
            written.
          </div>
        </div>
      </div>
      <label class="entity-reference-label">
        <span>Display text</span>
        <input
          bind:this={labelInput}
          bind:value={label}
          placeholder={isCustom
            ? "How this reference appears in the document"
            : (selectedEntity?.name ?? "Entity name (auto)")}
          disabled={!isCustom} />
      </label>
      <footer>
        <button type="button" class="quiet" onclick={onCancel}>Cancel</button>
        <button type="button" class="primary" disabled={!selectedEntity || (isCustom && !label.trim())} onclick={submit}
          >{initialSelectedId && initialIsCustom !== undefined ? "Update reference" : "Insert reference"}</button>
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
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
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
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
h2 {
  margin: 3px 0 0;
  color: var(--ink);
  font: 700 20px/1.2 var(--font-display, Georgia, serif);
}
header button,
footer button {
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
header button {
  width: 30px;
  height: 30px;
  font-size: 20px;
}
header button:hover,
header button:focus-visible {
  background: var(--surface-muted);
  color: var(--ink);
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
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font: 500 13px/1 var(--font-body, system-ui, sans-serif);
  outline: 0;
}
.entity-reference-search input:focus,
.entity-reference-label input:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.entity-reference-custom-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
}
.custom-checkbox {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  user-select: none;
}
.custom-checkbox input {
  width: 16px;
  height: 16px;
  accent-color: var(--accent-dark);
}
.custom-checkbox span {
  color: var(--ink);
  font: 600 12px/1 var(--font-body, system-ui, sans-serif);
  text-transform: none;
  letter-spacing: 0;
}
.hint-wrapper {
  position: relative;
  flex: 0 0 auto;
}
.hint-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 50%;
  background: var(--surface);
  color: var(--ink-soft);
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.hint-button:hover,
.hint-button:focus-visible {
  border-color: var(--accent);
  color: var(--accent-dark);
  outline: 0;
}
.hint-tooltip {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  width: 260px;
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--ink);
  color: var(--surface);
  font: 400 11px/1.4 var(--font-body, system-ui, sans-serif);
  box-shadow: 0 8px 20px rgba(38, 42, 33, 0.18);
  opacity: 0;
  visibility: hidden;
  transform: translateY(4px);
  transition:
    opacity 0.15s ease,
    transform 0.15s ease,
    visibility 0.15s;
  z-index: 2;
  pointer-events: none;
}
.hint-tooltip::after {
  content: "";
  position: absolute;
  top: -5px;
  right: 6px;
  width: 10px;
  height: 10px;
  background: var(--ink);
  transform: rotate(45deg);
}
.hint-wrapper:hover .hint-tooltip,
.hint-button:focus-visible + .hint-tooltip,
.hint-button:focus + .hint-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateY(0);
  pointer-events: auto;
}
.entity-reference-label input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  background: var(--surface-muted);
}
.entity-reference-results {
  display: grid;
  align-content: start;
  grid-auto-rows: min-content;
  min-height: 116px;
  max-height: 260px;
  overflow-y: auto;
  padding: 4px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
}
.entity-reference-results button {
  display: grid;
  gap: 2px;
  width: 100%;
  padding: 9px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.entity-reference-results button:hover,
.entity-reference-results button.selected {
  background: var(--surface);
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.05));
}
.entity-reference-results strong {
  font-size: 13px;
}
.entity-reference-results small,
.entity-reference-results p {
  color: var(--ink-soft);
  font-size: 11px;
}
.entity-reference-results p {
  align-self: center;
  margin: 0;
  padding: 12px;
  text-align: center;
}
.entity-reference-pager {
  display: flex;
  gap: 6px;
  padding: 6px 4px 2px;
}
.entity-reference-pager .quiet {
  flex: 1;
  min-height: 32px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink-soft);
  font: 600 11px/1 var(--font-body, system-ui, sans-serif);
}
.entity-reference-pager .quiet:disabled {
  opacity: 0.45;
  cursor: not-allowed;
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
  background: var(--surface-muted);
  color: var(--ink);
  outline: 0;
}
footer .primary {
  background: var(--accent-dark);
  color: white;
}
footer .primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
