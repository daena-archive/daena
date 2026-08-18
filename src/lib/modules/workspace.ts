import type { Entity } from "$lib/project/client";

export type WritingView = "manuscripts" | "reference";
export type TimelineView = "events" | "eras" | "calendars";

export const WRITING_VIEW_TYPES: Record<WritingView, string[]> = {
  manuscripts: ["manuscript"],
  reference: ["reference-page"],
};

export const TIMELINE_VIEW_TYPES: Record<TimelineView, string[]> = {
  events: ["event", "encounter"],
  eras: ["era"],
  calendars: ["calendar"],
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
