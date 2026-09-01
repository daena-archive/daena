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

function partnerPairKey(left: string, right: string): string {
  return left < right ? `${left}:${right}` : `${right}:${left}`;
}

function visiblePartnerPairs(graph: GenealogyGraph, visible: Set<string>): Set<string> {
  const pairs = new Set<string>();
  for (const personId of visible) {
    for (const partnerId of graph.partnersByPerson.get(personId) ?? []) {
      if (!visible.has(partnerId)) continue;
      pairs.add(partnerPairKey(personId, partnerId));
    }
  }
  return pairs;
}

function arePartners(pairs: Set<string>, left: string, right: string): boolean {
  return pairs.has(partnerPairKey(left, right));
}

function reusableParentUnion(unions: Map<string, LayoutNode>, parents: string[], childId: string): LayoutNode | null {
  if (parents.length <= 2) return null;
  let best: LayoutNode | null = null;
  let bestSize = 0;
  for (const union of unions.values()) {
    const members = (union.memberIds ?? []).filter((id) => parents.includes(id));
    if (members.length < 2 || members.length !== (union.memberIds?.length ?? 0)) continue;
    if ((union.memberIds ?? []).includes(childId)) continue;
    if (members.length > bestSize) {
      best = union;
      bestSize = members.length;
    }
  }
  return best;
}

function attachDirectParent(edges: LayoutEdge[], graph: GenealogyGraph, parentId: string, childId: string) {
  const relationship =
    (graph.parentRelationshipsByChild.get(childId) ?? []).find((candidate) => candidate.sourceId === parentId) ?? null;
  const edgeId = `edge:direct:${parentId}:${childId}`;
  if (edges.some((edge) => edge.id === edgeId)) return;
  edges.push({
    id: edgeId,
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
}

function attachChildEdge(edges: LayoutEdge[], graph: GenealogyGraph, union: LayoutNode, childId: string) {
  const edgeId = `edge:child:${union.id}:${childId}`;
  if (edges.some((edge) => edge.id === edgeId)) return;
  const unionMembers = new Set(union.memberIds ?? []);
  const parentLinks = (graph.parentRelationshipsByChild.get(childId) ?? [])
    .filter((candidate) => unionMembers.has(candidate.sourceId))
    .sort((left, right) => compareId(left.sourceId, right.sourceId));
  const representative = parentLinks[0] ?? null;
  edges.push({
    id: edgeId,
    source: union.id,
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

export function buildLayoutGraph(graph: GenealogyGraph, visibleIds: Iterable<string>): LayoutGraph {
  const visible = new Set([...visibleIds].filter((id) => graph.people.has(id)));
  const partnerPairs = visiblePartnerPairs(graph, visible);
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
    const rawParents = [...(graph.parentsByChild.get(childId) ?? [])].filter((id) => visible.has(id));
    const parents = rawParents.filter((id) => !arePartners(partnerPairs, id, childId)).sort(compareId);
    if (parents.length === 1) {
      attachDirectParent(edges, graph, parents[0], childId);
    } else if (parents.length >= 2) {
      const sharedUnion = reusableParentUnion(unions, parents, childId);
      if (sharedUnion) {
        attachChildEdge(edges, graph, sharedUnion, childId);
        for (const parentId of parents) {
          if (sharedUnion.memberIds?.includes(parentId)) continue;
          attachDirectParent(edges, graph, parentId, childId);
        }
      } else {
        const unionId = parentUnionId(parents);
        let union = unions.get(unionId);
        if (!union) {
          union = {
            id: unionId,
            kind: "union",
            memberIds: parents,
            width: UNION_NODE_WIDTH,
            height: UNION_NODE_HEIGHT,
          };
          unions.set(unionId, union);
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
        attachChildEdge(edges, graph, union, childId);
      }
    }
    for (const parentId of rawParents) {
      if (!arePartners(partnerPairs, parentId, childId)) continue;
      attachDirectParent(edges, graph, parentId, childId);
    }
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
