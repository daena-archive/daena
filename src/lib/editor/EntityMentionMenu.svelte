<script lang="ts">
import { tick } from "svelte";
import {
  createRequestGate,
  emptyAsyncEntityPage,
  runAsyncEntitySearch,
  type AsyncEntityOption,
  type AsyncEntitySearchFn,
  type AsyncEntitySearchPage,
} from "$lib/entity-lifecycle/asyncEntityQuery.ts";
import { humanizeId } from "$lib/schema-workbench/model.ts";

export let open = false;
export let query = "";
export let top = 0;
export let left = 0;
export let search: AsyncEntitySearchFn = async () => emptyAsyncEntityPage(8);
export let excludeIds: string[] = [];
export let keepLabel = false;
export let onSelect: (entity: AsyncEntityOption) => void = () => {};
export let onBrowse: () => void = () => {};
export let onWebUrl: (() => void) | null = null;
export let pageSize = 8;

let page: AsyncEntitySearchPage = emptyAsyncEntityPage(pageSize);
let busy = false;
let error = "";
let activeIndex = 0;
let rootEl: HTMLDivElement | null = null;
let searchTimer: ReturnType<typeof setTimeout> | null = null;
const gate = createRequestGate();

$: if (open) scheduleLoad(query);
$: if (!open && searchTimer) {
  clearTimeout(searchTimer);
  searchTimer = null;
}

export function moveActive(delta: number) {
  if (page.items.length === 0) return;
  const next = (activeIndex + delta + page.items.length) % page.items.length;
  activeIndex = next;
  void tick().then(() => rootEl?.querySelector(`[data-mention-index="${next}"]`)?.scrollIntoView({ block: "nearest" }));
}

export function confirmActive(): boolean {
  const entity = page.items[activeIndex] ?? page.items[0];
  if (!entity) return false;
  onSelect(entity);
  return true;
}

function scheduleLoad(text: string) {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => void load(text), text.trim() ? 80 : 0);
}

async function load(text: string) {
  busy = true;
  error = "";
  const result = await runAsyncEntitySearch(gate, search, {
    text: text.trim(),
    offset: 0,
    limit: pageSize,
    excludeIds,
    sortField: text.trim() ? "relevance" : "name",
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
  activeIndex = 0;
  busy = false;
}

function pick(entity: AsyncEntityOption) {
  onSelect(entity);
}

$: clampedLeft = Math.max(8, Math.min(left, typeof window === "undefined" ? left : window.innerWidth - 320));
$: clampedTop = Math.max(8, top);
</script>

{#if open}
  <div
    bind:this={rootEl}
    class="entity-mention-menu"
    style={`top: ${clampedTop}px; left: ${clampedLeft}px;`}
    role="listbox"
    tabindex="-1"
    aria-label="Entity suggestions"
    onmousedown={(event) => event.preventDefault()}>
    {#if error}
      <p class="entity-mention-status" role="alert">{error}</p>
    {:else if busy && page.items.length === 0}
      <p class="entity-mention-status">Searching…</p>
    {:else}
      {#each page.items as entity, index (entity.id)}
        <button
          type="button"
          role="option"
          data-mention-index={index}
          aria-selected={activeIndex === index}
          class:active={activeIndex === index}
          onmouseenter={() => (activeIndex = index)}
          onclick={() => pick(entity)}>
          <strong>{entity.name}</strong>
          <small>{entity.entityType ? humanizeId(entity.entityType) : "Uncategorized"}</small>
        </button>
      {:else}
        <p class="entity-mention-status">No matching entities.</p>
      {/each}
    {/if}
    <div class="entity-mention-footer">
      <button type="button" class="quiet" onclick={onBrowse}>Browse all…</button>
      {#if keepLabel && onWebUrl}
        <button type="button" class="quiet" onclick={onWebUrl}>Web URL…</button>
      {/if}
    </div>
  </div>
{/if}

<style>
.entity-mention-menu {
  position: fixed;
  z-index: 75;
  display: grid;
  gap: 2px;
  width: min(280px, calc(100vw - 16px));
  max-height: min(280px, calc(100vh - 24px));
  overflow: auto;
  padding: 6px;
  border: 1px solid var(--theme-warning-border, #d3c0a9);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 0 10px 24px rgba(48, 45, 38, 0.16);
}
.entity-mention-status {
  margin: 0;
  padding: 8px 10px;
  color: var(--ink-soft);
  font: 500 12px/1.4 var(--font-body, system-ui, sans-serif);
}
.entity-mention-menu button[role="option"] {
  display: grid;
  gap: 2px;
  width: 100%;
  padding: 7px 10px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.entity-mention-menu button[role="option"].active,
.entity-mention-menu button[role="option"]:hover {
  background: var(--surface-muted);
}
.entity-mention-menu strong {
  font: 700 13px/1.2 var(--font-body, system-ui, sans-serif);
}
.entity-mention-menu small {
  color: var(--ink-soft);
  font: 500 11px/1.2 var(--font-body, system-ui, sans-serif);
}
.entity-mention-footer {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
  padding-top: 4px;
  border-top: 1px solid var(--line);
}
.entity-mention-footer .quiet {
  min-height: 26px;
  padding: 0 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  font: 700 11px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.entity-mention-footer .quiet:hover,
.entity-mention-footer .quiet:focus-visible {
  background: var(--surface-muted);
  color: var(--ink);
  outline: 0;
}
</style>
