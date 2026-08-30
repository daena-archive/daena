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
  const spousesOf = new Map<string, string[]>();
  for (const union of unions.values()) {
    const members = (union.memberIds ?? []).filter((id) => peopleIds.has(id));
    if (members.length !== 2) continue;
    const [left, right] = members;
    spousesOf.set(left, [...(spousesOf.get(left) ?? []), right]);
    spousesOf.set(right, [...(spousesOf.get(right) ?? []), left]);
  }
  const seen = new Set(edges.map((edge) => `${edge.sources[0]}->${edge.targets[0]}`));
  for (const edge of [...edges]) {
    const target = edge.targets[0];
    const source = edge.sources[0];
    for (const spouse of spousesOf.get(target) ?? []) {
      if (spouse === source) continue;
      const key = `${source}->${spouse}`;
      if (seen.has(key)) continue;
      seen.add(key);
      edges.push({ id: `${edge.id}:spouse:${spouse}`, sources: [source], targets: [spouse] });
    }
  }
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

function shiftIds(ids: Iterable<string>, dx: number, dy: number, byId: Map<string, PositionedNode>) {
  for (const id of ids) {
    const node = byId.get(id);
    if (!node) continue;
    node.x += dx;
    node.y += dy;
  }
}

function unionChildren(unionId: string, graph: PositionedGraph, byId: Map<string, PositionedNode>) {
  return graph.edges
    .filter((edge) => edge.source === unionId && edge.role === "child")
    .map((edge) => edge.target)
    .filter((id) => byId.has(id));
}

function hangingChildren(
  unionId: string,
  graph: PositionedGraph,
  byId: Map<string, PositionedNode>,
  partnered: Set<string>,
) {
  return unionChildren(unionId, graph, byId).filter((id) => !partnered.has(id));
}

function componentContaining(id: string, components: string[][]) {
  return components.find((ids) => ids.includes(id)) ?? [id];
}

function shiftFamily(
  personId: string,
  dx: number,
  dy: number,
  byId: Map<string, PositionedNode>,
  graph: PositionedGraph,
  partnered: Set<string>,
  components: string[][],
) {
  const couple = componentContaining(personId, components);
  shiftIds(couple, dx, dy, byId);
  for (const id of couple) {
    const node = byId.get(id);
    if (node?.kind !== "union") continue;
    shiftIds(hangingChildren(node.id, graph, byId, partnered), dx, dy, byId);
  }
}

type ChildBlock = { ids: string[]; childId: string; x: number; y: number; width: number; height: number };

function childBlocks(
  unionId: string,
  graph: PositionedGraph,
  byId: Map<string, PositionedNode>,
  partnered: Set<string>,
  components: string[][],
): ChildBlock[] {
  const children = unionChildren(unionId, graph, byId)
    .map((id) => byId.get(id))
    .filter((node): node is PositionedNode => Boolean(node))
    .sort((left, right) => left.x - right.x || left.id.localeCompare(right.id));
  return children.map((child) => {
    const ids = partnered.has(child.id) ? componentContaining(child.id, components) : [child.id];
    return { ...boundsOf(ids, byId), ids, childId: child.id };
  });
}

function clusterWidth(nodes: PositionedNode[]) {
  if (nodes.length === 0) return 0;
  return nodes.reduce((sum, node) => sum + node.width, 0) + CHILD_GAP_X * (nodes.length - 1);
}

function twoMemberUnions(nodes: PositionedNode[], byId: Map<string, PositionedNode>) {
  return nodes.filter((node) => {
    if (node.kind !== "union") return false;
    const members = (node.memberIds ?? []).filter((id) => byId.has(id));
    return members.length === 2;
  });
}

function marriageComponents(nodes: PositionedNode[], byId: Map<string, PositionedNode>) {
  const unions = twoMemberUnions(nodes, byId);
  const parent = new Map<string, string>();
  const find = (id: string): string => {
    const next = parent.get(id) ?? id;
    if (next !== id) parent.set(id, find(next));
    parent.set(id, parent.get(id) ?? id);
    return parent.get(id) ?? id;
  };
  const unite = (left: string, right: string) => {
    const a = find(left);
    const b = find(right);
    if (a !== b) parent.set(a, b);
  };
  for (const union of unions) {
    const members = (union.memberIds ?? []).filter((id) => byId.has(id));
    unite(members[0], members[1]);
    unite(members[0], union.id);
  }
  const groups = new Map<string, string[]>();
  for (const union of unions) {
    const members = (union.memberIds ?? []).filter((id) => byId.has(id));
    const root = find(members[0]);
    const ids = groups.get(root) ?? [];
    for (const id of [...members, union.id]) {
      if (!ids.includes(id)) ids.push(id);
    }
    groups.set(root, ids);
  }
  return [...groups.values()];
}

