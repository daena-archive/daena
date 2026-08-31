<script lang="ts">
import { Search } from "@lucide/svelte";
import {
  createRequestGate,
  emptyAsyncEntityPage,
  runAsyncEntitySearch,
  type AsyncEntityOption,
  type AsyncEntitySearchFn,
  type AsyncEntitySearchPage,
  type AsyncEntitySortDirection,
  type AsyncEntitySortField,
} from "./asyncEntityQuery.ts";

let {
  search,
  entityTypes = undefined,
  excludedEntityTypes = undefined,
  excludeIds = [],
  selectedIds = [],
  pageSize = 20,
  debounceMs = 180,
  placeholder = "Search…",
  ariaLabel = "Search entities",
  openOnFocus = true,
  dropdown = true,
  disabled = false,
  emptyMessage = "No matching entities.",
  searchingMessage = "Searching…",
  sortField = "name" as AsyncEntitySortField,
  sortDirection = "asc" as AsyncEntitySortDirection,
  recents = [],
  onSelect,
  onToggle,
}: {
  search: AsyncEntitySearchFn;
  entityTypes?: string[];
  excludedEntityTypes?: string[];
  excludeIds?: string[];
  selectedIds?: string[];
  pageSize?: number;
  debounceMs?: number;
  placeholder?: string;
  ariaLabel?: string;
  openOnFocus?: boolean;
  /** When true, results appear in an absolute menu under the input. */
  dropdown?: boolean;
  disabled?: boolean;
  emptyMessage?: string;
  searchingMessage?: string;
  sortField?: AsyncEntitySortField;
  sortDirection?: AsyncEntitySortDirection;
  recents?: Array<{ id: string; name: string; entityType?: string | null }>;
  /** Single-select style: choose one row and close. */
  onSelect?: (entity: AsyncEntityOption) => void;
  /** Multi-select style: toggle membership without requiring close. */
  onToggle?: (entity: AsyncEntityOption, selected: boolean) => void;
} = $props();

let query = $state("");
let open = $state(false);
let busy = $state(false);
let error = $state("");
// svelte-ignore state_referenced_locally: initial page reflects initial pageSize prop
let page = $state<AsyncEntitySearchPage>(emptyAsyncEntityPage(pageSize));
let rootEl = $state<HTMLElement | null>(null);
let activeIndex = $state(-1);
const listboxId = `async-entity-listbox-${Math.random().toString(36).slice(2, 9)}`;
const gate = createRequestGate();

const selected = $derived(new Set(selectedIds));
const excluded = $derived(new Set(excludeIds));
const showMenu = $derived(!dropdown || open);
const showRecents = $derived(showMenu && !query.trim() && recents.length > 0);
const optionItems = $derived.by(() => {
  if (!showMenu) return [] as Array<{ key: string; entity: AsyncEntityOption }>;
  const items: Array<{ key: string; entity: AsyncEntityOption }> = [];
  if (showRecents) {
    for (const entry of recents) {
      items.push({
        key: `recent:${entry.id}`,
        entity: { id: entry.id, name: entry.name, entityType: entry.entityType ?? null },
      });
    }
  }
  for (const entity of page.items) {
    items.push({ key: `result:${entity.id}`, entity });
  }
  return items;
});
const activeOptionId = $derived(
  activeIndex >= 0 && activeIndex < optionItems.length ? `${listboxId}-option-${activeIndex}` : undefined,
);

$effect(() => {
  void optionItems;
  if (!showMenu || optionItems.length === 0) {
    activeIndex = -1;
    return;
  }
  if (activeIndex < 0 || activeIndex >= optionItems.length) activeIndex = 0;
});

function optionDomId(index: number) {
  return `${listboxId}-option-${index}`;
}

function moveActive(delta: number) {
  if (optionItems.length === 0) {
    activeIndex = -1;
    return;
  }
  if (activeIndex < 0) {
    activeIndex = delta > 0 ? 0 : optionItems.length - 1;
  } else {
    activeIndex = (activeIndex + delta + optionItems.length) % optionItems.length;
  }
  document.getElementById(optionDomId(activeIndex))?.scrollIntoView({ block: "nearest" });
}

