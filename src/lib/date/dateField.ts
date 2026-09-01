import { GREGORIAN_CALENDAR_ID, parseCalendarDate, serializeCalendarDate, type CalendarDate } from "../date.ts";
import {
  calendarDateToParts,
  partsToCalendarDate,
  type CalendarDefinition,
} from "../../../packages/modules/timeline/src/calendar.ts";

export type CalendarDatePatch = Partial<CalendarDate> & { calendar?: string };

function emptyParts(): { precision: "year" } {
  return { precision: "year" };
}

/** Apply a partial edit to a calendar date value. Returns null when nothing should be committed yet. */
export function patchCalendarDate(
  currentValue: unknown,
  patch: CalendarDatePatch,
  calendar: CalendarDefinition | null,
  calendarId: string,
): unknown | null {
  const calId = patch.calendar ?? calendarId ?? GREGORIAN_CALENDAR_ID;
  const prev = parseCalendarDate(currentValue);
  const currentParts =
    prev !== null ? calendarDateToParts(prev, calendar) : patch.year !== undefined ? emptyParts() : null;
  if (!currentParts && patch.year === undefined) return null;

  const nextParts: Record<string, unknown> = { ...currentParts, ...patch };
  if (patch.precision === undefined) {
    const hasMonth = nextParts.month !== undefined;
    const hasDay = nextParts.day !== undefined;
    if (!hasMonth) {
      nextParts.precision = "year";
      delete nextParts.day;
    } else if (!hasDay) {
      nextParts.precision = "month";
    } else if (!["hour", "minute", "second"].includes(String(nextParts.precision))) {
      nextParts.precision = "day";
    }
  }
  if (patch.precision === "year") {
    delete nextParts.month;
    delete nextParts.day;
  }
  if (patch.precision === "month") {
    delete nextParts.day;
    if (nextParts.month === undefined) {
      nextParts.precision = "year";
      delete nextParts.month;
    }
  }
  if (patch.precision === "day") {
    if (nextParts.month === undefined) {
      nextParts.precision = "year";
      delete nextParts.month;
      delete nextParts.day;
    } else if (nextParts.day === undefined) {
      nextParts.precision = "month";
      delete nextParts.day;
    }
  }
  if (nextParts.year === undefined || !Number.isFinite(Number(nextParts.year))) return null;

  const stored = partsToCalendarDate(nextParts as Parameters<typeof partsToCalendarDate>[0], calendar);
  stored.calendar = calId;
  if (prev) {
    stored.hour = patch.hour ?? prev.hour;
    stored.minute = patch.minute ?? prev.minute;
    stored.second = patch.second ?? prev.second;
  } else if (patch.hour !== undefined) {
    stored.hour = patch.hour;
    stored.minute = patch.minute;
    stored.second = patch.second;
  }
  if (patch.precision === "hour") {
    delete stored.minute;
    delete stored.second;
  } else if (patch.precision === "minute") {
    delete stored.second;
  }
  if (patch.precision === "hour" || patch.precision === "minute" || patch.precision === "second") {
    stored.precision = patch.precision;
  } else if (stored.precision === "hour" || stored.precision === "minute" || stored.precision === "second") {
    if (stored.second !== undefined) stored.precision = "second";
    else if (stored.minute !== undefined) stored.precision = "minute";
    else if (stored.hour !== undefined) stored.precision = "hour";
  }
  return serializeCalendarDate(stored);
}
