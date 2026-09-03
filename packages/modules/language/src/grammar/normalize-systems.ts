import {
  type AdjectiveBehaviorConfig,
  type AdpositionsConfig,
  type ArgumentIndexingConfig,
  type BasicWordOrderConfig,
  type CaseConfig,
  type ClauseNegationConfig,
  type ContentQuestionsConfig,
  type DefinitenessConfig,
  type DegreeConfig,
  type DemonstrativeConfig,
  type GrammarExample,
  type GrammarSystemConfig,
  type GrammarSystemId,
  type ImperativesConfig,
  type NegativeVerbConfig,
  type NounClassesConfig,
  type NumberConfig,
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

import {
  MAX_ALTERNATES,
  MAX_ARTICLES,
  MAX_AXIS_VALUES,
  MAX_CATEGORIES,
  MAX_FEATURES,
  NOTES,
  bool,
  emptyConfig,
  id,
  obj,
  optional,
  pick,
  pickList,
  text,
} from "./normalize-primitives.ts";
import { paradigm } from "./normalize-paradigm.ts";

export const WORD_ORDERS = ["sov", "svo", "vso", "vos", "ovs", "osv", "flexible", "custom"] as const;


export const STRENGTHS = ["strict", "strongly-preferred", "default-flexible", "context"] as const;


export const INFLUENCES = ["topic", "focus", "emphasis", "definiteness", "animacy", "discourse", "custom"] as const;


export const POSITIONS = ["before", "after", "either", "meaning-changes", "custom"] as const;


export const POSS_POS = ["possessor-before", "possessor-after", "either", "morphological", "multiple", "custom"] as const;


export const REL_POS = ["before", "after", "internally-headed", "multiple", "custom"] as const;


export const ADPOSITIONS = ["prepositions", "postpositions", "both", "other"] as const;


export const NUMBER_TEMPLATES = ["singular", "plural", "dual", "trial", "paucal", "collective", "custom"] as const;


export const MARKING = ["affix", "separate-word", "stem-change", "multiple", "unmarked", "custom"] as const;


export const CASE_TEMPLATES = [
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

export const CLASS_KINDS = ["gender", "noun-class", "custom"] as const;


export const DEF_STRATEGIES = [
  "definite-article",
  "indefinite-article",
  "both",
  "affixes",
  "demonstratives",
  "context",
  "other",
] as const;

export const POSS_STRATEGIES = [
  "possessive-pronouns",
  "genitive",
  "possessor-marking",
  "possessed-marking",
  "linking-particle",
  "word-order",
  "multiple",
] as const;

export const VERB_MARKING = [
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

export const PARTICIPANTS = ["none", "subject", "object", "subject-object", "other"] as const;


export const REPRESENTATION = ["endings", "prefixes", "full-forms", "auxiliaries", "flexible-table", "custom"] as const;


export const NEG_VERB = ["affix", "negative-auxiliary", "special-verb", "stem-change", "none", "multiple", "custom"] as const;


export const ADJ_BEHAVIOR = ["invariant", "agree-with-noun", "verb-like", "noun-like", "multiple-classes", "custom"] as const;


export const COMPARATIVE = ["synthetic", "particle", "affix", "exceed", "special", "multiple", "custom"] as const;


export const SUPERLATIVE = ["dedicated", "intensifier", "comparative", "definite", "none", "custom"] as const;


export const YES_NO = ["intonation", "particle", "word-order", "verb-morphology", "auxiliary", "multiple", "custom"] as const;


export const PLACEMENT = ["clause-initial", "clause-final", "before-verb", "after-verb", "other"] as const;


export const CONTENT_Q = ["in-situ", "fronted", "fixed-position", "special-structure", "mixed", "custom"] as const;


export const IMPERATIVE = ["bare-verb", "special-form", "particle", "auxiliary", "word-order", "multiple", "custom"] as const;


export const CLAUSE_NEG = ["particle", "affix", "auxiliary", "special-verb", "multiple", "custom"] as const;


export const REL_STRAT = [
  "relative-pronoun",
  "complementizer",
  "gap",
  "resumptive",
  "internally-headed",
  "multiple",
  "custom",
] as const;

export const CONTROLLERS = ["subject", "object", "noun", "possessor", "custom"] as const;


export const TARGETS = ["verb", "adjective", "article", "pronoun", "participle", "custom"] as const;


export const BEHAVIORS = ["full", "partial", "conditional"] as const;


export function inventoryItems(value: unknown, fields: { meaning?: boolean; marker?: boolean; extra?: boolean }) {
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

export function normalizeSystemConfig(
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
              .map((item): DefinitenessConfig["articles"][number] | null => {
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
          ? raw.distances
              .map((item) => text(item))
              .filter(Boolean)
              .slice(0, MAX_AXIS_VALUES)
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
              .map((item): NegativeVerbConfig["forms"][number] | null => {
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
          ? raw.agreementRecordIds
              .map((item) => text(item))
              .filter(Boolean)
              .slice(0, MAX_FEATURES)
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
              .map((item): ContentQuestionsConfig["interrogatives"][number] | null => {
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
      return (
        Boolean((config as NounClassesConfig).kind) &&
        (config as NounClassesConfig).classes?.some((item) => item.name?.trim()) === true
      );
    case "nouns.definiteness":
      return (config as DefinitenessConfig).strategies?.length > 0;
    case "nouns.possession":
      return (config as PossessionConfig).strategies?.length > 0;
    case "pronouns.personal":
      return (config as ParadigmConfig).axes?.length > 0;
    case "pronouns.demonstratives":
      return (config as DemonstrativeConfig).distances?.length > 0 || (config as DemonstrativeConfig).axes?.length > 0;
    case "verbs.marking-strategy": {
      const value = config as VerbMarkingConfig;
      if (!value.strategies?.length) return false;
      return !value.strategies.includes("custom") || Boolean(value.customStrategy?.trim());
    }
    case "verbs.tense":
    case "verbs.aspect":
    case "verbs.mood":
      return (config as TamConfig).categories?.some((item) => item.label?.trim()) === true;
    case "verbs.argument-indexing":
      return Boolean((config as ArgumentIndexingConfig).participants);
    case "verbs.negative-forms":
      return (config as NegativeVerbConfig).strategies?.length > 0;
    case "modifiers.adjective-behavior": {
      const value = config as AdjectiveBehaviorConfig;
      if (!value.behaviors?.length) return false;
      return !value.behaviors.includes("custom") || Boolean(value.customBehavior?.trim());
    }
    case "modifiers.comparative":
    case "modifiers.superlative":
      return (config as DegreeConfig).strategies?.length > 0;
    case "clauses.yes-no-questions": {
      const value = config as YesNoQuestionsConfig;
      if (!value.strategies?.length) return false;
      return !value.strategies.includes("particle") || Boolean(value.particle?.trim());
    }
    case "clauses.content-questions": {
      const value = config as ContentQuestionsConfig;
      if (!value.behavior) return false;
      return value.behavior !== "custom" || Boolean(value.customBehavior?.trim());
    }
    case "clauses.imperatives":
      return (config as ImperativesConfig).strategies?.length > 0;
    case "clauses.negation":
      return (config as ClauseNegationConfig).strategies?.length > 0;
    case "clauses.relative-clauses":
      return (config as RelativeClausesConfig).strategies?.length > 0;
  }
}
