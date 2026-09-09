import { chronologyCompareOrdinal, type CalendarDefinition } from "../../packages/modules/timeline/src/calendar.ts";
import { parseCalendarDate, signedGregorianYear } from "../../packages/modules/timeline/src/calendar-date.ts";

export {
  GREGORIAN_CALENDAR_ID,
  type CalendarDate,
  formatCalendarDate,
  isGregorianCalendarId,
  parseCalendarDate,
  serializeCalendarDate,
  signedGregorianYear,
} from "../../packages/modules/timeline/src/calendar-date.ts";

export function isCompleteCalendarDate(value: unknown): boolean {
  const date = parseCalendarDate(value);
  if (!date || !Number.isFinite(date.year) || !Number.isFinite(date.month) || !Number.isFinite(date.day)) return false;
  if (["year", "month", "day"].includes(date.precision ?? "day")) return true;
  if (!Number.isFinite(date.hour)) return false;
  if (date.precision === "hour") return true;
  if (!Number.isFinite(date.minute)) return false;
  return date.precision === "minute" || Number.isFinite(date.second);
}

export function compareCalendarDates(
  left: unknown,
  right: unknown,
  calendarDefinition: CalendarDefinition | null = null,
): number {
  if (calendarDefinition) {
    const leftOrdinal = chronologyCompareOrdinal(left, calendarDefinition);
    const rightOrdinal = chronologyCompareOrdinal(right, calendarDefinition);
    if (leftOrdinal !== null && rightOrdinal !== null) return leftOrdinal - rightOrdinal;
  }
  const a = parseCalendarDate(left);
  const b = parseCalendarDate(right);
  if (!a && !b) return 0;
  if (!a) return 1;
  if (!b) return -1;
  return (
    signedGregorianYear(a) - signedGregorianYear(b) ||
    (a.month ?? 0) - (b.month ?? 0) ||
    (a.day ?? 0) - (b.day ?? 0) ||
    (a.hour ?? 0) - (b.hour ?? 0) ||
    (a.minute ?? 0) - (b.minute ?? 0) ||
    (a.second ?? 0) - (b.second ?? 0)
  );
}

/** Runtime timestamps from the core are nanoseconds since the Unix epoch. */
export function formatRuntimeTimestampLabel(
  timestamp: string,
  options: Intl.DateTimeFormatOptions = { dateStyle: "medium" },
): string {
  const trimmed = timestamp.trim();
  if (!trimmed) return "Unknown";
  if (/^\d+$/.test(trimmed)) {
    try {
      const ms = Number(BigInt(trimmed) / 1_000_000n);
      const date = new Date(ms);
      if (Number.isFinite(ms) && ms > 0 && !Number.isNaN(date.getTime())) {
        return new Intl.DateTimeFormat(undefined, options).format(date);
      }
    } catch {
      // Fall through to ISO parsing.
    }
  }
  const date = new Date(trimmed);
  return Number.isNaN(date.getTime()) ? "Unknown" : new Intl.DateTimeFormat(undefined, options).format(date);
}

export function updatedDateLabel(timestamp: string) {
  return formatRuntimeTimestampLabel(timestamp, { dateStyle: "medium" });
}
