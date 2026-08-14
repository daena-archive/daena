import { grammarSystemDescriptor } from "./catalog.ts";
import {
  GRAMMAR_SCHEMA_VERSION,
  GRAMMAR_SYSTEM_IDS,
  type AdjectiveBehaviorConfig,
  type AdpositionsConfig,
  type AgreementEndpoint,
  type ArgumentIndexingConfig,
  type BasicWordOrderConfig,
  type CaseConfig,
  type ClauseNegationConfig,
  type ContentQuestionsConfig,
  type DefinitenessConfig,
  type DegreeConfig,
  type DemonstrativeConfig,
  type EmptyConfig,
  type GrammarAgreementRecord,
  type GrammarCustomRuleRecord,
  type GrammarExample,
  type GrammarIssue,
  type GrammarLink,
  type GrammarRecord,
  type GrammarSectionStateRecord,
  type GrammarStatus,
  type GrammarSystemConfig,
  type GrammarSystemId,
  type GrammarSystemRecord,
  type ImperativesConfig,
  type IndexedGrammar,
  type LoadedGrammarRecord,
  type NegativeVerbConfig,
  type NormalizeResult,
  type NounClassesConfig,
  type NumberConfig,
  type ParadigmAxis,
  type ParadigmCell,
  type ParadigmConfig,
  type PositionConfig,
  type PossessivePositionConfig,
  type PossessionConfig,
  type RelativeClausePositionConfig,
  type RelativeClausesConfig,
  type TamConfig,
  type VerbMarkingConfig,
  type YesNoQuestionsConfig,
} from "./types.ts";

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

const SYSTEM_IDS = new Set<string>(GRAMMAR_SYSTEM_IDS);
const STATUSES = new Set<GrammarStatus>(["unconfigured", "configured", "not-used"]);
const LINK_KINDS = new Set(["lexeme", "lexeme-example", "sample", "paradigm"]);
const CELL_STATES = new Set(["form", "same-as", "zero", "not-applicable"]);

function id() {
  return crypto.randomUUID();
}

export function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

function lines(value: unknown, limit: number) {
  return typeof value === "string" ? value.replace(/\r\n?/g, "\n").trim().slice(0, limit) : "";
}

