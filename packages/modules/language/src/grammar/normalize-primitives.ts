import { GRAMMAR_SYSTEM_IDS, type EmptyConfig, type GrammarIssue, type GrammarStatus } from "./types.ts";

export const TEXT = 500;

export const NOTES = 4_000;

export const BODY = 8_000;

export const CELL_FORM = 200;

export const MAX_LINKS = 32;

export const MAX_EXAMPLES = 16;

export const MAX_TAGS = 16;

export const MAX_AXES = 8;

export const MAX_AXIS_VALUES = 24;

export const MAX_CELLS = 384;

export const MAX_CATEGORIES = 32;

export const MAX_FEATURES = 16;

export const MAX_ARTICLES = 16;

export const MAX_ALTERNATES = 8;

export const MAX_STRATEGIES = 8;

export const SYSTEM_IDS = new Set<string>(GRAMMAR_SYSTEM_IDS);

export const STATUSES = new Set<GrammarStatus>(["unconfigured", "configured", "not-used"]);

export const LINK_KINDS = new Set(["lexeme", "lexeme-example", "sample", "paradigm"]);

export const CELL_STATES = new Set(["form", "same-as", "zero", "not-applicable"]);

export function id() {
  return crypto.randomUUID();
}

export function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

export function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

export function lines(value: unknown, limit: number) {
  return typeof value === "string" ? value.replace(/\r\n?/g, "\n").trim().slice(0, limit) : "";
}

export function obj(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

export function pick<T extends string>(value: unknown, allowed: readonly T[]): T | undefined {
  const next = text(value);
  return allowed.includes(next as T) ? (next as T) : undefined;
}

export function pickList<T extends string>(value: unknown, allowed: readonly T[], max = MAX_STRATEGIES): T[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<T>();
  const out: T[] = [];
  for (const item of value) {
    const next = pick(item, allowed);
    if (!next || seen.has(next)) continue;
    seen.add(next);
    out.push(next);
    if (out.length >= max) break;
  }
  return out;
}

export function bool(value: unknown) {
  return typeof value === "boolean" ? value : undefined;
}

export function compact<T>(value: T): T {
  if (Array.isArray(value)) return value.map((item) => compact(item)) as T;
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .filter(([, item]) => item !== undefined)
        .map(([key, item]) => [key, compact(item)]),
    ) as T;
  }
  return value;
}

export function issue(code: GrammarIssue["code"], message: string, path?: string): GrammarIssue {
  return { code, message, path };
}

export function emptyConfig(): EmptyConfig {
  return {};
}
