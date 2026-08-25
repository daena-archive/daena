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

function gregorianIso(date: CalendarDate): string {
  const dateParts =
    date.precision === "year"
      ? [date.year]
      : date.precision === "month"
        ? [date.year, date.month]
        : [date.year, date.month, date.day];
  if (!dateParts.every((part) => typeof part === "number" && Number.isFinite(part))) return "";
  if (["year", "month", "day"].includes(date.precision ?? "day")) return dateParts.join("-");
  const timeParts =
    date.precision === "hour"
      ? [date.hour]
      : date.precision === "minute"
        ? [date.hour, date.minute]
        : [date.hour, date.minute, date.second];
  return timeParts.every((part) => typeof part === "number" && Number.isFinite(part))
    ? `${dateParts.join("-")}T${timeParts.join(":")}`
    : "";
}

export function parseCalendarDate(value: unknown): CalendarDate | null {
  if (value && typeof value === "object") {
    const date = value as Partial<CalendarDate>;
    if (typeof date.year === "number" && Number.isFinite(date.year)) {
      const precision =
        date.precision ?? (date.day !== undefined ? "day" : date.month !== undefined ? "month" : "year");
      return { ...date, calendar: readCalendarId(date.calendar), era: date.era ?? "CE", precision } as CalendarDate;
    }
  }
  if (typeof value !== "string") return null;
  const match = /^(\d+)(?:-(\d+)(?:-(\d+))?)?(?:T(\d+)(?::(\d+)(?::(\d+))?)?)?$/.exec(value.trim());
  if (!match) return null;
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
    era: "CE",
    year: Number(match[1]),
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

export function compareCalendarDates(left: unknown, right: unknown): number {
  const a = parseCalendarDate(left);
  const b = parseCalendarDate(right);
  if (!a && !b) return 0;
  if (!a) return 1;
  if (!b) return -1;
  return (
    a.year - b.year ||
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
