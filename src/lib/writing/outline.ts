import {
  parseRelationshipMetadata,
  relationshipSourceId,
  relationshipTargetId,
  type RelationshipLike,
} from "../modules/contributed-fields.ts";

export const WRITING_PART_OF = "part_of";
export const WRITING_REVISES = "revises";
export const WRITING_FEATURES = "features";
export const MANUSCRIPT_TYPE = "daena.writing:manuscript";

export type OutlineEntity = {
  id: string;
  name: string;
  entity_type?: string | null;
};

export type PartOfEdge = {
  sourceId: string;
  targetId: string;
  order: number;
};

export type OutlineNode = {
  id: string;
  name: string;
  entityType: string | null;
  children: OutlineNode[];
};

export function isManuscriptType(entityType: string | null | undefined): boolean {
  if (!entityType) return false;
  return entityType === MANUSCRIPT_TYPE || entityType === "manuscript" || entityType.endsWith(":manuscript");
}

export function partOfOrder(metadata: unknown): number {
  const parsed = parseRelationshipMetadata(metadata);
  const raw = parsed.order;
  if (typeof raw === "number" && Number.isFinite(raw)) return raw;
  if (typeof raw === "string" && raw.trim()) {
    const parsedNumber = Number(raw);
    if (Number.isFinite(parsedNumber)) return parsedNumber;
  }
  return Number.POSITIVE_INFINITY;
}

export function partOfEdgesFromRelationships(relationships: readonly RelationshipLike[]): PartOfEdge[] {
  const edges: PartOfEdge[] = [];
  for (const relationship of relationships) {
    const type = relationship.relationship_type ?? relationship.type ?? "";
    if (type !== WRITING_PART_OF) continue;
    const sourceId = relationshipSourceId(relationship);
    const targetId = relationshipTargetId(relationship);
    if (!sourceId || !targetId || sourceId === targetId) continue;
    edges.push({
      sourceId,
      targetId,
      order: partOfOrder((relationship as { metadata?: unknown }).metadata),
    });
  }
  return edges;
}

export function parentByChild(edges: readonly PartOfEdge[]): Map<string, string> {
  const parents = new Map<string, string>();
  for (const edge of edges) parents.set(edge.sourceId, edge.targetId);
  return parents;
}

export function containmentNames(
  leafId: string,
  parents: ReadonlyMap<string, string>,
  nameById: ReadonlyMap<string, string>,
  maxDepth = 8,
): string[] {
  const names: string[] = [];
  const seen = new Set<string>();
  let current = leafId;
  for (let depth = 0; depth < maxDepth; depth += 1) {
    if (seen.has(current)) break;
    seen.add(current);
    const parent = parents.get(current);
    if (!parent) break;
    const name = nameById.get(parent);
    if (name) names.push(name);
    current = parent;
  }
  names.reverse();
  const leaf = nameById.get(leafId);
  if (leaf) names.push(leaf);
  return names;
}

export function appearanceLabel(names: readonly string[]): string {
  return names.filter(Boolean).join(" · ");
}

function compareChildren(left: OutlineNode, right: OutlineNode, orderById: ReadonlyMap<string, number>): number {
  const leftOrder = orderById.get(left.id) ?? Number.POSITIVE_INFINITY;
  const rightOrder = orderById.get(right.id) ?? Number.POSITIVE_INFINITY;
  if (leftOrder !== rightOrder) return leftOrder - rightOrder;
  return left.name.localeCompare(right.name);
}

export function buildManuscriptOutline(
  pageEntities: readonly OutlineEntity[],
  extraEntities: readonly OutlineEntity[],
  edges: readonly PartOfEdge[],
): OutlineNode[] {
  const byId = new Map<string, OutlineEntity>();
  for (const entity of [...extraEntities, ...pageEntities]) byId.set(entity.id, entity);
  const pageIds = new Set(pageEntities.map((entity) => entity.id));
  const parents = parentByChild(edges);
  const orderById = new Map<string, number>();
  const childIds = new Map<string, string[]>();
  for (const edge of edges) {
    orderById.set(edge.sourceId, edge.order);
    const siblings = childIds.get(edge.targetId);
    if (siblings) siblings.push(edge.sourceId);
    else childIds.set(edge.targetId, [edge.sourceId]);
  }

  const visiting = new Set<string>();
  const built = new Map<string, OutlineNode>();
  const toNode = (id: string): OutlineNode | null => {
    const existing = built.get(id);
    if (existing) return existing;
    const entity = byId.get(id);
    if (!entity || visiting.has(id)) return null;
    visiting.add(id);
    const children = (childIds.get(id) ?? [])
      .map((childId) => toNode(childId))
      .filter((node): node is OutlineNode => node !== null)
      .sort((left, right) => compareChildren(left, right, orderById));
    visiting.delete(id);
    const node: OutlineNode = {
      id: entity.id,
      name: entity.name,
      entityType: entity.entity_type ?? null,
      children,
    };
    built.set(id, node);
    return node;
  };

  const roots: OutlineNode[] = [];
  const nestedOnPage = new Set<string>();
  for (const entity of pageEntities) {
    const parent = parents.get(entity.id);
    if (parent && pageIds.has(parent)) nestedOnPage.add(entity.id);
  }
  for (const entity of pageEntities) {
    if (nestedOnPage.has(entity.id)) continue;
    const node = toNode(entity.id);
    if (node) roots.push(node);
  }
  return roots;
}

export function outlineHasNesting(nodes: readonly OutlineNode[]): boolean {
  return nodes.some((node) => node.children.length > 0);
}

export async function collectPartOfEdges(
  seedIds: readonly string[],
  query: (entityIds: string[]) => Promise<PartOfEdge[]>,
  maxRounds = 4,
): Promise<PartOfEdge[]> {
  const seen = new Set<string>();
  const edges: PartOfEdge[] = [];
  let frontier = [...seedIds];
  for (let round = 0; round < maxRounds && frontier.length > 0; round += 1) {
    const batch = frontier.filter((id) => !seen.has(id));
    for (const id of batch) seen.add(id);
    if (batch.length === 0) break;
    const found: PartOfEdge[] = [];
    for (let index = 0; index < batch.length; index += 200) {
      found.push(...(await query(batch.slice(index, index + 200))));
    }
    edges.push(...found);
    const next: string[] = [];
    for (const edge of found) {
      if (!seen.has(edge.sourceId)) next.push(edge.sourceId);
      if (!seen.has(edge.targetId)) next.push(edge.targetId);
    }
    frontier = next;
  }
  return edges;
}