function obj(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function pick<T extends string>(value: unknown, allowed: readonly T[]): T | undefined {
  const next = text(value);
  return allowed.includes(next as T) ? (next as T) : undefined;
}

function pickList<T extends string>(value: unknown, allowed: readonly T[], max = MAX_STRATEGIES): T[] {
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

function bool(value: unknown) {
  return typeof value === "boolean" ? value : undefined;
}

function compact<T>(value: T): T {
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

function issue(code: GrammarIssue["code"], message: string, path?: string): GrammarIssue {
  return { code, message, path };
}

export function emptyConfig(): EmptyConfig {
  return {};
}

export function emptySystemRecord(systemId: GrammarSystemId, status: GrammarStatus = "unconfigured"): GrammarSystemRecord {
  return {
    recordKind: "system",
    schemaVersion: GRAMMAR_SCHEMA_VERSION,
    systemId,
    status,
    config: emptyConfig(),
    notes: "",
    examples: [],
    links: [],
  };
}

export function emptyCustomRule(): GrammarCustomRuleRecord {
  return {
    recordKind: "custom-rule",
    schemaVersion: GRAMMAR_SCHEMA_VERSION,
    title: "",
    tags: [],
    body: "",
    examples: [],
    links: [],
  };
}

export function emptyAgreementSectionState(note?: string): GrammarSectionStateRecord {
  return {
    recordKind: "section-state",
    schemaVersion: GRAMMAR_SCHEMA_VERSION,
    sectionId: "agreement",
    status: "not-used",
    note,
  };
}

export function cloneGrammarRecord<T extends GrammarRecord>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function grammarRecordSnapshot(value: GrammarRecord) {
  return JSON.stringify(cloneGrammarRecord(value));
}

function choiceMinimumIssue(value: GrammarSystemRecord): GrammarIssue | undefined {
  const config = value.config;
  if (value.systemId === "syntax.basic-word-order") {
    const order = (config as BasicWordOrderConfig).order;
    if (!order) return issue("configured-minimum", "Choose a word-order pattern.", "order");
    if (order === "custom" && !((config as BasicWordOrderConfig).customOrder ?? "").trim()) {
      return issue("configured-minimum", "Describe the custom word order.", "customOrder");
    }
    return undefined;
  }
  if (
    value.systemId === "syntax.adjective-position" ||
    value.systemId === "syntax.possessive-position" ||
    value.systemId === "syntax.relative-clause-position"
  ) {
    const position = (config as PositionConfig).position;
    if (!position) return issue("configured-minimum", "Choose a position.", "position");
    if (position === "custom" && !((config as PositionConfig).customPosition ?? "").trim()) {
      return issue("configured-minimum", "Describe the custom position.", "customPosition");
    }
    return undefined;
  }
  if (value.systemId === "syntax.adpositions" && !(config as AdpositionsConfig).strategy) {
    return issue("configured-minimum", "Choose how adpositions work.", "strategy");
  }
  if (value.systemId === "nouns.number" && !(config as NumberConfig).categories?.some((item) => item.label?.trim())) {
    return issue("configured-minimum", "Add at least one number category.", "categories");
  }
  if (value.systemId === "nouns.case" && !(config as CaseConfig).cases?.some((item) => item.name?.trim() && item.primaryFunction?.trim())) {
    return issue("configured-minimum", "Each saved case needs a name and primary function.", "cases");
  }
  if (
    value.systemId === "nouns.classes" &&
    (!(config as NounClassesConfig).kind || !(config as NounClassesConfig).classes?.some((item) => item.name?.trim()))
  ) {
    return issue("configured-minimum", "Choose a classification kind and add at least one class.", "kind");
  }
  if (
    (value.systemId === "verbs.tense" || value.systemId === "verbs.aspect" || value.systemId === "verbs.mood") &&
    !(config as TamConfig).categories?.some((item) => item.label?.trim())
  ) {
    return issue("configured-minimum", "Add at least one category.", "categories");
  }
  return undefined;
}

export function validateGrammarDraft(value: GrammarRecord): GrammarIssue[] {
  if (value.recordKind === "system") {
    if (value.status !== "configured") return [];
    const missing = choiceMinimumIssue(value);
    if (missing) return [missing];
    if (!configuredMinimum(value.systemId, value.config)) {
      return [issue("configured-minimum", "This system needs its grammatical settings before it can be saved as configured.", "status")];
    }
    return [];
  }
  if (value.recordKind === "custom-rule" && !value.title.trim()) {
    return [issue("malformed", "Title is required.", "title")];
  }
  if (value.recordKind === "agreement" && !value.title.trim()) {
    return [issue("malformed", "Title is required.", "title")];
  }
  return [];
}

function normalizeExamples(value: unknown): GrammarExample[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => {
      const entry = obj(item);
      const exampleText = text(entry.text);
      if (!exampleText) return null;
      return {
        id: text(entry.id) || id(),
        text: exampleText,
        translation: optional(entry.translation),
        gloss: optional(entry.gloss, NOTES),
        notes: optional(entry.notes, NOTES),
      };
    })
    .filter((item): item is GrammarExample => item !== null)
    .slice(0, MAX_EXAMPLES);
}

function normalizeLinks(value: unknown): GrammarLink[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => {
      const entry = obj(item);
      const targetId = text(entry.targetId);
      const kind = text(entry.kind);
      if (!targetId || !LINK_KINDS.has(kind)) return null;
      return {
        id: text(entry.id) || id(),
        kind: kind as GrammarLink["kind"],
        targetId,
        secondaryId: kind === "lexeme-example" ? optional(entry.secondaryId) : optional(entry.secondaryId),
        label: optional(entry.label),
      };
    })
    .filter((item): item is GrammarLink => item !== null)
    .slice(0, MAX_LINKS);
}

function normalizeAxes(value: unknown): ParadigmAxis[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => {
      const entry = obj(item);
      const label = text(entry.label);
      const values = Array.isArray(entry.values)
        ? entry.values
            .map((raw) => {
              const valueEntry = obj(raw);
              const valueLabel = text(valueEntry.label);
              if (!valueLabel) return null;
              return {
                id: text(valueEntry.id) || id(),
                label: valueLabel,
                description: optional(valueEntry.description),
              };
            })
            .filter((item): item is ParadigmAxis["values"][number] => item !== null)
            .slice(0, MAX_AXIS_VALUES)
        : [];
      if (!label || values.length === 0) return null;
      return { id: text(entry.id) || id(), label, values };
    })
    .filter((item): item is ParadigmAxis => item !== null)
    .slice(0, MAX_AXES);
}