function activateActive() {
  if (activeIndex < 0 || activeIndex >= optionItems.length) return;
  choose(optionItems[activeIndex].entity);
}

async function load(nextOffset = 0) {
  if (disabled) return;
  busy = true;
  error = "";
  const result = await runAsyncEntitySearch(gate, search, {
    text: query.trim(),
    offset: nextOffset,
    limit: pageSize,
    entityTypes,
    excludedEntityTypes,
    excludeIds: [...excluded],
    sortField,
    sortDirection,
  });
  if ("stale" in result) return;
  if ("error" in result) {
    error = result.error instanceof Error ? result.error.message : String(result.error);
    page = emptyAsyncEntityPage(pageSize);
    busy = false;
    return;
  }
  page = result.page;
  busy = false;
}

function isSelected(id: string) {
  return selected.has(id);
}

function choose(entity: AsyncEntityOption) {
  if (onToggle) {
    onToggle(entity, !isSelected(entity.id));
    return;
  }
  onSelect?.(entity);
  if (dropdown) open = false;
  query = "";
}

$effect(() => {
  void query;
  void entityTypes?.join("\0");
  void excludedEntityTypes?.join("\0");
  void [...excluded].sort().join("\0");
  void pageSize;
  void sortField;
  void sortDirection;
  void disabled;
  if (dropdown && !open) return;
  const delay = query.trim() ? debounceMs : 0;
  const timer = setTimeout(() => void load(0), delay);
  return () => clearTimeout(timer);
});

$effect(() => {
  if (!dropdown || !open) return;
  const onPointer = (event: PointerEvent) => {
    if (rootEl?.contains(event.target as Node)) return;
    open = false;
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      open = false;
    }
  };
  window.addEventListener("pointerdown", onPointer, true);
  window.addEventListener("keydown", onKey, true);
  return () => {
    window.removeEventListener("pointerdown", onPointer, true);
    window.removeEventListener("keydown", onKey, true);
  };
});
</script>

