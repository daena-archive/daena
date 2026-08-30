<script lang="ts">
import type { EntitySummary, ModuleContext } from "../../../packages/module-api/src/index";
import type { Snippet } from "svelte";
import { Castle, ChevronRight, Plus, Search, UserPlus } from "@lucide/svelte";
import { PERSON_TYPE } from "./model.ts";
import { houseMemberCounts, listHouses } from "./fetch.ts";
import { createHouse, createMinimalPerson } from "./mutations.ts";

let {
  context,
  avatar,
  onSelect,
  onSelectHouse,
}: {
  context: ModuleContext;
  avatar?: Snippet<[string, string]>;
  onSelect: (person: EntitySummary) => void;
  onSelectHouse: (house: { id: string; name: string }) => void;
} = $props();

const pageSize = 20;
let peopleQuery = $state("");
let people = $state<EntitySummary[]>([]);
let peopleTotal = $state(0);
let peopleOffset = $state(0);
let peopleBusy = $state(false);
let peopleError = $state("");
let peopleToken = 0;
let createName = $state("");
let creatingPerson = $state(false);

let housesQuery = $state("");
let houses = $state<{ id: string; name: string }[]>([]);
let houseCounts = $state(new Map<string, number>());
let housesBusy = $state(false);
let housesError = $state("");
let createHouseName = $state("");
let creatingHouse = $state(false);
let housesToken = 0;

const visibleHouses = $derived.by(() => {
  const needle = housesQuery.trim().toLowerCase();
  if (!needle) return houses;
  return houses.filter((house) => house.name.toLowerCase().includes(needle));
});
const showCreatePerson = $derived(!peopleBusy && people.length === 0 && !peopleQuery.trim() && peopleTotal === 0);

async function searchPeople(nextOffset = 0) {
  const request = ++peopleToken;
  peopleBusy = true;
  peopleError = "";
  try {
    const page = await context.entities.query({
      types: [PERSON_TYPE],
      text: peopleQuery.trim() || undefined,
      sortField: "name",
      sortDirection: "asc",
      offset: nextOffset,
      limit: pageSize,
    });
    if (request !== peopleToken) return;
    people = page.items.filter((item) => !item.deleted);
    peopleTotal = page.total;
    peopleOffset = page.offset;
  } catch (cause) {
    if (request !== peopleToken) return;
    peopleError = cause instanceof Error ? cause.message : String(cause);
    people = [];
  } finally {
    if (request === peopleToken) peopleBusy = false;
  }
}

async function loadHouses() {
  const request = ++housesToken;
  housesBusy = true;
  housesError = "";
  try {
    const next = await listHouses(context);
    if (request !== housesToken) return;
    houses = next;
    houseCounts = await houseMemberCounts(
      context,
      next.map((house) => house.id),
    );
    if (request !== housesToken) return;
  } catch (cause) {
    if (request !== housesToken) return;
    housesError = cause instanceof Error ? cause.message : String(cause);
    houses = [];
    houseCounts = new Map();
  } finally {
    if (request === housesToken) housesBusy = false;
  }
}

async function createPerson() {
  if (creatingPerson || !createName.trim()) return;
  creatingPerson = true;
  peopleError = "";
  try {
    const person = await createMinimalPerson(context, createName, crypto.randomUUID());
    onSelect(person);
  } catch (cause) {
    peopleError = cause instanceof Error ? cause.message : String(cause);
  } finally {
    creatingPerson = false;
  }
}

async function addHouse() {
  if (creatingHouse || !createHouseName.trim()) return;
  creatingHouse = true;
  housesError = "";
  try {
    const created = await createHouse(context, createHouseName.trim(), crypto.randomUUID());
    createHouseName = "";
    houses = [...houses, { id: created.id, name: created.name }].sort(
      (left, right) => left.name.localeCompare(right.name) || left.id.localeCompare(right.id),
    );
    houseCounts = new Map(houseCounts).set(created.id, 0);
    onSelectHouse({ id: created.id, name: created.name });
  } catch (cause) {
    housesError = cause instanceof Error ? cause.message : String(cause);
  } finally {
    creatingHouse = false;
  }
}

function pickPerson(person: { id: string; name: string; revision?: string | null }) {
  onSelect({
    id: person.id as EntitySummary["id"],
    name: person.name,
    type: PERSON_TYPE,
    deleted: false,
    revision: person.revision ?? "",
  });
}