function normalizeCells(value: unknown, axes: ParadigmAxis[], examples: GrammarExample[]): ParadigmCell[] {
  const axisIds = new Set(axes.map((axis) => axis.id));
  const valueIds = new Map(axes.map((axis) => [axis.id, new Set(axis.values.map((item) => item.id))]));
  const exampleIds = new Set(examples.map((item) => item.id));
  if (!Array.isArray(value)) return [];
  const cells = value
    .map((item) => {
      const entry = obj(item);
      const coordinates = obj(entry.coordinates);
      const next: Record<string, string> = {};
      for (const [axisId, raw] of Object.entries(coordinates)) {
        const valueId = text(raw);
        if (!axisIds.has(axisId) || !valueIds.get(axisId)?.has(valueId)) continue;
        next[axisId] = valueId;
      }
      if (Object.keys(next).length !== axes.length) return null;
      const state = CELL_STATES.has(text(entry.state)) ? (text(entry.state) as ParadigmCell["state"]) : "form";
      const exampleId = optional(entry.exampleId);
      return {
        id: text(entry.id) || id(),
        coordinates: next,
        state,
        form: state === "form" ? optional(entry.form, CELL_FORM) : undefined,
        alternateForms: Array.isArray(entry.alternateForms)
          ? entry.alternateForms.map((item) => text(item, CELL_FORM)).filter(Boolean).slice(0, MAX_ALTERNATES)
          : undefined,
        sameAsCellId: state === "same-as" ? optional(entry.sameAsCellId) : undefined,
        notes: optional(entry.notes, NOTES),
        exampleId: exampleId && exampleIds.has(exampleId) ? exampleId : undefined,
      };
    })
    .filter((item): item is ParadigmCell => item !== null)
    .slice(0, MAX_CELLS);
  const ids = new Set(cells.map((cell) => cell.id));
  return cells.map((cell) =>
    cell.sameAsCellId && !ids.has(cell.sameAsCellId) ? { ...cell, sameAsCellId: undefined } : cell,
  );
}

function paradigm(raw: Record<string, unknown>, examples: GrammarExample[]): ParadigmConfig {
  const axes = normalizeAxes(raw.axes);
  return { axes, cells: normalizeCells(raw.cells, axes, examples) };
}

const WORD_ORDERS = ["sov", "svo", "vso", "vos", "ovs", "osv", "flexible", "custom"] as const;
const STRENGTHS = ["strict", "strongly-preferred", "default-flexible", "context"] as const;
const INFLUENCES = ["topic", "focus", "emphasis", "definiteness", "animacy", "discourse", "custom"] as const;
const POSITIONS = ["before", "after", "either", "meaning-changes", "custom"] as const;
const POSS_POS = ["possessor-before", "possessor-after", "either", "morphological", "multiple", "custom"] as const;
const REL_POS = ["before", "after", "internally-headed", "multiple", "custom"] as const;
const ADPOSITIONS = ["prepositions", "postpositions", "both", "other"] as const;
const NUMBER_TEMPLATES = ["singular", "plural", "dual", "trial", "paucal", "collective", "custom"] as const;
const MARKING = ["affix", "separate-word", "stem-change", "multiple", "unmarked", "custom"] as const;
const CASE_TEMPLATES = [
  "nominative",
  "accusative",
  "ergative",
  "absolutive",
  "genitive",
  "dative",
  "instrumental",
  "locative",
  "ablative",
  "allative",
  "vocative",
  "custom",
] as const;
const CLASS_KINDS = ["gender", "noun-class", "custom"] as const;
const DEF_STRATEGIES = [
  "definite-article",
  "indefinite-article",
  "both",
  "affixes",
  "demonstratives",
  "context",
  "other",
] as const;
const POSS_STRATEGIES = [
  "possessive-pronouns",
  "genitive",
  "possessor-marking",
  "possessed-marking",
  "linking-particle",
  "word-order",
  "multiple",
] as const;
const VERB_MARKING = [
  "invariant",
  "prefixes",
  "suffixes",
  "other-affixes",
  "stem-changes",
  "auxiliaries",
  "particles",
  "multiple",
  "custom",
] as const;
const PARTICIPANTS = ["none", "subject", "object", "subject-object", "other"] as const;
const REPRESENTATION = ["endings", "prefixes", "full-forms", "auxiliaries", "flexible-table", "custom"] as const;
const NEG_VERB = ["affix", "negative-auxiliary", "special-verb", "stem-change", "none", "multiple", "custom"] as const;
const ADJ_BEHAVIOR = ["invariant", "agree-with-noun", "verb-like", "noun-like", "multiple-classes", "custom"] as const;
const COMPARATIVE = ["synthetic", "particle", "affix", "exceed", "special", "multiple", "custom"] as const;
const SUPERLATIVE = ["dedicated", "intensifier", "comparative", "definite", "none", "custom"] as const;
const YES_NO = ["intonation", "particle", "word-order", "verb-morphology", "auxiliary", "multiple", "custom"] as const;
const PLACEMENT = ["clause-initial", "clause-final", "before-verb", "after-verb", "other"] as const;
const CONTENT_Q = ["in-situ", "fronted", "fixed-position", "special-structure", "mixed", "custom"] as const;
const IMPERATIVE = ["bare-verb", "special-form", "particle", "auxiliary", "word-order", "multiple", "custom"] as const;
const CLAUSE_NEG = ["particle", "affix", "auxiliary", "special-verb", "multiple", "custom"] as const;
const REL_STRAT = [
  "relative-pronoun",
  "complementizer",
  "gap",
  "resumptive",
  "internally-headed",
  "multiple",
  "custom",
] as const;
const CONTROLLERS = ["subject", "object", "noun", "possessor", "custom"] as const;
const TARGETS = ["verb", "adjective", "article", "pronoun", "participle", "custom"] as const;
const BEHAVIORS = ["full", "partial", "conditional"] as const;

