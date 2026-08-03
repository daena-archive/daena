export interface CalendarDate {
  calendar: "gregorian";
  year: number;
  month?: number;
  day?: number;
  /** Legacy editor fields; ignored by Gregorian serialization. */
  era?: "BCE" | "CE";
  precision?: "year" | "month" | "day";
}

export function parseCalendarDate(value: unknown): CalendarDate | null {
  if (value && typeof value === "object") {
    const date = value as Partial<CalendarDate>;
    if (date.calendar === "gregorian" && typeof date.year === "number" && Number.isFinite(date.year)) {
      const precision = date.precision ?? (date.day !== undefined ? "day" : date.month !== undefined ? "month" : "year");
      return { ...date, era: date.era ?? "CE", precision } as CalendarDate;
    }
  }
  if (typeof value !== "string") return null;
  const match = /^(\d+)(?:-(\d+)(?:-(\d+))?)?$/.exec(value.trim());
  if (!match) return null;
  const precision = match[3] ? "day" : match[2] ? "month" : "year";
  return { calendar: "gregorian", era: "CE", year: Number(match[1]), precision, ...(match[2] ? { month: Number(match[2]) } : {}), ...(match[3] ? { day: Number(match[3]) } : {}) };
}

export function serializeCalendarDate(date: CalendarDate): string {
  const parts = date.precision === "year" ? [date.year] : date.precision === "month" ? [date.year, date.month] : [date.year, date.month, date.day];
  return parts.every((part) => typeof part === "number" && Number.isFinite(part)) ? parts.join("-") : "";
}

export function formatCalendarDate(value: unknown): string {
  const date = parseCalendarDate(value);
  if (!date) return typeof value === "string" && value ? value : "Undated";
  return serializeCalendarDate(date);
}

export function isCompleteCalendarDate(value: unknown): boolean {
  const date = parseCalendarDate(value);
  return !!date && Number.isFinite(date.year) && Number.isFinite(date.month) && Number.isFinite(date.day);
}

export function compareCalendarDates(left: unknown, right: unknown): number {
  const a = parseCalendarDate(left); const b = parseCalendarDate(right);
  if (!a && !b) return 0; if (!a) return 1; if (!b) return -1;
  return a.year - b.year || (a.month ?? 0) - (b.month ?? 0) || (a.day ?? 0) - (b.day ?? 0);
}
