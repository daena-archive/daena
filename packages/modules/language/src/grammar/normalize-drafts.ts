import {
  GRAMMAR_SCHEMA_VERSION,
  type AdjectiveBehaviorConfig,
  type AdpositionsConfig,
  type BasicWordOrderConfig,
  type CaseConfig,
  type ClauseNegationConfig,
  type ContentQuestionsConfig,
  type DefinitenessConfig,
  type DegreeConfig,
  type GrammarAgreementRecord,
  type GrammarCustomRuleRecord,
  type GrammarIssue,
  type GrammarRecord,
  type GrammarSectionStateRecord,
  type GrammarStatus,
  type GrammarSystemId,
  type GrammarSystemRecord,
  type ImperativesConfig,
  type NegativeVerbConfig,
  type NounClassesConfig,
  type NumberConfig,
  type PositionConfig,
  type PossessionConfig,
  type RelativeClausesConfig,
  type TamConfig,
  type VerbMarkingConfig,
  type YesNoQuestionsConfig,
} from "./types.ts";

import {
  emptyConfig,
  issue,
} from "./normalize-primitives.ts";
import {
  configuredMinimum,
} from "./normalize-systems.ts";

export function emptySystemRecord(
  systemId: GrammarSystemId,
  status: GrammarStatus = "unconfigured",
): GrammarSystemRecord {
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

export function emptyAgreementRecord(): GrammarAgreementRecord {
  return {
    recordKind: "agreement",
    schemaVersion: GRAMMAR_SCHEMA_VERSION,
    title: "Subject → Verb",
    controller: { kind: "subject" },
    target: { kind: "verb" },
    features: [],
    behavior: "full",
    notes: "",
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

export function choiceMinimumIssue(value: GrammarSystemRecord): GrammarIssue | undefined {
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
  if (
    value.systemId === "nouns.case" &&
    !(config as CaseConfig).cases?.some((item) => item.name?.trim() && item.primaryFunction?.trim())
  ) {
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
  if (value.systemId === "nouns.definiteness" && !(config as DefinitenessConfig).strategies?.length) {
    return issue("configured-minimum", "Choose how definiteness is marked.", "strategies");
  }
  if (value.systemId === "nouns.possession" && !(config as PossessionConfig).strategies?.length) {
    return issue("configured-minimum", "Choose how possession is marked.", "strategies");
  }
  if (value.systemId === "verbs.marking-strategy") {
    const marking = config as VerbMarkingConfig;
    if (!marking.strategies?.length)
      return issue("configured-minimum", "Choose a verb marking strategy.", "strategies");
    if (marking.strategies.includes("custom") && !marking.customStrategy?.trim()) {
      return issue("configured-minimum", "Describe the custom verb marking strategy.", "customStrategy");
    }
  }
  if (value.systemId === "verbs.negative-forms" && !(config as NegativeVerbConfig).strategies?.length) {
    return issue("configured-minimum", "Choose how negative verb forms work.", "strategies");
  }
  if (value.systemId === "modifiers.adjective-behavior") {
    const behavior = config as AdjectiveBehaviorConfig;
    if (!behavior.behaviors?.length) return issue("configured-minimum", "Choose how adjectives behave.", "behaviors");
    if (behavior.behaviors.includes("custom") && !behavior.customBehavior?.trim()) {
      return issue("configured-minimum", "Describe the custom adjective behavior.", "customBehavior");
    }
  }
  if (
    (value.systemId === "modifiers.comparative" || value.systemId === "modifiers.superlative") &&
    !(config as DegreeConfig).strategies?.length
  ) {
    return issue("configured-minimum", "Choose at least one strategy.", "strategies");
  }
  if (value.systemId === "clauses.yes-no-questions") {
    const questions = config as YesNoQuestionsConfig;
    if (!questions.strategies?.length)
      return issue("configured-minimum", "Choose how yes/no questions are formed.", "strategies");
    if (questions.strategies.includes("particle") && !questions.particle?.trim()) {
      return issue("configured-minimum", "Enter the question particle.", "particle");
    }
  }
  if (value.systemId === "clauses.content-questions") {
    const content = config as ContentQuestionsConfig;
    if (!content.behavior) return issue("configured-minimum", "Choose where question words appear.", "behavior");
    if (content.behavior === "custom" && !content.customBehavior?.trim()) {
      return issue("configured-minimum", "Describe the custom question-word behavior.", "customBehavior");
    }
  }
  if (value.systemId === "clauses.imperatives" && !(config as ImperativesConfig).strategies?.length) {
    return issue("configured-minimum", "Choose how commands are formed.", "strategies");
  }
  if (value.systemId === "clauses.negation") {
    const negation = config as ClauseNegationConfig;
    if (!negation.strategies?.length)
      return issue("configured-minimum", "Choose a clause-negation strategy.", "strategies");
    if (negation.strategies.includes("particle") && !negation.particle?.trim()) {
      return issue("configured-minimum", "Enter the negation particle.", "particle");
    }
  }
  if (value.systemId === "clauses.relative-clauses" && !(config as RelativeClausesConfig).strategies?.length) {
    return issue("configured-minimum", "Choose a relativization strategy.", "strategies");
  }
  return undefined;
}

export function validateGrammarDraft(value: GrammarRecord): GrammarIssue[] {
  if (value.recordKind === "system") {
    if (value.status !== "configured") return [];
    const missing = choiceMinimumIssue(value);
    if (missing) return [missing];
    if (!configuredMinimum(value.systemId, value.config)) {
      return [
        issue(
          "configured-minimum",
          "This system needs its grammatical settings before it can be saved as configured.",
          "status",
        ),
      ];
    }
    return [];
  }
  if (value.recordKind === "custom-rule" && !value.title.trim()) {
    return [issue("malformed", "Title is required.", "title")];
  }
  if (value.recordKind === "agreement") {
    if (!value.title.trim()) return [issue("malformed", "Title is required.", "title")];
    if (value.controller.kind === "custom" && !value.controller.customLabel?.trim()) {
      return [issue("malformed", "Describe the custom controller.", "controllerCustom")];
    }
    if (value.target.kind === "custom" && !value.target.customLabel?.trim()) {
      return [issue("malformed", "Describe the custom target.", "targetCustom")];
    }
    if (value.features.some((item) => !item.label.trim())) {
      return [issue("malformed", "Each custom feature needs a label.", "customFeature")];
    }
  }
  return [];
}
