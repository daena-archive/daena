import { parseCalendarDate } from "../date.ts";
import { type CalendarDefinition } from "../../../packages/modules/timeline/src/calendar.ts";
import {
  belongsToEraScope,
  dateOutsideEraBounds,
  resolveChronologyCalendarDefinition,
} from "../../../packages/modules/timeline/src/era-scope.ts";

export { belongsToEraScope, dateOutsideEraBounds, resolveChronologyCalendarDefinition };

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
