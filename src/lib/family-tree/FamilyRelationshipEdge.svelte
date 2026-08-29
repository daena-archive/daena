<script lang="ts">
import { BaseEdge, getBezierPath, type Position } from "@xyflow/svelte";
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
  markerEnd,
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
  markerEnd?: string;
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
const [path, labelX, labelY] = $derived(
  getBezierPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }),
);
const [pathHigh] = $derived(
  getBezierPath({
    sourceX,
    sourceY: sourceY - 3,
    targetX,
    targetY: targetY - 3,
    sourcePosition,
    targetPosition,
  }),
);
const [pathLow] = $derived(
  getBezierPath({
    sourceX,
    sourceY: sourceY + 3,
    targetX,
    targetY: targetY + 3,
    sourcePosition,
    targetPosition,
  }),
);
</script>

{#if partner}
  <g class="family-edge family-edge-partner" class:selected aria-label={caption}>
    <title>{caption}</title>
    <BaseEdge {id} path={pathHigh} {labelX} {labelY} interactionWidth={20} />
    <BaseEdge id={`${id}-parallel`} path={pathLow} interactionWidth={0} />
  </g>
{:else}
  <g class="family-edge family-edge-parent" class:selected aria-label={caption}>
    <title>{caption}</title>
    <BaseEdge
      {id}
      {path}
      {labelX}
      {labelY}
      {markerEnd}
      interactionWidth={20}
      style={dash ? `stroke-dasharray:${dash}` : undefined} />
  </g>
{/if}
