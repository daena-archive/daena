import type { Entity, EntityPage } from "$lib/project/client";

export type WritingView = string;
export type TimelineView = string;
export type LanguagePane = "overview" | "lexicon" | "sounds" | "writing" | "grammar" | "forms" | "samples";
export type WorkspaceSection = "lore" | "timeline" | "writing" | "language" | "maps" | "houses";
export const WORKSPACE_MODULE_IDS = {
  lore: "daena.lore",
  timeline: "daena.timeline",
  writing: "daena.writing",
  language: "daena.language",
  maps: "daena.maps",
  houses: "daena.houses",
} as const satisfies Record<WorkspaceSection, string>;

export function workspaceSectionDescription(section: WorkspaceSection): string {
  if (section === "lore") return "People, places, factions, cultures, and the ideas that connect them.";
  if (section === "timeline") return "Events, eras, calendars, and the chronology of your world.";
  if (section === "writing") return "Manuscripts and reference pages beside the world they draw from.";
  if (section === "language") return "Sounds, writing systems, vocabulary, and grammar.";
  if (section === "houses") return "Houses, lineages, and the kinship that binds people together.";
  return "Maps, world surfaces, locations, and geographic links.";
}

export function workspaceModuleId(section: WorkspaceSection): string {
  return WORKSPACE_MODULE_IDS[section];
}
export type SettingsSection = "general";
export type ProjectSection =
  "overview" | "data" | "extensions" | "fields" | "ai" | "snapshots" | "archive" | "advanced";
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

export interface WorkspaceCollectionTab {
  id: string;
  label: string;
  entityTypes: string[];
}

export interface CollectionTabType {
  id: string;
  name: string;
}

export const WRITING_BUILTIN_TABS: WorkspaceCollectionTab[] = [
  { id: "manuscripts", label: "Manuscripts", entityTypes: ["daena.writing:manuscript"] },
  { id: "reference", label: "Reference", entityTypes: ["daena.writing:reference-page"] },
];

export const TIMELINE_BUILTIN_TABS: WorkspaceCollectionTab[] = [
  {
    id: "events",
    label: "Events",
    entityTypes: ["daena.timeline:event", "daena.timeline:encounter", "daena.timeline:era"],
  },
  { id: "calendars", label: "Calendars", entityTypes: ["daena.timeline:calendar"] },
];

export function workspaceCollectionTabs(
  section: WorkspaceSection,
  types: readonly CollectionTabType[],
): WorkspaceCollectionTab[] {
  const builtins = section === "timeline" ? TIMELINE_BUILTIN_TABS : section === "writing" ? WRITING_BUILTIN_TABS : [];
  if (builtins.length === 0) return [];
  const available = new Set(types.map((type) => type.id));
  const claimed = new Set<string>();
  const tabs: WorkspaceCollectionTab[] = [];
  for (const tab of builtins) {
    const entityTypes = tab.entityTypes.filter((id) => available.has(id));
    if (entityTypes.length === 0) continue;
    tabs.push({ id: tab.id, label: tab.label, entityTypes });
    for (const id of entityTypes) claimed.add(id);
  }
  const custom = [...types]
    .filter((type) => !claimed.has(type.id))
    .sort((left, right) => left.name.localeCompare(right.name) || left.id.localeCompare(right.id))
    .map((type) => ({ id: type.id, label: type.name, entityTypes: [type.id] }));
  return [...tabs, ...custom];
}

export function workspaceSectionViewNav(
  section: WorkspaceSection,
  types: readonly CollectionTabType[],
): { id: string; label: string }[] {
  if (section === "lore") {
    return [
      { id: "library", label: "Library" },
      { id: "wiki", label: "Wiki" },
      { id: "graph", label: "Graph" },
    ];
  }
  if (section === "timeline") {
    return [
      ...workspaceCollectionTabs(section, types).map((tab) => ({ id: tab.id, label: tab.label })),
      { id: "timeline", label: "Timeline" },
    ];
  }
  if (section === "writing") {
    return workspaceCollectionTabs(section, types).map((tab) => ({ id: tab.id, label: tab.label }));
  }
  if (section === "houses") {
    return [
      { id: "houses", label: "Houses" },
      { id: "tree", label: "Tree" },
    ];
  }
  return [];
}

export function collectionTabForEntityType(
  tabs: readonly WorkspaceCollectionTab[],
  entityType: string | null | undefined,
): WorkspaceCollectionTab | undefined {
  if (!entityType) return undefined;
  return tabs.find((tab) => tab.entityTypes.includes(entityType));
}

export const DEFAULT_COLLECTION_QUERY: Omit<CollectionQuery, "section"> = {
  textSearch: "",
  sortField: "name",
  sortDir: "asc",
  pageSize: 50,
  page: 0,
  excludedTypes: [],
  viewMode: "grouped",
};

/** Resolve the manifest-derived entity types that the backend should query. */
export function collectionEntityTypes(input: {
  entityTypes: ReadonlySet<string>;
  tabEntityTypes?: readonly string[];
}): string[] {
  let effective = [...input.entityTypes];
  if (input.tabEntityTypes) {
    const allowed = new Set(input.tabEntityTypes);
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