function inventoryItems(value: unknown, fields: { meaning?: boolean; marker?: boolean; extra?: boolean }) {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => {
      const entry = obj(item);
      const label = text(entry.label) || text(entry.name);
      if (!label) return null;
      return {
        id: text(entry.id) || id(),
        templateId: optional(entry.templateId),
        label,
        name: label,
        abbreviation: optional(entry.abbreviation, 24),
        meaning: fields.meaning ? optional(entry.meaning, NOTES) : undefined,
        primaryFunction: optional(entry.primaryFunction, NOTES),
        additionalFunctions: optional(entry.additionalFunctions, NOTES),
        marker: fields.marker ? optional(entry.marker) : undefined,
        marking: optional(entry.marking),
        position: optional(entry.position),
        interaction: optional(entry.interaction, NOTES),
        membership: optional(entry.membership, NOTES),
        exceptions: optional(entry.exceptions, NOTES),
        notes: optional(entry.notes, NOTES),
      };
    })
    .filter((item) => item !== null)
    .slice(0, MAX_CATEGORIES);
}

function normalizeSystemConfig(
  systemId: GrammarSystemId,
  raw: Record<string, unknown>,
  examples: GrammarExample[],
): GrammarSystemConfig {
  switch (systemId) {
    case "syntax.basic-word-order": {
      const order = pick(raw.order, WORD_ORDERS);
      if (!order) return emptyConfig();
      const config: BasicWordOrderConfig = {
        order,
        customOrder: order === "custom" ? optional(raw.customOrder) : undefined,
        strength: pick(raw.strength, STRENGTHS),
        influences: pickList(raw.influences, INFLUENCES),
        customInfluence: optional(raw.customInfluence),
        changeNotes: optional(raw.changeNotes, NOTES),
      };
      return config;
    }
    case "syntax.adjective-position": {
      const position = pick(raw.position, POSITIONS);
      if (!position) return emptyConfig();
      const config: PositionConfig = {
        position,
        customPosition: position === "custom" ? optional(raw.customPosition) : undefined,
        alternatePositions: pickList(raw.alternatePositions, POSITIONS, MAX_ALTERNATES),
        conditions: optional(raw.conditions, NOTES),
      };
      return config;
    }
    case "syntax.possessive-position": {
      const position = pick(raw.position, POSS_POS);
      if (!position) return emptyConfig();
      const config: PossessivePositionConfig = {
        position,
        customPosition: position === "custom" ? optional(raw.customPosition) : undefined,
        alternatePositions: pickList(raw.alternatePositions, POSS_POS, MAX_ALTERNATES),
        conditions: optional(raw.conditions, NOTES),
      };
      return config;
    }
    case "syntax.relative-clause-position": {
      const position = pick(raw.position, REL_POS);
      if (!position) return emptyConfig();
      const config: RelativeClausePositionConfig = {
        position,
        customPosition: position === "custom" ? optional(raw.customPosition) : undefined,
        alternatePositions: pickList(raw.alternatePositions, REL_POS, MAX_ALTERNATES),
        conditions: optional(raw.conditions, NOTES),
      };
      return config;
    }
    case "syntax.adpositions": {
      const strategy = pick(raw.strategy, ADPOSITIONS);
      if (!strategy) return emptyConfig();
      const config: AdpositionsConfig = { strategy, distributionNotes: optional(raw.distributionNotes, NOTES) };
      return config;
    }
    case "nouns.number": {
      const categories = inventoryItems(raw.categories, { meaning: true, marker: true }).map((item) => ({
        id: item.id,
        templateId: pick(item.templateId, NUMBER_TEMPLATES),
        label: item.label,
        meaning: item.meaning,
        marker: item.marker,
        position: item.position,
        notes: item.notes,
      }));
      const config: NumberConfig = { categories, markingStrategies: pickList(raw.markingStrategies, MARKING) };
      return config;
    }
    case "nouns.case": {
      const cases = inventoryItems(raw.cases, { meaning: true }).map((item) => ({
        id: item.id,
        templateId: pick(item.templateId, CASE_TEMPLATES),
        name: item.name,
        abbreviation: item.abbreviation,
        primaryFunction: item.primaryFunction || item.meaning || "",
        additionalFunctions: item.additionalFunctions,
        marking: item.marking,
        notes: item.notes,
      }));
      const config: CaseConfig = { cases };
      return config;
    }
    case "nouns.classes": {
      const kind = pick(raw.kind, CLASS_KINDS);
      if (!kind) return emptyConfig();
      const config: NounClassesConfig = {
        kind,
        classes: inventoryItems(raw.classes, {}).map((item) => ({
          id: item.id,
          name: item.name,
          abbreviation: item.abbreviation,
          membership: item.membership,
          exceptions: item.exceptions,
        })),
      };
      return config;
    }
    case "nouns.definiteness": {
      const config: DefinitenessConfig = {
        strategies: pickList(raw.strategies, DEF_STRATEGIES),
        articles: Array.isArray(raw.articles)
          ? raw.articles
              .map((item) => {
                const entry = obj(item);
                const form = text(entry.form);
                if (!form) return null;
                return {
                  id: text(entry.id) || id(),
                  form,
                  position: optional(entry.position),
                  notes: optional(entry.notes, NOTES),
                };
              })
              .filter((item): item is DefinitenessConfig["articles"][number] => item !== null)
              .slice(0, MAX_ARTICLES)
          : [],
      };
      return config;
    }
    case "nouns.possession": {
      const config: PossessionConfig = {
        strategies: pickList(raw.strategies, POSS_STRATEGIES),
        alienability: bool(raw.alienability),
        alienabilityNotes: optional(raw.alienabilityNotes, NOTES),
      };
      return config;
    }
    case "pronouns.personal":
      return paradigm(raw, examples);
    case "pronouns.demonstratives": {
      const base = paradigm(raw, examples);
      const config: DemonstrativeConfig = {
        ...base,
        distances: Array.isArray(raw.distances)
          ? raw.distances.map((item) => text(item)).filter(Boolean).slice(0, MAX_AXIS_VALUES)
          : [],
      };
      return config;
    }
    case "verbs.marking-strategy": {
      const config: VerbMarkingConfig = {
        strategies: pickList(raw.strategies, VERB_MARKING),
        customStrategy: optional(raw.customStrategy),
      };
      return config;
    }
    case "verbs.tense":
    case "verbs.aspect":
    case "verbs.mood": {
      const config: TamConfig = {
        categories: inventoryItems(raw.categories, { meaning: true, marker: true }).map((item) => ({
          id: item.id,
          templateId: item.templateId,
          label: item.label,
          meaning: item.meaning,
          marker: item.marker,
          interaction: item.interaction,
          notes: item.notes,
        })),
      };
      return config;
    }
    case "verbs.argument-indexing": {
      const participants = pick(raw.participants, PARTICIPANTS);
      if (!participants) return emptyConfig();
      const base = paradigm(raw, examples);
      const config: ArgumentIndexingConfig = {
        participants,
        representation: pick(raw.representation, REPRESENTATION),
        axes: base.axes,
        cells: base.cells,
        flexibleNotes: optional(raw.flexibleNotes, NOTES),
        agreementRecordId: optional(raw.agreementRecordId),
      };
      return config;
    }
    case "verbs.negative-forms": {
      const config: NegativeVerbConfig = {
        strategies: pickList(raw.strategies, NEG_VERB),
        forms: Array.isArray(raw.forms)
          ? raw.forms
              .map((item) => {
                const entry = obj(item);
                const form = text(entry.form);
                if (!form) return null;
                return {
                  id: text(entry.id) || id(),
                  form,
                  conditions: optional(entry.conditions, NOTES),
                  notes: optional(entry.notes, NOTES),
                };
              })
              .filter((item): item is NegativeVerbConfig["forms"][number] => item !== null)
              .slice(0, MAX_CATEGORIES)
          : [],
      };
      return config;
    }
    case "modifiers.adjective-behavior": {
      const config: AdjectiveBehaviorConfig = {
        behaviors: pickList(raw.behaviors, ADJ_BEHAVIOR),
        customBehavior: optional(raw.customBehavior),
        agreementRecordIds: Array.isArray(raw.agreementRecordIds)
          ? raw.agreementRecordIds.map((item) => text(item)).filter(Boolean).slice(0, MAX_FEATURES)
          : [],
      };
      return config;
    }
    case "modifiers.comparative":
    case "modifiers.superlative": {
      const allowed = systemId === "modifiers.comparative" ? COMPARATIVE : SUPERLATIVE;
      const config: DegreeConfig = {
        strategies: pickList(raw.strategies, allowed),
        marker: optional(raw.marker),
        construction: optional(raw.construction, NOTES),
      };
      return config;
    }
    case "clauses.yes-no-questions": {
      const config: YesNoQuestionsConfig = {
        strategies: pickList(raw.strategies, YES_NO),
        particle: optional(raw.particle),
        placement: pick(raw.placement, PLACEMENT),
      };
      return config;
    }
    case "clauses.content-questions": {
      const behavior = pick(raw.behavior, CONTENT_Q);
      if (!behavior) return emptyConfig();
      const config: ContentQuestionsConfig = {
        behavior,
        customBehavior: behavior === "custom" ? optional(raw.customBehavior) : undefined,
        interrogatives: Array.isArray(raw.interrogatives)
          ? raw.interrogatives
              .map((item) => {
                const entry = obj(item);
                const meaning = text(entry.meaning);
                if (!meaning) return null;
                return {
                  id: text(entry.id) || id(),
                  meaning,
                  form: optional(entry.form),
                  lexemeId: optional(entry.lexemeId),
                };
              })
              .filter((item): item is ContentQuestionsConfig["interrogatives"][number] => item !== null)
              .slice(0, MAX_CATEGORIES)
          : [],
      };
      return config;
    }
    case "clauses.imperatives": {
      const config: ImperativesConfig = {
        strategies: pickList(raw.strategies, IMPERATIVE),
        numberDistinction: bool(raw.numberDistinction),
        polarityDistinction: bool(raw.polarityDistinction),
        politenessDistinction: bool(raw.politenessDistinction),
      };
      return config;
    }
    case "clauses.negation": {
      const config: ClauseNegationConfig = {
        strategies: pickList(raw.strategies, CLAUSE_NEG),
        particle: optional(raw.particle),
        placement: pick(raw.placement, PLACEMENT),
        negativeQuestions: optional(raw.negativeQuestions, NOTES),
        negativeImperatives: optional(raw.negativeImperatives, NOTES),
      };
      return config;
    }
    case "clauses.relative-clauses": {
      const config: RelativeClausesConfig = {
        strategies: pickList(raw.strategies, REL_STRAT),
        headBehavior: optional(raw.headBehavior, NOTES),
        resumptives: optional(raw.resumptives, NOTES),
      };
      return config;
    }
  }
}

