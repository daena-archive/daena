<script lang="ts">
import { formatCalendarDate } from "$lib/date";
import type { FamilyPerson, RelativeRole } from "./model.ts";

let {
  person,
  isRoot,
  houses = [],
  hideAddChild = false,
  connections = [],
  onOpen,
  onMakeRoot,
  onAddRelative,
  onSelectPerson,
  onSelectRelationship,
  onClose,
}: {
  person: FamilyPerson;
  isRoot: boolean;
  houses?: string[];
  hideAddChild?: boolean;
  connections?: Array<{ id: string; label: string; otherId: string; relationshipId: string }>;
  onOpen: (id: string) => void;
  onMakeRoot: (id: string) => void;
  onAddRelative: (id: string, role: RelativeRole) => void;
  onSelectPerson: (id: string) => void;
  onSelectRelationship: (id: string) => void;
  onClose: () => void;
} = $props();

function lifeSpan(value: FamilyPerson) {
  const birth = value.birth ? formatCalendarDate(value.birth) : "";
  const death = value.death ? formatCalendarDate(value.death) : "";
  if (!birth && !death) return null;
  if (birth && death) return `${birth} – ${death}`;
  return birth || death;
}
</script>

<aside class="panel" aria-label={person.name}>
  <header>
    <div>
      <strong>{person.name}</strong>
      {#if isRoot}<em>Root</em>{/if}
      {#if lifeSpan(person)}<span>{lifeSpan(person)}</span>{/if}
      {#if person.secondaryLabel}<span>{person.secondaryLabel}</span>{/if}
      {#if houses.length}<span class="house">{houses.join(" · ")}</span>{/if}
    </div>
    <button type="button" class="quiet-button" onclick={onClose}>Close</button>
  </header>
  <div class="actions">
    <button type="button" class="quiet-button" onclick={() => onOpen(person.id)}>Open in Lore</button>
    {#if !isRoot}
      <button type="button" class="quiet-button" onclick={() => onMakeRoot(person.id)}>Make root</button>
    {/if}
  </div>
  <div class="actions">
    <button type="button" class="quiet-button" onclick={() => onAddRelative(person.id, "parent")}>Add parent</button>
    {#if !hideAddChild}
      <button type="button" class="quiet-button" onclick={() => onAddRelative(person.id, "child")}>Add child</button>
    {/if}
    <button type="button" class="quiet-button" onclick={() => onAddRelative(person.id, "partner")}>Add partner</button>
  </div>
  {#if connections.length > 0}
    <div class="connections" aria-label="Visible connections">
      <span>On this tree</span>
      <ul>
        {#each connections as connection (connection.id)}
          <li>
            <button type="button" class="quiet-button" onclick={() => onSelectPerson(connection.otherId)}>
              {connection.label}
            </button>
            <button type="button" class="quiet-button" onclick={() => onSelectRelationship(connection.relationshipId)}
              >Edit</button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</aside>

<style>
.panel {
  display: grid;
  align-content: start;
  gap: 12px;
  height: 100%;
  min-height: 0;
  overflow: auto;
  padding: 14px;
}
header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
header div {
  display: grid;
  gap: 2px;
  min-width: 0;
}
header strong {
  color: var(--ink);
  font:
    600 14px/1.3 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
header span,
header em {
  color: var(--ink-muted);
  font:
    12px/1.35 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
header em {
  font-style: normal;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.house {
  color: var(--accent);
  font-weight: 700;
}
.actions,
.connections ul,
.connections li {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.connections {
  display: grid;
  gap: 6px;
}
.connections span {
  color: var(--ink-muted);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.connections ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
.connections li {
  width: 100%;
  align-items: center;
}
.connections li .quiet-button:first-child {
  flex: 1 1 auto;
  text-align: left;
}
.quiet-button {
  padding: 7px 10px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
  font-size: 12px;
  cursor: pointer;
}
.quiet-button:hover {
  background: var(--surface-muted, var(--surface));
  color: var(--ink);
}
</style>
