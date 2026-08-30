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
    <div class="avatar-wrap">
      {#if data.avatar}
        {@render data.avatar(person.id, person.name)}
      {/if}
    </div>
    <div class="copy">
      <strong>{person.name}</strong>
      {#if lifeSpan(person)}<span class="lifespan">{lifeSpan(person)}</span>{/if}
      {#if person.secondaryLabel}<span class="secondary">{person.secondaryLabel}</span>{/if}
      {#if data.houses?.length}
        <span class="house">{data.houses[0]}{data.houses.length > 1 ? ` +${data.houses.length - 1}` : ""}</span>
      {/if}
      {#if data.isRoot}<em class="root-badge">Root</em>{/if}
    </div>
    {#if data.hidden}
      <div class="branches" role="group" aria-label="Branch controls">
        {#if data.hidden.parents > 0 || data.expanded?.parents}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-pressed={Boolean(data.expanded?.parents)}
            aria-label={data.expanded?.parents ? "Hide parents" : `Show ${data.hidden.parents} hidden parents`}
            onclick={(event) => toggle(event, "parents")}>
            {data.expanded?.parents && data.hidden.parents === 0
              ? "↑ hide"
              : `↑ ${countLabel(data.hidden.parents, data.hidden.truncated, data.hidden.lowerBound)}`}
          </button>
        {/if}
        {#if data.hidden.children > 0 || data.expanded?.children}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-pressed={Boolean(data.expanded?.children)}
            aria-label={data.expanded?.children ? "Hide children" : `Show ${data.hidden.children} hidden children`}
            onclick={(event) => toggle(event, "children")}>
            {data.expanded?.children && data.hidden.children === 0
              ? "↓ hide"
              : `↓ ${countLabel(data.hidden.children, data.hidden.truncated, data.hidden.lowerBound)}`}
          </button>
        {/if}
        {#if data.hidden.siblings > 0 || data.expanded?.siblings}
          <button
            type="button"
            class="chip nodrag nopan"
            aria-pressed={Boolean(data.expanded?.siblings)}
            aria-label={data.expanded?.siblings ? "Hide siblings" : `Show ${data.hidden.siblings} hidden siblings`}
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
            aria-label={data.expanded?.partners ? "Hide partners" : `Show ${data.hidden.partners} hidden partners`}
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
  grid-template-columns: 40px minmax(0, 1fr);
  gap: 0 10px;
  align-items: center;
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  overflow: visible;
  padding: 9px 10px 22px 9px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  color: var(--ink);
  box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.04));
  transition:
    border-color 140ms ease,
    box-shadow 140ms ease,
    transform 140ms ease;
}
.card:hover {
  border-color: var(--line-strong);
  box-shadow: var(--shadow-md, 0 4px 12px rgba(0, 0, 0, 0.06));
}
.card:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.card.is-root {
  border-color: var(--accent);
  border-width: 1.5px;
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 14%, transparent);
}
.card.is-dimmed {
  opacity: 0.62;
  filter: saturate(0.85);
}
.card.is-dimmed:hover {
  opacity: 0.78;
}
.avatar-wrap {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  overflow: hidden;
  background: var(--surface-muted, #f0f1ee);
  border: 1px solid var(--line-soft, var(--line));
}
.avatar-wrap :global(img) {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.copy {
  display: grid;
  gap: 1px;
  min-width: 0;
  align-content: center;
}
.copy strong {
  overflow: hidden;
  color: var(--ink);
  font: 600 13px/1.2 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  text-overflow: ellipsis;
  white-space: nowrap;
  letter-spacing: -0.01em;
}
.copy span {
  overflow: hidden;
  color: var(--ink-muted);
  font: 11px/1.25 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.copy span.lifespan {
  color: var(--ink-soft, var(--ink-muted));
  font-size: 10px;
}
.copy span.secondary {
  color: var(--ink-muted);
  font-style: italic;
}
.copy span.house {
  color: var(--theme-success-text, var(--accent-dark, #2f4e35));
  font-weight: 700;
  font-size: 10px;
}
.root-badge {
  display: inline-flex;
  align-self: start;
  margin-top: 2px;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--accent-bg, #e4ece4);
  color: var(--accent-dark, #2f4e35);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  border: 1px solid color-mix(in srgb, var(--accent) 22%, transparent);
}
.branches {
  position: absolute;
  right: 6px;
  bottom: 5px;
  left: 6px;
  z-index: 2;
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  justify-content: flex-end;
  pointer-events: none;
}
.chip {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  min-width: 0;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
  font: 700 11px/1 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  pointer-events: auto;
  white-space: nowrap;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  transition:
    background 140ms ease,
    border-color 140ms ease,
    color 140ms ease;
}
.chip:hover {
  border-color: var(--line-strong);
  background: var(--surface-muted, var(--surface));
  color: var(--ink);
}
.chip[aria-pressed="true"] {
  background: var(--theme-success-bg, var(--accent-bg));
  border-color: var(--theme-neutral-border-strong, var(--accent));
  color: var(--theme-success-text, var(--accent-dark));
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
@media (prefers-reduced-motion: reduce) {
  .card {
    transition: none;
  }
}
</style>
