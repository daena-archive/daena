<script lang="ts">
import { Handle, Position } from "@xyflow/svelte";
import { formatCalendarDate } from "$lib/date";
import { BRANCH_DIRECTIONS, type BranchDirection, type FamilyPerson, type HiddenCounts } from "./model.ts";
import { getTreeCanvasHost } from "./treeCanvasHost.ts";

let {
  data,
}: {
  data: {
    person?: FamilyPerson;
    isRoot: boolean;
    hidden?: HiddenCounts;
    expanded?: Record<BranchDirection, boolean>;
    houses?: string[];
    roleBadge?: string | null;
    dimmed?: boolean;
    reducedDetail?: boolean;
    tabIndex?: number;
  };
} = $props();

const host = getTreeCanvasHost();
const person = $derived(data.person);
const showSecondary = $derived(!data.reducedDetail);
const cardTabIndex = $derived(data.tabIndex ?? 0);

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
  if (event.shiftKey) host.onMakeRoot(person.id);
  else host.onSelectPerson(person.id);
}

function toggle(event: MouseEvent, direction: BranchDirection) {
  if (!person) return;
  event.preventDefault();
  event.stopPropagation();
  host.onToggleBranch(person.id, direction);
}

const BRANCH_COPY: Record<BranchDirection, { unit: string; units: string }> = {
  parents: { unit: "parent", units: "parents" },
  children: { unit: "child", units: "children" },
  siblings: { unit: "sibling", units: "siblings" },
  partners: { unit: "partner", units: "partners" },
};

function chipCount(direction: BranchDirection) {
  return data.hidden?.[direction] ?? 0;
}

function chipHiding(direction: BranchDirection) {
  return Boolean(data.expanded?.[direction]) && chipCount(direction) === 0;
}

function chipVisible(direction: BranchDirection) {
  return chipCount(direction) > 0 || Boolean(data.expanded?.[direction]);
}

function chipText(direction: BranchDirection) {
  const { unit, units } = BRANCH_COPY[direction];
  if (chipHiding(direction)) return `Hide ${units}`;
  const count = chipCount(direction);
  const n = countLabel(count, Boolean(data.hidden?.truncated), data.hidden?.lowerBound ?? 0);
  const word = count === 1 && n === "1" ? unit : units;
  return `${n} ${word}`;
}

function chipAria(direction: BranchDirection) {
  const { units } = BRANCH_COPY[direction];
  return chipHiding(direction) ? `Hide ${units}` : `Show ${chipText(direction)}`;
}

const branchTabIndex = $derived(cardTabIndex === 0 ? 0 : -1);
const hasBranchChips = $derived(Boolean(data.hidden) && BRANCH_DIRECTIONS.some((direction) => chipVisible(direction)));
const cardAriaLabel = $derived.by(() => {
  if (!person) return "";
  const hasBranches =
    Boolean(data.hidden) &&
    ((data.hidden?.parents ?? 0) > 0 ||
      (data.hidden?.children ?? 0) > 0 ||
      (data.hidden?.siblings ?? 0) > 0 ||
      (data.hidden?.partners ?? 0) > 0 ||
      Boolean(data.expanded?.parents) ||
      Boolean(data.expanded?.children) ||
      Boolean(data.expanded?.siblings) ||
      Boolean(data.expanded?.partners));
  return hasBranches ? `${person.name}. Tab for branch controls` : person.name;
});
</script>

{#if person}
  <div
    class="card"
    class:is-root={data.isRoot}
    class:is-dimmed={data.dimmed}
    data-person-id={person.id}
    tabindex={cardTabIndex}
    role="button"
    aria-label={cardAriaLabel}
    aria-roledescription="person"
    aria-current={data.isRoot ? "true" : undefined}
    onkeydown={onCardKeydown}>
    <Handle id="north" type="target" position={Position.Top} isConnectable={false} />
    <Handle id="west" type="source" position={Position.Left} isConnectable={false} />
    <Handle id="east" type="source" position={Position.Right} isConnectable={false} />
    <div class="avatar-wrap">
      {#if host.avatar}
        {@render host.avatar(person.id, person.name)}
      {/if}
    </div>
    <div class="copy">
      <strong>{person.name}</strong>
      {#if showSecondary && lifeSpan(person)}<span class="lifespan">{lifeSpan(person)}</span>{/if}
      {#if showSecondary && person.secondaryLabel}<span class="secondary">{person.secondaryLabel}</span>{/if}
      {#if data.houses?.length}
        <span class="house">{data.houses[0]}{data.houses.length > 1 ? ` +${data.houses.length - 1}` : ""}</span>
      {/if}
      {#if data.roleBadge}<em class="role-badge">{data.roleBadge}</em>{/if}
      {#if data.isRoot}<em class="root-badge">Root</em>{/if}
    </div>
    {#if hasBranchChips}
      <div class="branches" role="group" aria-label="Hidden relatives" aria-roledescription="branch controls">
        {#each BRANCH_DIRECTIONS as direction (direction)}
          {#if chipVisible(direction)}
            <button
              type="button"
              class="chip nodrag nopan"
              class:is-hide={chipHiding(direction)}
              tabindex={branchTabIndex}
              title={chipAria(direction)}
              aria-pressed={chipHiding(direction)}
              aria-label={chipAria(direction)}
              onclick={(event) => toggle(event, direction)}>
              {chipText(direction)}
            </button>
          {/if}
        {/each}
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
.root-badge,
.role-badge {
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
.role-badge {
  background: color-mix(in srgb, var(--accent) 14%, var(--surface));
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
  justify-content: center;
  min-height: 18px;
  padding: 1px 6px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
  font: 600 9px/1.2 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  pointer-events: auto;
  white-space: nowrap;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
}
.chip:hover {
  border-color: var(--line-strong);
  background: var(--surface-muted, var(--surface));
  color: var(--ink);
}
.chip.is-hide,
.chip[aria-pressed="true"] {
  background: var(--surface-muted, var(--surface));
  border-color: var(--line-strong);
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
@media (prefers-reduced-motion: reduce) {
  .card {
    transition: none;
  }
}
</style>
