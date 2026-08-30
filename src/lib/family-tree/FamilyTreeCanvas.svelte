<script lang="ts">
import {
  Background,
  Controls,
  SvelteFlow,
  type Edge,
  type EdgeTypes,
  type Node,
  type NodeTypes,
  type Viewport,
} from "@xyflow/svelte";
import "@xyflow/svelte/dist/style.css";
import type { Snippet } from "svelte";
import FamilyPersonNode from "./FamilyPersonNode.svelte";
import FamilyRelationshipEdge from "./FamilyRelationshipEdge.svelte";
import FamilyUnionNode from "./FamilyUnionNode.svelte";
import { familyEdgeHandles } from "./layout.ts";
import { coupleClickAction, unionClickAction } from "./unions.ts";
import type { BranchDirection, FamilyPerson, HiddenCounts, LayoutEdge, PositionedGraph } from "./model.ts";

let {
  layout,
  people,
  rootId,
  selectedPersonId,
  selectedRelationshipId = null,
  hiddenByPerson,
  expandedByPerson,
  avatar,
  onSelectPerson,
  onSelectRelationship,
  onMakeRoot,
  onToggleBranch,
  onAddUnionChild,
  onLinkPartners,
  housesByPerson = new Map(),
  memberHouseIds = new Map(),
  houseFilterId = null,
  fitToken = 0,
  fitView = true,
  initialViewport = null,
  onViewportChange,
}: {
  layout: PositionedGraph;
  people: Map<string, FamilyPerson>;
  rootId: string;
  selectedPersonId: string | null;
  selectedRelationshipId?: string | null;
  hiddenByPerson: Map<string, HiddenCounts>;
  expandedByPerson: Map<string, Record<BranchDirection, boolean>>;
  avatar?: Snippet<[string, string]>;
  onSelectPerson: (id: string | null) => void;
  onSelectRelationship: (id: string | null) => void;
  onMakeRoot: (id: string) => void;
  onToggleBranch: (id: string, direction: BranchDirection) => void;
  onAddUnionChild: (memberIds: string[]) => void;
  onLinkPartners: (memberIds: [string, string]) => void;
  housesByPerson?: Map<string, string[]>;
  memberHouseIds?: Map<string, string[]>;
  houseFilterId?: string | null;
  fitToken?: number;
  fitView?: boolean;
  initialViewport?: Viewport | null;
  onViewportChange?: (viewport: Viewport) => void;
} = $props();

const nodeTypes = { person: FamilyPersonNode, union: FamilyUnionNode } as unknown as NodeTypes;
const edgeTypes = { family: FamilyRelationshipEdge } as unknown as EdgeTypes;
const reducedMotion = $derived(
  typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches,
);
let nodes = $state.raw<Node[]>([]);
let edges = $state.raw<Edge[]>([]);

function flowNodes(): Node[] {
  return layout.nodes.flatMap((node): Node[] => {
    if (node.kind === "person") {
      const personId = node.personId;
      if (!personId) return [];
      const person = people.get(personId);
      if (!person) return [];
      return [
        {
          id: node.id,
          type: node.kind,
          position: { x: node.x, y: node.y },
          data: {
            person,
            isRoot: personId === rootId,
            hidden: hiddenByPerson.get(personId),
            expanded: expandedByPerson.get(personId),
            avatar,
            onSelect: onSelectPerson,
            onMakeRoot,
            onToggleBranch,
            houses: housesByPerson.get(personId) ?? [],
            dimmed: Boolean(houseFilterId) && !(memberHouseIds.get(personId) ?? []).includes(houseFilterId ?? ""),
          },
          selected: node.personId === selectedPersonId,
          draggable: false,
          connectable: false,
          style: `width:${node.width}px;height:${node.height}px;`,
        },
      ];
    }
    return [
      {
        id: node.id,
        type: node.kind,
        position: { x: node.x, y: node.y },
        data: {
          memberIds: node.memberIds ?? [],
          onAddChild: onAddUnionChild,
        },
        selected: false,
        draggable: false,
        connectable: false,
        style: `width:${node.width}px;height:${node.height}px;`,
      },
    ];
  });
}

function portHandles(edge: LayoutEdge) {
  return familyEdgeHandles(edge, layout.nodes);
}

function flowEdges(): Edge[] {
  return layout.edges.map((edge): Edge => ({
    id: edge.id,
    type: "family",
    source: edge.source,
    target: edge.target,
    ...portHandles(edge),
    data: {
      role: edge.role,
      parentKind: edge.parentKind,
      partnerKind: edge.partnerKind,
      label: edge.label,
      start: edge.start,
      end: edge.end,
    },
    selected: Boolean(edge.relationshipId && edge.relationshipId === selectedRelationshipId),
    ariaLabel: edge.label || undefined,
    selectable: true,
    focusable: true,
  }));
}

