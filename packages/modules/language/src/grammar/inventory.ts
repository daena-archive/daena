import { MAX_CATEGORIES } from "./normalize.ts";
import type {
  CaseConfig,
  CaseItem,
  CaseTemplateId,
  GrammarSystemId,
  GrammarSystemRecord,
  IndexedGrammar,
  MarkingStrategy,
  NounClassItem,
  NounClassKind,
  NounClassesConfig,
  NumberCategory,
  NumberCategoryId,
  NumberConfig,
  TamCategory,
  TamConfig,
} from "./types.ts";

export const INVENTORY_SYSTEM_IDS = [
  "nouns.number",
  "nouns.case",
  "nouns.classes",
  "verbs.tense",
  "verbs.aspect",
  "verbs.mood",
] as const satisfies readonly GrammarSystemId[];

export type InventorySystemId = (typeof INVENTORY_SYSTEM_IDS)[number];

export type InventoryTemplate<T extends string = string> = {
  id: T;
  label: string;
  meaning?: string;
  more?: boolean;
};

export const NUMBER_TEMPLATES: InventoryTemplate<NumberCategoryId>[] = [
  { id: "singular", label: "Singular", meaning: "One" },
  { id: "plural", label: "Plural", meaning: "More than one" },
  { id: "dual", label: "Dual", meaning: "Two" },
  { id: "trial", label: "Trial", meaning: "Three" },
  { id: "paucal", label: "Paucal", meaning: "A few" },
  { id: "collective", label: "Collective", meaning: "A group as a whole" },
  { id: "custom", label: "Custom" },
];

export const NUMBER_MARKING_OPTIONS: InventoryTemplate<MarkingStrategy>[] = [
  { id: "affix", label: "Affix" },
  { id: "separate-word", label: "Separate word" },
  { id: "stem-change", label: "Stem change" },
  { id: "multiple", label: "Multiple strategies" },
  { id: "unmarked", label: "Usually unmarked" },
  { id: "custom", label: "Custom" },
];

export const CASE_TEMPLATES: InventoryTemplate<CaseTemplateId>[] = [
  { id: "nominative", label: "Nominative", meaning: "Subject" },
  { id: "accusative", label: "Accusative", meaning: "Direct object" },
  { id: "ergative", label: "Ergative", meaning: "Agent of a transitive verb" },
  { id: "absolutive", label: "Absolutive", meaning: "Intransitive subject or transitive object" },
  { id: "genitive", label: "Genitive", meaning: "Possession" },
  { id: "dative", label: "Dative", meaning: "Recipient / goal" },
  { id: "instrumental", label: "Instrumental", meaning: "Means or instrument" },
  { id: "locative", label: "Locative", meaning: "Location" },
  { id: "ablative", label: "Ablative", meaning: "Source / motion away" },
  { id: "allative", label: "Allative", meaning: "Goal / motion toward" },
  { id: "vocative", label: "Vocative", meaning: "Addressee" },
  { id: "custom", label: "Custom" },
];

export const NOUN_CLASS_KIND_OPTIONS: InventoryTemplate<NounClassKind>[] = [
  { id: "gender", label: "Gender system", meaning: "A small set of classes such as masculine and feminine." },
  {
    id: "noun-class",
    label: "Noun class system",
    meaning: "A larger set of grammatical classes, not necessarily gendered.",
  },
  { id: "custom", label: "Custom classification" },
];

export const TENSE_TEMPLATES: InventoryTemplate[] = [
  { id: "past", label: "Past", meaning: "Before now" },
  { id: "present", label: "Present", meaning: "Now" },
  { id: "future", label: "Future", meaning: "After now" },
  { id: "recent-past", label: "Recent past", meaning: "A short time before now" },
  { id: "remote-past", label: "Remote past", meaning: "A long time before now" },
  { id: "near-future", label: "Near future", meaning: "A short time after now" },
  { id: "remote-future", label: "Remote future", meaning: "A long time after now" },
  { id: "custom", label: "Custom" },
];

export const ASPECT_TEMPLATES: InventoryTemplate[] = [
  { id: "perfective", label: "Perfective", meaning: "Presents an event as a bounded whole." },
  {
    id: "imperfective",
    label: "Imperfective",
    meaning: "Presents an event as ongoing, habitual, or internally structured.",
  },
  { id: "progressive", label: "Progressive", meaning: "An event in progress." },
  { id: "habitual", label: "Habitual", meaning: "A repeated or usual event." },
  { id: "perfect", label: "Perfect", meaning: "A past event with present relevance." },
  { id: "prospective", label: "Prospective", meaning: "An event viewed as upcoming." },
  { id: "iterative", label: "Iterative", meaning: "An event repeated in a series." },
  { id: "custom", label: "Custom" },
];

