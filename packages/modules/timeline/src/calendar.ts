import {
  formatCalendarDate,
  parseCalendarDate,
  serializeCalendarDate,
  type CalendarDate,
} from "../../../../src/lib/date.ts";

export const DEFAULT_CALENDAR_ID = "gregorian";
export const CALENDAR_DEFINITION_COLLECTION = "calendar-definition";
export const CALENDAR_DEFINITION_SCHEMA_VERSION = 1;

export type CalendarNamedUnit = {
  id: string;
  name: string;
  shortName?: string;
};

export type CalendarMonth = CalendarNamedUnit & {
  days: number;
};

export type CalendarSeason = CalendarNamedUnit & {
  startMonth: number;
  startDay: number;
  endMonth: number;
  endDay: number;
};

export type CalendarEpoch = {
  year: number;
  month?: number;
  day?: number;
};

export type CalendarDefinition = {
  schemaVersion: number;
  startingYear?: number;
  epoch?: CalendarEpoch;
  dateFormat?: string;
  months: CalendarMonth[];
  weekdays: CalendarNamedUnit[];
  seasons: CalendarSeason[];
  allowNegativeYears?: boolean;
  eraLabels?: { bce?: string; ce?: string };
};

export type YearPresetId = "custom" | "gregorian" | "twelve-30" | "ten-36";

export type YearPreset = {
  id: YearPresetId;
  name: string;
  description: string;
};

export const YEAR_PRESETS: YearPreset[] = [
  { id: "custom", name: "Custom year", description: "Start empty and name the months yourself." },
  { id: "gregorian", name: "Gregorian", description: "12 Earth months, 365 days, 7-day week." },
  { id: "twelve-30", name: "12 × 30 days", description: "A 360-day year of equal months." },
  { id: "ten-36", name: "10 × 36 days", description: "A 360-day year with ten months." },
];

export const DATE_FORMAT_PRESETS = [
  { id: "ymd-slash", label: "YYYY/MM/DD", pattern: "YYYY/MM/DD" },
  { id: "ymd-dash", label: "YYYY-MM-DD", pattern: "YYYY-MM-DD" },
  { id: "dmy-slash", label: "DD/MM/YYYY", pattern: "DD/MM/YYYY" },
  { id: "month-text", label: "D Month YYYY", pattern: "D MMMM YYYY" },
  { id: "text-weekday", label: "D Month YYYY, Weekday", pattern: "D MMMM YYYY, WWWW" },
] as const;

export const DEFAULT_DATE_FORMAT = "D MMMM YYYY";

export const DATE_FORMAT_GUIDE = [
  { token: "YYYY", meaning: "Full year, such as 842." },
  { token: "YY", meaning: "Last two digits of the year." },
  { token: "MMMM", meaning: "Full month name from this calendar." },
  { token: "MMM", meaning: "Short month name, or the full name if there is no short form." },
  { token: "MM", meaning: "Month number with two digits, such as 03." },
  { token: "M", meaning: "Month number, such as 3." },
  { token: "DD", meaning: "Day of the month with two digits, such as 07." },
  { token: "D", meaning: "Day of the month, such as 7." },
  { token: "WWWW", meaning: "Weekday name. Omitted when this calendar has no week." },
  { token: "WWW", meaning: "Short weekday name." },
  { token: "SSSS", meaning: "Season name. Omitted when the date is not in a season." },
] as const;

export type CalendarIssue = {
  level: "error" | "warning";
  message: string;
};

export type CalendarParts = {
  year: number;
  month?: number;
  day?: number;
  weekday?: number;
  season?: string;
  precision: CalendarDate["precision"];
};

