<script lang="ts">
import { Archive, RotateCcw, Trash2 } from "@lucide/svelte";
import { onMount } from "svelte";
import { confirmDialog } from "$lib/dialogs.svelte";
import { formatRuntimeTimestampLabel } from "$lib/date";
import { project, type Entity } from "$lib/project/client";

let {
  typeLabel,
  onChanged,
  onToast,
}: {
  typeLabel: (entityType: string | null) => string;
  onChanged?: () => void;
  onToast?: (message: string) => void;
} = $props();

let items = $state<Entity[]>([]);
let loading = $state(true);
let busyId = $state<string | null>(null);

async function loadArchived() {
  loading = true;
  try {
    const loaded: Entity[] = [];
    let offset = 0;
    const limit = 200;
    while (true) {
      const page = await project.queryEntities({
        archived: true,
        sortField: "updated_at",
        sortDirection: "desc",
        offset,
        limit,
      });
      loaded.push(...page.items);
      if (!page.has_more) break;
      offset += page.items.length;
    }
    items = loaded;
  } catch (cause) {
    onToast?.(cause instanceof Error ? cause.message : String(cause));
  } finally {
    loading = false;
  }
}

async function restoreEntry(entry: Entity) {
  if (busyId) return;
  if (
    !(await confirmDialog({
      title: `Restore "${entry.name}"?`,
      message: "Returns the entry to the workspace and search.",
      confirmLabel: "Restore",
    }))
  )
    return;

  busyId = entry.id;
  try {
    await project.restoreEntity(entry.id, { expectedRevision: entry.revision });
    items = items.filter((item) => item.id !== entry.id);
    onToast?.(`"${entry.name}" restored.`);
    onChanged?.();
  } catch (cause) {
    onToast?.(cause instanceof Error ? cause.message : String(cause));
  } finally {
    busyId = null;
  }
}

async function purgeEntry(entry: Entity) {
  if (busyId) return;
  if (
    !(await confirmDialog({
      title: `Delete "${entry.name}" permanently?`,
      message: "Removes the entry, its content, and relationships. This cannot be undone.",
      confirmLabel: "Delete permanently",
      danger: true,
    }))
  )
    return;

  busyId = entry.id;
  try {
    await project.purgeEntity(entry.id, { expectedRevision: entry.revision });
    items = items.filter((item) => item.id !== entry.id);
    onToast?.(`"${entry.name}" deleted permanently.`);
    onChanged?.();
  } catch (cause) {
    onToast?.(cause instanceof Error ? cause.message : String(cause));
  } finally {
    busyId = null;
  }
}

onMount(() => {
  void loadArchived();
});
</script>

<div class="section-heading">
  <span class="heading-icon"><Archive size={17} /></span>
  <div>
    <span class="kicker">ARCHIVE</span>
    <h2>Archived entries</h2>
    <p>Entries archived from the workspace are kept here until you restore or permanently delete them.</p>
  </div>
</div>

{#if loading}
  <p class="archive-status">Loading archived entries…</p>
{:else if items.length === 0}
  <section class="operation-card">
    <p class="archive-empty">No archived entries.</p>
  </section>
{:else}
  <section class="archive-list" aria-label="Archived entries">
    {#each items as entry (entry.id)}
      <article class="archive-row">
        <div class="archive-copy">
          <strong>{entry.name}</strong>
          <small>{typeLabel(entry.entity_type)} · Archived {formatRuntimeTimestampLabel(entry.updated_at)}</small>
        </div>
        <div class="action-row">
          <button
            type="button"
            class="quiet-button"
            disabled={busyId !== null}
            onclick={() => void restoreEntry(entry)}
            ><RotateCcw size={14} /> Restore</button>
          <button
            type="button"
            class="danger-button"
            disabled={busyId !== null}
            onclick={() => void purgeEntry(entry)}
            ><Trash2 size={14} /> Delete permanently</button>
        </div>
      </article>
    {/each}
  </section>
{/if}

<style>
.section-heading {
  display: flex;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.heading-icon {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  flex: 0 0 38px;
  border-radius: 10px;
  background: var(--surface-warm);
  color: var(--accent-dark);
}
.kicker {
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.1em;
}
.section-heading h2 {
  margin: 4px 0 6px;
  color: var(--ink);
  font: 600 19px var(--font-display);
}
.section-heading p,
.archive-status,
.archive-empty {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12.5px;
  line-height: 1.5;
}
.operation-card {
  display: grid;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
}
.archive-list {
  display: grid;
  gap: 10px;
}
.archive-row {
  display: grid;
  gap: 12px;
  padding: 14px 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
}
.archive-copy {
  min-width: 0;
  display: grid;
  gap: 4px;
}
.archive-copy strong {
  color: var(--ink);
  font-size: 14px;
}
.archive-copy small {
  color: var(--ink-soft);
  font-size: 11px;
}
.action-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.quiet-button,
.danger-button {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 9px 12px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.quiet-button {
  border: 1px solid var(--line);
  background: var(--surface);
  color: var(--ink-soft);
}
.danger-button {
  border: 1px solid var(--danger-line);
  background: var(--danger-bg);
  color: var(--danger);
}
button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}
@media (min-width: 640px) {
  .archive-row {
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
  }
}
</style>