export function configuredMinimum(systemId: GrammarSystemId, config: GrammarSystemConfig): boolean {
  if (!config || Object.keys(config).length === 0) return false;
  switch (systemId) {
    case "syntax.basic-word-order": {
      const value = config as BasicWordOrderConfig;
      if (!value.order) return false;
      return value.order !== "custom" || Boolean(value.customOrder?.trim());
    }
    case "syntax.adjective-position":
    case "syntax.possessive-position":
    case "syntax.relative-clause-position": {
      const value = config as PositionConfig;
      if (!value.position) return false;
      return value.position !== "custom" || Boolean(value.customPosition?.trim());
    }
    case "syntax.adpositions":
      return "strategy" in config && Boolean((config as AdpositionsConfig).strategy);
    case "nouns.number":
      return (config as NumberConfig).categories?.some((item) => item.label?.trim()) === true;
    case "nouns.case":
      return (config as CaseConfig).cases?.some((item) => item.name?.trim() && item.primaryFunction?.trim()) === true;
    case "nouns.classes":
      return Boolean((config as NounClassesConfig).kind) && (config as NounClassesConfig).classes?.some((item) => item.name?.trim()) === true;
    case "nouns.definiteness":
      return (config as DefinitenessConfig).strategies?.length > 0;
    case "nouns.possession":
      return (config as PossessionConfig).strategies?.length > 0;
    case "pronouns.personal":
      return (config as ParadigmConfig).axes?.length > 0;
    case "pronouns.demonstratives":
      return (config as DemonstrativeConfig).distances?.length > 0 || (config as DemonstrativeConfig).axes?.length > 0;
    case "verbs.marking-strategy":
      return (config as VerbMarkingConfig).strategies?.length > 0;
    case "verbs.tense":
    case "verbs.aspect":
    case "verbs.mood":
      return (config as TamConfig).categories?.some((item) => item.label?.trim()) === true;
    case "verbs.argument-indexing":
      return Boolean((config as ArgumentIndexingConfig).participants);
    case "verbs.negative-forms":
      return (config as NegativeVerbConfig).strategies?.length > 0;
    case "modifiers.adjective-behavior":
      return (config as AdjectiveBehaviorConfig).behaviors?.length > 0;
    case "modifiers.comparative":
    case "modifiers.superlative":
      return (config as DegreeConfig).strategies?.length > 0;
    case "clauses.yes-no-questions":
      return (config as YesNoQuestionsConfig).strategies?.length > 0;
    case "clauses.content-questions":
      return Boolean((config as ContentQuestionsConfig).behavior);
    case "clauses.imperatives":
      return (config as ImperativesConfig).strategies?.length > 0;
    case "clauses.negation":
      return (config as ClauseNegationConfig).strategies?.length > 0;
    case "clauses.relative-clauses":
      return (config as RelativeClausesConfig).strategies?.length > 0;
  }
}

