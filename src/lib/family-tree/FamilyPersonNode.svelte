<script lang="ts">
import { Handle, Position } from "@xyflow/svelte";
import type { Snippet } from "svelte";
import { formatCalendarDate } from "$lib/date";
import type { BranchDirection, FamilyPerson, HiddenCounts } from "./model.ts";

let {
  data,
}: {
  data: {
    person?: FamilyPerson;
    isRoot: boolean;
    hidden?: HiddenCounts;
    expanded?: Record<BranchDirection, boolean>;
    avatar?: Snippet<[string, string]>;
    onSelect?: (id: string) => void;
    onMakeRoot: (id: string) => void;
    onToggleBranch?: (id: string, direction: BranchDirection) => void;
    houses?: string[];
    dimmed?: boolean;
  };
} = $props();

const person = $derived(data.person);

function countLabel(count: number, truncated: boolean, lowerBound = 0) {
  if (!truncated) return String(count);
  if (count === 0) return "0";
  if (lowerBound > 0) return `${Math.max(count, lowerBound)}+`;
  return "99+";
}

function lifeSpan(value: FamilyPerson | undefined) {
  if (!value) return null;
  const birth = value.birth ? formatCalendarDate(value.birth) : "";
  const death = value.death ? formatCalendarDate(value.death) : "";
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

function toggle(event: MouseEvent, direction: BranchDirection) {
  if (!person) return;
  event.preventDefault();
  event.stopPropagation();
  data.onToggleBranch?.(person.id, direction);
}
</script>

{#if person}
  <div
    class="card"
    class:is-root={data.isRoot}
    class:is-dimmed={data.dimmed}
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
      {#if data.houses?.length}
        <span class="house">{data.houses[0]}{data.houses.length > 1 ? ` +${data.houses.length - 1}` : ""}</span>
      {/if}
      {#if data.isRoot}<em>Root</em>{/if}
    </div>
    {#if data.hidden}
      <div class="branches">
        {#if data.hidden.parents > 0 || data.expanded?.parents}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-pressed={Boolean(data.expanded?.parents)}
            onclick={(event) => toggle(event, "parents")}>
            {data.expanded?.parents && data.hidden.parents === 0
              ? "↑"
              : `↑ ${countLabel(data.hidden.parents, data.hidden.truncated, data.hidden.lowerBound)}`}
          </button>
        {/if}
        {#if data.hidden.children > 0 || data.expanded?.children}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-pressed={Boolean(data.expanded?.children)}
            onclick={(event) => toggle(event, "children")}>
            {data.expanded?.children && data.hidden.children === 0
              ? "↓"
              : `↓ ${countLabel(data.hidden.children, data.hidden.truncated, data.hidden.lowerBound)}`}
          </button>
        {/if}
        {#if data.hidden.siblings > 0 || data.expanded?.siblings}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-pressed={Boolean(data.expanded?.siblings)}
            onclick={(event) => toggle(event, "siblings")}>
            {data.expanded?.siblings && data.hidden.siblings === 0
              ? "sib"
              : `sib ${countLabel(data.hidden.siblings, data.hidden.truncated, data.hidden.lowerBound)}`}
          </button>
        {/if}
        {#if data.hidden.partners > 0 || data.expanded?.partners}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-pressed={Boolean(data.expanded?.partners)}
            onclick={(event) => toggle(event, "partners")}>
            {data.expanded?.partners && data.hidden.partners === 0
              ? "par"
              : `par ${countLabel(data.hidden.partners, data.hidden.truncated, data.hidden.lowerBound)}`}
          </button>
        {/if}
      </div>
    {/if}
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
.card.is-dimmed {
  opacity: 0.38;
}
.copy span.house {
  color: var(--accent);
  font-weight: 700;
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
.branches {
  position: absolute;
  right: 6px;
  bottom: 4px;
  left: 6px;
  z-index: 2;
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  justify-content: flex-end;
  pointer-events: none;
}
.chip {
  display: inline-flex;
  align-items: center;
  min-height: 16px;
  min-width: 0;
  padding: 0 5px;
  border: 1px solid var(--line-strong);
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface) 88%, var(--ink) 12%);
  color: var(--ink-soft, var(--ink));
  font:
    700 9px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  pointer-events: auto;
  white-space: nowrap;
  cursor: pointer;
}
.chip:hover,
.chip[aria-pressed="true"] {
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
