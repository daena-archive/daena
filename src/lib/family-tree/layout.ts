import type { LayoutGraph, PositionedGraph, PositionedNode } from "./model.ts";

export const ELK_LAYOUT_OPTIONS = {
  "elk.algorithm": "layered",
  "elk.direction": "DOWN",
  "elk.layered.spacing.nodeNodeBetweenLayers": "110",
  "elk.spacing.nodeNode": "72",
  "elk.spacing.edgeNode": "24",
  "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
  "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
  "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
} as const;

export interface ElkNodeInput {
  id: string;
  width: number;
  height: number;
  children?: ElkNodeInput[];
  edges?: ElkEdgeInput[];
  layoutOptions?: Record<string, string>;
  x?: number;
  y?: number;
}

export interface ElkEdgeInput {
  id: string;
  sources: string[];
  targets: string[];
}

export interface LayoutRequest {
  generation: number;
  graph: ElkNodeInput;
}

export interface LayoutResponse {
  generation: number;
  ok: boolean;
  message?: string;
  graph?: ElkNodeInput;
}

export function isCurrentGeneration(current: number, result: number): boolean {
  return current === result && current > 0;
}

export function nextGeneration(current: number): number {
  return current + 1;
}

function compareNodes(left: { id: string }, right: { id: string }, order: Map<string, number>) {
  const leftOrder = order.get(left.id);
  const rightOrder = order.get(right.id);
  if (leftOrder !== undefined || rightOrder !== undefined) {
    return (leftOrder ?? Number.MAX_SAFE_INTEGER) - (rightOrder ?? Number.MAX_SAFE_INTEGER);
  }
  return left.id.localeCompare(right.id);
}

export function buildElkGraph(layout: LayoutGraph, previousOrder: string[] = []): ElkNodeInput {
  const order = new Map(previousOrder.map((id, index) => [id, index]));
  const people = layout.nodes
    .filter((node) => node.kind === "person")
    .sort((left, right) => compareNodes(left, right, order));
  const peopleIds = new Set(people.map((node) => node.id));
  const unions = new Map(layout.nodes.filter((node) => node.kind === "union").map((node) => [node.id, node]));
  const edges = layout.edges.flatMap((edge) => {
    if (edge.role === "direct-parent") {
      return [{ id: edge.id, sources: [edge.source], targets: [edge.target] }];
    }
    if (edge.role !== "child") return [];
    const members = (unions.get(edge.source)?.memberIds ?? []).filter((id) => peopleIds.has(id));
    return members.map((member) => ({
      id: `${edge.id}:${member}`,
      sources: [member],
      targets: [edge.target],
    }));
  });
  edges.sort((left, right) => left.id.localeCompare(right.id));
  return {
    id: "family-tree",
    width: 0,
    height: 0,
    layoutOptions: { ...ELK_LAYOUT_OPTIONS },
    children: people.map((node) => ({
      id: node.id,
      width: node.width,
      height: node.height,
    })),
    edges,
  };
}

export function positionedFromElk(generation: number, layout: LayoutGraph, elk: ElkNodeInput): PositionedGraph {
  const positions = new Map<string, { x: number; y: number }>();
  for (const child of elk.children ?? []) {
    positions.set(child.id, { x: child.x ?? 0, y: child.y ?? 0 });
  }
  const nodes: PositionedNode[] = layout.nodes.map((node) => ({
    ...node,
    x: positions.get(node.id)?.x ?? 0,
    y: positions.get(node.id)?.y ?? 0,
  }));
  return { generation, nodes, edges: layout.edges };
}

const COUPLE_PAD = 28;
const FAMILY_GAP = 72;
const CHILD_GAP_Y = 72;
const CHILD_GAP_X = 36;

function partneredIds(nodes: PositionedNode[]) {
  const ids = new Set<string>();
  for (const node of nodes) {
    if (node.kind !== "union" || (node.memberIds?.length ?? 0) !== 2) continue;
    for (const id of node.memberIds ?? []) ids.add(id);
  }
  return ids;
}

function memberY(union: PositionedNode, byId: Map<string, PositionedNode>) {
  const years = (union.memberIds ?? [])
    .map((id) => byId.get(id)?.y)
    .filter((value): value is number => value !== undefined);
  return years.length > 0 ? Math.min(...years) : union.y;
}

function shiftIds(ids: Iterable<string>, dx: number, dy: number, byId: Map<string, PositionedNode>) {
  for (const id of ids) {
    const node = byId.get(id);
    if (!node) continue;
    node.x += dx;
    node.y += dy;
  }
}

function hangingChildren(
  unionId: string,
  graph: PositionedGraph,
  byId: Map<string, PositionedNode>,
  partnered: Set<string>,
) {
  return graph.edges
    .filter((edge) => edge.source === unionId && edge.role === "child")
    .map((edge) => edge.target)
    .filter((id) => byId.has(id) && !partnered.has(id));
}