const GREGORIAN_MONTHS: Omit<CalendarMonth, "id">[] = [
  { name: "January", shortName: "Jan", days: 31 },
  { name: "February", shortName: "Feb", days: 28 },
  { name: "March", shortName: "Mar", days: 31 },
  { name: "April", shortName: "Apr", days: 30 },
  { name: "May", shortName: "May", days: 31 },
  { name: "June", shortName: "Jun", days: 30 },
  { name: "July", shortName: "Jul", days: 31 },
  { name: "August", shortName: "Aug", days: 31 },
  { name: "September", shortName: "Sep", days: 30 },
  { name: "October", shortName: "Oct", days: 31 },
  { name: "November", shortName: "Nov", days: 30 },
  { name: "December", shortName: "Dec", days: 31 },
];

const GREGORIAN_WEEKDAYS: Omit<CalendarNamedUnit, "id">[] = [
  { name: "Sunday", shortName: "Sun" },
  { name: "Monday", shortName: "Mon" },
  { name: "Tuesday", shortName: "Tue" },
  { name: "Wednesday", shortName: "Wed" },
  { name: "Thursday", shortName: "Thu" },
  { name: "Friday", shortName: "Fri" },
  { name: "Saturday", shortName: "Sat" },
];

const GREGORIAN_SEASONS: Omit<CalendarSeason, "id">[] = [
  { name: "Spring", startMonth: 3, startDay: 20, endMonth: 6, endDay: 20 },
  { name: "Summer", startMonth: 6, startDay: 21, endMonth: 9, endDay: 21 },
  { name: "Autumn", startMonth: 9, startDay: 22, endMonth: 12, endDay: 20 },
  { name: "Winter", startMonth: 12, startDay: 21, endMonth: 3, endDay: 19 },
];

