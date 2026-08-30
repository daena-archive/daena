import {
  PERSON_NODE_HEIGHT,
  PERSON_NODE_WIDTH,
  UNION_NODE_HEIGHT,
  UNION_NODE_WIDTH,
  layoutExceedsLimits,
  type FamilyRelationship,
  type GenealogyGraph,
  type LayoutEdge,
  type LayoutGraph,
  type LayoutNode,
} from "./model.ts";

function parentUnionId(parentIds: string[]): string {
  return `union:parents:${[...parentIds].sort().join(":")}`;
}

function partnerUnionId(relationshipId: string): string {
  return `union:partner:${relationshipId}`;
}

function compareId(left: string, right: string) {
  return left.localeCompare(right);
}

export function buildLayoutGraph(graph: GenealogyGraph, visibleIds: Iterable<string>): LayoutGraph {
  const visible = new Set([...visibleIds].filter((id) => graph.people.has(id)));
  const nodes: LayoutNode[] = [...visible].sort(compareId).map((id) => ({
    id,
    kind: "person" as const,
    personId: id,
    width: PERSON_NODE_WIDTH,
    height: PERSON_NODE_HEIGHT,
  }));
  const unions = new Map<string, LayoutNode>();
  const edges: LayoutEdge[] = [];

  for (const childId of [...visible].sort(compareId)) {
    const parents = [...(graph.parentsByChild.get(childId) ?? [])].filter((id) => visible.has(id)).sort(compareId);
    if (parents.length === 0) continue;
    if (parents.length === 1) {
      const parentId = parents[0];
      const relationship =
        (graph.parentRelationshipsByChild.get(childId) ?? []).find((candidate) => candidate.sourceId === parentId) ??
        null;
      edges.push({
        id: `edge:direct:${parentId}:${childId}`,
        source: parentId,
        target: childId,
        relationshipId: relationship?.id ?? null,
        role: "direct-parent",
        parentKind: relationship?.parentKind ?? null,
        partnerKind: null,
        label: relationship?.label ?? "Parent",
        arrow: true,
        start: relationship?.start ?? null,
        end: relationship?.end ?? null,
      });
      continue;
    }
    const unionId = parentUnionId(parents);
    if (!unions.has(unionId)) {
      unions.set(unionId, {
        id: unionId,
        kind: "union",
        memberIds: parents,
        width: UNION_NODE_WIDTH,
        height: UNION_NODE_HEIGHT,
      });
      for (const parentId of parents) {
        edges.push({
          id: `edge:parent:${unionId}:${parentId}`,
          source: parentId,
          target: unionId,
          relationshipId: null,
          role: "parent",
          parentKind: null,
          partnerKind: null,
          label: "",
          arrow: false,
          start: null,
          end: null,
        });
      }
    }
    const representative =
      (graph.parentRelationshipsByChild.get(childId) ?? []).find((candidate) => parents.includes(candidate.sourceId)) ??
      null;
    edges.push({
      id: `edge:child:${unionId}:${childId}`,
      source: unionId,
      target: childId,
      relationshipId: representative?.id ?? null,
      role: "child",
      parentKind: representative?.parentKind ?? null,
      partnerKind: null,
      label: representative?.label ?? "Child",
      arrow: true,
      start: representative?.start ?? null,
      end: representative?.end ?? null,
    });
  }

  const partnerSeen = new Set<string>();
  for (const personId of [...visible].sort(compareId)) {
    for (const relationship of graph.partnerRelationshipsByPerson.get(personId) ?? []) {
      if (partnerSeen.has(relationship.id)) continue;
      partnerSeen.add(relationship.id);
      if (!visible.has(relationship.sourceId) || !visible.has(relationship.targetId)) continue;
      const matchingParentUnion = [...unions.values()].find((node) =>
        samePair(node.memberIds, relationship.sourceId, relationship.targetId),
      );
      const unionId = matchingParentUnion?.id ?? partnerUnionId(relationship.id);
      if (!unions.has(unionId)) {
        unions.set(unionId, {
          id: unionId,
          kind: "union",
          memberIds: [relationship.sourceId, relationship.targetId].sort(compareId),
          width: UNION_NODE_WIDTH,
          height: UNION_NODE_HEIGHT,
        });
      }
      attachPartner(edges, relationship, unionId);
    }
  }

  const unionNodes = [...unions.values()].sort((left, right) => compareId(left.id, right.id));
  const keptEdges = edges.sort((left, right) => compareId(left.id, right.id));
  return { nodes: [...nodes, ...unionNodes], edges: keptEdges };
}

export function layoutGraphExceedsLimits(
  graph: LayoutGraph,
  limits?: Parameters<typeof layoutExceedsLimits>[3],
): boolean {
  const people = graph.nodes.filter((node) => node.kind === "person").length;
  const unions = graph.nodes.filter((node) => node.kind === "union").length;
  return layoutExceedsLimits(people, unions, graph.edges.length, limits);
}

function samePair(members: string[] | undefined, left: string, right: string) {
  return Boolean(members && members.length === 2 && members.includes(left) && members.includes(right));
}

function attachPartner(edges: LayoutEdge[], relationship: FamilyRelationship, unionId: string) {
  for (const personId of [relationship.sourceId, relationship.targetId].sort(compareId)) {
    const edgeId = `edge:partner:${unionId}:${personId}`;
    const parentEdgeId = `edge:parent:${unionId}:${personId}`;
    const parentIndex = edges.findIndex((edge) => edge.id === parentEdgeId);
    if (parentIndex >= 0) edges.splice(parentIndex, 1);
    if (edges.some((edge) => edge.id === edgeId)) continue;
    edges.push({
      id: edgeId,
      source: personId,
      target: unionId,
      relationshipId: relationship.id,
      role: "partner",
      parentKind: null,
      partnerKind: relationship.partnerKind,
      label: relationship.label,
      arrow: false,
      start: relationship.start,
      end: relationship.end,
    });
  }
}

export function parentSetKey(parentIds: string[]): string {
  return parentUnionId(parentIds);
}

export function coupleClickAction(
  edge: LayoutEdge,
  nodes: LayoutNode[],
  edges: LayoutEdge[],
): { relationshipId: string } | { memberIds: [string, string] } | null {
  if (edge.relationshipId) return { relationshipId: edge.relationshipId };
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const source = byId.get(edge.source);
  const target = byId.get(edge.target);
  const union = source?.kind === "union" ? source : target?.kind === "union" ? target : null;
  const members = union?.memberIds?.filter((id) => byId.get(id)?.kind === "person") ?? [];
  if (members.length !== 2 || edge.role !== "parent") return null;
  const [left, right] = [...members].sort(compareId);
  return { memberIds: [left, right] as [string, string] };
}

export function unionClickAction(
  unionId: string,
  nodes: LayoutNode[],
  edges: LayoutEdge[],
): { relationshipId: string } | { memberIds: [string, string] } | null {
  const union = nodes.find((node) => node.id === unionId && node.kind === "union");
  const members =
    union?.memberIds?.filter((id) => nodes.some((node) => node.id === id && node.kind === "person")) ?? [];
  if (members.length !== 2) return null;
  const partner = edges.find((edge) => edge.role === "partner" && edge.target === unionId && edge.relationshipId);
  if (partner?.relationshipId) return { relationshipId: partner.relationshipId };
  const [left, right] = [...members].sort(compareId);
  return { memberIds: [left, right] as [string, string] };
}
