export const TIMELINE_ENTITY_TYPES = new Set(["event", "encounter", "era", "calendar"]);

export type SchemaFieldLike = {
  type: string;
  entityTypes?: string[];
  targetEntityTypes?: string[];
};

/**
 * Whether a schema field should appear on an entity. Relationship fields whose
 * targets are not among the enabled modules' entity types stay hidden. Date
 * fields on Lore types also stay hidden unless Timeline is enabled.
 */
export function fieldAppliesToEntity(
  field: SchemaFieldLike,
  entityType: string | null | undefined,
  enabledEntityTypes: ReadonlySet<string> | null,
): boolean {
  const appliesToSource = !field.entityTypes || !entityType || field.entityTypes.includes(entityType);
  if (!appliesToSource) return false;
  if (enabledEntityTypes === null) return true;
  if (field.type === "relationship" && field.targetEntityTypes?.length) {
    return field.targetEntityTypes.some((targetType) => enabledEntityTypes.has(targetType));
  }
  if (field.type === "date") {
    const ownedByTimeline = field.entityTypes?.some((type) => TIMELINE_ENTITY_TYPES.has(type));
    if (!ownedByTimeline && !enabledEntityTypes.has("event")) return false;
  }
  return true;
}
