export type OrthographyMapping = {
  id: string;
  grapheme: string;
  sounds: string[];
  environment?: string;
  notes?: string;
};

export type OrthographyValue = {
  name: string;
  status?: string;
  notes?: string;
  mappings: OrthographyMapping[];
};

const TEXT = 500;
const LONG = 2000;
const MAX_MAPPINGS = 256;
const MAX_SOUNDS = 16;

function id() {
  return crypto.randomUUID();
}

function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

function sounds(value: unknown) {
  const items = Array.isArray(value) ? value : typeof value === "string" ? value.split(/[\s,]+/) : [];
  return [...new Set(items.map((item) => text(item)).filter(Boolean))].slice(0, MAX_SOUNDS);
}

export function emptyOrthography(): OrthographyValue {
  return { name: "", mappings: [] };
}

export function normalizeOrthography(value: unknown): OrthographyValue {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const mappings = Array.isArray(record.mappings)
    ? record.mappings
        .map((item): OrthographyMapping | null => {
          const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
          const grapheme = text(entry.grapheme);
          const mapped = sounds(entry.sounds);
          return grapheme || mapped.length
            ? {
                id: text(entry.id) || id(),
                grapheme,
                sounds: mapped,
                environment: optional(entry.environment),
                notes: optional(entry.notes, LONG),
              }
            : null;
        })
        .filter((item): item is OrthographyMapping => item !== null)
        .slice(0, MAX_MAPPINGS)
    : [];
  return {
    name: text(record.name),
    status: optional(record.status),
    notes: optional(record.notes, LONG),
    mappings,
  };
}

export function serializeOrthography(value: OrthographyValue): Record<string, unknown> {
  return normalizeOrthography(value);
}
