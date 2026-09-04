<script lang="ts">
import type { EntitySummary, ModuleContext } from "../../../packages/module-api/src/index";
import { UserPlus } from "@lucide/svelte";
import AsyncEntityPicker from "$lib/entity-lifecycle/AsyncEntityPicker.svelte";
import { toAsyncEntityPage, type AsyncEntityOption } from "$lib/entity-lifecycle/asyncEntityQuery.ts";
import { HOUSE_TYPE, PERSON_TYPE } from "./model";
import { createMinimalPerson } from "./mutations";

let {
  context,
  onSelect,
  compact = false,
  dropdown = false,
  recents = [],
  kind = "person",
}: {
  context: ModuleContext;
  onSelect: (person: EntitySummary) => void;
  compact?: boolean;
  dropdown?: boolean;
  recents?: { id: string; name: string }[];
  kind?: "person" | "house";
} = $props();

let createName = $state("");
let creating = $state(false);
let error = $state("");
let emptyCatalog = $state(false);

const entityType = $derived(kind === "house" ? HOUSE_TYPE : PERSON_TYPE);

async function searchEntities(query: { text: string; offset: number; limit: number; excludeIds?: string[] }) {
  const page = await context.entities.query({
    types: [entityType],
    text: query.text || undefined,
    sortField: "name",
    sortDirection: "asc",
    offset: query.offset,
    limit: query.limit,
  });
  const normalized = toAsyncEntityPage(page, { excludeIds: query.excludeIds });
  if (!query.text && query.offset === 0) emptyCatalog = normalized.total === 0;
  return normalized;
}

function toSummary(entity: AsyncEntityOption): EntitySummary {
  return {
    id: entity.id as EntitySummary["id"],
    name: entity.name,
    type: entity.entityType ?? entityType,
    deleted: false,
    revision: entity.revision ?? "",
  };
}

function pick(entity: AsyncEntityOption) {
  onSelect(toSummary(entity));
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

const showCreate = $derived(kind === "person" && emptyCatalog && !dropdown);
</script>

<div class="picker" class:compact class:dropdown>
  {#if !compact}<span class="overline">{kind === "house" ? "House" : "Root person"}</span>{/if}
  <AsyncEntityPicker
    search={searchEntities}
    entityTypes={[entityType]}
    pageSize={20}
    {dropdown}
    openOnFocus={dropdown || compact}
    placeholder={kind === "house" ? "Search houses" : "Search Lore people"}
    ariaLabel={kind === "house" ? "Search houses" : "Search Lore people"}
    emptyMessage={kind === "house" ? "No houses match this search." : "No Lore people match this search."}
    resultsSectionLabel={kind === "house" ? "Houses" : "People"}
    recents={recents.map((entry) => ({ ...entry, entityType }))}
    onSelect={pick} />
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if showCreate}
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
  {/if}
</div>

<style>
.picker {
  display: grid;
  gap: 10px;
  max-width: 420px;
}
.picker.compact,
.picker.dropdown {
  max-width: none;
}
.picker.dropdown {
  position: relative;
  min-width: 220px;
  max-width: 320px;
  gap: 6px;
}
.overline {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
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
</style>