function commonFields(record: Record<string, unknown>) {
  return {
    notes: lines(record.notes, NOTES),
    examples: normalizeExamples(record.examples),
    links: normalizeLinks(record.links),
  };
}

function endpoint(value: unknown, kinds: readonly string[]): AgreementEndpoint | null {
  const entry = obj(value);
  const kind = pick(entry.kind, kinds as readonly AgreementEndpoint["kind"][]);
  if (!kind) return null;
  return { kind, customLabel: kind === "custom" ? optional(entry.customLabel) : undefined };
}

export function normalizeGrammarRecord(value: unknown): NormalizeResult {
  const record = obj(value);
  if ("section" in record && "body" in record && !("recordKind" in record)) {
    return { ok: false, issues: [issue("legacy-topic", "Legacy grammar topics are not accepted.")] };
  }
  const schemaVersion = record.schemaVersion;
  if (schemaVersion !== GRAMMAR_SCHEMA_VERSION) {
    return { ok: false, issues: [issue("invalid-schema-version", "schemaVersion must be 1.")] };
  }
  const kind = text(record.recordKind);
  if (kind === "system") {
    const systemId = text(record.systemId);
    if (!SYSTEM_IDS.has(systemId)) {
      return { ok: false, issues: [issue("unknown-system", `Unknown systemId: ${systemId || "(missing)"}.`)] };
    }
    const status = text(record.status) as GrammarStatus;
    if (!STATUSES.has(status)) {
      return { ok: false, issues: [issue("invalid-status", "status must be unconfigured, configured, or not-used.")] };
    }
    const common = commonFields(record);
    const rawConfig = obj(record.config);
    const issues: GrammarIssue[] = [];
    let config: GrammarSystemConfig = emptyConfig();
    if (status === "not-used" || status === "unconfigured") {
      if (Object.keys(rawConfig).length > 0) issues.push(issue("empty-config-required", "Unconfigured and not-used records keep config empty."));
      config = emptyConfig();
    } else {
      config = normalizeSystemConfig(systemId as GrammarSystemId, rawConfig, common.examples);
      if (!configuredMinimum(systemId as GrammarSystemId, config)) {
        issues.push(issue("configured-minimum", "Configured systems need meaningful data.", "config"));
      }
    }
    const next: GrammarSystemRecord = {
      recordKind: "system",
      schemaVersion: 1,
      systemId: systemId as GrammarSystemId,
      status,
      config,
      ...common,
    };
    return { ok: true, record: compact(next), issues };
  }
  if (kind === "agreement") {
    const controller = endpoint(record.controller, CONTROLLERS);
    const target = endpoint(record.target, TARGETS);
    const title = text(record.title);
    if (!controller || !target || !title) {
      return { ok: false, issues: [issue("malformed", "Agreement records need title, controller, and target.")] };
    }
    const features = Array.isArray(record.features)
      ? record.features
          .map((item) => {
            const entry = obj(item);
            const label = text(entry.label);
            if (!label) return null;
            const sourceSystemId = optional(entry.sourceSystemId);
            return {
              sourceSystemId: sourceSystemId && SYSTEM_IDS.has(sourceSystemId) ? (sourceSystemId as GrammarSystemId) : undefined,
              categoryId: optional(entry.categoryId),
              label,
            };
          })
          .filter((item): item is GrammarAgreementRecord["features"][number] => item !== null)
          .slice(0, MAX_FEATURES)
      : [];
    const next: GrammarAgreementRecord = {
      recordKind: "agreement",
      schemaVersion: 1,
      title,
      controller,
      target,
      features,
      behavior: pick(record.behavior, BEHAVIORS) ?? "full",
      defaultForm: optional(record.defaultForm),
      conditions: optional(record.conditions, NOTES),
      exceptions: optional(record.exceptions, NOTES),
      ...commonFields(record),
    };
    return { ok: true, record: compact(next), issues: [] };
  }
  if (kind === "custom-rule") {
    const title = text(record.title);
    if (!title) return { ok: false, issues: [issue("malformed", "Custom rules need a title.")] };
    const next: GrammarCustomRuleRecord = {
      recordKind: "custom-rule",
      schemaVersion: 1,
      title,
      tags: Array.isArray(record.tags) ? record.tags.map((item) => text(item)).filter(Boolean).slice(0, MAX_TAGS) : [],
      body: lines(record.body, BODY),
      examples: normalizeExamples(record.examples),
      links: normalizeLinks(record.links),
    };
    return { ok: true, record: compact(next), issues: [] };
  }
  if (kind === "section-state") {
    if (text(record.sectionId) !== "agreement" || text(record.status) !== "not-used") {
      return { ok: false, issues: [issue("malformed", "Section state currently supports only agreement not-used.")] };
    }
    const next: GrammarSectionStateRecord = {
      recordKind: "section-state",
      schemaVersion: 1,
      sectionId: "agreement",
      status: "not-used",
      note: optional(record.note, NOTES),
    };
    return { ok: true, record: compact(next), issues: [] };
  }
  if (!kind) return { ok: false, issues: [issue("unknown-kind", "recordKind is required.")] };
  return { ok: false, issues: [issue("unknown-kind", `Unknown recordKind: ${kind}.`)] };
}