export const MOOD_TEMPLATES: InventoryTemplate[] = [
  { id: "indicative", label: "Indicative", meaning: "Ordinary statements." },
  { id: "imperative", label: "Imperative", meaning: "Commands." },
  { id: "subjunctive", label: "Subjunctive", meaning: "Hypothetical or non-factual clauses." },
  { id: "conditional", label: "Conditional", meaning: "Conditions and their outcomes." },
  { id: "optative", label: "Optative", meaning: "Wishes.", more: true },
  { id: "potential", label: "Potential", meaning: "Possibility.", more: true },
  { id: "irrealis", label: "Irrealis", meaning: "Events not asserted as real.", more: true },
  { id: "jussive", label: "Jussive", meaning: "Indirect commands or exhortations.", more: true },
  { id: "custom", label: "Custom" },
];

export type InventoryMutation = {
  draft: GrammarSystemRecord;
  blocked?: { id: string; label: string };
};

export function isInventorySystem(systemId: GrammarSystemId): systemId is InventorySystemId {
  return (INVENTORY_SYSTEM_IDS as readonly string[]).includes(systemId);
}

export function referencedCategoryIds(index: IndexedGrammar, systemId: GrammarSystemId): Set<string> {
  const ids = new Set<string>();
  for (const record of index.agreements) {
    if (record.value.recordKind !== "agreement") continue;
    for (const feature of record.value.features) {
      if (feature.sourceSystemId === systemId && feature.categoryId) ids.add(feature.categoryId);
    }
  }
  return ids;
}

export function toggleNumberTemplate(
  draft: GrammarSystemRecord,
  templateId: NumberCategoryId,
  options?: { referenced?: Set<string> },
): InventoryMutation {
  if (draft.systemId !== "nouns.number") return { draft };
  const config = numberConfig(draft);
  if (templateId === "custom") {
    return {
      draft: setNumber(draft, { ...config, categories: append(config.categories, numberFromTemplate("custom")) }),
    };
  }
  const existing = config.categories.find((item) => item.templateId === templateId);
  if (existing) return removeById(draft, existing.id, options);
  return {
    draft: setNumber(draft, { ...config, categories: append(config.categories, numberFromTemplate(templateId)) }),
  };
}

export function updateNumberCategory(
  draft: GrammarSystemRecord,
  id: string,
  patch: Partial<Omit<NumberCategory, "id">>,
): GrammarSystemRecord {
  if (draft.systemId !== "nouns.number") return draft;
  const config = numberConfig(draft);
  return setNumber(draft, {
    ...config,
    categories: config.categories.map((item) => (item.id === id ? { ...item, ...patch, id: item.id } : item)),
  });
}

export function moveNumberCategory(draft: GrammarSystemRecord, id: string, delta: number): GrammarSystemRecord {
  if (draft.systemId !== "nouns.number") return draft;
  const config = numberConfig(draft);
  return setNumber(draft, { ...config, categories: move(config.categories, id, delta) });
}

export function removeNumberCategory(
  draft: GrammarSystemRecord,
  id: string,
  options?: { force?: boolean; referenced?: Set<string> },
) {
  return removeById(draft, id, options);
}

export function toggleNumberMarking(draft: GrammarSystemRecord, strategy: MarkingStrategy): GrammarSystemRecord {
  if (draft.systemId !== "nouns.number") return draft;
  const config = numberConfig(draft);
  const markingStrategies = config.markingStrategies.includes(strategy)
    ? config.markingStrategies.filter((item) => item !== strategy)
    : [...config.markingStrategies, strategy];
  return setNumber(draft, { ...config, markingStrategies });
}

export function addCase(draft: GrammarSystemRecord, templateId: CaseTemplateId): GrammarSystemRecord {
  if (draft.systemId !== "nouns.case") return draft;
  const config = caseConfig(draft);
  return setCase(draft, { cases: append(config.cases, caseFromTemplate(templateId)) });
}

