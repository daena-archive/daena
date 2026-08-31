<script lang="ts">
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  SvelteFlow,
  type Edge,
  type EdgeTypes,
  type Node,
  type NodeTypes,
  type Viewport,
} from "@xyflow/svelte";
import "@xyflow/svelte/dist/style.css";
import type { Snippet } from "svelte";
import { TREE_KEYBOARD } from "$lib/entity-lifecycle/vocabulary.ts";
import FamilyPersonNode from "./FamilyPersonNode.svelte";
import FamilyRelationshipEdge from "./FamilyRelationshipEdge.svelte";
import FamilyUnionNode from "./FamilyUnionNode.svelte";
import { familyEdgeHandles } from "./layout.ts";
import { setTreeCanvasHost } from "./treeCanvasHost.ts";
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
  rolesByPerson = new Map(),
  memberHouseIds = new Map(),
  houseFilterId = null,
  fitToken = 0,
  fitView = true,
  initialViewport = null,
  onViewportChange,
  showMinimap = true,
  reducedDetail = false,
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
  rolesByPerson?: Map<string, string>;
  memberHouseIds?: Map<string, string[]>;
  houseFilterId?: string | null;
  fitToken?: number;
  fitView?: boolean;
  initialViewport?: Viewport | null;
  onViewportChange?: (viewport: Viewport) => void;
  showMinimap?: boolean;
  reducedDetail?: boolean;
} = $props();

setTreeCanvasHost({
  get avatar() {
    return avatar;
  },
  onSelectPerson: (id) => onSelectPerson(id),
  onMakeRoot: (id) => onMakeRoot(id),
  onToggleBranch: (id, direction) => onToggleBranch(id, direction),
  onAddUnionChild: (memberIds) => onAddUnionChild(memberIds),
  onSelectRelationship: (id) => onSelectRelationship(id),
});

const nodeTypes = { person: FamilyPersonNode, union: FamilyUnionNode } as unknown as NodeTypes;
const edgeTypes = { family: FamilyRelationshipEdge } as unknown as EdgeTypes;
let reducedMotion = $state(
  typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches,
);
let colorMode = $state<"light" | "dark">("light");
const miniMapBg = $derived(colorMode === "dark" ? "#182720" : "#f4f2ec");
const miniMapMask = $derived(colorMode === "dark" ? "rgba(242,238,228,0.22)" : "rgba(37,37,31,0.14)");
const miniMapMaskStroke = $derived(colorMode === "dark" ? "rgba(242,238,228,0.42)" : "rgba(37,37,31,0.22)");
const miniMapNode = $derived(colorMode === "dark" ? "#b8b1a5" : "#62594e");
const miniMapNodeStroke = $derived(colorMode === "dark" ? "#31443a" : "#d9cdbd");
let nodes = $state.raw<Node[]>([]);
let edges = $state.raw<Edge[]>([]);
let canvasEl = $state<HTMLElement | null>(null);
const activePersonId = $derived(selectedPersonId ?? rootId);

$effect(() => {
  if (typeof window === "undefined") return;
  const media = window.matchMedia("(prefers-reduced-motion: reduce)");
  const update = () => {
    reducedMotion = media.matches;
  };
  update();
  media.addEventListener?.("change", update);
  return () => media.removeEventListener?.("change", update);
});

$effect(() => {
  if (typeof document === "undefined" || typeof window === "undefined") return;
  const update = () => {
    const attr = document.documentElement.dataset.theme;
    if (attr === "dark" || attr === "light") colorMode = attr;
    else colorMode = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  };
  update();
  const observer = new MutationObserver(update);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme", "data-theme-preference"],
  });
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  const onMedia = () => update();
  media.addEventListener?.("change", onMedia);
  return () => {
    observer.disconnect();
    media.removeEventListener?.("change", onMedia);
  };
});

function focusPersonCard(personId: string | null) {
  if (!personId || typeof document === "undefined") return;
  queueMicrotask(() => {
    const card = canvasEl?.querySelector(`[data-person-id="${CSS.escape(personId)}"]`);
    if (card instanceof HTMLElement) card.focus({ preventScroll: true });
  });
}

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
          data: $state.snapshot({
            person,
            isRoot: personId === rootId,
            hidden: hiddenByPerson.get(personId),
            expanded: expandedByPerson.get(personId),
            houses: reducedDetail ? [] : (housesByPerson.get(personId) ?? []),
            roleBadge: rolesByPerson.get(personId) ?? null,
            dimmed: Boolean(houseFilterId) && !(memberHouseIds.get(personId) ?? []).includes(houseFilterId ?? ""),
            reducedDetail,
            tabIndex: personId === activePersonId ? 0 : -1,
          }),
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
        data: $state.snapshot({
          memberIds: node.memberIds ?? [],
        }),
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
    data: $state.snapshot({
      role: edge.role,
      parentKind: edge.parentKind,
      partnerKind: edge.partnerKind,
      label: edge.label,
      start: edge.start,
      end: edge.end,
      relationshipId: edge.relationshipId,
    }),
    selected: Boolean(edge.relationshipId && edge.relationshipId === selectedRelationshipId),
    ariaLabel: edge.label || undefined,
    selectable: true,
    focusable: true,
  }));
}

function relationshipIdsAroundPerson(personId: string): string[] {
  const relatedNodeIds = new Set<string>();
  for (const node of layout.nodes) {
    if (node.kind === "person" && node.personId === personId) relatedNodeIds.add(node.id);
    if (node.kind === "union" && node.memberIds?.includes(personId)) relatedNodeIds.add(node.id);
  }
  if (relatedNodeIds.size === 0) return [];
  const ids: string[] = [];
  for (const edge of layout.edges) {
    if (!edge.relationshipId) continue;
    if (relatedNodeIds.has(edge.source) || relatedNodeIds.has(edge.target)) ids.push(edge.relationshipId);
  }
  return [...new Set(ids)];
}

