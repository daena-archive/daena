<script lang="ts">
import type { EntitySummary, ModuleContext } from "../../../packages/module-api/src/index";
import { Search, UserPlus } from "@lucide/svelte";
import { PERSON_TYPE } from "./model";

import { createMinimalPerson } from "./mutations";

let {
  context,
  onSelect,
  compact = false,
  dropdown = false,
  recents = [],
}: {
  context: ModuleContext;
  onSelect: (person: EntitySummary) => void;
  compact?: boolean;
  dropdown?: boolean;
  recents?: { id: string; name: string }[];
} = $props();

let createName = $state("");
let creating = $state(false);
let open = $state(false);
let rootEl = $state<HTMLElement | null>(null);

let query = $state("");
let results = $state<EntitySummary[]>([]);
let total = $state(0);
let offset = $state(0);
let busy = $state(false);
let error = $state("");
let token = 0;
const pageSize = 20;

async function search(nextOffset = 0) {
  const request = ++token;
  busy = true;
  error = "";
  try {
    const page = await context.entities.query({
      types: [PERSON_TYPE],
      text: query.trim() || undefined,
      sortField: "name",
      sortDirection: "asc",
      offset: nextOffset,
      limit: pageSize,
    });
    if (request !== token) return;
    results = page.items.filter((item) => !item.deleted);
    total = page.total;
    offset = page.offset;
  } catch (cause) {
    if (request !== token) return;
    error = cause instanceof Error ? cause.message : String(cause);
    results = [];
  } finally {
    if (request === token) busy = false;
  }
}

async function createPerson() {
  if (creating || !createName.trim()) return;
  creating = true;
  error = "";
  try {
    const person = await createMinimalPerson(context, createName, crypto.randomUUID());
    onSelect(person);
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    creating = false;
  }
}

function pick(person: EntitySummary) {
  open = false;
  onSelect(person);
}

function pickRecent(entry: { id: string; name: string }) {
  pick({
    id: entry.id as EntitySummary["id"],
    name: entry.name,
    type: PERSON_TYPE,
    deleted: false,
    revision: "",
  });
}

$effect(() => {
  void query;
  const timer = setTimeout(() => void search(0), 180);
  return () => clearTimeout(timer);
});

$effect(() => {
  if (!dropdown || !open) return;
  function onPointer(event: PointerEvent) {
    if (rootEl?.contains(event.target as Node)) return;
    open = false;
  }
  window.addEventListener("pointerdown", onPointer);
  return () => window.removeEventListener("pointerdown", onPointer);
});

const showMenu = $derived(!dropdown || open);
const showCreate = $derived(!dropdown && results.length === 0 && !query.trim() && total === 0);
const showRecents = $derived(dropdown && open && !query.trim() && recents.length > 0);
</script>

<div class="picker" class:compact class:dropdown bind:this={rootEl}>
  <label class="search-field">
    {#if !compact}<span class="overline">Root person</span>{/if}
    <span class="input-wrap">
      <span class="input-icon" aria-hidden="true"><Search size={14} strokeWidth={1.8} /></span>
      <input
        type="search"
        bind:value={query}
        placeholder="Search Lore people"
        aria-label="Search Lore people"
        onfocus={() => {
          if (dropdown) open = true;
        }} />
    </span>
  </label>
  {#if showMenu}
    <div class="menu" role="listbox" aria-label="People results">
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      {#if showRecents}
        <span class="section">Recent</span>
        <ul>
          {#each recents as entry (entry.id)}
            <li>
              <button type="button" class="result" onclick={() => pickRecent(entry)}>{entry.name}</button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if busy && results.length === 0}<p class="hint">Searching…</p>
      {:else if showCreate}
        <div class="create-cta">
          <p class="hint">No Lore people yet. Create a person — they will also appear in Lore.</p>
          <label class="field">Name <input bind:value={createName} placeholder="Person name" /></label>
          <button
            type="button"
            class="primary-button"
            disabled={creating || !createName.trim()}
            onclick={() => void createPerson()}
            ><UserPlus size={14} strokeWidth={1.8} aria-hidden="true" /> Create person</button>
        </div>
      {:else if results.length === 0}<p class="hint">No Lore people match this search.</p>
      {:else}
        {#if showRecents}<span class="section">People</span>{/if}
        <ul>
          {#each results as person (person.id)}
            <li>
              <button type="button" class="result" onclick={() => pick(person)}>{person.name}</button>
            </li>
          {/each}
        </ul>
        {#if total > results.length}
          <div class="pager">
            <button
              type="button"
              class="quiet-button"
              disabled={offset === 0}
              onclick={() => void search(Math.max(0, offset - pageSize))}>Previous</button>
            <button
              type="button"
              class="quiet-button"
              disabled={offset + results.length >= total}
              onclick={() => void search(offset + pageSize)}>Next</button>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
.picker {
  display: grid;
  gap: 10px;
  max-width: 420px;
}
.picker.compact {
  max-width: none;
}
.picker.dropdown {
  position: relative;
  min-width: 220px;
  max-width: 320px;
  gap: 0;
}
.picker.dropdown .menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  left: 0;
  z-index: 20;
  display: grid;
  gap: 6px;
  max-height: 320px;
  overflow: auto;
  padding: 6px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-lg, 0 12px 32px rgba(0, 0, 0, 0.09));
}
.search-field {
  display: grid;
  gap: 6px;
}
.overline {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.input-wrap {
  position: relative;
  display: flex;
  align-items: center;
}
.input-icon {
  position: absolute;
  left: 9px;
  display: grid;
  place-items: center;
  color: var(--ink-muted);
  pointer-events: none;
}
.input-wrap input {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 10px 8px 32px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 13px;
}
.input-wrap input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
  border-color: var(--accent);
}
.picker.dropdown .input-wrap input {
  min-height: 32px;
  padding-block: 4px;
  border-radius: 8px;
}
ul {
  display: grid;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}
button.result {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--line-soft, var(--line));
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
  font-size: 13px;
}
button.result:hover {
  border-color: var(--line-strong);
  background: var(--surface-muted);
}
.section {
  padding: 4px 2px 0;
  color: var(--ink-muted);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.field {
  display: grid;
  gap: 4px;
  font-size: 12px;
}
.field input {
  padding: 7px 9px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
}
.create-cta {
  display: grid;
  gap: 8px;
  padding: 4px 2px;
}
.hint,
.error {
  margin: 0;
  color: var(--ink-muted);
  font: 12px/1.4 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.error {
  color: var(--theme-danger-text, #8a2b2b);
  background: var(--danger-bg, #fff2ee);
  border: 1px solid var(--danger-line, #edcec5);
  border-radius: 8px;
  padding: 6px 8px;
}
.pager {
  display: flex;
  gap: 8px;
  padding-top: 4px;
}
.quiet-button {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
  font-size: 11px;
  cursor: pointer;
}
.quiet-button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.primary-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  justify-content: center;
  padding: 8px 12px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: var(--accent-dark, var(--accent));
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.primary-button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
@media (prefers-reduced-motion: reduce) {
  .picker,
  .menu {
    transition: none;
  }
}
</style>
