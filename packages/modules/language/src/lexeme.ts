export type LexemeExample = {
  id: string;
  text: string;
  translation?: string;
};

export type LexemeSense = {
  id: string;
  gloss?: string;
  definition?: string;
  usageNotes?: string;
  examples: LexemeExample[];
};

export type LexemeForm = {
  id: string;
  form: string;
  kind?: string;
  pronunciation?: string;
};

export type LexemePronunciation = {
  id: string;
  value: string;
  note?: string;
};

export type LexemeValue = {
  lemma: string;
  partOfSpeech?: string;
  status?: string;
  tags: string[];
  etymology?: string;
  sourceNotes?: string;
  notes?: string;
  meanings: string[];
  pronunciations: LexemePronunciation[];
  forms: LexemeForm[];
  senses: LexemeSense[];
};

export const LEXICON_FORMAT = "daena.language.lexicon";
export const PART_OF_SPEECH_SUGGESTIONS = [
  "noun",
  "verb",
  "adjective",
  "adverb",
  "pronoun",
  "preposition",
  "conjunction",
  "particle",
  "interjection",
  "determiner",
];
export const STATUS_SUGGESTIONS = ["draft", "active", "archaic", "obsolete", "reconstructed"];

const TEXT = 500;
const LONG = 2000;
const MAX_TAGS = 16;
const MAX_SENSES = 32;
const MAX_EXAMPLES = 8;
const MAX_FORMS = 16;
const MAX_PRONUNCIATIONS = 8;

function id() {
  return crypto.randomUUID();
}

function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

function strings(value: unknown, limit = MAX_TAGS) {
  const items = Array.isArray(value) ? value : typeof value === "string" ? value.split(/[\n,]/) : [];
  return [...new Set(items.map((item) => text(item)).filter(Boolean))].slice(0, limit);
}

function examples(value: unknown): LexemeExample[] {
  const items = Array.isArray(value) ? value : [];
  return items
    .map((item) => {
      const record = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
      const exampleText = text(record.text, LONG);
      return exampleText
        ? {
            id: text(record.id) || id(),
            text: exampleText,
            translation: optional(record.translation, LONG),
          }
        : null;
    })
    .filter((item): item is LexemeExample => item !== null)
    .slice(0, MAX_EXAMPLES);
}

export function emptyLexeme(): LexemeValue {
  return {
    lemma: "",
    tags: [],
    meanings: [],
    pronunciations: [],
    forms: [],
    senses: [{ id: id(), examples: [] }],
  };
}

export function normalizeLexeme(value: unknown): LexemeValue {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const legacyExample = record.example && typeof record.example === "object" ? examples([record.example]) : [];
  const pronunciations = Array.isArray(record.pronunciations)
    ? record.pronunciations
        .map((item) => {
          const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
          const spoken = text(entry.value);
          return spoken ? { id: text(entry.id) || id(), value: spoken, note: optional(entry.note) } : null;
        })
        .filter((item): item is LexemePronunciation => item !== null)
        .slice(0, MAX_PRONUNCIATIONS)
    : optional(record.pronunciation)
      ? [{ id: id(), value: String(record.pronunciation).trim() }]
      : [];
  const forms = Array.isArray(record.forms)
    ? record.forms
        .map((item) => {
          const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
          const form = text(entry.form);
          return form
            ? {
                id: text(entry.id) || id(),
                form,
                kind: optional(entry.kind),
                pronunciation: optional(entry.pronunciation),
              }
            : null;
        })
        .filter((item): item is LexemeForm => item !== null)
        .slice(0, MAX_FORMS)
    : [];
  const senses = Array.isArray(record.senses)
    ? record.senses
        .map((item) => {
          const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
          return {
            id: text(entry.id) || id(),
            gloss: optional(entry.gloss),
            definition: optional(entry.definition, LONG),
            usageNotes: optional(entry.usageNotes, LONG),
            examples: examples(entry.examples),
          };
        })
        .slice(0, MAX_SENSES)
    : strings(record.meanings, MAX_SENSES).map((gloss, index) => ({
        id: id(),
        gloss,
        examples: index === 0 ? legacyExample : [],
      }));
  const normalizedSenses = senses.length ? senses : [{ id: id(), examples: legacyExample }];
  const meanings = normalizedSenses.map((sense) => sense.gloss).filter((item): item is string => Boolean(item));
  return {
    lemma: text(record.lemma),
    partOfSpeech: optional(record.partOfSpeech),
    status: optional(record.status),
    tags: strings(record.tags),
    etymology: optional(record.etymology, LONG),
    sourceNotes: optional(record.sourceNotes, LONG),
    notes: optional(record.notes, LONG),
    meanings,
    pronunciations,
    forms,
    senses: normalizedSenses,
  };
}

export function serializeLexeme(value: LexemeValue): Record<string, unknown> {
  const normalized = normalizeLexeme(value);
  return {
    lemma: normalized.lemma,
    partOfSpeech: normalized.partOfSpeech,
    status: normalized.status,
    tags: normalized.tags,
    etymology: normalized.etymology,
    sourceNotes: normalized.sourceNotes,
    notes: normalized.notes,
    meanings: normalized.meanings,
    pronunciations: normalized.pronunciations,
    forms: normalized.forms,
    senses: normalized.senses,
  };
}

export function firstGloss(value: LexemeValue) {
  return value.senses.find((sense) => sense.gloss)?.gloss || value.meanings[0] || "";
}

export function parseLexiconImport(raw: string): LexemeValue[] {
  const parsed = JSON.parse(raw) as unknown;
  const items = Array.isArray(parsed)
    ? parsed
    : parsed && typeof parsed === "object" && Array.isArray((parsed as { lexemes?: unknown }).lexemes)
      ? (parsed as { lexemes: unknown[] }).lexemes
      : parsed && typeof parsed === "object" && Array.isArray((parsed as { records?: unknown }).records)
        ? (parsed as { records: unknown[] }).records.map((item) =>
            item && typeof item === "object" && "value" in item ? (item as { value: unknown }).value : item,
          )
        : null;
  if (!items) throw new Error("Lexicon import must be a lexeme array or { lexemes: [...] }.");
  return items.map(normalizeLexeme).filter((item) => item.lemma);
}

export function lexiconExport(languageName: string, values: LexemeValue[]) {
  return `${JSON.stringify(
    {
      format: LEXICON_FORMAT,
      version: 1,
      language: languageName,
      lexemes: values.map((value) => serializeLexeme(value)),
    },
    null,
    2,
  )}\n`;
}
