import { isGregorianCalendarId, parseCalendarDate } from "./calendar-date.ts";
import { chronologyCompareOrdinal, type CalendarDefinition } from "./calendar.ts";
import { timelineDateAnchor } from "./projection.ts";

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
