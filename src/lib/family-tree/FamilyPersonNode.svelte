<script lang="ts">
import { Handle, Position } from "@xyflow/svelte";
import type { Snippet } from "svelte";
import { formatCalendarDate } from "$lib/date";
import type { BranchDirection, FamilyPerson, HiddenCounts, RelativeRole } from "./model.ts";

let {
  data,
}: {
  data: {
    person?: FamilyPerson;
    isRoot: boolean;
    hidden?: HiddenCounts;
    expanded?: Record<BranchDirection, boolean>;
    avatar?: Snippet<[string, string]>;
    onOpen: (id: string) => void;
    onSelect?: (id: string) => void;
    onMakeRoot: (id: string) => void;
    onToggleBranch?: (id: string, direction: BranchDirection) => void;
    onAddRelative?: (id: string, role: RelativeRole) => void;
    hideAddChild?: boolean;
  };
} = $props();

const person = $derived(data.person);

function countLabel(count: number, truncated: boolean, lowerBound = 0) {
  if (!truncated) return String(count);
  if (count === 0) return "0";
  if (lowerBound > 0) return `${Math.max(count, lowerBound)}+`;
  return "99+";
}

function lifeSpan(person: FamilyPerson | undefined) {
  if (!person) return null;
  const birth = person.birth ? formatCalendarDate(person.birth) : "";
  const death = person.death ? formatCalendarDate(person.death) : "";
  if (!birth && !death) return null;
  if (birth && death) return `${birth} – ${death}`;
  return birth || death;
}

function onCardKeydown(event: KeyboardEvent) {
  if (event.key !== "Enter" || !person) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.shiftKey) data.onMakeRoot(person.id);
  else data.onSelect?.(person.id);
}
</script>

{#if person}
  <div
    class="card"
    class:is-root={data.isRoot}
    data-person-id={person.id}
    tabindex="0"
    role="button"
    aria-label={person.name}
    onkeydown={onCardKeydown}>
    <Handle id="north" type="target" position={Position.Top} isConnectable={false} />
    <Handle id="west" type="source" position={Position.Left} isConnectable={false} />
    <Handle id="east" type="source" position={Position.Right} isConnectable={false} />
    {#if data.avatar}
      {@render data.avatar(person.id, person.name)}
    {/if}
    <div class="copy">
      <strong>{person.name}</strong>
      {#if lifeSpan(person)}<span>{lifeSpan(person)}</span>{/if}
      {#if person.secondaryLabel}<span>{person.secondaryLabel}</span>{/if}
      {#if data.isRoot}<em>Root</em>{/if}
    </div>
    <div class="toolbar">
      <div class="actions">
        <button type="button" class="chip nodrag nopan" aria-label="Open in Lore" onclick={() => data.onOpen(person.id)}
          >Open</button>
        {#if !data.isRoot}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-label="Make root"
            onclick={() => data.onMakeRoot(person.id)}>Root</button>
        {/if}
        <button
          type="button"
          class="chip nodrag nopan"
          aria-label="Add parent"
          onclick={() => data.onAddRelative?.(person.id, "parent")}>+ Parent</button>
        {#if !data.hideAddChild}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-label="Add child"
            onclick={() => data.onAddRelative?.(person.id, "child")}>+ Child</button>
        {/if}
        <button
          type="button"
          class="chip nodrag nopan"
          aria-label="Add partner"
          onclick={() => data.onAddRelative?.(person.id, "partner")}>+ Partner</button>
      </div>
      {#if data.hidden}
        <div class="branches">
          {#if data.hidden.parents > 0 || data.expanded?.parents}
            <button type="button" class="chip nodrag nopan" onclick={() => data.onToggleBranch?.(person.id, "parents")}>
              {data.expanded?.parents && data.hidden.parents === 0
                ? "Hide parents"
                : `↑ ${countLabel(data.hidden.parents, data.hidden.truncated, data.hidden.lowerBound)} parents`}
            </button>
          {/if}
          {#if data.hidden.children > 0 || data.expanded?.children}
            <button
              type="button"
              class="chip nodrag nopan"
              onclick={() => data.onToggleBranch?.(person.id, "children")}>
              {data.expanded?.children && data.hidden.children === 0
                ? "Hide children"
                : `↓ ${countLabel(data.hidden.children, data.hidden.truncated, data.hidden.lowerBound)} children`}
            </button>
          {/if}
          {#if data.hidden.siblings > 0 || data.expanded?.siblings}
            <button
              type="button"
              class="chip nodrag nopan"
              onclick={() => data.onToggleBranch?.(person.id, "siblings")}>
              {data.expanded?.siblings && data.hidden.siblings === 0
                ? "Hide siblings"
                : `${countLabel(data.hidden.siblings, data.hidden.truncated, data.hidden.lowerBound)} siblings`}
            </button>
          {/if}
          {#if data.hidden.partners > 0 || data.expanded?.partners}
            <button
              type="button"
              class="chip nodrag nopan"
              onclick={() => data.onToggleBranch?.(person.id, "partners")}>
              {data.expanded?.partners && data.hidden.partners === 0
                ? "Hide partners"
                : `${countLabel(data.hidden.partners, data.hidden.truncated, data.hidden.lowerBound)} partners`}
            </button>
          {/if}
        </div>
      {/if}
    </div>
    <Handle id="south" type="source" position={Position.Bottom} isConnectable={false} />
  </div>
{/if}

<style>
.card {
  position: relative;
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 10px;
  align-items: center;
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  overflow: visible;
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
.toolbar {
  display: none;
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  z-index: 4;
  flex-direction: column;
  gap: 4px;
  min-width: 100%;
  padding: 6px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 0 8px 20px color-mix(in srgb, var(--ink) 12%, transparent);
}
.card:hover .toolbar,
.card:focus-within .toolbar,
:global(.svelte-flow__node.selected) .toolbar {
  display: flex;
}
.actions,
.branches {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.chip {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  min-width: 0;
  padding: 2px 7px;
  border: 1px solid var(--line-strong);
  border-radius: 6px;
  background: color-mix(in srgb, var(--surface-muted, var(--surface)) 88%, var(--ink) 12%);
  color: var(--ink-soft, var(--ink));
  font:
    600 10px/1.2 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  white-space: nowrap;
  cursor: pointer;
}
.chip:hover {
  background: var(--surface-muted, var(--surface));
  color: var(--ink);
}
.chip:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
:global(.svelte-flow__handle) {
  width: 8px;
  height: 8px;
  opacity: 0;
  border: none;
  background: transparent;
}
</style>
