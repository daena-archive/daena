import { formatCalendarDate, parseCalendarDate } from "$lib/date";

export function isEmptyValue(value: unknown) {
  if (value === null || value === undefined) return true;
  if (typeof value === "string" && value.trim() === "") return true;
  if (Array.isArray(value) && value.length === 0) return true;
  if (typeof value === "object" && value !== null) {
    try {
      if (parseCalendarDate(value)) return false;
      if (Object.keys(value).length === 0) return true;
    } catch {}
  }
  return false;
}

export function fieldDisplay(value: unknown) {
  if (Array.isArray(value)) return value.join(", ");
  if (value === null || value === undefined || value === "") return "";
  if (typeof value === "object") {
    try {
      if (parseCalendarDate(value)) return formatCalendarDate(value);
    } catch {}
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  return String(value);
}

export function humanizeType(value: string) {
  const label = value.replaceAll("_", " ").replaceAll("-", " ");
  return label.charAt(0).toUpperCase() + label.slice(1);
}

export function formatSystemTimestamp(value: unknown): string {
  const timestamp = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(timestamp)) return "";
  const date = new Date(Math.floor(timestamp / 1_000_000));
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function formatAttributeValue(value: unknown, field: { type?: string } | null): string {
  if (field?.type === "date") {
    try {
      if (parseCalendarDate(value)) return formatCalendarDate(value as any);
    } catch {}
  }
  return fieldDisplay(value);
}