export function updateCase(
  draft: GrammarSystemRecord,
  id: string,
  patch: Partial<Omit<CaseItem, "id">>,
): GrammarSystemRecord {
  if (draft.systemId !== "nouns.case") return draft;
  const config = caseConfig(draft);
  return setCase(draft, {
    cases: config.cases.map((item) => (item.id === id ? { ...item, ...patch, id: item.id } : item)),
  });
}

export function moveCase(draft: GrammarSystemRecord, id: string, delta: number): GrammarSystemRecord {
  if (draft.systemId !== "nouns.case") return draft;
  const config = caseConfig(draft);
  return setCase(draft, { cases: move(config.cases, id, delta) });
}

export function removeCase(
  draft: GrammarSystemRecord,
  id: string,
  options?: { force?: boolean; referenced?: Set<string> },
) {
  return removeById(draft, id, options);
}

export function setNounClassKind(draft: GrammarSystemRecord, kind: NounClassKind): GrammarSystemRecord {
  if (draft.systemId !== "nouns.classes") return draft;
  const config = classConfig(draft);
  return setClasses(draft, { kind, classes: config.classes });
}

export function addNounClass(draft: GrammarSystemRecord, name = "Class"): GrammarSystemRecord {
  if (draft.systemId !== "nouns.classes") return draft;
  const config = classConfig(draft);
  return setClasses(draft, {
    kind: config.kind ?? "gender",
    classes: append(config.classes, { id: newId(), name }),
  });
}

export function updateNounClass(
  draft: GrammarSystemRecord,
  id: string,
  patch: Partial<Omit<NounClassItem, "id">>,
): GrammarSystemRecord {
  if (draft.systemId !== "nouns.classes") return draft;
  const config = classConfig(draft);
  return setClasses(draft, {
    kind: config.kind ?? "gender",
    classes: config.classes.map((item) => (item.id === id ? { ...item, ...patch, id: item.id } : item)),
  });
}

export function moveNounClass(draft: GrammarSystemRecord, id: string, delta: number): GrammarSystemRecord {
  if (draft.systemId !== "nouns.classes") return draft;
  const config = classConfig(draft);
  return setClasses(draft, { kind: config.kind ?? "gender", classes: move(config.classes, id, delta) });
}

export function removeNounClass(
  draft: GrammarSystemRecord,
  id: string,
  options?: { force?: boolean; referenced?: Set<string> },
) {
  return removeById(draft, id, options);
}

export function toggleTamTemplate(
  draft: GrammarSystemRecord,
  templateId: string,
  options?: { referenced?: Set<string> },
): InventoryMutation {
  if (!isTam(draft.systemId)) return { draft };
  const templates = tamTemplates(draft.systemId);
  const config = tamConfig(draft);
  if (templateId === "custom") {
    return { draft: setTam(draft, { categories: append(config.categories, tamFromTemplate(templates, "custom")) }) };
  }
  const existing = config.categories.find((item) => item.templateId === templateId);
  if (existing) return removeById(draft, existing.id, options);
  const next = tamFromTemplate(templates, templateId);
  if (!next.label) return { draft };
  return { draft: setTam(draft, { categories: append(config.categories, next) }) };
}

export function updateTamCategory(
  draft: GrammarSystemRecord,
  id: string,
  patch: Partial<Omit<TamCategory, "id">>,
): GrammarSystemRecord {
  if (!isTam(draft.systemId)) return draft;
  const config = tamConfig(draft);
  return setTam(draft, {
    categories: config.categories.map((item) => (item.id === id ? { ...item, ...patch, id: item.id } : item)),
  });
}

export function moveTamCategory(draft: GrammarSystemRecord, id: string, delta: number): GrammarSystemRecord {
  if (!isTam(draft.systemId)) return draft;
  const config = tamConfig(draft);
  return setTam(draft, { categories: move(config.categories, id, delta) });
}

export function removeTamCategory(
  draft: GrammarSystemRecord,
  id: string,
  options?: { force?: boolean; referenced?: Set<string> },
) {
  return removeById(draft, id, options);
}