function placeCouple(
  union: PositionedNode,
  members: PositionedNode[],
  byId: Map<string, PositionedNode>,
  graph: PositionedGraph,
) {
  const left = members[0].x <= members[1].x ? members[0] : members[1];
  const right = left === members[0] ? members[1] : members[0];
  const y = Math.min(left.y, right.y);
  left.y = y;
  right.y = y;
  const mid = (left.x + left.width / 2 + right.x + right.width / 2) / 2;
  const packed = left.width + COUPLE_PAD + union.width + COUPLE_PAD + right.width;
  left.x = mid - packed / 2;
  union.x = left.x + left.width + COUPLE_PAD;
  union.y = y + Math.max(0, (left.height - union.height) / 2);
  right.x = union.x + union.width + COUPLE_PAD;
  const partnered = partneredIds(graph.nodes);
  const children = hangingChildren(union.id, graph, byId, partnered)
    .map((id) => byId.get(id))
    .filter((node): node is PositionedNode => Boolean(node))
    .sort((a, b) => a.x - b.x || a.id.localeCompare(b.id));
  if (children.length === 0) return;
  const unionMid = union.x + union.width / 2;
  const total = children.reduce((sum, child) => sum + child.width, 0) + CHILD_GAP_X * (children.length - 1);
  let x = unionMid - total / 2;
  const childY = y + left.height + CHILD_GAP_Y;
  for (const child of children) {
    shiftIds([child.id], x - child.x, childY - child.y, byId);
    x += child.width + CHILD_GAP_X;
  }
}

function placeParentGroup(union: PositionedNode, members: PositionedNode[]) {
  const minX = Math.min(...members.map((node) => node.x));
  const maxX = Math.max(...members.map((node) => node.x + node.width));
  const maxBottom = Math.max(...members.map((node) => node.y + node.height));
  union.x = (minX + maxX) / 2 - union.width / 2;
  union.y = maxBottom + 24;
}

type LayoutUnit = { ids: string[]; x: number; y: number; width: number; height: number };

const ROW_BAND = 48;

function boundsOf(ids: string[], byId: Map<string, PositionedNode>): LayoutUnit {
  const nodes = ids.map((id) => byId.get(id)).filter((node): node is PositionedNode => Boolean(node));
  const x = Math.min(...nodes.map((node) => node.x));
  const y = Math.min(...nodes.map((node) => node.y));
  const right = Math.max(...nodes.map((node) => node.x + node.width));
  const bottom = Math.max(...nodes.map((node) => node.y + node.height));
  return { ids, x, y, width: right - x, height: bottom - y };
}

function collectUnits(graph: PositionedGraph, byId: Map<string, PositionedNode>): LayoutUnit[] {
  const claimed = new Set<string>();
  const units: LayoutUnit[] = [];
  for (const union of graph.nodes) {
    if (union.kind !== "union" || (union.memberIds?.length ?? 0) !== 2) continue;
    const members = (union.memberIds ?? [])
      .map((id) => byId.get(id))
      .filter((node): node is PositionedNode => Boolean(node));
    if (members.length < 2) continue;
    const ids = [...members.map((node) => node.id), union.id];
    units.push(boundsOf(ids, byId));
    for (const member of members) claimed.add(member.id);
  }
  for (const node of graph.nodes) {
    if (node.kind !== "person" || claimed.has(node.id)) continue;
    units.push(boundsOf([node.id], byId));
  }
  return units;
}

function resolveCollisions(graph: PositionedGraph, byId: Map<string, PositionedNode>) {
  const units = collectUnits(graph, byId).sort(
    (left, right) => left.y - right.y || left.x - right.x || left.ids[0].localeCompare(right.ids[0]),
  );
  const rows: LayoutUnit[][] = [];
  for (const unit of units) {
    const row = rows.find((candidate) => Math.abs(unit.y - Math.min(...candidate.map((item) => item.y))) < ROW_BAND);
    if (row) row.push(unit);
    else rows.push([unit]);
  }
  let yCursor = Number.NEGATIVE_INFINITY;
  for (const row of rows) {
    row.sort((left, right) => left.x - right.x || left.ids[0].localeCompare(right.ids[0]));
    const rowY = Math.max(Math.min(...row.map((unit) => unit.y)), yCursor);
    let xCursor = Number.NEGATIVE_INFINITY;
    let rowBottom = rowY;
    for (const unit of row) {
      const dx = Math.max(0, xCursor - unit.x);
      const dy = rowY - unit.y;
      if (dx !== 0 || dy !== 0) {
        shiftIds(unit.ids, dx, dy, byId);
        unit.x += dx;
        unit.y += dy;
      }
      xCursor = unit.x + unit.width + FAMILY_GAP;
      rowBottom = Math.max(rowBottom, unit.y + unit.height);
    }
    yCursor = rowBottom + CHILD_GAP_Y;
  }
}

export function placeUnions(graph: PositionedGraph): PositionedGraph {
  const nodes = graph.nodes.map((node) => ({ ...node }));
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const next = { ...graph, nodes };
  const unions = nodes
    .filter((node) => node.kind === "union")
    .sort((left, right) => memberY(left, byId) - memberY(right, byId) || left.id.localeCompare(right.id));
  for (const union of unions) {
    const members = (union.memberIds ?? [])
      .map((id) => byId.get(id))
      .filter((node): node is PositionedNode => Boolean(node));
    if (members.length === 2) placeCouple(union, members, byId, next);
    else if (members.length > 2) placeParentGroup(union, members);
  }
  resolveCollisions(next, byId);
  return next;
}

export class LayoutGeneration {
  private current = 0;

  get value() {
    return this.current;
  }

  start(): number {
    this.current += 1;
    return this.current;
  }

  accept(generation: number): boolean {
    return isCurrentGeneration(this.current, generation);
  }
}
