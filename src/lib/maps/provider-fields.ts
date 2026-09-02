/** Packaged Maps fields that store provider JSON, not author-facing values. */

export const MAPS_PROVIDER_FIELD_KEYS = [
  "map",
  "locations",
  "physicalChronology",
  "layers",
  "physicalCalendarBinding",
  "atlasPresets",
] as const;

const PROVIDER_KEYS = new Set<string>(MAPS_PROVIDER_FIELD_KEYS);

export function isMapsProviderField(field: { key?: string; type?: string } | null | undefined): boolean {
  if (!field?.key) return false;
  if (field.type === "relationship") return false;
  return PROVIDER_KEYS.has(field.key);
}
