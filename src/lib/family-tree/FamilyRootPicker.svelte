<script lang="ts">
import type { EntitySummary, ModuleContext } from "../../../packages/module-api/src/index";
import { PERSON_TYPE } from "./model";

let {
  context,
  onSelect,
  compact = false,
}: {
  context: ModuleContext;
  onSelect: (person: EntitySummary) => void;
  compact?: boolean;
} = $props();

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

$effect(() => {
  void query;
  const timer = setTimeout(() => void search(0), 180);
  return () => clearTimeout(timer);
});
</script>

<div class="picker" class:compact>
  <label>
    {#if !compact}<span class="overline">Root person</span>{/if}
    <input type="search" bind:value={query} placeholder="Search Lore people" aria-label="Search Lore people" />
  </label>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if busy && results.length === 0}<p class="hint">Searching…</p>
  {:else if results.length === 0}<p class="hint">No Lore people match this search.</p>
  {:else}
    <ul>
      {#each results as person (person.id)}
        <li>
          <button type="button" onclick={() => onSelect(person)}>{person.name}</button>
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

<style>
.picker {
  display: grid;
  gap: 12px;
  max-width: 420px;
}
.picker.compact {
  max-width: none;
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
