export type GrammarSectionId =
  "word-order" | "noun" | "pronoun" | "verb" | "modifier" | "clause" | "agreement" | "other";

export type GrammarLinkKind = "lexeme" | "example";

export type GrammarLink = {
  id: string;
  kind: GrammarLinkKind;
  lexemeId: string;
  exampleId?: string;
  label?: string;
};

export type GrammarTopic = {
  title: string;
  section: GrammarSectionId;
  body: string;
  links: GrammarLink[];
};

export const GRAMMAR_SECTIONS: { id: GrammarSectionId; label: string }[] = [
  { id: "word-order", label: "Word order" },
  { id: "noun", label: "Nouns" },
  { id: "pronoun", label: "Pronouns" },
  { id: "verb", label: "Verbs" },
  { id: "modifier", label: "Modifiers" },
  { id: "clause", label: "Clauses" },
  { id: "agreement", label: "Agreement" },
  { id: "other", label: "Other" },
];

const TEXT = 500;
const BODY = 24_000;
const MAX_LINKS = 64;
const SECTION_IDS = new Set(GRAMMAR_SECTIONS.map((item) => item.id));

function id() {
  return crypto.randomUUID();
}

function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

function section(value: unknown): GrammarSectionId {
  return SECTION_IDS.has(value as GrammarSectionId) ? (value as GrammarSectionId) : "other";
}

function escapeHtml(value: string) {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

export function emptyGrammarTopic(section: GrammarSectionId = "other"): GrammarTopic {
  return { title: "", section, body: "", links: [] };
}

export function grammarSectionLabel(id: string) {
  return GRAMMAR_SECTIONS.find((item) => item.id === id)?.label ?? "Other";
}

export function normalizeGrammarTopic(value: unknown): GrammarTopic {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const links = Array.isArray(record.links)
    ? record.links
        .map((item) => {
          const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
          const lexemeId = text(entry.lexemeId);
          if (!lexemeId) return null;
          const kind: GrammarLinkKind = entry.kind === "example" ? "example" : "lexeme";
          return {
            id: text(entry.id) || id(),
            kind,
            lexemeId,
            exampleId: kind === "example" ? optional(entry.exampleId) : undefined,
            label: optional(entry.label),
          };
        })
        .filter((item): item is GrammarLink => item !== null)
        .slice(0, MAX_LINKS)
    : [];
  return {
    title: text(record.title),
    section: section(record.section),
    body: typeof record.body === "string" ? record.body.replace(/\r\n?/g, "\n").slice(0, BODY) : "",
    links,
  };
}

export function serializeGrammarTopic(value: GrammarTopic): Record<string, unknown> {
  return normalizeGrammarTopic(value);
}

export function grammarLinkMarkup(link: GrammarLink) {
  const label = link.label || "lexeme";
  return link.kind === "example" && link.exampleId
    ? `[[${label}]](example:${link.lexemeId}:${link.exampleId})`
    : `[[${label}]](lexeme:${link.lexemeId})`;
}

export function groupGrammarTopics<T extends { value: GrammarTopic }>(topics: T[]) {
  const grouped = new Map<GrammarSectionId, T[]>();
  for (const section of GRAMMAR_SECTIONS) grouped.set(section.id, []);
  for (const topic of topics) {
    const bucket = grouped.get(topic.value.section) ?? grouped.get("other")!;
    bucket.push(topic);
  }
  return GRAMMAR_SECTIONS.map((section) => ({ ...section, topics: grouped.get(section.id) ?? [] }));
}

function inlineMarkdown(value: string) {
  const tokens: string[] = [];
  const protect = (html: string) => {
    const index = tokens.push(html) - 1;
    return `\u0000${index}\u0000`;
  };
  let text = value.replace(
    /\[\[([^\]]+)\]\]\(example:([0-9a-f-]{36}):([0-9a-f-]{36})\)/gi,
    (_, label: string, lexemeId: string, exampleId: string) =>
      protect(
        `<button type="button" class="grammar-ref" data-lexeme-id="${escapeHtml(lexemeId)}" data-example-id="${escapeHtml(exampleId)}">${escapeHtml(label)}</button>`,
      ),
  );
  text = text.replace(/\[\[([^\]]+)\]\]\(lexeme:([0-9a-f-]{36})\)/gi, (_, label: string, lexemeId: string) =>
    protect(
      `<button type="button" class="grammar-ref" data-lexeme-id="${escapeHtml(lexemeId)}">${escapeHtml(label)}</button>`,
    ),
  );
  text = text.replace(/`([^`\n]+)`/g, (_, code: string) => protect(`<code>${escapeHtml(code)}</code>`));
  text = escapeHtml(text);
  text = text.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>");
  text = text.replace(/(?<!\w)\*([^*\n]+)\*(?!\w)/g, "<em>$1</em>");
  return text.replace(/\u0000(\d+)\u0000/g, (_, index: string) => tokens[Number(index)] ?? "");
}

export function grammarMarkdownToHtml(markdown: string) {
  const lines = markdown.replace(/\r\n?/g, "\n").split("\n");
  const output: string[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) {
      index += 1;
      continue;
    }
    const heading = line.match(/^\s{0,3}(#{1,3})\s+(.+)$/);
    if (heading) {
      output.push(`<h${heading[1].length}>${inlineMarkdown(heading[2])}</h${heading[1].length}>`);
      index += 1;
      continue;
    }
    const list = line.match(/^\s*[-*]\s+(.+)$/) ?? line.match(/^\s*\d+[.)]\s+(.+)$/);
    if (list) {
      const ordered = /^\s*\d/.test(line);
      const items: string[] = [];
      while (index < lines.length) {
        const next = lines[index].match(ordered ? /^\s*\d+[.)]\s+(.+)$/ : /^\s*[-*]\s+(.+)$/);
        if (!next) break;
        items.push(`<li>${inlineMarkdown(next[1])}</li>`);
        index += 1;
      }
      output.push(ordered ? `<ol>${items.join("")}</ol>` : `<ul>${items.join("")}</ul>`);
      continue;
    }
    const paragraph: string[] = [];
    while (index < lines.length && lines[index].trim() && !/^\s{0,3}#{1,3}\s/.test(lines[index])) {
      if (/^\s*[-*]\s+/.test(lines[index]) || /^\s*\d+[.)]\s+/.test(lines[index])) break;
      paragraph.push(lines[index++]);
    }
    output.push(`<p>${inlineMarkdown(paragraph.join(" "))}</p>`);
  }
  return output.join("");
}