function openRelationshipAround(personId: string) {
  const ids = relationshipIdsAroundPerson(personId);
  if (ids.length === 0) return false;
  const current = selectedRelationshipId ? ids.indexOf(selectedRelationshipId) : -1;
  const next = ids[(current + 1) % ids.length];
  onSelectRelationship(next);
  queueMicrotask(() => {
    const edge = canvasEl?.querySelector(`[data-relationship-id="${CSS.escape(next)}"]`);
    if (edge instanceof HTMLElement) edge.focus({ preventScroll: true });
  });
  return true;
}

$effect(() => {
  const nextNodes = flowNodes();
  const nextEdges = flowEdges();
  if (typeof requestAnimationFrame === "undefined") {
    nodes = nextNodes;
    edges = nextEdges;
    return;
  }
  const frame = requestAnimationFrame(() => {
    nodes = nextNodes;
    edges = nextEdges;
  });
  return () => cancelAnimationFrame(frame);
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
  const originId =
    selectedPersonId ??
    (event.target instanceof HTMLElement
      ? event.target.closest("[data-person-id]")?.getAttribute("data-person-id")
      : null) ??
    rootId;
  if (!originId) return;
  if (event.key === "Enter") {
    event.preventDefault();
    if (event.shiftKey) onMakeRoot(originId);
    else onSelectPerson(originId);
    return;
  }
  if (event.key === "r" || event.key === "R") {
    if (openRelationshipAround(originId)) {
      event.preventDefault();
    }
    return;
  }
  if (event.key.startsWith("Arrow")) {
    const next = nearestPerson(originId, event.key);
    if (next) {
      event.preventDefault();
      onSelectPerson(next);
      focusPersonCard(next);
    }
  }
}
</script>

<p id={TREE_KEYBOARD.canvasDescribedById} class="sr-only">{TREE_KEYBOARD.helpText}</p>
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="canvas"
  class:reduced={reducedMotion}
  bind:this={canvasEl}
  role="group"
  aria-label={TREE_KEYBOARD.canvasAriaLabel}
  aria-describedby={TREE_KEYBOARD.canvasDescribedById}
  onkeydown={onCanvasKeydown}>
  {#key `${fitToken}:${rootId}`}
    <SvelteFlow
      {nodes}
      {edges}
      {nodeTypes}
      {edgeTypes}
      {fitView}
      {colorMode}
      fitViewOptions={{ duration: reducedMotion ? 0 : 420, padding: 0.18 }}
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
          focusPersonCard(node.id);
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
      <Controls showLock={false} position="top-right" />
      {#if showMinimap}
        <MiniMap
          position="bottom-left"
          pannable
          zoomable
          bgColor={miniMapBg}
          nodeColor={miniMapNode}
          nodeStrokeColor={miniMapNodeStroke}
          maskColor={miniMapMask}
          maskStrokeColor={miniMapMaskStroke}
          nodeStrokeWidth={1.2}
          nodeBorderRadius={2}
          style="border:1px solid var(--line-strong); border-radius:8px; overflow:hidden; box-shadow: var(--shadow-sm);" />
      {/if}
      <Background variant={BackgroundVariant.Dots} gap={18} size={1} />
    </SvelteFlow>
  {/key}
</div>

<style>
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
.canvas {
  flex: 1 1 auto;
  width: 100%;
  height: 100%;
  min-height: 0;
  border: 1px solid var(--line);
  border-radius: 12px;
  overflow: hidden;
  background: var(--surface, #fff);
  box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.04));
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
  --xy-minimap-background-color: color-mix(in srgb, var(--surface) 96%, var(--canvas, var(--surface-muted)));
  --xy-minimap-mask-background-color: color-mix(in srgb, var(--ink) 13%, transparent);
  --xy-minimap-mask-stroke-color: color-mix(in srgb, var(--ink) 22%, transparent);
  --xy-attribution-background-color: var(--surface);
  --xy-background-pattern-dots-color: color-mix(in srgb, var(--line-strong) 38%, transparent);
}
.canvas :global(.svelte-flow__controls) {
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  box-shadow: var(--shadow-sm, 0 1px 4px rgba(0, 0, 0, 0.06));
}
.canvas :global(.svelte-flow__controls-button) {
  width: 34px;
  height: 34px;
  min-width: 34px;
  min-height: 34px;
}
.canvas :global(.svelte-flow__minimap) {
  width: 148px;
  height: 96px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  box-shadow: var(--shadow-sm);
  overflow: hidden;
}
.canvas :global(.svelte-flow__minimap svg) {
  background: color-mix(in srgb, var(--surface) 96%, var(--canvas, var(--surface-muted)));
}
.canvas :global(.svelte-flow[data-theme="dark"] .svelte-flow__minimap) {
  border-color: #435a4e;
}
.canvas.reduced :global(.svelte-flow__node),
.canvas.reduced :global(.svelte-flow__edge-path),
.canvas.reduced :global(.svelte-flow__controls-button),
.canvas.reduced :global(.svelte-flow__minimap) {
  transition: none !important;
  animation: none !important;
}
@media (prefers-reduced-motion: reduce) {
  .canvas :global(.svelte-flow__node),
  .canvas :global(.svelte-flow__edge-path) {
    transition: none !important;
  }
}
@media (max-width: 900px) {
  .canvas :global(.svelte-flow__minimap) {
    width: 112px;
    height: 72px;
  }
}
</style>
