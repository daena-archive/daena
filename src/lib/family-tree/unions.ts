import {
  PERSON_NODE_HEIGHT,
  PERSON_NODE_WIDTH,
  UNION_NODE_SIZE,
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
      unions.set(unionId, { id: unionId, kind: "union", width: UNION_NODE_SIZE, height: UNION_NODE_SIZE });
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
      const matchingParentUnion = [...unions.keys()].find((id) => {
        const members = id.startsWith("union:parents:") ? id.slice("union:parents:".length).split(":") : [];
        return (
          members.length === 2 && members.includes(relationship.sourceId) && members.includes(relationship.targetId)
        );
      });
      const unionId = matchingParentUnion ?? partnerUnionId(relationship.id);
      if (!unions.has(unionId)) {
        unions.set(unionId, { id: unionId, kind: "union", width: UNION_NODE_SIZE, height: UNION_NODE_SIZE });
      }
      attachPartner(edges, relationship, unionId);
    }
  }

  const unionNodes = [...unions.values()].sort((left, right) => compareId(left.id, right.id));
  const keptEdges = edges.sort((left, right) => compareId(left.id, right.id));
  return { nodes: [...nodes, ...unionNodes], edges: keptEdges };
}

export function layoutGraphExceedsLimits(graph: LayoutGraph): boolean {
  const people = graph.nodes.filter((node) => node.kind === "person").length;
  const unions = graph.nodes.filter((node) => node.kind === "union").length;
  return layoutExceedsLimits(people, unions, graph.edges.length);
}

function attachPartner(edges: LayoutEdge[], relationship: FamilyRelationship, unionId: string) {
  for (const personId of [relationship.sourceId, relationship.targetId].sort(compareId)) {
    const edgeId = `edge:partner:${unionId}:${personId}`;
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