export function serializeGrammarRecord(value: GrammarRecord): Record<string, unknown> {
  const result = normalizeGrammarRecord(value);
  if (!result.ok) throw new Error(result.issues.map((item) => item.message).join(" "));
  return JSON.parse(JSON.stringify(result.record)) as Record<string, unknown>;
}

export function indexGrammarRecords(records: { id: string; revision?: string; value: unknown }[]): IndexedGrammar {
  const systems = new Map<GrammarSystemId, LoadedGrammarRecord>();
  const duplicates = new Map<GrammarSystemId, string[]>();
  const seen = new Map<GrammarSystemId, string[]>();
  const agreements: LoadedGrammarRecord[] = [];
  const customRules: LoadedGrammarRecord[] = [];
  const sectionStates = new Map<string, LoadedGrammarRecord>();
  const rejected: IndexedGrammar["rejected"] = [];
  const diagnostics: IndexedGrammar["diagnostics"] = [];

  for (const record of records) {
    const result = normalizeGrammarRecord(record.value);
    if (!result.ok) {
      rejected.push({ id: record.id, issues: result.issues });
      diagnostics.push({ ...result.issues[0], recordIds: [record.id] });
      continue;
    }
    const loaded: LoadedGrammarRecord = {
      id: record.id,
      revision: record.revision ?? "",
      value: result.record,
    };
    if (result.record.recordKind === "system") {
      const ids = seen.get(result.record.systemId) ?? [];
      ids.push(record.id);
      seen.set(result.record.systemId, ids);
    } else if (result.record.recordKind === "agreement") agreements.push(loaded);
    else if (result.record.recordKind === "custom-rule") customRules.push(loaded);
    else sectionStates.set(result.record.sectionId, loaded);
  }

  for (const [systemId, ids] of seen) {
    if (ids.length > 1) {
      duplicates.set(systemId, ids);
      diagnostics.push({
        code: "duplicate-system",
        message: `${grammarSystemDescriptor(systemId)?.label ?? systemId} has duplicate records. Edits are disabled until the conflict is resolved.`,
        systemId,
        recordIds: ids,
      });
      continue;
    }
    const source = records.find((item) => item.id === ids[0]);
    const result = source ? normalizeGrammarRecord(source.value) : null;
    if (result?.ok && result.record.recordKind === "system") {
      systems.set(systemId, { id: ids[0], revision: source?.revision ?? "", value: result.record });
      for (const item of result.issues) diagnostics.push({ ...item, recordIds: ids, systemId });
    }
  }

  return { systems, duplicates, agreements, customRules, sectionStates, rejected, diagnostics };
}

