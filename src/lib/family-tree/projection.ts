import type { EntityRecord, EntitySummary, Relationship } from "../../../packages/module-api/src/index";
import {
  type FamilyPerson,
  type FamilyRelationship,
  type GenealogyGraph,
  type GenealogyWarning,
  INITIAL_ANCESTOR_GENERATIONS,
  INITIAL_DESCENDANT_GENERATIONS,
  PARENT_RELATIONSHIP,
  PARTNER_RELATIONSHIP,
  VISIBLE_PERSON_LIMIT,
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

export function initialNeighborhood(graph: GenealogyGraph, rootId: string): Set<string> {
  const visible = new Set<string>([rootId]);
  if (!graph.people.has(rootId)) return visible;
  for (const id of walk([rootId], INITIAL_ANCESTOR_GENERATIONS, (id) => graph.parentsByChild.get(id))) visible.add(id);
  for (const id of walk([rootId], INITIAL_DESCENDANT_GENERATIONS, (id) => graph.childrenByParent.get(id)))
    visible.add(id);
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

export function wouldExceedVisibleLimit(count: number): boolean {
  return count > VISIBLE_PERSON_LIMIT;
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
