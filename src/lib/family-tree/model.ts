import type { CalendarDate } from "$lib/date.ts";

export const PERSON_TYPE = "daena.lore:person";
export const PARENT_RELATIONSHIP = "family_parent_of";
export const PARTNER_RELATIONSHIP = "family_partner_with";
export const LORE_NAMESPACE = "lore";
export const DEFAULT_SECONDARY_FIELD = "occupation";
export const PERSON_NODE_WIDTH = 220;
export const PERSON_NODE_HEIGHT = 92;
export const UNION_NODE_SIZE = 12;
export const UNION_NODE_WIDTH = 12;
export const UNION_NODE_HEIGHT = 12;
export const VISIBLE_PERSON_LIMIT = 250;
export const VISIBLE_UNION_LIMIT = 150;
export const VISIBLE_EDGE_LIMIT = 500;
export const INITIAL_ANCESTOR_GENERATIONS = 2;
export const INITIAL_DESCENDANT_GENERATIONS = 2;
export const RECENT_ROOT_LIMIT = 10;
export const FIELD_HYDRATE_BATCH = 20;
export const RELATIONSHIP_QUERY_ENTITY_LIMIT = 200;
export const RELATIONSHIP_QUERY_PAGE = 200;
export const RELATIONSHIP_QUERY_FETCH_CAP = 500;
export const ENTITY_GET_MANY_LIMIT = 500;
export const MAX_EXPANSION_DEPTH = 6;
export const MAX_ANCESTOR_GENERATIONS = 12;
export const MAX_DESCENDANT_GENERATIONS = 12;
export const MAX_VISIBLE_PERSON_LIMIT = 2000;
export const MAX_VISIBLE_UNION_LIMIT = 1200;
export const MAX_VISIBLE_EDGE_LIMIT = 4000;
export const MAX_EXPANSION_DEPTH_LIMIT = 24;
export const BRANCH_TOO_LARGE = "This branch is too large to display at once. Re-root on a nearby person to continue.";
export const LIMITS_OVER_BUDGET =
  "Above the recommended 2 generations / 250 people. Layout may hitch; raise the cap only if you need it.";
export const BRANCH_TOO_DEEP = BRANCH_TOO_LARGE;
export const BRANCH_DIRECTIONS = ["parents", "children", "siblings", "partners"] as const;

export const PARENT_KINDS = ["biological", "adoptive", "legal", "guardian", "step", "custom"] as const;
export const PARTNER_KINDS = ["marriage", "partnership", "betrothal", "concubinage", "custom"] as const;
export const PARTNER_STATUSES = ["active", "ended", "planned", "unknown"] as const;

export type ParentKind = (typeof PARENT_KINDS)[number];
export type PartnerKind = (typeof PARTNER_KINDS)[number];
export type PartnerStatus = (typeof PARTNER_STATUSES)[number];
export type FamilyRelationshipKind = "parent" | "partner";
export type BranchDirection = (typeof BRANCH_DIRECTIONS)[number];
export type ExpansionKey = `${string}:${BranchDirection}`;
export type RelativeRole = "parent" | "child" | "partner";

export interface HiddenCounts {
  parents: number;
  children: number;
  siblings: number;
  partners: number;
  truncated: boolean;
  lowerBound: number;
}

export interface FamilyViewport {
  x: number;
  y: number;
  zoom: number;
}

export interface FamilyTreeLimits {
  ancestorGenerations: number;
  descendantGenerations: number;
  visiblePersonLimit: number;
  visibleUnionLimit: number;
  visibleEdgeLimit: number;
  maxExpansionDepth: number;
}

export const DEFAULT_FAMILY_TREE_LIMITS: FamilyTreeLimits = {
  ancestorGenerations: INITIAL_ANCESTOR_GENERATIONS,
  descendantGenerations: INITIAL_DESCENDANT_GENERATIONS,
  visiblePersonLimit: VISIBLE_PERSON_LIMIT,
  visibleUnionLimit: VISIBLE_UNION_LIMIT,
  visibleEdgeLimit: VISIBLE_EDGE_LIMIT,
  maxExpansionDepth: MAX_EXPANSION_DEPTH,
};