export function removeById(
  draft: GrammarSystemRecord,
  id: string,
  options?: { force?: boolean; referenced?: Set<string> },
): InventoryMutation {
  const referenced = options?.referenced ?? new Set<string>();
  if (referenced.has(id) && !options?.force) {
    return { draft, blocked: { id, label: itemLabel(draft, id) } };
  }
  if (draft.systemId === "nouns.number") {
    const config = numberConfig(draft);
    return { draft: setNumber(draft, { ...config, categories: config.categories.filter((item) => item.id !== id) }) };
  }
  if (draft.systemId === "nouns.case") {
    const config = caseConfig(draft);
    return { draft: setCase(draft, { cases: config.cases.filter((item) => item.id !== id) }) };
  }
  if (draft.systemId === "nouns.classes") {
    const config = classConfig(draft);
    return {
      draft: setClasses(draft, {
        kind: config.kind ?? "gender",
        classes: config.classes.filter((item) => item.id !== id),
      }),
    };
  }
  if (isTam(draft.systemId)) {
    const config = tamConfig(draft);
    return { draft: setTam(draft, { categories: config.categories.filter((item) => item.id !== id) }) };
  }
  return { draft };
}

function itemLabel(draft: GrammarSystemRecord, id: string) {
  if (draft.systemId === "nouns.number")
    return numberConfig(draft).categories.find((item) => item.id === id)?.label ?? "category";
  if (draft.systemId === "nouns.case") return caseConfig(draft).cases.find((item) => item.id === id)?.name ?? "case";
  if (draft.systemId === "nouns.classes")
    return classConfig(draft).classes.find((item) => item.id === id)?.name ?? "class";
  if (isTam(draft.systemId)) return tamConfig(draft).categories.find((item) => item.id === id)?.label ?? "category";
  return "category";
}

function numberFromTemplate(templateId: NumberCategoryId): NumberCategory {
  const template = NUMBER_TEMPLATES.find((item) => item.id === templateId)!;
  return { id: newId(), templateId, label: template.label, meaning: template.meaning };
}

function caseFromTemplate(templateId: CaseTemplateId): CaseItem {
  const template = CASE_TEMPLATES.find((item) => item.id === templateId)!;
  return {
    id: newId(),
    templateId,
    name: template.label,
    abbreviation: templateId === "custom" ? undefined : template.label.slice(0, 3).toUpperCase(),
    primaryFunction: template.meaning ?? "",
  };
}

function tamFromTemplate(templates: InventoryTemplate[], templateId: string): TamCategory {
  const template = templates.find((item) => item.id === templateId) ?? { id: "custom", label: "Custom" };
  return { id: newId(), templateId: template.id, label: template.label, meaning: template.meaning };
}

function numberConfig(draft: GrammarSystemRecord): NumberConfig {
  return "categories" in draft.config && Array.isArray((draft.config as NumberConfig).categories)
    ? (draft.config as NumberConfig)
    : { categories: [], markingStrategies: [] };
}

function caseConfig(draft: GrammarSystemRecord): CaseConfig {
  return "cases" in draft.config && Array.isArray((draft.config as CaseConfig).cases)
    ? (draft.config as CaseConfig)
    : { cases: [] };
}

function classConfig(draft: GrammarSystemRecord): { kind?: NounClassKind; classes: NounClassItem[] } {
  if ("kind" in draft.config || "classes" in draft.config) {
    const config = draft.config as NounClassesConfig;
    return { kind: config.kind, classes: config.classes ?? [] };
  }
  return { classes: [] };
}

function tamConfig(draft: GrammarSystemRecord): TamConfig {
  return "categories" in draft.config && Array.isArray((draft.config as TamConfig).categories)
    ? (draft.config as TamConfig)
    : { categories: [] };
}

function setNumber(draft: GrammarSystemRecord, config: NumberConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}

function setCase(draft: GrammarSystemRecord, config: CaseConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}

function setClasses(draft: GrammarSystemRecord, config: NounClassesConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}

function setTam(draft: GrammarSystemRecord, config: TamConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}

function isTam(systemId: GrammarSystemId) {
  return systemId === "verbs.tense" || systemId === "verbs.aspect" || systemId === "verbs.mood";
}

export function tamTemplates(systemId: GrammarSystemId) {
  if (systemId === "verbs.aspect") return ASPECT_TEMPLATES;
  if (systemId === "verbs.mood") return MOOD_TEMPLATES;
  return TENSE_TEMPLATES;
}

function append<T>(items: T[], item: T): T[] {
  return items.length >= MAX_CATEGORIES ? items : [...items, item];
}

function move<T extends { id: string }>(items: T[], id: string, delta: number): T[] {
  const index = items.findIndex((item) => item.id === id);
  const next = index + delta;
  if (index < 0 || next < 0 || next >= items.length) return items;
  const copy = [...items];
  const [row] = copy.splice(index, 1);
  copy.splice(next, 0, row);
  return copy;
}

function newId() {
  return crypto.randomUUID();
}