function slugId(prefix: string, name: string, index: number) {
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${prefix}-${slug || index + 1}`;
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function asFiniteNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function asPositiveInt(value: unknown): number | undefined {
  const number = asFiniteNumber(value);
  return number !== undefined && Number.isInteger(number) && number > 0 ? number : undefined;
}

function asString(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function namedUnit(prefix: string, value: unknown, index: number): CalendarNamedUnit {
  const record = asRecord(value);
  const name = asString(record.name) || `Day ${index + 1}`;
  const shortName = asString(record.shortName);
  return {
    id: asString(record.id) || slugId(prefix, name, index),
    name,
    ...(shortName ? { shortName } : {}),
  };
}

export function emptyCalendarDefinition(): CalendarDefinition {
  return {
    schemaVersion: CALENDAR_DEFINITION_SCHEMA_VERSION,
    months: [],
    weekdays: [],
    seasons: [],
  };
}

function numberedMonths(count: number, days: number, prefix: string): CalendarMonth[] {
  return Array.from({ length: count }, (_, index) => ({
    id: slugId("month", `${prefix}-${index + 1}`, index),
    name: `Month ${index + 1}`,
    days,
  }));
}

export function gregorianPresetDefinition(): CalendarDefinition {
  return {
    schemaVersion: CALENDAR_DEFINITION_SCHEMA_VERSION,
    startingYear: 1,
    epoch: { year: 1, month: 1, day: 1 },
    dateFormat: DEFAULT_DATE_FORMAT,
    months: GREGORIAN_MONTHS.map((month, index) => ({
      ...month,
      id: slugId("month", month.name, index),
    })),
    weekdays: GREGORIAN_WEEKDAYS.map((day, index) => ({
      ...day,
      id: slugId("weekday", day.name, index),
    })),
    seasons: GREGORIAN_SEASONS.map((season, index) => ({
      ...season,
      id: slugId("season", season.name, index),
    })),
  };
}

export function applyYearPreset(id: YearPresetId, current: CalendarDefinition): CalendarDefinition {
  const keep = {
    schemaVersion: CALENDAR_DEFINITION_SCHEMA_VERSION,
    startingYear: current.startingYear,
    epoch: current.epoch,
    dateFormat: current.dateFormat,
  };
  if (id === "gregorian") {
    const preset = gregorianPresetDefinition();
    return { ...preset, ...keep, months: preset.months, weekdays: preset.weekdays, seasons: preset.seasons };
  }
  if (id === "twelve-30") {
    return { ...emptyCalendarDefinition(), ...keep, months: numberedMonths(12, 30, "equal") };
  }
  if (id === "ten-36") {
    return { ...emptyCalendarDefinition(), ...keep, months: numberedMonths(10, 36, "ten") };
  }
  return { ...emptyCalendarDefinition(), ...keep };
}

export function matchYearPreset(definition: CalendarDefinition): YearPresetId {
  const lengths = definition.months.map((month) => month.days);
  if (definition.months.length === 12 && lengths.every((days, index) => days === GREGORIAN_MONTHS[index]?.days)) {
    return "gregorian";
  }
  if (definition.months.length === 12 && lengths.every((days) => days === 30)) return "twelve-30";
  if (definition.months.length === 10 && lengths.every((days) => days === 36)) return "ten-36";
  return "custom";
}

export function isDefaultCalendarId(calendarId: string | null | undefined): boolean {
  return !calendarId || calendarId === DEFAULT_CALENDAR_ID;
}

export function normalizeCalendarDefinition(value: unknown): CalendarDefinition {
  const record = asRecord(value);
  const months = Array.isArray(record.months)
    ? record.months.map((item, index) => {
        const unit = namedUnit("month", item, index);
        const days = asPositiveInt(asRecord(item).days) ?? 1;
        return { ...unit, days };
      })
    : [];
  const weekdays = Array.isArray(record.weekdays)
    ? record.weekdays.map((item, index) => namedUnit("weekday", item, index))
    : [];
  const seasons = Array.isArray(record.seasons)
    ? record.seasons.map((item, index) => {
        const unit = namedUnit("season", item, index);
        const season = asRecord(item);
        return {
          ...unit,
          startMonth: asPositiveInt(season.startMonth) ?? 1,
          startDay: asPositiveInt(season.startDay) ?? 1,
          endMonth: asPositiveInt(season.endMonth) ?? 1,
          endDay: asPositiveInt(season.endDay) ?? 1,
        };
      })
    : [];
  const epochRecord = asRecord(record.epoch);
  const epochYear = asFiniteNumber(epochRecord.year);
  const definition: CalendarDefinition = {
    schemaVersion: CALENDAR_DEFINITION_SCHEMA_VERSION,
    months,
    weekdays,
    seasons,
  };
  const startingYear = asFiniteNumber(record.startingYear);
  if (startingYear !== undefined && Number.isInteger(startingYear)) definition.startingYear = startingYear;
  if (epochYear !== undefined && Number.isInteger(epochYear)) {
    definition.epoch = {
      year: epochYear,
      ...(asPositiveInt(epochRecord.month) ? { month: asPositiveInt(epochRecord.month) } : {}),
      ...(asPositiveInt(epochRecord.day) ? { day: asPositiveInt(epochRecord.day) } : {}),
    };
  }
  const dateFormat = asString(record.dateFormat);
  if (dateFormat) definition.dateFormat = dateFormat;
  const allowNegativeYears = record.allowNegativeYears;
  if (typeof allowNegativeYears === "boolean") definition.allowNegativeYears = allowNegativeYears;
  const eraLabelsRecord = asRecord(record.eraLabels);
  const bce = asString(eraLabelsRecord.bce);
  const ce = asString(eraLabelsRecord.ce);
  if (bce || ce) definition.eraLabels = { ...(bce ? { bce } : {}), ...(ce ? { ce } : {}) };
  return definition;
}

export function computedYearLength(definition: CalendarDefinition): number | null {
  if (definition.months.length === 0) return null;
  return definition.months.reduce((total, month) => total + month.days, 0);
}

export function calendarHasStructure(definition: CalendarDefinition): boolean {
  return definition.months.length > 0 || definition.weekdays.length > 0 || definition.seasons.length > 0;
}

export function validateCalendarDefinition(definition: CalendarDefinition): CalendarIssue[] {
  const issues: CalendarIssue[] = [];
  for (const month of definition.months) {
    if (!month.name) issues.push({ level: "error", message: "A month must have a name." });
    if (!Number.isInteger(month.days) || month.days < 1) {
      issues.push({ level: "error", message: `${month.name || "A month"} must contain at least one day.` });
    }
  }
  if (definition.weekdays.some((day) => !day.name)) {
    issues.push({ level: "error", message: "Every weekday needs a name." });
  }
  const monthCount = definition.months.length;
  if (monthCount > 0) {
    for (const season of definition.seasons) {
      if (season.startMonth > monthCount || season.endMonth > monthCount) {
        issues.push({
          level: "error",
          message: `${season.name || "A season"} refers to a month that does not exist.`,
        });
      }
    }
  }
  if (!calendarHasStructure(definition)) {
    issues.push({
      level: "warning",
      message: "This calendar currently has no structure, so dates still use Gregorian.",
    });
  }
  if (definition.allowNegativeYears && !definition.eraLabels?.bce?.trim()) {
    issues.push({
      level: "error",
      message: "A label for years before the epoch is required when allowing negative years.",
    });
  }
  return issues;
}

function utcDayNumber(year: number, month: number, day: number): number {
  const date = new Date(0);
  date.setUTCFullYear(year, month - 1, day);
  date.setUTCHours(0, 0, 0, 0);
  return Math.round(date.getTime() / 86_400_000);
}

function fromUtcDayNumber(dayNumber: number): { year: number; month: number; day: number } {
  const date = new Date(dayNumber * 86_400_000);
  return {
    year: date.getUTCFullYear(),
    month: date.getUTCMonth() + 1,
    day: date.getUTCDate(),
  };
}

function startingYear(definition: CalendarDefinition): number {
  return definition.startingYear ?? 1;
}

function epochDayNumber(definition: CalendarDefinition): number {
  const epoch = definition.epoch ?? { year: 1, month: 1, day: 1 };
  return utcDayNumber(epoch.year, epoch.month ?? 1, epoch.day ?? 1);
}

function dayOfYear(definition: CalendarDefinition, month?: number, day?: number): number {
  if (definition.months.length === 0) return day ?? 1;
  const monthIndex = Math.max(1, Math.min(definition.months.length, month ?? 1)) - 1;
  const before = definition.months.slice(0, monthIndex).reduce((total, item) => total + item.days, 0);
  const monthDays = definition.months[monthIndex]?.days ?? 1;
  return before + Math.max(1, Math.min(monthDays, day ?? 1));
}

function partsFromDayOfYear(definition: CalendarDefinition, dayOfYearValue: number): { month?: number; day: number } {
  if (definition.months.length === 0) return { day: dayOfYearValue };
  let remaining = dayOfYearValue;
  for (const [index, month] of definition.months.entries()) {
    if (remaining <= month.days) return { month: index + 1, day: remaining };
    remaining -= month.days;
  }
  const last = definition.months.length;
  return { month: last, day: definition.months[last - 1]?.days ?? 1 };
}

function seasonName(definition: CalendarDefinition, month?: number, day?: number): string | undefined {
  if (!month || definition.seasons.length === 0) return undefined;
  const ordinal = dayOfYear(definition, month, day ?? 1);
  for (const season of definition.seasons) {
    const start = dayOfYear(definition, season.startMonth, season.startDay);
    const end = dayOfYear(definition, season.endMonth, season.endDay);
    if (start <= end) {
      if (ordinal >= start && ordinal <= end) return season.name;
    } else if (ordinal >= start || ordinal <= end) {
      return season.name;
    }
  }
  return undefined;
}

export function calendarDateToParts(value: unknown, definition: CalendarDefinition | null): CalendarParts | null {
  const date = parseCalendarDate(value);
  if (!date) return null;
  if (!definition || !calendarHasStructure(definition)) {
    return {
      year: date.year,
      month: date.month,
      day: date.day,
      precision: date.precision ?? "year",
    };
  }
  const yearLength = computedYearLength(definition);
  if (!yearLength) {
    return {
      year: startingYear(definition) + (date.year - (definition.epoch?.year ?? 1)),
      precision: "year",
    };
  }
  if ((date.precision ?? "year") === "year" || date.month === undefined) {
    return {
      year: startingYear(definition) + (date.year - (definition.epoch?.year ?? 1)),
      precision: "year",
    };
  }
  if ((date.precision ?? "day") === "month" || date.day === undefined) {
    const storedDay = utcDayNumber(date.year, date.month, 1);
    const delta = storedDay - epochDayNumber(definition);
    const yearOffset = Math.floor(delta / yearLength);
    let doy = (delta % yearLength) + 1;
    if (doy <= 0) doy += yearLength;
    const parts = partsFromDayOfYear(definition, doy);
    const year = startingYear(definition) + yearOffset;
    const weekday =
      definition.weekdays.length > 0
        ? ((delta % definition.weekdays.length) + definition.weekdays.length) % definition.weekdays.length
        : undefined;
    return {
      year,
      month: parts.month,
      ...(weekday !== undefined ? { weekday } : {}),
      ...(seasonName(definition, parts.month, 1) ? { season: seasonName(definition, parts.month, 1) } : {}),
      precision: "month",
    };
  }
  const storedDay = utcDayNumber(date.year, date.month, date.day ?? 1);
  const delta = storedDay - epochDayNumber(definition);
  const yearOffset = Math.floor(delta / yearLength);
  let doy = (delta % yearLength) + 1;
  if (doy <= 0) doy += yearLength;
  const parts = partsFromDayOfYear(definition, doy);
  const year = startingYear(definition) + yearOffset;
  const weekday =
    definition.weekdays.length > 0
      ? ((delta % definition.weekdays.length) + definition.weekdays.length) % definition.weekdays.length
      : undefined;
  return {
    year,
    ...parts,
    ...(weekday !== undefined ? { weekday } : {}),
    ...(seasonName(definition, parts.month, parts.day)
      ? { season: seasonName(definition, parts.month, parts.day) }
      : {}),
    precision: date.precision ?? "day",
  };
}

export function partsToCalendarDate(parts: CalendarParts, definition: CalendarDefinition | null): CalendarDate {
  if (!definition || !calendarHasStructure(definition)) {
    return {
      calendar: "gregorian",
      era: "CE",
      year: parts.year,
      ...(parts.month !== undefined ? { month: parts.month } : {}),
      ...(parts.day !== undefined ? { day: parts.day } : {}),
      precision: parts.precision,
    };
  }
  const yearLength = computedYearLength(definition);
  if (!yearLength || parts.precision === "year") {
    const year = (definition.epoch?.year ?? 1) + (parts.year - startingYear(definition));
    return { calendar: "gregorian", era: "CE", year, precision: "year" };
  }
  if (parts.precision === "month") {
    const doy = dayOfYear(definition, parts.month, 1);
    const dayNumber = epochDayNumber(definition) + (parts.year - startingYear(definition)) * yearLength + (doy - 1);
    const gregorian = fromUtcDayNumber(dayNumber);
    return { calendar: "gregorian", era: "CE", year: gregorian.year, month: gregorian.month, precision: "month" };
  }
  const doy = dayOfYear(definition, parts.month, parts.day);
  const dayNumber = epochDayNumber(definition) + (parts.year - startingYear(definition)) * yearLength + (doy - 1);
  const gregorian = fromUtcDayNumber(dayNumber);
  return {
    calendar: "gregorian",
    era: "CE",
    year: gregorian.year,
    month: gregorian.month,
    day: gregorian.day,
    precision: "day",
  };
}

function eraSuffix(year: number, definition: CalendarDefinition): string {
  if (year < 1) {
    const label = definition.eraLabels?.bce?.trim() || "BCE";
    return label ? ` ${label}` : "";
  }
  const label = definition.eraLabels?.ce?.trim();
  return label ? ` ${label}` : "";
}

export function formatCalendarParts(parts: CalendarParts, definition: CalendarDefinition): string {
  const pattern = definition.dateFormat?.trim() || DEFAULT_DATE_FORMAT;
  const month = parts.month !== undefined ? definition.months[parts.month - 1] : undefined;
  const weekday = parts.weekday !== undefined ? definition.weekdays[parts.weekday] : undefined;
  const pad = (value: number) => String(value).padStart(2, "0");
  const displayYear = parts.year < 1 ? 1 - parts.year : parts.year;
  const tokens: Record<string, string> = {
    YYYY: String(displayYear),
    YY: String(Math.abs(displayYear)).slice(-2).padStart(2, "0"),
    MMMM: month?.name ?? "",
    MMM: month?.shortName || month?.name || "",
    MM: parts.month !== undefined ? pad(parts.month) : "",
    M: parts.month !== undefined ? String(parts.month) : "",
    DD: parts.day !== undefined ? pad(parts.day) : "",
    D: parts.day !== undefined ? String(parts.day) : "",
    WWWW: weekday?.name ?? "",
    WWW: weekday?.shortName || weekday?.name || "",
    SSSS: parts.season ?? "",
  };
  const base = pattern
    .replace(/YYYY|MMMM|WWWW|SSSS|MMM|WWW|YY|MM|DD|M|D/g, (token) => tokens[token] ?? token)
    .replace(/\s+,/g, ",")
    .replace(/[/\-.]{2,}/g, (match) => match[0] ?? "")
    .replace(/\s{2,}/g, " ")
    .replace(/^[/\-.,\s]+|[/\-.,\s]+$/g, "")
    .trim();
  const suffix = eraSuffix(parts.year, definition);
  return suffix ? `${base}${suffix}` : base;
}

export function previewCalendarParts(definition: CalendarDefinition): CalendarParts {
  const year = definition.startingYear ?? 842;
  if (definition.months.length === 0) return { year, precision: "year" };
  const month = 1;
  const day = Math.min(17, definition.months[0]?.days ?? 1);
  return {
    year,
    month,
    day,
    weekday: definition.weekdays.length ? 0 : undefined,
    season: seasonName(definition, month, day),
    precision: "day",
  };
}

export function calendarSummary(definition: CalendarDefinition): string {
  const yearLength = computedYearLength(definition);
  const format = definition.dateFormat?.trim() || DEFAULT_DATE_FORMAT;
  if (!yearLength) return "No year structure yet. Dates still use Gregorian.";
  return `${definition.months.length} months · ${yearLength} days · ${format}`;
}

export function epochSummary(definition: CalendarDefinition): string {
  const epoch = definition.epoch ?? { year: 1, month: 1, day: 1 };
  const start = definition.startingYear ?? 1;
  return `Year ${start} begins on Gregorian ${epoch.year}-${epoch.month ?? 1}-${epoch.day ?? 1}`;
}

export function formatWithCalendar(value: unknown, definition: CalendarDefinition | null): string {
  const date = parseCalendarDate(value);
  if (!date) return typeof value === "string" && value ? value : "Undated";
  if (!definition || !calendarHasStructure(definition) || definition.months.length === 0) {
    return formatCalendarDate(date);
  }
  const parts = calendarDateToParts(date, definition);
  if (!parts) return formatCalendarDate(date);
  return formatCalendarParts(parts, definition) || formatCalendarDate(date);
}

export function serializeStoredDate(
  parts: CalendarParts,
  definition: CalendarDefinition | null,
): string | CalendarDate {
  return serializeCalendarDate(partsToCalendarDate(parts, definition));
}

export function daysInCalendarMonth(definition: CalendarDefinition | null, year: number, month: number): number {
  if (!definition || definition.months.length === 0) {
    const date = new Date(0);
    date.setUTCFullYear(year, month, 0);
    return date.getUTCDate();
  }
  return definition.months[month - 1]?.days ?? 1;
}