function clampInt(value: unknown, min: number, max: number, fallback: number): number {
  const next = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(next)) return fallback;
  return Math.min(max, Math.max(min, Math.floor(next)));
}

export function clampFamilyTreeLimits(input: Partial<FamilyTreeLimits> | null | undefined): FamilyTreeLimits {
  const ancestorGenerations = clampInt(
    input?.ancestorGenerations,
    1,
    MAX_ANCESTOR_GENERATIONS,
    DEFAULT_FAMILY_TREE_LIMITS.ancestorGenerations,
  );
  const descendantGenerations = clampInt(
    input?.descendantGenerations,
    1,
    MAX_DESCENDANT_GENERATIONS,
    DEFAULT_FAMILY_TREE_LIMITS.descendantGenerations,
  );
  const visiblePersonLimit = clampInt(
    input?.visiblePersonLimit,
    1,
    MAX_VISIBLE_PERSON_LIMIT,
    DEFAULT_FAMILY_TREE_LIMITS.visiblePersonLimit,
  );
  const scale = visiblePersonLimit / VISIBLE_PERSON_LIMIT;
  const visibleUnionLimit = clampInt(
    input?.visibleUnionLimit ?? Math.round(VISIBLE_UNION_LIMIT * scale),
    1,
    MAX_VISIBLE_UNION_LIMIT,
    DEFAULT_FAMILY_TREE_LIMITS.visibleUnionLimit,
  );
  const visibleEdgeLimit = clampInt(
    input?.visibleEdgeLimit ?? Math.round(VISIBLE_EDGE_LIMIT * scale),
    1,
    MAX_VISIBLE_EDGE_LIMIT,
    DEFAULT_FAMILY_TREE_LIMITS.visibleEdgeLimit,
  );
  const maxExpansionDepth = clampInt(
    input?.maxExpansionDepth ?? Math.max(MAX_EXPANSION_DEPTH, ancestorGenerations, descendantGenerations),
    1,
    MAX_EXPANSION_DEPTH_LIMIT,
    DEFAULT_FAMILY_TREE_LIMITS.maxExpansionDepth,
  );
  return {
    ancestorGenerations,
    descendantGenerations,
    visiblePersonLimit,
    visibleUnionLimit,
    visibleEdgeLimit,
    maxExpansionDepth,
  };
}

export function familyTreeLimitsOverBudget(limits: FamilyTreeLimits): boolean {
  return (
    limits.ancestorGenerations > INITIAL_ANCESTOR_GENERATIONS ||
    limits.descendantGenerations > INITIAL_DESCENDANT_GENERATIONS ||
    limits.visiblePersonLimit > VISIBLE_PERSON_LIMIT ||
    limits.visibleUnionLimit > VISIBLE_UNION_LIMIT ||
    limits.visibleEdgeLimit > VISIBLE_EDGE_LIMIT ||
    limits.maxExpansionDepth > MAX_EXPANSION_DEPTH
  );
}

export interface FamilyTreeSession {
  expansions: string[];
  selectedPersonId: string | null;
  selectedRelationshipId: string | null;
  viewport?: FamilyViewport | null;
}

export interface FamilyPerson {
  id: string;
  name: string;
  revision: string;
  birth: CalendarDate | string | null;
  death: CalendarDate | string | null;
  secondaryLabel: string | null;
}

export interface FamilyRelationship {
  id: string;
  kind: FamilyRelationshipKind;
  type: string;
  sourceId: string;
  targetId: string;
  revision: string;
  parentKind: ParentKind | null;
  partnerKind: PartnerKind | null;
  status: PartnerStatus | null;
  customLabel: string | null;
  start: unknown;
  end: unknown;
  notes: string | null;
  label: string;
  unknown: boolean;
}