function chainOrder(unions: PositionedNode[], byId: Map<string, PositionedNode>) {
  const adj = new Map<string, { spouse: string; union: PositionedNode }[]>();
  for (const union of unions) {
    const members = (union.memberIds ?? []).filter((id) => byId.has(id));
    if (members.length !== 2) continue;
    const [left, right] = members;
    adj.set(left, [...(adj.get(left) ?? []), { spouse: right, union }]);
    adj.set(right, [...(adj.get(right) ?? []), { spouse: left, union }]);
  }
  const people = [...adj.keys()].sort((left, right) => {
    const a = byId.get(left);
    const b = byId.get(right);
    return (a?.x ?? 0) - (b?.x ?? 0) || left.localeCompare(right);
  });
  if (people.length === 0) return { people: [] as string[], unions: [] as PositionedNode[] };
  const leaves = people.filter((id) => (adj.get(id)?.length ?? 0) <= 1);
  const start = (leaves[0] ?? people[0]) as string;
  const orderedPeople = [start];
  const orderedUnions: PositionedNode[] = [];
  const used = new Set<string>();
  let current = start;
  while (true) {
    const next = (adj.get(current) ?? []).find((link) => !used.has(link.union.id));
    if (!next) break;
    used.add(next.union.id);
    orderedUnions.push(next.union);
    orderedPeople.push(next.spouse);
    current = next.spouse;
  }
  for (const union of unions) {
    if (used.has(union.id)) continue;
    const members = (union.memberIds ?? []).filter((id) => byId.has(id));
    const attached = members.find((id) => orderedPeople.includes(id));
    const other = members.find((id) => id !== attached);
    if (!attached || !other) continue;
    orderedPeople.push(other);
    orderedUnions.push(union);
    used.add(union.id);
  }
  return { people: orderedPeople, unions: orderedUnions };
}

function placeHangingCluster(children: PositionedNode[], mid: number, y: number, byId: Map<string, PositionedNode>) {
  let x = mid - clusterWidth(children) / 2;
  for (const child of children) {
    shiftIds([child.id], x - child.x, y - child.y, byId);
    x += child.width + CHILD_GAP_X;
  }
}

function blockRowWidth(blocks: ChildBlock[]) {
  if (blocks.length === 0) return 0;
  return blocks.reduce((sum, block) => sum + block.width, 0) + CHILD_GAP_X * (blocks.length - 1);
}

function placeChildBlocks(
  blocks: ChildBlock[],
  mid: number,
  y: number,
  byId: Map<string, PositionedNode>,
  graph: PositionedGraph,
  partnered: Set<string>,
  components: string[][],
) {
  let x = mid - blockRowWidth(blocks) / 2;
  for (const block of blocks) {
    const child = byId.get(block.childId);
    const dy = child ? y - child.y : y - block.y;
    const dx = x - block.x;
    if (partnered.has(block.childId)) shiftFamily(block.childId, dx, dy, byId, graph, partnered, components);
    else shiftIds(block.ids, dx, dy, byId);
    x += block.width + CHILD_GAP_X;
  }
}

function placeMarriageChain(unions: PositionedNode[], byId: Map<string, PositionedNode>, extras: number[] = []) {
  const { people, unions: orderedUnions } = chainOrder(unions, byId);
  const members = people.map((id) => byId.get(id)).filter((node): node is PositionedNode => Boolean(node));
  if (members.length === 0 || orderedUnions.length === 0) return { people, unions: orderedUnions };
  const y = Math.max(...members.map((node) => node.y));
  const pad = orderedUnions.map((_, index) => extras[index] ?? 0);
  const packed =
    members.reduce((sum, node) => sum + node.width, 0) +
    orderedUnions.reduce((sum, union) => sum + union.width, 0) +
    pad.reduce((sum, extra) => sum + extra, 0) +
    COUPLE_PAD * (members.length + orderedUnions.length - 1);
  const centroid = members.reduce((sum, node) => sum + node.x + node.width / 2, 0) / members.length;
  let x = centroid - packed / 2;
  const height = members[0]?.height ?? 0;
  for (let index = 0; index < people.length; index += 1) {
    const person = byId.get(people[index] ?? "");
    if (!person) continue;
    person.x = x;
    person.y = y;
    x += person.width + COUPLE_PAD;
    const union = orderedUnions[index];
    if (!union) continue;
    union.x = x;
    union.y = y + Math.max(0, (height - union.height) / 2);
    x += union.width + COUPLE_PAD + (pad[index] ?? 0);
  }
  return { people, unions: orderedUnions };
}

