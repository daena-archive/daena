import { parseCalendarDate, serializeCalendarDate } from "../date.ts";
import type { FieldDefinition } from "$lib/project/client";

export function fieldRevisionKey(namespace: string, key: string) {
  return `${namespace}\u0000${key}`;
}

export function fieldDisplayValue(value: unknown): string {
  if (Array.isArray(value)) return value.map((item) => fieldDisplayValue(item)).join(", ");
  if (typeof value === "object" && value !== null) {
    try {
      return JSON.stringify(value);
    } catch {
      return "";
    }
  }
  return String(value ?? "");
}

export function fieldInputValue(definition: FieldDefinition, value: unknown): string | number | boolean | string[] {
  if (definition.type === "boolean") return value === true;
  if (definition.type === "number") return typeof value === "number" && Number.isFinite(value) ? value : "";
  if ((definition as any).type === "oneof") return fieldDisplayValue(value);
  if (definition.multiple) return Array.isArray(value) ? value.map((item) => String(item)) : [];
  return fieldDisplayValue(value);
}

export function fieldValueForSave(definition: FieldDefinition, value: unknown) {
  if (definition.type === "number") {
    if (value === "" || value === null || value === undefined) return "";
    const numberValue = typeof value === "number" ? value : Number(value);
    return Number.isFinite(numberValue) ? numberValue : "";
  }
  if (definition.type === "boolean") return value === true || value === "true";
  if ((definition as any).type === "oneof") return value;
  if (definition.multiple) return Array.isArray(value) ? value.map((item) => String(item)) : [];
  return value;
}

export function aiScalarValue(definition: FieldDefinition, raw: unknown): unknown | null {
  if (definition.type === "number") {
    const value = typeof raw === "number" ? raw : typeof raw === "string" ? Number(raw.trim()) : Number.NaN;
    return Number.isFinite(value) ? value : null;
  }
  if (definition.type === "boolean") {
    if (typeof raw === "boolean") return raw;
    if (raw === "true") return true;
    if (raw === "false") return false;
    return null;
  }
  if (definition.type === "enum") {
    return typeof raw === "string" && definition.options?.includes(raw) ? raw : null;
  }
  if ((definition as any).type === "oneof") {
    const opts =
      definition.options ??
      ((definition as any).oneOf as Array<{ options?: string[] }> | undefined)?.flatMap((v) => v.options ?? []) ??
      [];
    return typeof raw === "string" && opts.includes(raw) ? raw : null;
  }
  if (definition.type === "date") {
    if (typeof raw !== "string") return null;
    const date = parseCalendarDate(raw.trim());
    return date ? serializeCalendarDate(date) : null;
  }
  return typeof raw === "string" && raw.trim() ? raw : null;
}

export function coerceAiFieldValue(definition: FieldDefinition, raw: unknown): unknown | null {
  const isOne = (definition as any).cardinality === "one";
  const isRelationship = definition.type === "relationship";
  const isMultiple = definition.multiple || (isRelationship && !isOne);
  if (isMultiple || isRelationship) {
    // For cardinality "one", allow single string as well as array with 1
    if (isOne && typeof raw === "string" && raw.trim()) {
      return raw.trim();
    }
    if (!Array.isArray(raw) || raw.length === 0 || raw.length > 5) {
      // For "one", also allow single string case already handled, so fail for array
      if (isOne && typeof raw === "string") return null;
      return null;
    }
    if (isOne && raw.length > 1) return null;
    const values = raw.map((item) =>
      isRelationship ? (typeof item === "string" && item.trim() ? item : null) : aiScalarValue(definition, item),
    );
    return values.every((value) => value !== null) ? values : null;
  }
  return aiScalarValue(definition, raw);
}

export function aiJsonValueSchema(definition: FieldDefinition) {
  const isOne = (definition as any).cardinality === "one";
  const scalarType = definition.type === "number" ? "number" : definition.type === "boolean" ? "boolean" : "string";
  const isOneOf = (definition as any).type === "oneof";
  const enumOptions = isOneOf
    ? (((definition as any).oneOf as Array<{ options?: string[] }> | undefined)?.flatMap((v) => v.options ?? []) ??
      definition.options)
    : definition.options;
  const scalar: any = {
    type: scalarType,
    ...(definition.type === "enum" && definition.options?.length ? { enum: definition.options } : {}),
    ...(isOneOf && (enumOptions as string[])?.length ? { enum: enumOptions } : {}),
  };
  const isMulti = definition.multiple || (definition.type === "relationship" && !isOne);
  return isMulti || definition.type === "relationship"
    ? { type: "array", items: scalar, maxItems: isOne ? 1 : 5, uniqueItems: true }
    : scalar;
}

export function suggestionConfidenceTone(confidence: string) {
  const normalized = confidence.trim().toLowerCase();
  return normalized === "high" || normalized === "medium" || normalized === "low" ? normalized : "unknown";
}

export function suggestionConfidenceLabel(confidence: string) {
  const tone = suggestionConfidenceTone(confidence);
  return tone.charAt(0).toUpperCase() + tone.slice(1);
}
