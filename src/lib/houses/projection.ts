import type { EntityRecord, EntitySummary, Relationship } from "../../../packages/module-api/src/index";
import {
  type BranchDirection,
  type FamilyPerson,
  type FamilyRelationship,
  type GenealogyGraph,
  type GenealogyWarning,
  type HiddenCounts,
  BRANCH_DIRECTIONS,
  INITIAL_ANCESTOR_GENERATIONS,
  INITIAL_DESCENDANT_GENERATIONS,
  MAX_EXPANSION_DEPTH,
  PARENT_RELATIONSHIP,
  PARTNER_RELATIONSHIP,
  VISIBLE_PERSON_LIMIT,
  expansionKey,
  isParentKind,
  isPartnerKind,
  isPartnerStatus,
  isPersonType,
} from "./model.ts";

function emptyIndex<T>(): Map<string, Set<T>> {
  return new Map();
}

function addToIndex(index: Map<string, Set<string>>, key: string, value: string) {
  const existing = index.get(key);
  if (existing) existing.add(value);
  else index.set(key, new Set([value]));
}

function addRelIndex(index: Map<string, FamilyRelationship[]>, key: string, value: FamilyRelationship) {
  const existing = index.get(key);
  if (existing) existing.push(value);
  else index.set(key, [value]);
}

