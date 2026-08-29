<script lang="ts">
import { Handle, Position } from "@xyflow/svelte";
import type { Snippet } from "svelte";
import { formatCalendarDate } from "$lib/date";
import type { FamilyPerson } from "./model";

let {
  data,
}: {
  data: {
    person: FamilyPerson;
    isRoot: boolean;
    avatar?: Snippet<[string, string]>;
    onOpen: (id: string) => void;
    onMakeRoot: (id: string) => void;
  };
} = $props();

function lifeSpan(person: FamilyPerson) {
  const birth = person.birth ? formatCalendarDate(person.birth) : "";
  const death = person.death ? formatCalendarDate(person.death) : "";
  if (!birth && !death) return null;
  if (birth && death) return `${birth} – ${death}`;
  return birth || death;
}

function onCardKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && event.shiftKey) {
    event.preventDefault();
    event.stopPropagation();
    data.onMakeRoot(data.person.id);
  }
}
</script>

<div
  class="card"
  class:is-root={data.isRoot}
  tabindex="0"
  role="button"
  aria-label={data.person.name}
  onkeydown={onCardKeydown}>
  <Handle type="target" position={Position.Top} isConnectable={false} />
  {#if data.avatar}
    {@render data.avatar(data.person.id, data.person.name)}
  {/if}
  <div class="copy">
    <strong>{data.person.name}</strong>
    {#if lifeSpan(data.person)}<span>{lifeSpan(data.person)}</span>{/if}
    {#if data.person.secondaryLabel}<span>{data.person.secondaryLabel}</span>{/if}
    {#if data.isRoot}<em>Root</em>{/if}
  </div>
  <div class="actions">
    <button type="button" class="quiet-button" onclick={() => data.onOpen(data.person.id)}>Open in Lore</button>
    {#if !data.isRoot}
      <button type="button" class="quiet-button" onclick={() => data.onMakeRoot(data.person.id)}>Make root</button>
    {/if}
  </div>
  <Handle type="source" position={Position.Bottom} isConnectable={false} />
</div>

<style>
.card {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 8px 10px;
  align-items: center;
  box-sizing: border-box;
  width: 220px;
  height: 92px;
  padding: 8px 10px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface);
  color: var(--ink);
  box-shadow: 0 1px 0 color-mix(in srgb, var(--ink) 6%, transparent);
}
.card:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.card.is-root {
  border-width: 2px;
  border-color: var(--accent);
}
.copy {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.copy strong {
  overflow: hidden;
  color: var(--ink);
  font:
    600 12px/1.2 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.copy span,
.copy em {
  overflow: hidden;
  color: var(--ink-muted);
  font:
    10px/1.2 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.copy em {
  font-style: normal;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.actions {
  display: flex;
  grid-column: 1 / -1;
  gap: 6px;
}
.actions :global(.quiet-button) {
  min-height: 22px;
  padding: 2px 7px;
  font-size: 10px;
}
</style>
