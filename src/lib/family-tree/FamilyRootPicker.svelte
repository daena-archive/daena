<script lang="ts">
import type { EntitySummary, ModuleContext } from "../../../packages/module-api/src/index";
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
  <label>
    {#if !compact}<span class="overline">Root person</span>{/if}
    <input
      type="search"
      bind:value={query}
      placeholder="Search Lore people"
      aria-label="Search Lore people"
      onfocus={() => {
        if (dropdown) open = true;
      }} />
  </label>
  {#if showMenu}
    <div class="menu">
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      {#if showRecents}
        <span class="section">Recent</span>
        <ul>
          {#each recents as entry (entry.id)}
            <li>
              <button type="button" onclick={() => pickRecent(entry)}>{entry.name}</button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if busy && results.length === 0}<p class="hint">Searching…</p>
      {:else if showCreate}
        <p class="hint">No Lore people yet. Create a person — they will also appear in Lore.</p>
        <label>Name <input bind:value={createName} /></label>
        <button
          type="button"
          class="primary-button"
          disabled={creating || !createName.trim()}
          onclick={() => void createPerson()}>Create person</button>
      {:else if results.length === 0}<p class="hint">No Lore people match this search.</p>
      {:else}
        {#if showRecents}<span class="section">People</span>{/if}
        <ul>
          {#each results as person (person.id)}
            <li>
              <button type="button" onclick={() => pick(person)}>{person.name}</button>
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
  gap: 12px;
  max-width: 420px;
}
.picker.compact {
  max-width: none;
}
.picker.dropdown {
  position: relative;
  min-width: 220px;
  max-width: 280px;
  gap: 0;
}
.picker.dropdown .menu {
  position: absolute;
  top: calc(100% + 4px);
  right: 0;
  left: 0;
  z-index: 6;
  display: grid;
  gap: 6px;
  max-height: 280px;
  overflow: auto;
  padding: 8px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
}
.overline {
  display: block;
  margin-bottom: 6px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.18em;
}
input {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 10px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
}
.picker.dropdown input {
  min-height: 32px;
  padding: 4px 8px;
}
ul {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
button {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--line-soft);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.section {
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.hint,
.error {
  margin: 0;
  color: var(--ink-muted);
  font:
    12px/1.4 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.error {
  color: var(--theme-danger-text, #8a2b2b);
}
.pager {
  display: flex;
  gap: 8px;
}
</style>
