<script lang="ts">
import {
  Background,
  Controls,
  MarkerType,
  SvelteFlow,
  type Edge,
  type EdgeTypes,
  type Node,
  type NodeTypes,
} from "@xyflow/svelte";
import "@xyflow/svelte/dist/style.css";
import type { Snippet } from "svelte";
import FamilyPersonNode from "./FamilyPersonNode.svelte";
import FamilyRelationshipEdge from "./FamilyRelationshipEdge.svelte";
import FamilyUnionNode from "./FamilyUnionNode.svelte";
import type { FamilyPerson, LayoutEdge, PositionedGraph } from "./model";

let {
  layout,
  people,
  rootId,
  selectedPersonId,
  avatar,
  onSelectPerson,
  onOpenEntity,
  onMakeRoot,
  fitToken = 0,
}: {
  layout: PositionedGraph;
  people: Map<string, FamilyPerson>;
  rootId: string;
  selectedPersonId: string | null;
  avatar?: Snippet<[string, string]>;
  onSelectPerson: (id: string | null) => void;
  onOpenEntity: (id: string) => void;
  onMakeRoot: (id: string) => void;
  fitToken?: number;
} = $props();

const nodeTypes = { person: FamilyPersonNode, union: FamilyUnionNode } as unknown as NodeTypes;
const edgeTypes = { family: FamilyRelationshipEdge } as unknown as EdgeTypes;
const reducedMotion = $derived(
  typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches,
);

const nodes = $derived(
  layout.nodes.map((node): Node => ({
    id: node.id,
    type: node.kind,
    position: { x: node.x, y: node.y },
    data:
      node.kind === "person" && node.personId
        ? {
            person: people.get(node.personId)!,
            isRoot: node.personId === rootId,
            avatar,
            onOpen: onOpenEntity,
            onMakeRoot,
          }
        : {},
    selected: node.personId === selectedPersonId,
    draggable: false,
    connectable: false,
    style: `width:${node.width}px;height:${node.height}px;`,
  })),
);

const edges = $derived(
  layout.edges.map((edge): Edge => ({
    id: edge.id,
    type: "family",
    source: edge.source,
    target: edge.target,
    data: {
      role: edge.role,
      parentKind: edge.parentKind,
      partnerKind: edge.partnerKind,
      label: edge.label,
      start: edge.start,
      end: edge.end,
    },
    ariaLabel: edge.label || undefined,
    markerEnd: edge.arrow ? { type: MarkerType.ArrowClosed } : undefined,
    selectable: true,
    focusable: true,
  })),
);

const connections = $derived.by(() => {
  if (!selectedPersonId) return [] as { id: string; label: string; otherId: string | null }[];
  return layout.edges
    .filter((edge) => edge.source === selectedPersonId || edge.target === selectedPersonId)
    .map((edge) => ({
      id: edge.id,
      label: connectionLabel(edge, selectedPersonId),
      otherId: otherPerson(edge, selectedPersonId),
    }));
});

function connectionLabel(edge: LayoutEdge, personId: string) {
  const other = otherPerson(edge, personId);
  const name = other ? (people.get(other)?.name ?? other) : edge.role;
  return edge.label ? `${edge.label} — ${name}` : name;
}

function otherPerson(edge: LayoutEdge, personId: string) {
  if (people.has(edge.source) && edge.source !== personId) return edge.source;
  if (people.has(edge.target) && edge.target !== personId) return edge.target;
  return null;
}

function nearestPerson(fromId: string, key: string) {
  const from = layout.nodes.find((node) => node.personId === fromId);
  if (!from) return null;
  let best: { id: string; score: number } | null = null;
  for (const node of layout.nodes) {
    if (node.kind !== "person" || !node.personId || node.personId === fromId) continue;
    const dx = node.x - from.x;
    const dy = node.y - from.y;
    const aligned =
      (key === "ArrowUp" && dy < -8) ||
      (key === "ArrowDown" && dy > 8) ||
      (key === "ArrowLeft" && dx < -8) ||
      (key === "ArrowRight" && dx > 8);
    if (!aligned) continue;
    const score = dx * dx + dy * dy;
    if (!best || score < best.score) best = { id: node.personId, score };
  }
  return best?.id ?? null;
}

function onCanvasKeydown(event: KeyboardEvent) {
  if (!selectedPersonId) return;
  if (event.key === "Enter" && event.shiftKey) {
    event.preventDefault();
    onMakeRoot(selectedPersonId);
    return;
  }
  if (event.key.startsWith("Arrow")) {
    const next = nearestPerson(selectedPersonId, event.key);
    if (next) {
      event.preventDefault();
      onSelectPerson(next);
    }
  }
}
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
<div
  class="canvas"
  class:reduced={reducedMotion}
  tabindex="0"
  role="application"
  aria-label="Family tree canvas"
  onkeydown={onCanvasKeydown}>
  {#key fitToken}
    <SvelteFlow
      {nodes}
      {edges}
      {nodeTypes}
      {edgeTypes}
      fitView={fitToken >= 0}
      fitViewOptions={{ duration: reducedMotion ? 0 : 200 }}
      nodesDraggable={false}
      nodesConnectable={false}
      elementsSelectable={true}
      deleteKey={null}
      minZoom={0.25}
      maxZoom={2}
      onnodeclick={({ node }) => {
        onSelectPerson(node.type === "person" ? node.id : null);
      }}
      onpaneclick={() => onSelectPerson(null)}>
      <Controls showLock={false} />
      <Background />
    </SvelteFlow>
  {/key}
</div>
{#if selectedPersonId && connections.length > 0}
  <ul class="connections" aria-label="Connections for selected person">
    {#each connections as connection (connection.id)}
      <li>
        {#if connection.otherId}
          <button type="button" class="quiet-button" onclick={() => onSelectPerson(connection.otherId)}>
            {connection.label}
          </button>
        {:else}
          <span>{connection.label}</span>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
.canvas {
  height: 100%;
  min-height: 420px;
  border: 1px solid var(--line-soft);
  border-radius: 12px;
  overflow: hidden;
  background: var(--surface-warm, var(--surface));
}
.canvas:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.canvas.reduced :global(.svelte-flow__node) {
  transition: none;
}
.canvas :global(.family-edge-parent) :global(.svelte-flow__edge-path) {
  stroke: var(--ink-muted);
  stroke-width: 1.6;
}
.canvas :global(.family-edge-partner) :global(.svelte-flow__edge-path) {
  stroke: var(--ink);
  stroke-width: 1.4;
}
.canvas :global(.family-edge.selected) :global(.svelte-flow__edge-path) {
  stroke-width: 3.2;
}
.connections {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
}
</style>
