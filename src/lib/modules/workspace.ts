import type { Entity } from "$lib/project/client";

export type WritingView = "manuscripts" | "reference";
export type TimelineView = "events" | "eras" | "calendars";
export type WorkspaceSection = "lore" | "timeline" | "writing" | "language" | "maps";
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
  pageSize: 0,
  page: 0,
  excludedTypes: [],
  viewMode: "grouped",
};

function tabTypesFor(input: { writingView?: WritingView; timelineView?: TimelineView }): string[] | undefined {
  if (input.writingView) return WRITING_VIEW_TYPES[input.writingView];
  if (input.timelineView) return TIMELINE_VIEW_TYPES[input.timelineView];
  return undefined;
}

/**
 * Filters workspace entities by section. Writing and Timeline narrow the set to
 * the active tab's entity types so tab labels match the displayed collection.
 */
export function filterWorkspaceEntities(input: {
  entityTypes: ReadonlySet<string>;
  entities: readonly Entity[];
  query: string;
  writingView?: WritingView;
  timelineView?: TimelineView;
}): Entity[] {
  const term = input.query.trim().toLowerCase();
  const tabTypes = tabTypesFor(input);
  let effective = input.entityTypes;
  if (tabTypes) {
    const allowed = new Set(tabTypes);
    effective =
      input.entityTypes.size === 0
        ? input.entityTypes
        : new Set([...input.entityTypes].filter((type) => allowed.has(type)));
  }
  return input.entities.filter((entity) => {
    const belongs = entity.entity_type !== null && effective.has(entity.entity_type);
    return belongs && (!term || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(term));
  });
}

function sortEntities(entities: Entity[], sortField: SortField, sortDir: SortDirection): Entity[] {
  const dir = sortDir === "asc" ? 1 : -1;
  return [...entities].sort((a, b) => {
    if (sortField === "name") return a.name.localeCompare(b.name) * dir;
    const aVal = a[sortField] ?? "";
    const bVal = b[sortField] ?? "";
    return aVal.localeCompare(bVal) * dir;
  });
}

function groupByType(entities: Entity[], labelFn: (type: string) => string): CollectionGroup[] {
  const map = new Map<string, Entity[]>();
  for (const entity of entities) {
    const type = entity.entity_type ?? "__uncategorized";
    const list = map.get(type);
    if (list) list.push(entity);
    else map.set(type, [entity]);
  }
  return [...map.entries()]
    .map(([type, list]) => ({ type, label: labelFn(type), entities: list, count: list.length }))
    .sort((a, b) => a.label.localeCompare(b.label));
}

export function queryCollection(
  entities: readonly Entity[],
  query: CollectionQuery,
  entityTypes: ReadonlySet<string>,
  labelFn: (type: string) => string,
  writingView?: WritingView,
  timelineView?: TimelineView,
): CollectionResult {
  const excluded = new Set(query.excludedTypes);
  let filtered = filterWorkspaceEntities({
    entityTypes,
    entities,
    query: query.textSearch,
    writingView,
    timelineView,
  });
  if (excluded.size > 0) {
    filtered = filtered.filter((e) => !excluded.has(e.entity_type ?? "__uncategorized"));
  }
  const total = filtered.length;
  const sorted = sortEntities(filtered, query.sortField, query.sortDir);
  if (query.viewMode === "grouped") {
    const groups = groupByType(sorted, labelFn);
    if (query.pageSize > 0) {
      const offset = query.page * query.pageSize;
      for (const group of groups) {
        group.entities = group.entities.slice(offset, offset + query.pageSize);
      }
    }
    return { entities: sorted, total, groups };
  }
  if (query.pageSize > 0) {
    const offset = query.page * query.pageSize;
    return { entities: sorted.slice(offset, offset + query.pageSize), total };
  }
  return { entities: sorted, total };
}
