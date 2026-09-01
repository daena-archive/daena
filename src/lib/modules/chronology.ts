import { isGregorianCalendarId, parseCalendarDate } from "../date.ts";
import { chronologyCompareOrdinal, type CalendarDefinition } from "../../../packages/modules/timeline/src/calendar.ts";
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

/**
 * Pick the calendar definition used to compare a date against era bounds: the date's own
 * custom calendar when it is available, otherwise the era's first known calendar.
 */
export function resolveChronologyCalendarDefinition(
  value: unknown,
  eraCalendarIds: readonly string[],
  calendarDefinitions: Readonly<Record<string, CalendarDefinition>>,
): CalendarDefinition | null {
  const date = parseCalendarDate(value);
  const dateCalendarId = date?.calendar;
  if (dateCalendarId && !isGregorianCalendarId(dateCalendarId) && calendarDefinitions[dateCalendarId]) {
    return calendarDefinitions[dateCalendarId];
  }
  for (const eraCalendarId of eraCalendarIds) {
    if (calendarDefinitions[eraCalendarId]) {
      return calendarDefinitions[eraCalendarId];
    }
  }
  return null;
}

function compareChronologyValues(
  left: unknown,
  right: unknown,
  calendarDefinition: CalendarDefinition | null,
): number | null {
  const leftOrdinal = chronologyCompareOrdinal(left, calendarDefinition);
  const rightOrdinal = chronologyCompareOrdinal(right, calendarDefinition);
  if (leftOrdinal !== null && rightOrdinal !== null) return leftOrdinal - rightOrdinal;
  const leftAnchor = timelineDateAnchor(left);
  const rightAnchor = timelineDateAnchor(right);
  if (!leftAnchor || !rightAnchor) return null;
  return leftAnchor.date.getTime() - rightAnchor.date.getTime();
}

export function dateOutsideEraBounds(
  date: unknown,
  start: unknown,
  end: unknown,
  calendarDefinition?: CalendarDefinition | null,
): boolean {
  if (!parseCalendarDate(date)) return false;
  if (start) {
    const compared = compareChronologyValues(date, start, calendarDefinition ?? null);
    if (compared !== null && compared < 0) return true;
  }
  if (end) {
    const compared = compareChronologyValues(date, end, calendarDefinition ?? null);
    if (compared !== null && compared > 0) return true;
  }
  return false;
}

export function chronologyWarnings(
  dates: readonly { label: string; value: unknown }[],
  eras: readonly EraContext[],
  calendarDefinitions: Readonly<Record<string, CalendarDefinition>> = {},
): string[] {
  const warnings: string[] = [];
  for (const date of dates) {
    if (!parseCalendarDate(date.value)) continue;
    for (const era of eras) {
      const calendarDefinition = resolveChronologyCalendarDefinition(date.value, era.calendarIds, calendarDefinitions);
      if (dateOutsideEraBounds(date.value, era.start, era.end, calendarDefinition)) {
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
  calendarDefinition?: CalendarDefinition | null;
}): boolean {
  if (input.eraIds.includes(input.eraId)) return true;
  const value = input.startValue ?? input.endValue;
  if (!value || (!input.eraStart && !input.eraEnd)) return false;
  return !dateOutsideEraBounds(value, input.eraStart, input.eraEnd, input.calendarDefinition ?? null);
}
