import { formatCalendarDate, parseCalendarDate } from "../date.ts";

const SENTINEL_DATE_STRINGS = new Set(["1-1", "1-1-1"]);

export function isSentinelDateValue(value: unknown): boolean {
  if (typeof value === "string") {
    return SENTINEL_DATE_STRINGS.has(value.trim());
  }
  const parsed = parseCalendarDate(value);
  if (!parsed) return false;
  if (
    parsed.year === 1 &&
    parsed.month === 1 &&
    parsed.day === 1 &&
    parsed.era !== "BCE" &&
    (!parsed.precision || parsed.precision === "day")
  ) {
    return true;
  }
  const iso = formatCalendarDate(parsed);
  return SENTINEL_DATE_STRINGS.has(iso);
}

export function hasMeaningfulDateValue(value: unknown): boolean {
  if (isEmptyFieldValue(value) || isSentinelDateValue(value)) return false;
  return parseCalendarDate(value) !== null;
}

export function isEmptyFieldValue(value: unknown): boolean {
  return (
    value === undefined ||
    value === null ||
    value === "" ||
    (typeof value === "string" && !value.trim()) ||
    (Array.isArray(value) && value.length === 0)
  );
}

export function shouldPersistFieldValue(value: unknown, exists: boolean): boolean {
  return exists || !isEmptyFieldValue(value);
}

export function isStructuredFieldValue(value: unknown): boolean {
  return value !== null && typeof value === "object";
}

export function restoreStructuredFieldValue(value: unknown, wasStructured: boolean, label: string): unknown {
  if (!wasStructured || typeof value !== "string") return value;

  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw new Error(`${label} must contain valid JSON.`);
  }

  if (!isStructuredFieldValue(parsed)) {
    throw new Error(`${label} must contain a JSON object or array.`);
  }
  return parsed;
}
