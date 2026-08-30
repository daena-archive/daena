<script lang="ts">
import { formatCalendarDate } from "$lib/date";
import { Crown, ExternalLink, GitBranch, Heart, UserPlus } from "@lucide/svelte";
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
  <header class="panel-head">
    <div class="head-copy">
      <span class="kicker">{isRoot ? "ROOT PERSON" : "PERSON"}</span>
      <strong class="person-name">{person.name}</strong>
      {#if lifeSpan(person)}<span class="meta">{lifeSpan(person)}</span>{/if}
      {#if person.secondaryLabel}<span class="meta secondary">{person.secondaryLabel}</span>{/if}
      {#if houses.length}<span class="houses">{houses.join(" · ")}</span>{/if}
    </div>
    <button type="button" class="quiet-button ghost" onclick={onClose} aria-label="Close details">Close</button>
  </header>

  <div class="panel-actions" role="group" aria-label="Primary actions">
    <button type="button" class="quiet-button pill" onclick={() => onOpen(person.id)}>
      <ExternalLink size={13} strokeWidth={1.8} aria-hidden="true" /> Open in Lore
    </button>
    {#if !isRoot}
      <button type="button" class="quiet-button pill" onclick={() => onMakeRoot(person.id)}>
        <Crown size={13} strokeWidth={1.8} aria-hidden="true" /> Make root
      </button>
    {/if}
  </div>

  <div class="panel-actions" role="group" aria-label="Add relatives">
    <button type="button" class="quiet-button" onclick={() => onAddRelative(person.id, "parent")}>
      <UserPlus size={13} strokeWidth={1.8} aria-hidden="true" /> Add parent
    </button>
    {#if !hideAddChild}
      <button type="button" class="quiet-button" onclick={() => onAddRelative(person.id, "child")}>
        <GitBranch size={13} strokeWidth={1.8} aria-hidden="true" /> Add child
      </button>
    {/if}
    <button type="button" class="quiet-button" onclick={() => onAddRelative(person.id, "partner")}>
      <Heart size={13} strokeWidth={1.8} aria-hidden="true" /> Add partner
    </button>
  </div>

  {#if connections.length > 0}
    <section class="connections" aria-label="Visible connections">
      <h3>On this tree</h3>
      <ul>
        {#each connections as connection (connection.id)}
          <li>
            <button type="button" class="connection-main" onclick={() => onSelectPerson(connection.otherId)}>
              {connection.label}
            </button>
            <button
              type="button"
              class="quiet-button ghost small"
              onclick={() => onSelectRelationship(connection.relationshipId)}>Edit</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</aside>

<style>
.panel {
  display: grid;
  align-content: start;
  gap: 14px;
  height: 100%;
  min-height: 0;
  overflow: auto;
  padding: 16px;
  background: var(--surface);
}
.panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--line-soft, var(--line));
}
.head-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.kicker {
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.person-name {
  color: var(--ink);
  font: 600 15px/1.25 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: -0.01em;
}
.meta {
  color: var(--ink-muted);
  font: 11px/1.35 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.meta.secondary {
  font-style: italic;
}
.houses {
  color: var(--theme-success-text, var(--accent-dark));
  font-size: 10px;
  font-weight: 700;
}
.panel-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.pill {
  border-radius: 999px !important;
}
.ghost {
  border-color: transparent;
  background: transparent;
}
.ghost:hover {
  border-color: var(--line);
  background: var(--surface-muted);
}
.small {
  padding: 6px 8px;
  font-size: 11px;
}
.connections {
  display: grid;
  gap: 8px;
  padding-top: 4px;
  border-top: 1px solid var(--line-soft, var(--line));
}
.connections h3 {
  margin: 0;
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.connections ul {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.connections li {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
}
.connection-main {
  flex: 1 1 auto;
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font-size: 12px;
  text-align: left;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.connection-main:hover {
  border-color: var(--line-strong);
  background: var(--surface-muted);
}
.quiet-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
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
@media (prefers-reduced-motion: reduce) {
  .panel,
  .quiet-button {
    transition: none;
  }
}
</style>
