import { chronologyCompareOrdinal, type CalendarDefinition } from "../../packages/modules/timeline/src/calendar.ts";

export const GREGORIAN_CALENDAR_ID = "gregorian";

export interface CalendarDate {
  /** Gregorian default, or a world calendar entity id. Absolute values stay Gregorian. */
  calendar: string;
  year: number;
  month?: number;
  day?: number;
  /** Legacy editor fields; ignored by Gregorian serialization. */
  era?: "BCE" | "CE";
  hour?: number;
  minute?: number;
  second?: number;
  precision?: "year" | "month" | "day" | "hour" | "minute" | "second";
}

export function isGregorianCalendarId(calendarId: string | null | undefined): boolean {
  return !calendarId || calendarId === GREGORIAN_CALENDAR_ID;
}

function readCalendarId(value: unknown): string {
  return typeof value === "string" && value.trim() ? value.trim() : GREGORIAN_CALENDAR_ID;
}

/** Gregorian sort/compare year: BCE dates use negative years in ISO strings. */
export function signedGregorianYear(date: CalendarDate): number {
  if (date.era === "BCE" && date.year > 0) return -date.year;
  return date.year;
}

function normalizeEraYear(date: CalendarDate): CalendarDate {
  if (date.era === "BCE" && date.year < 0) return { ...date, year: -date.year };
  return date;
}

function gregorianIso(date: CalendarDate): string {
  const normalized = normalizeEraYear(date);
  const year = signedGregorianYear(normalized);
  const dateParts =
    normalized.precision === "year"
      ? [year]
      : normalized.precision === "month"
        ? [year, normalized.month]
        : [year, normalized.month, normalized.day];
  if (!dateParts.every((part) => typeof part === "number" && Number.isFinite(part))) return "";
  if (["year", "month", "day"].includes(normalized.precision ?? "day")) return dateParts.join("-");
  const timeParts =
    normalized.precision === "hour"
      ? [normalized.hour]
      : normalized.precision === "minute"
        ? [normalized.hour, normalized.minute]
        : [normalized.hour, normalized.minute, normalized.second];
  return timeParts.every((part) => typeof part === "number" && Number.isFinite(part))
    ? `${dateParts.join("-")}T${timeParts.join(":")}`
    : "";
}

export function parseCalendarDate(value: unknown): CalendarDate | null {
  if (value && typeof value === "object") {
    const date = value as Partial<CalendarDate>;
    if (typeof date.year === "number" && Number.isFinite(date.year)) {
      const era = date.era ?? "CE";
      const precision =
        date.precision ?? (date.day !== undefined ? "day" : date.month !== undefined ? "month" : "year");
      return {
        ...date,
        calendar: readCalendarId(date.calendar),
        era,
        year: era === "BCE" && date.year < 0 ? -date.year : date.year,
        precision,
      } as CalendarDate;
    }
  }
  if (typeof value !== "string") return null;
  const match = /^(-?\d+)(?:-(\d+)(?:-(\d+))?)?(?:T(\d+)(?::(\d+)(?::(\d+))?)?)?$/.exec(value.trim());
  if (!match) return null;
  // Normalize year sign + era: keep signed year, preserve BCE era if negative
  const rawYear = Number(match[1]);
  const era: "BCE" | "CE" = rawYear < 0 ? "BCE" : "CE";
  const year = era === "BCE" ? -rawYear : rawYear;
  const precision = match[6]
    ? "second"
    : match[5]
      ? "minute"
      : match[4]
        ? "hour"
        : match[3]
          ? "day"
          : match[2]
            ? "month"
            : "year";
  return {
    calendar: GREGORIAN_CALENDAR_ID,
    era,
    year,
    precision,
    ...(match[2] ? { month: Number(match[2]) } : {}),
    ...(match[3] ? { day: Number(match[3]) } : {}),
    ...(match[4] ? { hour: Number(match[4]) } : {}),
    ...(match[5] ? { minute: Number(match[5]) } : {}),
    ...(match[6] ? { second: Number(match[6]) } : {}),
  };
}

export function serializeCalendarDate(date: CalendarDate): string | CalendarDate {
  const iso = gregorianIso(date);
  if (!iso) return "";
  if (isGregorianCalendarId(date.calendar)) return iso;
  return {
    calendar: date.calendar,
    year: date.year,
    era: date.era ?? "CE",
    precision: date.precision,
    ...(date.month !== undefined ? { month: date.month } : {}),
    ...(date.day !== undefined ? { day: date.day } : {}),
    ...(date.hour !== undefined ? { hour: date.hour } : {}),
    ...(date.minute !== undefined ? { minute: date.minute } : {}),
    ...(date.second !== undefined ? { second: date.second } : {}),
  };
}

export function formatCalendarDate(value: unknown): string {
  const date = parseCalendarDate(value);
  if (!date) return typeof value === "string" && value ? value : "Undated";
  return gregorianIso(date) || "Undated";
}

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