<div
  class="async-entity-picker"
  class:dropdown
  bind:this={rootEl}
  onfocusout={(event) => {
    if (!dropdown) return;
    const next = event.relatedTarget as Node | null;
    const picker = event.currentTarget as HTMLElement;
    if (next && picker.contains(next)) return;
    window.setTimeout(() => {
      if (!picker.contains(document.activeElement)) open = false;
    }, 0);
  }}>
  <div class="async-entity-search">
    <span aria-hidden="true"><Search size={14} strokeWidth={1.8} /></span>
    <input
      type="search"
      role="combobox"
      aria-label={ariaLabel}
      aria-expanded={showMenu}
      aria-autocomplete="list"
      aria-controls={listboxId}
      aria-activedescendant={activeOptionId}
      {placeholder}
      {disabled}
      value={query}
      autocomplete="off"
      onfocus={() => {
        if (openOnFocus) open = true;
      }}
      oninput={(event) => {
        query = event.currentTarget.value;
        if (openOnFocus || dropdown) open = true;
      }}
      onkeydown={(event) => {
        if (event.key === "Escape" && dropdown) {
          event.preventDefault();
          open = false;
          return;
        }
        if (!showMenu) {
          if (event.key === "ArrowDown" || event.key === "ArrowUp") {
            event.preventDefault();
            open = true;
          }
          return;
        }
        if (event.key === "ArrowDown") {
          event.preventDefault();
          moveActive(1);
          return;
        }
        if (event.key === "ArrowUp") {
          event.preventDefault();
          moveActive(-1);
          return;
        }
        if (event.key === "Home" && optionItems.length > 0) {
          event.preventDefault();
          activeIndex = 0;
          return;
        }
        if (event.key === "End" && optionItems.length > 0) {
          event.preventDefault();
          activeIndex = optionItems.length - 1;
          return;
        }
        if (event.key === "Enter") {
          event.preventDefault();
          activateActive();
        }
      }} />
  </div>
  {#if showMenu}
    <div class="async-entity-menu" id={listboxId} role="listbox" aria-label={ariaLabel}>
      {#if error}
        <p class="async-entity-error" role="alert">{error}</p>
      {/if}
      {#if showRecents}
        <span class="async-entity-section">Recent</span>
      {/if}
      {#if busy && page.items.length === 0 && !showRecents}
        <p class="async-entity-hint">{searchingMessage}</p>
      {:else if optionItems.length === 0}
        <p class="async-entity-hint">{emptyMessage}</p>
      {:else}
        {#each optionItems as item, index (item.key)}
          {#if showRecents && index === recents.length}
            <span class="async-entity-section">People</span>
          {/if}
          <button
            type="button"
            id={optionDomId(index)}
            role="option"
            tabindex="-1"
            aria-selected={isSelected(item.entity.id) || index === activeIndex}
            class:selected={isSelected(item.entity.id)}
            class:active={index === activeIndex}
            {disabled}
            onmousemove={() => (activeIndex = index)}
            onpointerdown={(event) => {
              event.preventDefault();
              choose(item.entity);
            }}>
            <span><strong>{item.entity.name}</strong><small>{item.entity.entityType ?? "Uncategorized"}</small></span>
            {#if isSelected(item.entity.id)}<b aria-hidden="true">✓</b>{/if}
          </button>
        {/each}
        {#if page.total > page.items.length || page.hasMore || page.offset > 0}
          <div class="async-entity-pager">
            <button
              type="button"
              class="quiet-button"
              disabled={busy || page.offset === 0}
              onclick={() => void load(Math.max(0, page.offset - pageSize))}>Previous</button>
            <button
              type="button"
              class="quiet-button"
              disabled={busy || (!page.hasMore && page.offset + page.items.length >= page.total)}
              onclick={() => void load(page.offset + pageSize)}>Next</button>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
.async-entity-picker {
  position: relative;
  display: grid;
  gap: 6px;
}
.async-entity-search {
  display: flex;
  align-items: center;
  gap: 7px;
  box-sizing: border-box;
  min-height: 34px;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas, var(--surface-muted));
  color: var(--ink-faint);
}
.async-entity-search:focus-within {
  border-color: var(--accent-soft, var(--accent));
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.async-entity-search input {
  min-width: 0;
  width: 100%;
  padding: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--ink);
  font-size: 11px;
}
.async-entity-menu {
  display: grid;
  gap: 2px;
  max-height: 220px;
  overflow-y: auto;
  padding: 4px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: var(--shadow-lg, 0 12px 32px rgba(0, 0, 0, 0.09));
}
.async-entity-picker.dropdown .async-entity-menu {
  position: absolute;
  inset-inline: 0;
  top: calc(100% + 4px);
  z-index: 20;
}
.async-entity-menu button[role="option"] {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  min-height: var(--touch-target-min, 44px);
  padding: 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
}
.async-entity-menu button[role="option"]:hover,
.async-entity-menu button[role="option"].selected,
.async-entity-menu button[role="option"].active {
  background: var(--surface-muted);
  color: var(--ink);
}
.async-entity-menu strong,
.async-entity-menu small {
  display: block;
}
.async-entity-menu strong {
  font-size: 11px;
}
.async-entity-menu small {
  margin-top: 2px;
  color: var(--ink-faint);
  font-size: 9px;
}
.async-entity-menu button[role="option"] > b {
  color: var(--accent);
}
.async-entity-hint,
.async-entity-error {
  margin: 0;
  padding: 10px 8px;
  color: var(--ink-faint);
  font-size: 10px;
}
.async-entity-section {
  padding: 6px 8px 2px;
  color: var(--ink-muted, var(--ink-faint));
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.async-entity-error {
  color: var(--danger, #8a2b2b);
}
.async-entity-pager {
  display: flex;
  gap: 6px;
  padding: 4px;
}
.quiet-button {
  flex: 1;
  min-height: 32px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink-soft);
  font-size: 11px;
  cursor: pointer;
}
.quiet-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