function readString(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

export function parseFamilyRelationship(relationship: Relationship): {
  parsed: FamilyRelationship;
  warning: GenealogyWarning | null;
} {
  const metadata = relationship.metadata ?? {};
  if (relationship.type === PARENT_RELATIONSHIP) {
    const parentKind = isParentKind(metadata.kind) ? metadata.kind : null;
    const customLabel = readString(metadata.customLabel);
    const unknown = parentKind === null;
    const label =
      parentKind === "custom" && customLabel
        ? customLabel
        : parentKind
          ? parentKind.replace(/^\w/, (letter) => letter.toUpperCase())
          : "Unknown";
    return {
      parsed: {
        id: relationship.id,
        kind: "parent",
        type: relationship.type,
        sourceId: relationship.sourceId,
        targetId: relationship.targetId,
        revision: relationship.revision,
        parentKind,
        partnerKind: null,
        status: null,
        customLabel,
        start: metadata.start ?? null,
        end: metadata.end ?? null,
        notes: readString(metadata.notes),
        label,
        unknown,
      },
      warning: unknown
        ? { relationshipId: relationship.id, message: "Parent relationship metadata is missing or invalid." }
        : null,
    };
  }
  if (relationship.type === PARTNER_RELATIONSHIP) {
    const partnerKind = isPartnerKind(metadata.kind) ? metadata.kind : null;
    const status =
      metadata.status === undefined || metadata.status === null
        ? "unknown"
        : isPartnerStatus(metadata.status)
          ? metadata.status
          : null;
    const customLabel = readString(metadata.customLabel);
    const unknown = partnerKind === null || status === null;
    const label =
      partnerKind === "custom" && customLabel
        ? customLabel
        : partnerKind
          ? partnerKind.replace(/^\w/, (letter) => letter.toUpperCase())
          : "Unknown";
    return {
      parsed: {
        id: relationship.id,
        kind: "partner",
        type: relationship.type,
        sourceId: relationship.sourceId,
        targetId: relationship.targetId,
        revision: relationship.revision,
        parentKind: null,
        partnerKind,
        status,
        customLabel,
        start: metadata.start ?? null,
        end: metadata.end ?? null,
        notes: readString(metadata.notes),
        label,
        unknown,
      },
      warning: unknown
        ? { relationshipId: relationship.id, message: "Partner relationship metadata is missing or invalid." }
        : null,
    };
  }
  return {
    parsed: {
      id: relationship.id,
      kind: "parent",
      type: relationship.type,
      sourceId: relationship.sourceId,
      targetId: relationship.targetId,
      revision: relationship.revision,
      parentKind: null,
      partnerKind: null,
      status: null,
      customLabel: null,
      start: null,
      end: null,
      notes: null,
      label: "Unknown",
      unknown: true,
    },
    warning: { relationshipId: relationship.id, message: `Unsupported relationship type ${relationship.type}.` },
  };
}

export function personFromRecord(
  record: Pick<EntityRecord, "id" | "name" | "revision" | "type" | "deleted">,
  fields: Record<string, unknown>,
  secondaryField = "occupation",
): FamilyPerson | null {
  if (record.deleted || !isPersonType(record.type)) return null;
  const secondary = fields[secondaryField];
  return {
    id: record.id,
    name: record.name,
    revision: record.revision,
    birth: (fields.birth as FamilyPerson["birth"]) ?? null,
    death: (fields.death as FamilyPerson["death"]) ?? null,
    secondaryLabel: typeof secondary === "string" && secondary.trim() ? secondary.trim() : null,
  };
}

export function normalizeGenealogy(
  people: Iterable<FamilyPerson>,
  relationships: Iterable<Relationship>,
): { graph: GenealogyGraph; warnings: GenealogyWarning[] } {
  const graph: GenealogyGraph = {
    people: new Map(),
    parentsByChild: emptyIndex(),
    childrenByParent: emptyIndex(),
    partnersByPerson: emptyIndex(),
    relationships: new Map(),
    parentRelationshipsByChild: new Map(),
    partnerRelationshipsByPerson: new Map(),
  };
  const warnings: GenealogyWarning[] = [];
  for (const person of people) graph.people.set(person.id, person);
  const seen = new Set<string>();
  for (const relationship of relationships) {
    if (seen.has(relationship.id)) continue;
    seen.add(relationship.id);
    if (relationship.type !== PARENT_RELATIONSHIP && relationship.type !== PARTNER_RELATIONSHIP) continue;
    const source = graph.people.get(relationship.sourceId);
    const target = graph.people.get(relationship.targetId);
    if (!source || !target) {
      warnings.push({
        relationshipId: relationship.id,
        message: "Relationship endpoints are missing, deleted, or not Lore people.",
      });
      continue;
    }
    const { parsed, warning } = parseFamilyRelationship(relationship);
    if (warning) warnings.push(warning);
    graph.relationships.set(parsed.id, parsed);
    if (parsed.kind === "parent") {
      addToIndex(graph.parentsByChild, parsed.targetId, parsed.sourceId);
      addToIndex(graph.childrenByParent, parsed.sourceId, parsed.targetId);
      addRelIndex(graph.parentRelationshipsByChild, parsed.targetId, parsed);
    } else {
      addToIndex(graph.partnersByPerson, parsed.sourceId, parsed.targetId);
      addToIndex(graph.partnersByPerson, parsed.targetId, parsed.sourceId);
      addRelIndex(graph.partnerRelationshipsByPerson, parsed.sourceId, parsed);
      addRelIndex(graph.partnerRelationshipsByPerson, parsed.targetId, parsed);
    }
  }
  return { graph, warnings };
}

function walk(ids: Iterable<string>, hops: number, next: (id: string) => Iterable<string> | undefined): Set<string> {
  const visible = new Set<string>();
  let frontier = [...ids];
  for (let generation = 0; generation < hops; generation += 1) {
    const upcoming: string[] = [];
    for (const id of frontier) {
      for (const neighbor of next(id) ?? []) {
        if (visible.has(neighbor)) continue;
        visible.add(neighbor);
        upcoming.push(neighbor);
      }
    }
    frontier = upcoming;
  }
  return visible;
}

function shareVisibleChild(graph: GenealogyGraph, left: string, right: string, visible: Set<string>): boolean {
  const leftChildren = graph.childrenByParent.get(left);
  const rightChildren = graph.childrenByParent.get(right);
  if (!leftChildren || !rightChildren) return false;
  for (const child of leftChildren) {
    if (visible.has(child) && rightChildren.has(child)) return true;
  }
  return false;
}

export function initialNeighborhood(
  graph: GenealogyGraph,
  rootId: string,
  ancestorGenerations = INITIAL_ANCESTOR_GENERATIONS,
  descendantGenerations = INITIAL_DESCENDANT_GENERATIONS,
): Set<string> {
  const visible = new Set<string>([rootId]);
  if (!graph.people.has(rootId)) return visible;
  for (const id of walk([rootId], ancestorGenerations, (id) => graph.parentsByChild.get(id))) visible.add(id);
  for (const id of walk([rootId], descendantGenerations, (id) => graph.childrenByParent.get(id))) visible.add(id);
  for (const parent of graph.parentsByChild.get(rootId) ?? []) {
    for (const sibling of graph.childrenByParent.get(parent) ?? []) visible.add(sibling);
  }
  const included = [...visible];
  for (const id of included) {
    for (const partner of graph.partnersByPerson.get(id) ?? []) {
      const rels = graph.partnerRelationshipsByPerson.get(id) ?? [];
      const rel = rels.find((candidate) => candidate.sourceId === partner || candidate.targetId === partner);
      if (rel?.status === "ended") {
        if (shareVisibleChild(graph, id, partner, visible)) visible.add(partner);
        continue;
      }
      visible.add(partner);
    }
  }
  return visible;
}

export function visiblePeople(graph: GenealogyGraph, ids: Iterable<string>): FamilyPerson[] {
  return [...ids]
    .map((id) => graph.people.get(id))
    .filter((person): person is FamilyPerson => Boolean(person))
    .sort((left, right) => left.id.localeCompare(right.id));
}

export function wouldExceedVisibleLimit(count: number, limit = VISIBLE_PERSON_LIMIT): boolean {
  return count > limit;
}

export function summariesToPeople(
  summaries: EntitySummary[],
  fieldsById: Map<string, Record<string, unknown>>,
  secondaryField = "occupation",
): { people: FamilyPerson[]; warnings: GenealogyWarning[] } {
  const people: FamilyPerson[] = [];
  const warnings: GenealogyWarning[] = [];
  for (const summary of summaries) {
    const person = personFromRecord(summary, fieldsById.get(summary.id) ?? {}, secondaryField);
    if (!person) {
      warnings.push({ entityId: summary.id, message: "Skipped a non-person or deleted entity." });
      continue;
    }
    people.push(person);
  }
  return { people, warnings };
}

export function siblingsOf(graph: GenealogyGraph, id: string): Set<string> {
  const siblings = new Set<string>();
  for (const parent of graph.parentsByChild.get(id) ?? []) {
    for (const child of graph.childrenByParent.get(parent) ?? []) {
      if (child !== id) siblings.add(child);
    }
  }
  return siblings;
}

function neighborsFor(graph: GenealogyGraph, id: string, direction: BranchDirection): Iterable<string> {
  if (direction === "parents") return graph.parentsByChild.get(id) ?? [];
  if (direction === "children") return graph.childrenByParent.get(id) ?? [];
  if (direction === "siblings") return siblingsOf(graph, id);
  return graph.partnersByPerson.get(id) ?? [];
}

function partnerIncluded(
  graph: GenealogyGraph,
  id: string,
  partner: string,
  visible: Set<string>,
  explicit: boolean,
): boolean {
  const rel = (graph.partnerRelationshipsByPerson.get(id) ?? []).find(
    (candidate) => candidate.sourceId === partner || candidate.targetId === partner,
  );
  if (!rel) return false;
  if (rel.status !== "ended") return true;
  if (shareVisibleChild(graph, id, partner, visible)) return true;
  return explicit;
}

export function seedInitialExpansions(
  graph: GenealogyGraph,
  rootId: string,
  ancestorGenerations = INITIAL_ANCESTOR_GENERATIONS,
  descendantGenerations = INITIAL_DESCENDANT_GENERATIONS,
): Set<string> {
  const keys = new Set<string>([
    expansionKey(rootId, "parents"),
    expansionKey(rootId, "children"),
    expansionKey(rootId, "siblings"),
  ]);
  let ancestors = [...(graph.parentsByChild.get(rootId) ?? [])];
  for (let generation = 1; generation < ancestorGenerations; generation += 1) {
    const upcoming: string[] = [];
    for (const id of ancestors) {
      keys.add(expansionKey(id, "parents"));
      if (generation === 1) keys.add(expansionKey(id, "children"));
      upcoming.push(...(graph.parentsByChild.get(id) ?? []));
    }
    ancestors = upcoming;
  }
  let descendants = [...(graph.childrenByParent.get(rootId) ?? [])];
  for (let generation = 1; generation < descendantGenerations; generation += 1) {
    const upcoming: string[] = [];
    for (const id of descendants) {
      keys.add(expansionKey(id, "children"));
      upcoming.push(...(graph.childrenByParent.get(id) ?? []));
    }
    descendants = upcoming;
  }
  return keys;
}

export function visibleFromExpansions(
  graph: GenealogyGraph,
  rootId: string,
  expansions: Iterable<string>,
  protectIds: Iterable<string> = [],
): { visible: Set<string>; refs: Map<string, number> } {
  const keys = new Set([...expansions]);
  const visible = new Set<string>(graph.people.has(rootId) || rootId ? [rootId] : []);
  let grew = true;
  for (let guard = 0; grew && guard < 64; guard += 1) {
    grew = false;
    for (const id of [...visible]) {
      for (const direction of BRANCH_DIRECTIONS) {
        if (!keys.has(expansionKey(id, direction))) continue;
        for (const neighbor of neighborsFor(graph, id, direction)) {
          if (direction === "partners" && !partnerIncluded(graph, id, neighbor, visible, true)) continue;
          if (!graph.people.has(neighbor) || visible.has(neighbor)) continue;
          visible.add(neighbor);
          grew = true;
        }
      }
      for (const partner of graph.partnersByPerson.get(id) ?? []) {
        if (!partnerIncluded(graph, id, partner, visible, false)) continue;
        if (!graph.people.has(partner) || visible.has(partner)) continue;
        visible.add(partner);
        grew = true;
      }
    }
  }
  for (const id of protectIds) {
    if (graph.people.has(id)) visible.add(id);
  }
  const refs = new Map<string, number>();
  refs.set(rootId, 1);
  for (const id of visible) {
    for (const direction of BRANCH_DIRECTIONS) {
      if (!keys.has(expansionKey(id, direction))) continue;
      for (const neighbor of neighborsFor(graph, id, direction)) {
        if (!visible.has(neighbor)) continue;
        if (direction === "partners" && !partnerIncluded(graph, id, neighbor, visible, true)) continue;
        refs.set(neighbor, (refs.get(neighbor) ?? 0) + 1);
      }
    }
    for (const partner of graph.partnersByPerson.get(id) ?? []) {
      if (!visible.has(partner) || !partnerIncluded(graph, id, partner, visible, false)) continue;
      refs.set(partner, (refs.get(partner) ?? 0) + 1);
    }
  }
  return { visible, refs };
}

export function hiddenCounts(
  graph: GenealogyGraph,
  personId: string,
  visible: Set<string>,
  truncated = false,
  lowerBound = 0,
): HiddenCounts {
  const count = (ids: Iterable<string>) => [...ids].filter((id) => graph.people.has(id) && !visible.has(id)).length;
  const partners = [...(graph.partnersByPerson.get(personId) ?? [])].filter(
    (id) => graph.people.has(id) && !visible.has(id) && partnerIncluded(graph, personId, id, visible, true),
  ).length;
  return {
    parents: count(graph.parentsByChild.get(personId) ?? []),
    children: count(graph.childrenByParent.get(personId) ?? []),
    siblings: count(siblingsOf(graph, personId)),
    partners,
    truncated,
    lowerBound,
  };
}

export function generationDistance(
  graph: GenealogyGraph,
  rootId: string,
  personId: string,
  direction: "parents" | "children",
): number {
  if (personId === rootId) return 0;
  const next = (id: string) =>
    direction === "parents" ? graph.parentsByChild.get(id) : graph.childrenByParent.get(id);
  let frontier = [rootId];
  const seen = new Set([rootId]);
  for (let distance = 1; frontier.length > 0; distance += 1) {
    const upcoming: string[] = [];
    for (const id of frontier) {
      for (const neighbor of next(id) ?? []) {
        if (seen.has(neighbor)) continue;
        if (neighbor === personId) return distance;
        seen.add(neighbor);
        upcoming.push(neighbor);
      }
    }
    frontier = upcoming;
  }
  return Number.POSITIVE_INFINITY;
}

export function expansionBlocked(
  graph: GenealogyGraph,
  rootId: string,
  personId: string,
  direction: BranchDirection,
  maxExpansionDepth = MAX_EXPANSION_DEPTH,
): boolean {
  if (direction !== "parents" && direction !== "children") return false;
  return generationDistance(graph, rootId, personId, direction) >= maxExpansionDepth;
}

export function wouldCreateParentCycle(graph: GenealogyGraph, parentId: string, childId: string): boolean {
  return parentCyclePath(graph, parentId, childId) !== null;
}

export function parentCyclePath(graph: GenealogyGraph, parentId: string, childId: string): string[] | null {
  if (parentId === childId) return [childId, parentId];
  const cameFrom = new Map<string, string>();
  const seen = new Set<string>([childId]);
  const queue = [childId];
  while (queue.length > 0) {
    const id = queue.shift()!;
    for (const child of graph.childrenByParent.get(id) ?? []) {
      if (seen.has(child)) continue;
      cameFrom.set(child, id);
      if (child === parentId) {
        const path = [parentId];
        let current = parentId;
        while (current !== childId) {
          current = cameFrom.get(current)!;
          path.push(current);
        }
        path.reverse();
        return path;
      }
      seen.add(child);
      queue.push(child);
    }
  }
  return null;
}

export function formatParentCycleMessage(path: string[], nameOf: (id: string) => string): string {
  if (path.length === 0) return "That parent link would create a cycle.";
  return `That parent link would create a cycle: ${path.map(nameOf).join(" → ")}.`;
}

export function wouldCreateDuplicate(
  graph: GenealogyGraph,
  role: "parent" | "child" | "partner",
  currentId: string,
  otherId: string,
): boolean {
  if (role === "parent") {
    return [...graph.relationships.values()].some(
      (relationship) =>
        relationship.kind === "parent" && relationship.sourceId === otherId && relationship.targetId === currentId,
    );
  }
  if (role === "child") {
    return [...graph.relationships.values()].some(
      (relationship) =>
        relationship.kind === "parent" && relationship.sourceId === currentId && relationship.targetId === otherId,
    );
  }
  return [...graph.relationships.values()].some((relationship) => {
    if (relationship.kind !== "partner") return false;
    return (
      (relationship.sourceId === currentId && relationship.targetId === otherId) ||
      (relationship.sourceId === otherId && relationship.targetId === currentId)
    );
  });
}
