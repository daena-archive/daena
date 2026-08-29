import { parseCalendarDate } from "../date.ts";
import { timelineDateAnchor } from "../../../packages/modules/timeline/src/projection.ts";

export const ERA_RELATIONSHIP_TYPE = "during";
export const CALENDAR_ADOPTION_TYPE = "uses_calendar";
export const ERA_FIELD_KEY = "era";

export type SchemaFieldLike = {
  key: string;
  type: string;
  relationshipType?: string;
};

export type EraContext = {
  id: string;
  name: string;
  start: unknown;
  end: unknown;
  calendarIds: string[];
};

export function isEraRelationshipField(field: SchemaFieldLike): boolean {
  return (
    field.type === "relationship" && (field.relationshipType === ERA_RELATIONSHIP_TYPE || field.key === ERA_FIELD_KEY)
  );
}

export function isChronologyDateKey(key: string): boolean {
  return key === "startsAt" || key === "endsAt";
}

export function isChronologyPropertyField(field: SchemaFieldLike): boolean {
  return (field.type === "date" && isChronologyDateKey(field.key)) || isEraRelationshipField(field);
}

export function firstEraCalendarId(contexts: readonly EraContext[]): string | undefined {
  return contexts.map((context) => context.calendarIds[0]).find((id): id is string => Boolean(id));
}

export function dateOutsideEraBounds(date: unknown, start: unknown, end: unknown): boolean {
  const anchor = timelineDateAnchor(date);
  if (!anchor) return false;
  const startAnchor = start ? timelineDateAnchor(start) : null;
  const endAnchor = end ? timelineDateAnchor(end) : null;
  if (startAnchor && anchor.date.getTime() < startAnchor.date.getTime()) return true;
  if (endAnchor && anchor.date.getTime() > endAnchor.date.getTime()) return true;
  return false;
}

export function chronologyWarnings(
  dates: readonly { label: string; value: unknown }[],
  eras: readonly EraContext[],
): string[] {
  const warnings: string[] = [];
  for (const date of dates) {
    if (!parseCalendarDate(date.value)) continue;
    for (const era of eras) {
      if (dateOutsideEraBounds(date.value, era.start, era.end)) {
        warnings.push(`${date.label} falls outside ${era.name}.`);
      }
    }
  }
  return warnings;
}

export function belongsToEraScope(input: {
  eraIds: readonly string[];
  startValue?: unknown;
  endValue?: unknown;
  eraId: string;
  eraStart?: unknown;
  eraEnd?: unknown;
}): boolean {
  if (input.eraIds.includes(input.eraId)) return true;
  const value = input.startValue ?? input.endValue;
  if (!value || (!input.eraStart && !input.eraEnd)) return false;
  return !dateOutsideEraBounds(value, input.eraStart, input.eraEnd);
}
