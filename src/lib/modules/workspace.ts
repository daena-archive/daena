import type { Entity } from "$lib/project/client";

export type WritingView = "manuscripts" | "reference";

export const WRITING_VIEW_TYPES: Record<WritingView, string[]> = {
  manuscripts: ["manuscript"],
  reference: ["reference-page"],
};

/**
 * Filters workspace entities by section. For writing, the set is narrowed to the
 * active tab's entity types so tab labels match the displayed collection.
 */
export function filterWorkspaceEntities(input: {
  entityTypes: ReadonlySet<string>;
  entities: readonly Entity[];
  query: string;
  writingView?: WritingView;
}): Entity[] {
  const term = input.query.trim().toLowerCase();
  let effective = input.entityTypes;
  if (input.writingView) {
    const tabTypes = new Set(WRITING_VIEW_TYPES[input.writingView]);
    effective =
      input.entityTypes.size === 0
        ? input.entityTypes
        : new Set([...input.entityTypes].filter((type) => tabTypes.has(type)));
  }
  return input.entities.filter((entity) => {
    const belongs = entity.entity_type !== null && effective.has(entity.entity_type);
    return belongs && (!term || `${entity.name} ${entity.entity_type ?? ""}`.toLowerCase().includes(term));
  });
}
