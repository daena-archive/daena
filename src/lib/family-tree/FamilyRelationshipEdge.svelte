<script lang="ts">
import { BaseEdge, EdgeLabel, getSmoothStepPath, getStraightPath, Position } from "@xyflow/svelte";
import { formatCalendarDate } from "$lib/date";
import type { ParentKind, PartnerKind } from "./model";

let {
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  selected = false,
  data,
}: {
  id?: string;
  sourceX: number;
  sourceY: number;
  targetX: number;
  targetY: number;
  sourcePosition: Position;
  targetPosition: Position;
  selected?: boolean;
  data?: {
    role: string;
    parentKind: ParentKind | null;
    partnerKind: PartnerKind | null;
    label: string;
    start: unknown;
    end: unknown;
  };
} = $props();

const partner = $derived(data?.role === "partner");
const caption = $derived.by(() => {
  const label = data?.label?.trim() || (partner ? "Partner" : "Parent");
  const start = data?.start ? formatCalendarDate(data.start) : "";
  const end = data?.end ? formatCalendarDate(data.end) : "";
  if (start && end && start !== "Undated" && end !== "Undated") return `${label} (${start} – ${end})`;
  if (start && start !== "Undated") return `${label} (${start})`;
  return label;
});
const shortLabel = $derived.by(() => {
  const raw = data?.label?.trim() || (partner ? "Partner" : "Parent");
  return raw.length > 18 ? raw.slice(0, 18) + "…" : raw;
});
const dash = $derived.by(() => {
  switch (data?.parentKind) {
    case "adoptive":
      return "6 4";
    case "legal":
      return "2 3";
    case "guardian":
      return "8 3 2 3";
    case "step":
      return "1 4";
    default:
      return undefined;
  }
});
const sideways = $derived(sourcePosition === Position.Left || sourcePosition === Position.Right);
const markerKind = $derived.by(() => {
  if (partner) return "partner" as const;
  switch (data?.parentKind) {
    case "adoptive":
      return "adoptive" as const;
    case "legal":
      return "legal" as const;
    case "guardian":
      return "guardian" as const;
    case "step":
      return "step" as const;
    default:
      return null;
  }
});
const [path, labelX, labelY] = $derived(
  partner || sideways
    ? getStraightPath({ sourceX, sourceY, targetX, targetY })
    : getSmoothStepPath({
        sourceX,
        sourceY,
        targetX,
        targetY,
        sourcePosition,
        targetPosition,
        borderRadius: 6,
      }),
);
</script>

{#if partner}
  <g class="family-edge family-edge-partner" class:selected aria-label={caption}>
    <title>{caption}</title>
    <path d={path} class="partner-rail" fill="none" />
    <BaseEdge {id} {path} {labelX} {labelY} interactionWidth={20} />
    {#if selected}
      <EdgeLabel x={labelX} y={labelY}>
        <div class="edge-pill selected">{shortLabel}</div>
      </EdgeLabel>
    {/if}
  </g>
{:else}
  <g class="family-edge family-edge-parent" class:selected aria-label={caption}>
    <title>{caption}</title>
    <!-- halo for contrast on light/warm backgrounds -->
    <path
      d={path}
      class="edge-halo"
      fill="none"
      stroke="var(--surface)"
      stroke-width="5.2"
      stroke-linecap="round"
      opacity="0.95"
      aria-hidden="true" />
    <BaseEdge
      {id}
      {path}
      {labelX}
      {labelY}
      interactionWidth={20}
      style={dash ? `stroke-dasharray:${dash}` : undefined} />
    {#if markerKind}
      <g transform="translate({labelX},{labelY})" aria-hidden="true" pointer-events="none">
        {#if markerKind === "adoptive"}
          <circle r="3.2" fill="var(--surface)" stroke="currentColor" stroke-width="1.2" />
        {:else if markerKind === "legal"}
          <rect
            x="-3.2"
            y="-3.2"
            width="6.4"
            height="6.4"
            rx="1"
            fill="var(--surface)"
            stroke="currentColor"
            stroke-width="1.2"
            transform="rotate(45)" />
        {:else if markerKind === "guardian"}
          <path d="M 0 -4 L 4 0 L 0 4 L -4 0 Z" fill="var(--surface)" stroke="currentColor" stroke-width="1.1" />
        {:else if markerKind === "step"}
          <circle r="1.8" fill="currentColor" />
        {/if}
      </g>
    {/if}
    {#if selected}
      <EdgeLabel x={labelX} y={labelY}>
        <div class="edge-pill selected">{shortLabel}</div>
      </EdgeLabel>
    {/if}
  </g>
{/if}

<style>
.edge-pill {
  position: absolute;
  padding: 2px 6px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-soft, var(--ink));
  font: 700 9px/1 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.06));
  pointer-events: none;
  white-space: nowrap;
}
.edge-pill.selected {
  border-color: var(--accent);
  color: var(--accent-dark);
  background: var(--accent-bg);
}
</style>
