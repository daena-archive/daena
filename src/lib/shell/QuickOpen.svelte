<script lang="ts">
import { onMount, tick } from "svelte";
import { Clock3, Command, Compass, FileText, Plus, Search, X } from "@lucide/svelte";
import { groupQuickOpenItems, moveQuickOpenIndex, type QuickOpenItem } from "$lib/quick-open/model";
import { trapModalTab } from "./modalFocus";

interface Props {
  query: string;
  items: QuickOpenItem[];
  loading: boolean;
  onQueryChange: (query: string) => void;
  onSelect: (item: QuickOpenItem) => void;
  onClose: () => void;
}

let { query, items, loading, onQueryChange, onSelect, onClose }: Props = $props();
let dialog = $state<HTMLElement | null>(null);
let searchInput = $state<HTMLInputElement | null>(null);
let activeIndex = $state(0);
let groups = $derived(groupQuickOpenItems(items));
let orderedItems = $derived(groups.flatMap((group) => group.items));

$effect(() => {
  void query;
  void orderedItems;
  activeIndex = orderedItems.length > 0 ? 0 : -1;
});

onMount(() => {
  const returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  void tick().then(() => searchInput?.focus());
  return () => {
    void tick().then(() => returnFocus?.focus());
  };
});

function itemIcon(item: QuickOpenItem) {
  if (item.action.kind === "entity") return item.category === "Recent" ? Clock3 : FileText;
  if (item.action.kind === "destination") return Compass;
  if (item.action.kind === "create") return Plus;
  return Command;
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    onClose();
    return;
  }
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    activeIndex = moveQuickOpenIndex(activeIndex, event.key === "ArrowDown" ? 1 : -1, orderedItems.length);
    document.getElementById(`quick-open-item-${activeIndex}`)?.scrollIntoView({ block: "nearest" });
    return;
  }
  if (event.key === "Home" && orderedItems.length > 0) {
    event.preventDefault();
    activeIndex = 0;
    return;
  }
  if (event.key === "End" && orderedItems.length > 0) {
    event.preventDefault();
    activeIndex = orderedItems.length - 1;
    return;
  }
  if (event.key === "Enter" && activeIndex >= 0 && orderedItems[activeIndex]) {
    event.preventDefault();
    onSelect(orderedItems[activeIndex]);
    return;
  }
  trapModalTab(event, dialog);
}
</script>

