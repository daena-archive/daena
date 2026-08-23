export type SampleKind = "sentence" | "paragraph";

export type SampleToken = {
  id: string;
  text: string;
  gloss?: string;
  grammar?: string;
  lexemeId?: string;
};

export type Sample = {
  title: string;
  kind: SampleKind;
  text: string;
  translation?: string;
  transliteration?: string;
  notes?: string;
  tokens: SampleToken[];
};

export const SAMPLE_KINDS: { id: SampleKind; label: string }[] = [
  { id: "sentence", label: "Sentences" },
  { id: "paragraph", label: "Paragraphs" },
];

const TEXT = 500;
const BODY = 24_000;
const TOKEN = 200;
const MAX_TOKENS = 256;
const KIND_IDS = new Set(SAMPLE_KINDS.map((item) => item.id));

function id() {
  return crypto.randomUUID();
}

function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

function kind(value: unknown): SampleKind {
  return KIND_IDS.has(value as SampleKind) ? (value as SampleKind) : "sentence";
}

function body(value: unknown) {
  return typeof value === "string" ? value.replace(/\r\n?/g, "\n").slice(0, BODY) : "";
}

export function emptySample(kind: SampleKind = "sentence"): Sample {
  return { title: "", kind, text: "", tokens: [] };
}

export function emptyToken(): SampleToken {
  return { id: id(), text: "" };
}

export function sampleTitle(sample: Sample) {
  return sample.title || sample.text.trim().split("\n")[0]?.trim() || "Untitled sample";
}

export function splitSampleText(value: string) {
  return value
    .replace(/\r\n?/g, "\n")
    .split(/\s+/)
    .map((item) => item.trim())
    .filter(Boolean)
    .slice(0, MAX_TOKENS);
}

export function tokenizeSample(source: string, existing: SampleToken[] = []): SampleToken[] {
  const unused = existing.map((token) => ({ ...token }));
  return splitSampleText(source).map((surface) => {
    const index = unused.findIndex((token) => token.text === surface);
    if (index >= 0) {
      const [match] = unused.splice(index, 1);
      return { ...match, text: surface };
    }
    return { id: id(), text: surface };
  });
}

export function normalizeSample(value: unknown): Sample {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const tokens = Array.isArray(record.tokens)
    ? record.tokens
        .map((item): SampleToken | null => {
          const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
          const tokenText = text(entry.text, TOKEN);
          return tokenText
            ? {
                id: text(entry.id) || id(),
                text: tokenText,
                gloss: optional(entry.gloss, TOKEN),
                grammar: optional(entry.grammar, TOKEN),
                lexemeId: optional(entry.lexemeId),
              }
            : null;
        })
        .filter((item): item is SampleToken => item !== null)
        .slice(0, MAX_TOKENS)
    : [];
  return {
    title: text(record.title),
    kind: kind(record.kind),
    text: body(record.text),
    translation: optional(record.translation, BODY),
    transliteration: optional(record.transliteration, BODY),
    notes: optional(record.notes, BODY),
    tokens,
  };
}

export function serializeSample(value: Sample): Record<string, unknown> {
  return normalizeSample(value);
}

export function groupSamples<T extends { value: Sample }>(samples: T[]) {
  const grouped = new Map<SampleKind, T[]>();
  for (const item of SAMPLE_KINDS) grouped.set(item.id, []);
  for (const sample of samples) {
    const bucket = grouped.get(sample.value.kind) ?? grouped.get("sentence")!;
    bucket.push(sample);
  }
  return SAMPLE_KINDS.map((item) => ({ ...item, samples: grouped.get(item.id) ?? [] }));
}

export function filterSamples<T extends { value: Sample }>(samples: T[], query: string): T[] {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return samples;
  return samples.filter(({ value }) =>
    [
      value.title,
      value.text,
      value.translation,
      value.transliteration,
      value.notes,
      ...value.tokens.flatMap((token) => [token.text, token.gloss, token.grammar]),
    ]
      .filter((item): item is string => Boolean(item))
      .some((item) => item.toLocaleLowerCase().includes(needle)),
  );
}

function escapeHtml(value: string) {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

export function sampleTokenHtml(token: SampleToken) {
  const surface = token.lexemeId
    ? `<button type="button" class="sample-ref" data-lexeme-id="${escapeHtml(token.lexemeId)}">${escapeHtml(token.text)}</button>`
    : `<span class="surface">${escapeHtml(token.text)}</span>`;
  const gloss = token.gloss ? `<span class="gloss">${escapeHtml(token.gloss)}</span>` : "";
  const grammar = token.grammar ? `<span class="grammar">${escapeHtml(token.grammar)}</span>` : "";
  return `<span class="sample-token">${surface}${gloss}${grammar}</span>`;
}

export function samplePreviewHtml(sample: Sample) {
  const parts: string[] = [];
  if (sample.text.trim())
    parts.push(`<p class="sample-source">${escapeHtml(sample.text).replaceAll("\n", "<br>")}</p>`);
  if (sample.tokens.length) {
    parts.push(`<div class="sample-interlinear">${sample.tokens.map(sampleTokenHtml).join("")}</div>`);
  }
  if (sample.transliteration) {
    parts.push(`<p class="sample-transliteration">${escapeHtml(sample.transliteration).replaceAll("\n", "<br>")}</p>`);
  }
  if (sample.translation) {
    parts.push(`<p class="sample-translation">${escapeHtml(sample.translation).replaceAll("\n", "<br>")}</p>`);
  }
  return parts.join("");
}