$effect.pre(() => {
  nodes = flowNodes();
  edges = flowEdges();
});

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
  if (event.key === "Enter") {
    event.preventDefault();
    if (event.shiftKey) onMakeRoot(selectedPersonId);
    else onSelectPerson(selectedPersonId);
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

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="canvas"
  class:reduced={reducedMotion}
  tabindex="0"
  role="application"
  aria-label="Family tree canvas"
  onkeydown={onCanvasKeydown}>
  {#key `${fitToken}:${rootId}`}
    <SvelteFlow
      {nodes}
      {edges}
      {nodeTypes}
      {edgeTypes}
      {fitView}
      colorMode="dark"
      fitViewOptions={{ duration: reducedMotion ? 0 : 200 }}
      initialViewport={initialViewport ?? undefined}
      nodesDraggable={false}
      nodesConnectable={false}
      elementsSelectable={true}
      deleteKey={[]}
      minZoom={0.25}
      maxZoom={2}
      onmoveend={(_event, next) => onViewportChange?.(next)}
      onnodeclick={({ node }) => {
        if (node.type === "person") {
          onSelectPerson(node.id);
          return;
        }
        onSelectPerson(null);
        const action = unionClickAction(node.id, layout.nodes, layout.edges);
        if (action && "relationshipId" in action) onSelectRelationship(action.relationshipId);
        else if (action && "memberIds" in action) onLinkPartners(action.memberIds);
        else onSelectRelationship(null);
      }}
      onedgeclick={({ edge }) => {
        const layoutEdge = layout.edges.find((item) => item.id === edge.id);
        if (!layoutEdge) {
          onSelectRelationship(null);
          return;
        }
        const action = coupleClickAction(layoutEdge, layout.nodes, layout.edges);
        if (action && "relationshipId" in action) onSelectRelationship(action.relationshipId);
        else if (action && "memberIds" in action) onLinkPartners(action.memberIds);
        else onSelectRelationship(null);
      }}
      onpaneclick={() => {
        onSelectPerson(null);
        onSelectRelationship(null);
      }}>
      <Controls showLock={false} />
      <Background />
    </SvelteFlow>
  {/key}
</div>

<style>
.canvas {
  flex: 1 1 auto;
  width: 100%;
  height: 100%;
  min-height: 0;
  border: 1px solid var(--line-soft);
  border-radius: 12px;
  overflow: hidden;
  background: var(--surface-warm, var(--surface));
}
.canvas :global(.svelte-flow) {
  width: 100%;
  height: 100%;
  --xy-controls-button-background-color: var(--surface);
  --xy-controls-button-background-color-hover: var(--surface-muted, var(--surface));
  --xy-controls-button-color: var(--ink);
  --xy-controls-button-color-hover: var(--ink);
  --xy-controls-button-border-color: var(--line-strong);
  --xy-controls-box-shadow: none;
  --xy-attribution-background-color: var(--surface);
  --xy-background-pattern-dots-color: color-mix(in srgb, var(--ink-muted) 55%, transparent);
}
.canvas :global(.svelte-flow__controls) {
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
}
.canvas :global(.svelte-flow__controls-button) {
  width: 28px;
  height: 28px;
}
.canvas :global(.svelte-flow__attribution) {
  margin: 0;
  border: 1px solid var(--line-strong);
  border-right: 0;
  border-bottom: 0;
  border-radius: 8px 0 0 0;
  color: var(--ink-muted);
}
.canvas :global(.svelte-flow__attribution a) {
  color: var(--ink-muted);
}
.canvas:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.canvas.reduced :global(.svelte-flow__node) {
  transition: none;
}
.canvas :global(.svelte-flow__node) {
  overflow: visible;
}
.canvas :global(.family-edge-parent) :global(.svelte-flow__edge-path) {
  stroke: var(--ink-muted);
  stroke-width: 1.5;
}
.canvas :global(.family-edge-partner) :global(.partner-rail) {
  stroke: var(--ink);
  stroke-width: 5;
  stroke-linecap: round;
}
.canvas :global(.family-edge-partner) :global(.svelte-flow__edge-path) {
  stroke: var(--surface-warm, var(--surface));
  stroke-width: 2;
  stroke-linecap: round;
}
.canvas :global(.family-edge.selected) :global(.svelte-flow__edge-path) {
  stroke-width: 3.2;
}
.canvas :global(.family-edge-partner.selected) :global(.partner-rail) {
  stroke-width: 7;
}
.canvas :global(.family-edge-partner.selected) :global(.svelte-flow__edge-path) {
  stroke-width: 3;
}
</style>