export function systemStatus(index: IndexedGrammar, systemId: GrammarSystemId): GrammarStatus {
  if (index.duplicates.has(systemId)) return "unconfigured";
  return index.systems.get(systemId)?.value.recordKind === "system"
    ? index.systems.get(systemId)!.value.status
    : "unconfigured";
}

export function brokenAgreementFeatures(index: IndexedGrammar): GrammarDiagnostic[] {
  const diagnostics: GrammarDiagnostic[] = [];
  for (const record of index.agreements) {
    if (record.value.recordKind !== "agreement") continue;
    for (const feature of record.value.features) {
      if (!feature.sourceSystemId || !feature.categoryId) continue;
      const system = index.systems.get(feature.sourceSystemId);
      if (!system || system.value.recordKind !== "system") {
        diagnostics.push({
          code: "broken-reference",
          message: `Agreement “${record.value.title}” references missing system ${feature.sourceSystemId}.`,
          recordIds: [record.id],
          systemId: feature.sourceSystemId,
        });
        continue;
      }
      const config = system.value.config as { categories?: { id: string }[]; cases?: { id: string }[]; classes?: { id: string }[] };
      const ids = new Set(
        [...(config.categories ?? []), ...(config.cases ?? []), ...(config.classes ?? [])].map((item) => item.id),
      );
      if (!ids.has(feature.categoryId)) {
        diagnostics.push({
          code: "broken-reference",
          message: `Agreement “${record.value.title}” references a missing category in ${feature.sourceSystemId}.`,
          recordIds: [record.id],
          systemId: feature.sourceSystemId,
        });
      }
    }
  }
  return diagnostics;
}