<div class="quick-open-backdrop" role="presentation" onclick={onClose}>
  <div
    bind:this={dialog}
    class="quick-open"
    role="dialog"
    aria-modal="true"
    aria-labelledby="quick-open-title"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={handleKeydown}>
    <header class="quick-open-search">
      <Search size={18} strokeWidth={1.8} aria-hidden="true" />
      <label class="sr-only" for="quick-open-query" id="quick-open-title">Quick Open</label>
      <input
        bind:this={searchInput}
        id="quick-open-query"
        value={query}
        role="combobox"
        aria-expanded="true"
        aria-autocomplete="list"
        aria-controls="quick-open-results"
        aria-activedescendant={activeIndex >= 0 ? `quick-open-item-${activeIndex}` : undefined}
        autocomplete="off"
        placeholder="Find entries, destinations, templates, and commands"
        oninput={(event) => onQueryChange(event.currentTarget.value)} />
      {#if query}<button class="clear-query" type="button" aria-label="Clear query" onclick={() => onQueryChange("")}
          ><X size={15} strokeWidth={1.8} aria-hidden="true" /></button
        >{/if}<kbd>Esc</kbd>
    </header>

    <div id="quick-open-results" class="quick-open-results" role="listbox" aria-label="Quick Open results">
      {#if loading}<div class="quick-open-loading" role="status">Searching project…</div>{/if}
      {#each groups as group}
        <section class="quick-open-group" role="group" aria-labelledby={`quick-open-${group.category.toLowerCase()}`}>
          <h2 id={`quick-open-${group.category.toLowerCase()}`}>{group.category}</h2>
          {#each group.items as item (item.id)}
            {@const index = orderedItems.indexOf(item)}
            {@const Icon = itemIcon(item)}
            <button
              id={`quick-open-item-${index}`}
              type="button"
              role="option"
              aria-selected={index === activeIndex}
              class:active={index === activeIndex}
              class="quick-open-item"
              onmousemove={() => (activeIndex = index)}
              onclick={() => onSelect(item)}>
              <span class="quick-open-icon"><Icon size={16} strokeWidth={1.8} aria-hidden="true" /></span>
              <span><strong>{item.label}</strong><small>{item.description}</small></span>
              {#if index === activeIndex}<kbd>↵</kbd>{/if}
            </button>
          {/each}
        </section>
      {/each}
      {#if !loading && orderedItems.length === 0}<div class="quick-open-empty" role="status">
          <strong>No matches</strong><span>Try another name, destination, or command.</span>
        </div>{/if}
    </div>
    <footer>
      <span><kbd>↑</kbd><kbd>↓</kbd> Navigate</span><span><kbd>↵</kbd> Open</span><span><kbd>Esc</kbd> Close</span>
    </footer>
  </div>
</div>

<style>
.quick-open-backdrop {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: grid;
  align-items: start;
  justify-items: center;
  padding: min(14vh, 120px) 20px 20px;
  background: color-mix(in srgb, var(--ink) 34%, transparent);
  backdrop-filter: blur(3px);
}
.quick-open {
  width: min(680px, 100%);
  max-height: min(680px, calc(100vh - 80px));
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 14px;
  outline: 0;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.quick-open-search {
  display: flex;
  min-height: 58px;
  align-items: center;
  gap: 11px;
  padding: 0 16px;
  border-bottom: 1px solid var(--line);
  color: var(--ink-faint);
}
.quick-open-search:focus-within {
  box-shadow: inset 0 -3px 0 var(--focus-ring);
}
.quick-open-search input {
  min-width: 0;
  flex: 1;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--ink);
  font-size: 15px;
}
.clear-query {
  display: grid;
  width: 36px;
  height: 36px;
  padding: 0;
  place-items: center;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
}
.clear-query:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
kbd {
  padding: 2px 6px;
  border: 1px solid var(--line);
  border-radius: 5px;
  background: var(--canvas);
  color: var(--ink-faint);
  font: 500 10px var(--font-sans);
  box-shadow: 0 1px 0 var(--line);
}
.quick-open-results {
  max-height: min(530px, calc(100vh - 210px));
  overflow-y: auto;
  padding: 8px;
}
.quick-open-group + .quick-open-group {
  margin-top: 7px;
}
.quick-open-group h2 {
  margin: 0;
  padding: 7px 9px 5px;
  color: var(--ink-faint);
  font-size: 9px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}
.quick-open-item {
  display: grid;
  width: 100%;
  min-width: 0;
  min-height: 44px;
  grid-template-columns: 32px minmax(0, 1fr) auto;
  align-items: center;
  gap: 9px;
  padding: 9px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.quick-open-item:hover,
.quick-open-item.active {
  border-color: var(--line);
  background: var(--surface-muted);
}
.quick-open-icon {
  display: grid;
  width: 30px;
  height: 30px;
  place-items: center;
  border-radius: 8px;
  background: var(--canvas);
  color: var(--accent-dark);
}
.quick-open-item strong,
.quick-open-item small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.quick-open-item strong {
  font-size: 12px;
}
.quick-open-item small {
  margin-top: 2px;
  color: var(--ink-faint);
  font-size: 10px;
}
.quick-open-loading,
.quick-open-empty {
  padding: 28px 18px;
  color: var(--ink-faint);
  text-align: center;
  font-size: 11px;
}
.quick-open-empty strong,
.quick-open-empty span {
  display: block;
}
.quick-open-empty strong {
  margin-bottom: 5px;
  color: var(--ink-soft);
}
footer {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 38px;
  padding: 0 16px;
  border-top: 1px solid var(--line);
  background: var(--canvas);
  color: var(--ink-faint);
  font-size: 9px;
}
footer span {
  display: flex;
  align-items: center;
  gap: 4px;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
}
@media (max-width: 760px) {
  .quick-open-backdrop {
    align-items: stretch;
    padding: 10px;
  }
  .quick-open {
    width: 100%;
    max-height: calc(100vh - 20px);
  }
  .quick-open-results {
    max-height: calc(100vh - 145px);
  }
}
</style>
