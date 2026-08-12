export type PhonemeKind = "consonant" | "vowel" | "tone" | "other";

export type PhonemeValue = {
  symbol: string;
  ipa?: string;
  kind: PhonemeKind;
  place?: string;
  manner?: string;
  voicing?: string;
  height?: string;
  backness?: string;
  rounding?: string;
  notes?: string;
  example?: string;
};

export type PhonologyNotes = {
  syllableStructure?: string;
  stress?: string;
  tone?: string;
  phonotactics?: string;
  notes?: string;
};

export const PHONEME_KINDS: PhonemeKind[] = ["consonant", "vowel", "tone", "other"];
export const PLACE_SUGGESTIONS = [
  "bilabial",
  "labiodental",
  "dental",
  "alveolar",
  "postalveolar",
  "retroflex",
  "palatal",
  "velar",
  "uvular",
  "pharyngeal",
  "glottal",
];
export const MANNER_SUGGESTIONS = [
  "plosive",
  "nasal",
  "trill",
  "tap",
  "fricative",
  "affricate",
  "approximant",
  "lateral",
];
export const HEIGHT_SUGGESTIONS = ["close", "near-close", "close-mid", "mid", "open-mid", "near-open", "open"];
export const BACKNESS_SUGGESTIONS = ["front", "central", "back"];
export const VOICING_SUGGESTIONS = ["voiceless", "voiced"];
export const ROUNDING_SUGGESTIONS = ["unrounded", "rounded"];

const TEXT = 500;
const LONG = 2000;

function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

function kind(value: unknown): PhonemeKind {
  return PHONEME_KINDS.includes(value as PhonemeKind) ? (value as PhonemeKind) : "other";
}

export function emptyPhoneme(kind: PhonemeKind = "consonant"): PhonemeValue {
  return { symbol: "", kind };
}

export function emptyPhonologyNotes(): PhonologyNotes {
  return {};
}

export function normalizePhoneme(value: unknown): PhonemeValue {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  return {
    symbol: text(record.symbol),
    ipa: optional(record.ipa),
    kind: kind(record.kind),
    place: optional(record.place),
    manner: optional(record.manner),
    voicing: optional(record.voicing),
    height: optional(record.height),
    backness: optional(record.backness),
    rounding: optional(record.rounding),
    notes: optional(record.notes, LONG),
    example: optional(record.example),
  };
}

export function serializePhoneme(value: PhonemeValue): Record<string, unknown> {
  return normalizePhoneme(value);
}

export function normalizePhonologyNotes(value: unknown): PhonologyNotes {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  return {
    syllableStructure: optional(record.syllableStructure, LONG),
    stress: optional(record.stress, LONG),
    tone: optional(record.tone, LONG),
    phonotactics: optional(record.phonotactics, LONG),
    notes: optional(record.notes, LONG),
  };
}

export function serializePhonologyNotes(value: PhonologyNotes): Record<string, unknown> {
  return normalizePhonologyNotes(value);
}

function ordered(values: string[], preferred: string[]) {
  const unique = [...new Set(values.map((item) => item.toLocaleLowerCase()))];
  const rank = new Map(preferred.map((item, index) => [item, index]));
  return unique.sort((left, right) => {
    const leftRank = rank.get(left) ?? preferred.length;
    const rightRank = rank.get(right) ?? preferred.length;
    return leftRank - rightRank || left.localeCompare(right);
  });
}

export type ChartCell<T> = { row: string; column: string; items: T[] };

export function consonantChart(phonemes: PhonemeValue[]): {
  rows: string[];
  columns: string[];
  cells: ChartCell<PhonemeValue>[];
  unplaced: PhonemeValue[];
} {
  const placed = phonemes.filter((item) => item.kind === "consonant" && item.place && item.manner);
  const unplaced = phonemes.filter((item) => item.kind === "consonant" && !(item.place && item.manner));
  const columns = ordered(
    placed.map((item) => item.place!),
    PLACE_SUGGESTIONS,
  );
  const rows = ordered(
    placed.map((item) => item.manner!),
    MANNER_SUGGESTIONS,
  );
  const grouped = new Map<string, PhonemeValue[]>();
  for (const item of placed) {
    const key = `${item.manner!.toLocaleLowerCase()}|${item.place!.toLocaleLowerCase()}`;
    grouped.set(key, [...(grouped.get(key) ?? []), item]);
  }
  const cells = rows.flatMap((row) =>
    columns.map((column) => ({
      row,
      column,
      items: grouped.get(`${row}|${column}`) ?? [],
    })),
  );
  return { rows, columns, cells, unplaced };
}

export function vowelChart(phonemes: PhonemeValue[]): {
  rows: string[];
  columns: string[];
  cells: ChartCell<PhonemeValue>[];
  unplaced: PhonemeValue[];
} {
  const placed = phonemes.filter((item) => item.kind === "vowel" && item.height && item.backness);
  const unplaced = phonemes.filter((item) => item.kind === "vowel" && !(item.height && item.backness));
  const columns = ordered(
    placed.map((item) => item.backness!),
    BACKNESS_SUGGESTIONS,
  );
  const rows = ordered(
    placed.map((item) => item.height!),
    HEIGHT_SUGGESTIONS,
  );
  const grouped = new Map<string, PhonemeValue[]>();
  for (const item of placed) {
    const key = `${item.height!.toLocaleLowerCase()}|${item.backness!.toLocaleLowerCase()}`;
    grouped.set(key, [...(grouped.get(key) ?? []), item]);
  }
  const cells = rows.flatMap((row) =>
    columns.map((column) => ({
      row,
      column,
      items: grouped.get(`${row}|${column}`) ?? [],
    })),
  );
  return { rows, columns, cells, unplaced };
}