function placeChainChildren(unions: PositionedNode[], byId: Map<string, PositionedNode>, graph: PositionedGraph) {
  const { people, unions: orderedUnions } = chainOrder(unions, byId);
  const members = people.map((id) => byId.get(id)).filter((node): node is PositionedNode => Boolean(node));
  if (members.length === 0 || orderedUnions.length === 0) return;
  const partnered = partneredIds(graph.nodes);
  const components = marriageComponents(graph.nodes, byId);
  const groups = orderedUnions.map((union) => childBlocks(union.id, graph, byId, partnered, components));
  const extras = orderedUnions.map(() => 0);
  for (let index = 0; index < orderedUnions.length - 1; index += 1) {
    const leftWidth = blockRowWidth(groups[index] ?? []);
    const rightWidth = blockRowWidth(groups[index + 1] ?? []);
    if (leftWidth === 0 || rightWidth === 0) continue;
    const shared = byId.get(people[index + 1]);
    const leftUnion = orderedUnions[index];
    const rightUnion = orderedUnions[index + 1];
    if (!shared || !leftUnion || !rightUnion) continue;
    const natural = leftUnion.width / 2 + COUPLE_PAD + shared.width + COUPLE_PAD + rightUnion.width / 2;
    extras[index] = Math.max(0, leftWidth / 2 + FAMILY_GAP + rightWidth / 2 - natural);
  }
  placeMarriageChain(unions, byId, extras);
  const y = Math.max(...members.map((node) => node.y));
  const height = members[0]?.height ?? 0;
  const childY = y + height + CHILD_GAP_Y;
  const serial = orderedUnions.length > 1;
  for (let index = 0; index < orderedUnions.length; index += 1) {
    const union = orderedUnions[index];
    const blocks = groups[index] ?? [];
    if (!union || blocks.length === 0) continue;
    const hanging = blocks.filter((block) => !partnered.has(block.childId));
    const forceGroup = serial || hanging.length > 0;
    if (forceGroup) {
      const next = childBlocks(union.id, graph, byId, partnered, components);
      placeChildBlocks(next, union.x + union.width / 2, childY, byId, graph, partnered, components);
      continue;
    }
    for (const block of blocks) {
      if (!partnered.has(block.childId)) continue;
      const child = byId.get(block.childId);
      if (!child) continue;
      shiftFamily(block.childId, 0, childY - child.y, byId, graph, partnered, components);
    }
    placeHangingCluster(
      hanging.map((block) => byId.get(block.childId)).filter((node): node is PositionedNode => Boolean(node)),
      union.x + union.width / 2,
      childY,
      byId,
    );
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
  const partnered = partneredIds(graph.nodes);
  const components = marriageComponents(graph.nodes, byId);
  for (const union of twoMemberUnions(graph.nodes, byId)) {
    const ids: string[] = [];
    for (const block of childBlocks(union.id, graph, byId, partnered, components)) {
      for (const id of block.ids) {
        if (!ids.includes(id)) ids.push(id);
      }
    }
    if (ids.length === 0) continue;
    units.push(boundsOf(ids, byId));
    for (const id of ids) {
      if (byId.get(id)?.kind === "person") claimed.add(id);
    }
  }
  for (const ids of components) {
    const remaining = ids.filter((id) => {
      const node = byId.get(id);
      return node && (node.kind === "union" || !claimed.has(id));
    });
    if (!remaining.some((id) => byId.get(id)?.kind === "person")) continue;
    units.push(boundsOf(remaining, byId));
    for (const id of remaining) {
      if (byId.get(id)?.kind === "person") claimed.add(id);
    }
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
  const placed = new Set<string>();
  const components = marriageComponents(nodes, byId).sort((left, right) => {
    const leftY = Math.min(...left.map((id) => byId.get(id)?.y ?? 0));
    const rightY = Math.min(...right.map((id) => byId.get(id)?.y ?? 0));
    return leftY - rightY || (left[0] ?? "").localeCompare(right[0] ?? "");
  });
  for (const ids of components) {
    const unions = ids.map((id) => byId.get(id)).filter((node): node is PositionedNode => node?.kind === "union");
    placeMarriageChain(unions, byId);
    for (const union of unions) placed.add(union.id);
  }
  for (const union of nodes) {
    if (union.kind !== "union" || placed.has(union.id)) continue;
    const members = (union.memberIds ?? [])
      .map((id) => byId.get(id))
      .filter((node): node is PositionedNode => Boolean(node));
    if (members.length > 2) placeParentGroup(union, members);
  }
  for (const ids of components) {
    const unions = ids.map((id) => byId.get(id)).filter((node): node is PositionedNode => node?.kind === "union");
    placeChainChildren(unions, byId, next);
  }
  resolveCollisions(next, byId);
  return next;
}

export type FamilyPortHandle = "north" | "south" | "east" | "west";

export function familyEdgeHandles(
  edge: { role: string; source: string; target: string },
  nodes: Iterable<{ id: string; x: number; y: number; width: number; height: number }>,
): { sourceHandle: FamilyPortHandle; targetHandle: FamilyPortHandle } {
  const byId = new Map([...nodes].map((node) => [node.id, node]));
  const source = byId.get(edge.source);
  const target = byId.get(edge.target);
  const sourceMidX = (source?.x ?? 0) + (source?.width ?? 0) / 2;
  const targetMidX = (target?.x ?? 0) + (target?.width ?? 0) / 2;
  const sameRow =
    source !== undefined &&
    target !== undefined &&
    source.y < target.y + target.height &&
    target.y < source.y + source.height;
  const sideways = edge.role === "partner" || (edge.role === "parent" && sameRow);
  if (sideways) {
    if (sourceMidX <= targetMidX) return { sourceHandle: "east", targetHandle: "west" };
    return { sourceHandle: "west", targetHandle: "east" };
  }
  return { sourceHandle: "south", targetHandle: "north" };
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