export interface GenealogyGraph {
  people: Map<string, FamilyPerson>;
  parentsByChild: Map<string, Set<string>>;
  childrenByParent: Map<string, Set<string>>;
  partnersByPerson: Map<string, Set<string>>;
  relationships: Map<string, FamilyRelationship>;
  parentRelationshipsByChild: Map<string, FamilyRelationship[]>;
  partnerRelationshipsByPerson: Map<string, FamilyRelationship[]>;
}

export interface GenealogyWarning {
  relationshipId?: string;
  entityId?: string;
  message: string;
}

export type LayoutNodeKind = "person" | "union";

export interface LayoutNode {
  id: string;
  kind: LayoutNodeKind;
  personId?: string;
  memberIds?: string[];
  width: number;
  height: number;
}

export interface LayoutEdge {
  id: string;
  source: string;
  target: string;
  relationshipId: string | null;
  role: "parent" | "child" | "partner" | "direct-parent";
  parentKind: ParentKind | null;
  partnerKind: PartnerKind | null;
  label: string;
  arrow: boolean;
  start: unknown;
  end: unknown;
}

export interface LayoutGraph {
  nodes: LayoutNode[];
  edges: LayoutEdge[];
}

export interface PositionedNode extends LayoutNode {
  x: number;
  y: number;
}

export interface PositionedGraph {
  generation: number;
  nodes: PositionedNode[];
  edges: LayoutEdge[];
}

export function isPersonType(type: string | null | undefined): boolean {
  return type === PERSON_TYPE;
}

export function isParentKind(value: unknown): value is ParentKind {
  return typeof value === "string" && (PARENT_KINDS as readonly string[]).includes(value);
}

export function isPartnerKind(value: unknown): value is PartnerKind {
  return typeof value === "string" && (PARTNER_KINDS as readonly string[]).includes(value);
}

export function isPartnerStatus(value: unknown): value is PartnerStatus {
  return typeof value === "string" && (PARTNER_STATUSES as readonly string[]).includes(value);
}

export function truncationWarning(lowerBound: number): GenealogyWarning {
  const shown = lowerBound > 0 ? `${lowerBound}+` : "99+";
  return { message: `Relationship query truncated (${shown}). Counts may be a lower bound.` };
}

export function layoutExceedsLimits(
  people: number,
  unions: number,
  edges: number,
  limits: Pick<
    FamilyTreeLimits,
    "visiblePersonLimit" | "visibleUnionLimit" | "visibleEdgeLimit"
  > = DEFAULT_FAMILY_TREE_LIMITS,
): boolean {
  return people > limits.visiblePersonLimit || unions > limits.visibleUnionLimit || edges > limits.visibleEdgeLimit;
}

export function expansionKey(id: string, direction: BranchDirection): ExpansionKey {
  return `${id}:${direction}`;
}

export function parseExpansionKey(key: string): { id: string; direction: BranchDirection } | null {
  const index = key.lastIndexOf(":");
  if (index <= 0) return null;
  const direction = key.slice(index + 1);
  if (!(BRANCH_DIRECTIONS as readonly string[]).includes(direction)) return null;
  return { id: key.slice(0, index), direction: direction as BranchDirection };
}

export function isBranchDirection(value: string): value is BranchDirection {
  return (BRANCH_DIRECTIONS as readonly string[]).includes(value);
}

export function familyTreeHistoryKey(session: FamilyTreeSession | null | undefined): string {
  if (!session) return "";
  return JSON.stringify({
    expansions: session.expansions,
    selectedPersonId: session.selectedPersonId ?? null,
    selectedRelationshipId: session.selectedRelationshipId ?? null,
  });
}

export function sameFamilyTreeSession(
  left: FamilyTreeSession | null | undefined,
  right: FamilyTreeSession | null | undefined,
): boolean {
  if (left === right) return true;
  if (!left || !right) return false;
  if (familyTreeHistoryKey(left) !== familyTreeHistoryKey(right)) return false;
  const lv = left.viewport;
  const rv = right.viewport;
  if (!lv && !rv) return true;
  if (!lv || !rv) return false;
  return lv.x === rv.x && lv.y === rv.y && lv.zoom === rv.zoom;
}
