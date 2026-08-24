import type { Entity, EntityPage } from "$lib/project/client";

export type WritingView = "manuscripts" | "reference";
export type TimelineView = "events" | "eras" | "calendars";
export type WorkspaceSection = "lore" | "timeline" | "writing" | "language" | "maps";
export const WORKSPACE_MODULE_IDS = {
  lore: "daena.lore",
  timeline: "daena.timeline",
  writing: "daena.writing",
  language: "daena.language",
  maps: "daena.maps",
} as const satisfies Record<WorkspaceSection, string>;

export function workspaceSectionDescription(section: WorkspaceSection): string {
  if (section === "lore") return "People, places, factions, cultures, and the ideas that connect them.";
  if (section === "timeline") return "Events, eras, calendars, and the chronology of your world.";
  if (section === "writing") return "Manuscripts and reference pages beside the world they draw from.";
  if (section === "language") return "Sounds, writing systems, vocabulary, and grammar.";
  return "Maps, world surfaces, locations, and geographic links.";
}

export function workspaceModuleId(section: WorkspaceSection): string {
  return WORKSPACE_MODULE_IDS[section];
}
export type SettingsSection = "general" | "ai";
export type ProjectSection = "overview" | "data" | "extensions" | "fields" | "snapshots" | "archive" | "advanced";
export type SortField = "name" | "created_at" | "updated_at";
export type SortDirection = "asc" | "desc";
export type CollectionViewMode = "flat" | "grouped";

export interface CollectionQuery {
  section: WorkspaceSection;
  textSearch: string;
  sortField: SortField;
  sortDir: SortDirection;
  pageSize: number;
  page: number;
  excludedTypes: string[];
  viewMode: CollectionViewMode;
}

export interface CollectionGroup {
  type: string;
  label: string;
  entities: Entity[];
  count: number;
}

export interface CollectionResult {
  entities: Entity[];
  total: number;
  groups?: CollectionGroup[];
}

export const WRITING_VIEW_TYPES: Record<WritingView, string[]> = {
  manuscripts: ["manuscript"],
  reference: ["reference-page"],
};

export const TIMELINE_VIEW_TYPES: Record<TimelineView, string[]> = {
  events: ["event", "encounter"],
  eras: ["era"],
  calendars: ["calendar"],
};

export const DEFAULT_COLLECTION_QUERY: Omit<CollectionQuery, "section"> = {
  textSearch: "",
  sortField: "name",
  sortDir: "asc",
  pageSize: 50,
  page: 0,
  excludedTypes: [],
  viewMode: "grouped",
};

function tabTypesFor(input: { writingView?: WritingView; timelineView?: TimelineView }): string[] | undefined {
  if (input.writingView) return WRITING_VIEW_TYPES[input.writingView];
  if (input.timelineView) return TIMELINE_VIEW_TYPES[input.timelineView];
  return undefined;
}

/** Resolve the manifest-derived entity types that the backend should query. */
export function collectionEntityTypes(input: {
  entityTypes: ReadonlySet<string>;
  writingView?: WritingView;
  timelineView?: TimelineView;
}): string[] {
  const tabTypes = tabTypesFor(input);
  let effective = [...input.entityTypes];
  if (tabTypes) {
    const allowed = new Set(tabTypes);
    effective = effective.filter((type) => allowed.has(type));
  }
  return [...new Set(effective)].sort();
}

function groupByType(
  entities: Entity[],
  typeCounts: EntityPage["type_counts"],
  labelFn: (type: string) => string,
): CollectionGroup[] {
  const counts = new Map(typeCounts.map((entry) => [entry.entity_type ?? "__uncategorized", entry.count]));
  const map = new Map<string, Entity[]>();
  for (const entity of entities) {
    const type = entity.entity_type ?? "__uncategorized";
    const list = map.get(type);
    if (list) list.push(entity);
    else map.set(type, [entity]);
  }
  return [...map.entries()]
    .map(([type, list]) => ({ type, label: labelFn(type), entities: list, count: counts.get(type) ?? list.length }))
    .sort((a, b) => a.label.localeCompare(b.label));
}

/** Convert an already-filtered backend page into presentation groups. */
export function presentCollectionPage(
  page: EntityPage,
  viewMode: CollectionViewMode,
  labelFn: (type: string) => string,
): CollectionResult {
  const entities = page.items;
  return viewMode === "grouped"
    ? { entities, total: page.total, groups: groupByType(entities, page.type_counts, labelFn) }
    : { entities, total: page.total };
}
