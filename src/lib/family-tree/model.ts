import type { CalendarDate } from "$lib/date.ts";

export const PERSON_TYPE = "daena.lore:person";
export const PARENT_RELATIONSHIP = "family_parent_of";
export const PARTNER_RELATIONSHIP = "family_partner_with";
export const LORE_NAMESPACE = "lore";
export const DEFAULT_SECONDARY_FIELD = "occupation";
export const PERSON_NODE_WIDTH = 220;
export const PERSON_NODE_HEIGHT = 92;
export const UNION_NODE_SIZE = 12;
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
export const BRANCH_TOO_LARGE = "This branch is too large to display at once. Re-root on a nearby person to continue.";

export const PARENT_KINDS = ["biological", "adoptive", "legal", "guardian", "step", "custom"] as const;
export const PARTNER_KINDS = ["marriage", "partnership", "betrothal", "concubinage", "custom"] as const;
export const PARTNER_STATUSES = ["active", "ended", "planned", "unknown"] as const;

export type ParentKind = (typeof PARENT_KINDS)[number];
export type PartnerKind = (typeof PARTNER_KINDS)[number];
export type PartnerStatus = (typeof PARTNER_STATUSES)[number];
export type FamilyRelationshipKind = "parent" | "partner";

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

export function layoutExceedsLimits(people: number, unions: number, edges: number): boolean {
  return people > VISIBLE_PERSON_LIMIT || unions > VISIBLE_UNION_LIMIT || edges > VISIBLE_EDGE_LIMIT;
}