$effect(() => {
  void peopleQuery;
  const timer = setTimeout(() => void searchPeople(0), 180);
  return () => clearTimeout(timer);
});

$effect(() => {
  void loadHouses();
});
</script>

<section class="landing">
  <header class="landing-copy">
    <span class="overline">TREE</span>
    <h1>Choose a root</h1>
    <p>Open a person neighborhood, or the kinship tree of a house.</p>
  </header>

  <div class="columns">
    <section class="panel" aria-labelledby="landing-people-title">
      <div class="panel-heading">
        <div>
          <span class="panel-kicker">PEOPLE</span>
          <strong id="landing-people-title">{peopleTotal} {peopleTotal === 1 ? "person" : "people"}</strong>
        </div>
      </div>
      <label class="search-field">
        <span class="input-wrap">
          <span class="input-icon" aria-hidden="true"><Search size={14} strokeWidth={1.8} /></span>
          <input type="search" bind:value={peopleQuery} placeholder="Search people" aria-label="Search people" />
        </span>
      </label>
      <div class="panel-list" role="listbox" aria-label="People">
        {#if peopleError}<p class="error" role="alert">{peopleError}</p>{/if}
        {#if peopleBusy && people.length === 0}<p class="hint">Loading people…</p>
        {:else if showCreatePerson}
          <div class="create-cta">
            <p class="hint">No people yet. Create one — they also appear in Lore.</p>
            <label class="field">Name <input bind:value={createName} placeholder="Person name" /></label>
            <button
              type="button"
              class="primary-button"
              disabled={creatingPerson || !createName.trim()}
              onclick={() => void createPerson()}
              ><UserPlus size={14} strokeWidth={1.8} aria-hidden="true" /> Create person</button>
          </div>
        {:else if people.length === 0}<p class="hint">No people match this search.</p>
        {:else}
          {#each people as person (person.id)}
            <button type="button" class="collection-item" onclick={() => pickPerson(person)}>
              {#if avatar}
                <span class="item-glyph">{@render avatar(person.id, person.name)}</span>
              {:else}
                <span class="item-glyph fallback" aria-hidden="true">{person.name.slice(0, 1)}</span>
              {/if}
              <span class="item-copy"><strong>{person.name}</strong><small>Person</small></span>
              <span class="item-arrow" aria-hidden="true"><ChevronRight size={16} strokeWidth={1.8} /></span>
            </button>
          {/each}
        {/if}
      </div>
      {#if peopleTotal > people.length}
        <div class="pager">
          <button
            type="button"
            class="quiet-button"
            disabled={peopleOffset === 0}
            onclick={() => void searchPeople(Math.max(0, peopleOffset - pageSize))}>Previous</button>
          <button
            type="button"
            class="quiet-button"
            disabled={peopleOffset + people.length >= peopleTotal}
            onclick={() => void searchPeople(peopleOffset + pageSize)}>Next</button>
        </div>
      {/if}
    </section>

    <section class="panel" aria-labelledby="landing-houses-title">
      <div class="panel-heading">
        <div>
          <span class="panel-kicker">HOUSES</span>
          <strong id="landing-houses-title">{houses.length} {houses.length === 1 ? "house" : "houses"}</strong>
        </div>
      </div>
      <label class="search-field">
        <span class="input-wrap">
          <span class="input-icon" aria-hidden="true"><Search size={14} strokeWidth={1.8} /></span>
          <input type="search" bind:value={housesQuery} placeholder="Search houses" aria-label="Search houses" />
        </span>
      </label>
      <div class="panel-list" role="listbox" aria-label="Houses">
        {#if housesError}<p class="error" role="alert">{housesError}</p>{/if}
        {#if housesBusy && houses.length === 0}
          <p class="hint">Loading houses…</p>
        {:else if houses.length === 0}
          <div class="create-cta">
            <p class="hint">No houses yet. Create one to group people as a starting point.</p>
            <label class="field">Name <input bind:value={createHouseName} placeholder="House name" /></label>
            <button
              type="button"
              class="primary-button"
              disabled={creatingHouse || !createHouseName.trim()}
              onclick={() => void addHouse()}
              ><Plus size={14} strokeWidth={1.8} aria-hidden="true" /> Create house</button>
          </div>
        {:else if visibleHouses.length === 0}
          <p class="hint">No houses match this search.</p>
        {:else}
          {#each visibleHouses as house (house.id)}
            {@const count = houseCounts.get(house.id) ?? 0}
            <button type="button" class="collection-item" onclick={() => onSelectHouse(house)}>
              <span class="item-glyph house" aria-hidden="true"><Castle size={16} strokeWidth={1.8} /></span>
              <span class="item-copy"
                ><strong>{house.name}</strong><small>{count} {count === 1 ? "member" : "members"}</small></span>
              <span class="item-arrow" aria-hidden="true"><ChevronRight size={16} strokeWidth={1.8} /></span>
            </button>
          {/each}
        {/if}
      </div>
    </section>
  </div>
</section>

<style>
.landing {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-height: 0;
  height: 100%;
  gap: 18px;
  padding: 22px 24px 24px;
}
.landing-copy h1 {
  margin: 4px 0 0;
  color: var(--ink);
  font: 500 28px/1.15 var(--font-display, "Iowan Old Style", Palatino, serif);
}
.landing-copy p {
  max-width: 52ch;
  margin: 6px 0 0;
  color: var(--ink-muted);
  font-size: 12px;
  line-height: 1.5;
}
.overline,
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  min-height: 0;
  flex: 1 1 auto;
}
.panel {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.panel-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 16px 10px;
}
.panel-heading strong {
  display: block;
  margin-top: 4px;
  font: 500 22px var(--font-display, "Iowan Old Style", Palatino, serif);
}
.search-field {
  padding: 0 12px 8px;
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
.input-wrap input,
.field input {
  width: 100%;
  box-sizing: border-box;
  min-height: 34px;
  padding: 7px 10px 7px 32px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 13px;
}
.field input {
  padding-left: 10px;
}
.panel-list {
  display: grid;
  gap: 8px;
  min-height: 0;
  flex: 1 1 auto;
  align-content: start;
  overflow: auto;
  padding: 4px 12px 12px;
}
.collection-item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  min-height: 58px;
  margin: 0;
  padding: 9px 10px;
  overflow: hidden;
  border: 1px solid var(--theme-warning-border, #ebe7de);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  line-height: 1.2;
  text-align: left;
  cursor: pointer;
}
.collection-item:hover {
  border-color: var(--theme-warning-border, #e5d8c6);
  box-shadow: var(--shadow-sm);
}
.collection-item:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.25);
  outline-offset: 1px;
}
.item-glyph {
  display: grid;
  width: 40px;
  height: 40px;
  flex: 0 0 40px;
  place-items: center;
  overflow: hidden;
  border-radius: 10px;
}
.item-glyph.fallback,
.item-glyph.house {
  background: var(--theme-warning-bg, #f4f0e8);
  color: var(--accent-dark, var(--accent));
  font-size: 14px;
  font-weight: 700;
}
.item-copy {
  display: grid;
  min-width: 0;
  align-content: center;
  gap: 4px;
  overflow: hidden;
}
.item-copy strong {
  overflow: hidden;
  font-size: 13px;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-copy small {
  width: max-content;
  max-width: 220px;
  padding: 3px 6px;
  border-radius: 4px;
  background: var(--theme-warning-bg, #f4f0e8);
  color: var(--ink-faint);
  font-size: 10px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.item-arrow {
  flex: 0 0 10px;
  margin-left: auto;
  color: var(--theme-warning-text, #c3b6a4);
}
.collection-item:hover .item-arrow {
  color: var(--accent);
}
.hint,
.error {
  margin: 0;
  color: var(--ink-muted);
  font-size: 12px;
  line-height: 1.45;
}
.error {
  padding: 6px 8px;
  border: 1px solid var(--danger-line, #edcec5);
  border-radius: 8px;
  background: var(--danger-bg, #fff2ee);
  color: var(--theme-danger-text, #8a2b2b);
}
.create-cta,
.field {
  display: grid;
  gap: 8px;
}
.field {
  font-size: 12px;
}
.pager {
  display: flex;
  gap: 8px;
  padding: 8px 12px 12px;
  border-top: 1px solid var(--line);
}
.quiet-button,
.primary-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 34px;
  padding: 0 12px;
  border-radius: 8px;
  font-size: 12px;
  cursor: pointer;
}
.quiet-button {
  border: 1px solid var(--line);
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
}
.primary-button {
  border: 1px solid transparent;
  background: var(--accent-dark, var(--accent));
  color: #fff;
  font-weight: 600;
}
.quiet-button:disabled,
.primary-button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
@media (max-width: 900px) {
  .columns {
    grid-template-columns: 1fr;
  }
  .landing {
    height: auto;
    overflow: auto;
  }
}
</style>
