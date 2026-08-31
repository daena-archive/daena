<script lang="ts">
import { X } from "@lucide/svelte";
import type { FieldDefinition } from "../../packages/module-api/src/index";
import AsyncEntityPicker from "./entity-lifecycle/AsyncEntityPicker.svelte";
import {
  type AsyncEntityOption,
  type AsyncEntityResolveFn,
  type AsyncEntitySearchFn,
  type AsyncEntitySearchQuery,
} from "./entity-lifecycle/asyncEntityQuery.ts";

let {
  field,
  selectedIds,
  onChange,
  search,
  resolveSelected,
  placeholder = "Search and select entities…",
  hideChips = false,
  onCreate,
  pageSize = 20,
}: {
  field: FieldDefinition;
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  /** Backend-paged search. Must not rely on a full in-memory entity list. */
  search: AsyncEntitySearchFn;
  /** Resolve chip labels for selected IDs (exact reads / small cache). */
  resolveSelected?: AsyncEntityResolveFn;
  placeholder?: string;
  hideChips?: boolean;
  onCreate?: (name: string) => Promise<string | null>;
  pageSize?: number;
} = $props();

let creating = $state(false);
let selectedEntities = $state<AsyncEntityOption[]>([]);
let createQuery = $state("");

const entityTypes = $derived(field.targetEntityTypes?.length ? [...field.targetEntityTypes] : undefined);

const scopedSearch: AsyncEntitySearchFn = async (query: AsyncEntitySearchQuery) => {
  createQuery = query.text;
  return search({
    ...query,
    entityTypes: query.entityTypes ?? entityTypes,
    excludedEntityTypes: query.excludedEntityTypes,
    excludeIds: [...(query.excludeIds ?? []), ...selectedIds],
  });
};

function isOne() {
  return (field as { cardinality?: string }).cardinality === "one";
}

function toggle(entity: AsyncEntityOption, nextSelected: boolean) {
  if (isOne()) {
    onChange(nextSelected ? [entity.id] : []);
    return;
  }
  const next = new Set(selectedIds);
  if (nextSelected) next.add(entity.id);
  else next.delete(entity.id);
  onChange([...next]);
}

function remove(id: string) {
  onChange(selectedIds.filter((selectedId) => selectedId !== id));
}

function entityFor(id: string) {
  return selectedEntities.find((entity) => entity.id === id);
}

const canCreate = $derived(
  Boolean(onCreate) &&
    createQuery.trim().length > 0 &&
    !creating &&
    !selectedEntities.some((entity) => entity.name.toLowerCase() === createQuery.trim().toLowerCase()),
);

async function createNamed() {
  if (!onCreate || !canCreate) return;
  creating = true;
  try {
    const id = await onCreate(createQuery.trim());
    if (id) {
      if (isOne()) onChange([id]);
      else onChange([...new Set([...selectedIds, id])]);
    }
  } finally {
    creating = false;
  }
}

$effect(() => {
  const ids = [...selectedIds];
  let cancelled = false;
  void (async () => {
    if (ids.length === 0) {
      if (!cancelled) selectedEntities = [];
      return;
    }
    if (resolveSelected) {
      const resolved = await resolveSelected(ids);
      if (!cancelled) {
        selectedEntities = ids.map(
          (id) => resolved.find((entity) => entity.id === id) ?? { id, name: id, entityType: null },
        );
      }
      return;
    }
    if (!cancelled) {
      selectedEntities = ids.map((id) => ({ id, name: id, entityType: null }));
    }
  })();
  return () => {
    cancelled = true;
  };
});
</script>

<div class="relationship-picker">
  {#if selectedIds.length > 0 && !hideChips}
    <div class="relationship-selection" aria-label={`Selected ${field.label}`}>
      {#each selectedIds as id}
        {@const entity = entityFor(id)}
        <button
          type="button"
          class="relationship-chip"
          data-entity-id={id}
          onclick={() => remove(id)}
          title={`Remove ${entity?.name ?? id}`}>
          <span>{entity?.name ?? id}</span><b aria-hidden="true"><X size={12} strokeWidth={1.8} /></b>
        </button>
      {/each}
    </div>
  {/if}
  <AsyncEntityPicker
    search={scopedSearch}
    {entityTypes}
    {selectedIds}
    {pageSize}
    {placeholder}
    ariaLabel={field.label}
    emptyMessage="No matching entities."
    onToggle={(entity, selected) => toggle(entity, selected)} />
  {#if canCreate}
    <button type="button" class="relationship-create" onclick={() => void createNamed()}
      >{creating ? "Creating…" : `Create “${createQuery.trim()}”`}</button>
  {/if}
</div>

<style>
.relationship-picker {
  position: relative;
  display: grid;
  gap: 6px;
}
.relationship-selection {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}
.relationship-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: auto;
  margin: 0;
  padding: 5px 7px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 10px;
  cursor: pointer;
}
.relationship-chip:hover {
  border-color: var(--accent-soft);
  color: var(--ink);
}
.relationship-chip b {
  color: var(--accent);
  font-size: 13px;
  line-height: 1;
}
.relationship-create {
  width: 100%;
  min-height: var(--touch-target-min, 44px);
  padding: 8px 10px;
  border: 1px dashed var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
  font-size: 11px;
}
.relationship-create:hover {
  border-color: var(--accent);
  color: var(--ink);
}
</style>
