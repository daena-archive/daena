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

export type OutlineRole = "manuscript" | "series" | "book" | "chapter";

export const MANUSCRIPT_OUTLINE_CAP = 2000;

export const OUTLINE_ROLE_LABEL: Record<OutlineRole, string> = {
  manuscript: "Manuscript",
  series: "Series",
  book: "Book",
  chapter: "Chapter",
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
  const parents = parentByChild(edges);
  const orderById = new Map<string, number>();
  const childIds = new Map<string, string[]>();
  for (const edge of edges) {
    orderById.set(edge.sourceId, edge.order);
    const siblings = childIds.get(edge.targetId) ?? [];
    if (!siblings.includes(edge.sourceId)) siblings.push(edge.sourceId);
    childIds.set(edge.targetId, siblings);
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

  const loadedRootId = (startId: string): string | null => {
    if (!byId.has(startId)) return null;
    let current = startId;
    const seen = new Set<string>();
    while (true) {
      if (seen.has(current)) return current;
      seen.add(current);
      const parent = parents.get(current);
      if (parent && byId.has(parent)) current = parent;
      else return current;
    }
  };

  const roots: OutlineNode[] = [];
  const seenRoots = new Set<string>();
  for (const entity of pageEntities) {
    const rootId = loadedRootId(entity.id);
    if (!rootId || seenRoots.has(rootId)) continue;
    seenRoots.add(rootId);
    const node = toNode(rootId);
    if (node) roots.push(node);
  }
  const pageOrder = new Map(pageEntities.map((entity, index) => [entity.id, index]));
  roots.sort((left, right) => {
    const leftOrder = pageOrder.get(left.id) ?? Number.POSITIVE_INFINITY;
    const rightOrder = pageOrder.get(right.id) ?? Number.POSITIVE_INFINITY;
    if (leftOrder !== rightOrder) return leftOrder - rightOrder;
    return left.name.localeCompare(right.name);
  });
  return roots;
}

export function outlineRole(node: OutlineNode, hasParent: boolean): OutlineRole {
  if (!hasParent) {
    if (node.children.length === 0) return "manuscript";
    if (node.children.some((child) => child.children.length > 0)) return "series";
    return "book";
  }
  if (node.children.length > 0) return "book";
  return "chapter";
}

export function outlineRoleLabel(node: OutlineNode, hasParent: boolean): string {
  return OUTLINE_ROLE_LABEL[outlineRole(node, hasParent)];
}

export function paginateOutlineRoots(
  roots: readonly OutlineNode[],
  page: number,
  pageSize: number,
): { items: OutlineNode[]; total: number; offset: number; hasMore: boolean } {
  const size = Math.max(1, pageSize);
  const total = roots.length;
  const lastPage = Math.max(0, Math.ceil(total / size) - 1);
  const current = Math.min(Math.max(0, page), lastPage);
  const offset = current * size;
  const items = roots.slice(offset, offset + size);
  return { items, total, offset, hasMore: offset + items.length < total };
}

export function defaultExpandedOutlineIds(roots: readonly OutlineNode[]): Set<string> {
  const expanded = new Set<string>();
  for (const root of roots) {
    if (root.children.length > 0) expanded.add(root.id);
  }
  return expanded;
}

export function outlinePathIds(roots: readonly OutlineNode[], id: string): string[] | null {
  const path: string[] = [];
  const visit = (nodes: readonly OutlineNode[]): boolean => {
    for (const node of nodes) {
      path.push(node.id);
      if (node.id === id) return true;
      if (visit(node.children)) return true;
      path.pop();
    }
    return false;
  };
  return visit(roots) ? path : null;
}

export function collectOutlineIds(nodes: readonly OutlineNode[], into = new Set<string>()): Set<string> {
  for (const node of nodes) {
    into.add(node.id);
    collectOutlineIds(node.children, into);
  }
  return into;
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
  const byPair = new Map<string, PartOfEdge>();
  let frontier = [...seedIds];
  for (let round = 0; round < maxRounds && frontier.length > 0; round += 1) {
    const batch = frontier.filter((id) => !seen.has(id));
    for (const id of batch) seen.add(id);
    if (batch.length === 0) break;
    const found: PartOfEdge[] = [];
    for (let index = 0; index < batch.length; index += 200) {
      found.push(...(await query(batch.slice(index, index + 200))));
    }
    const next: string[] = [];
    for (const edge of found) {
      const key = `${edge.sourceId}\0${edge.targetId}`;
      if (!byPair.has(key)) byPair.set(key, edge);
      if (!seen.has(edge.sourceId)) next.push(edge.sourceId);
      if (!seen.has(edge.targetId)) next.push(edge.targetId);
    }
    frontier = next;
  }
  return [...byPair.values()];
}
