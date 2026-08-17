import { MAX_ARTICLES, MAX_CATEGORIES, MAX_FEATURES, MAX_STRATEGIES } from "./normalize.ts";
import type {
  AdjectiveBehaviorConfig,
  AdjectiveBehaviorKind,
  ArticleForm,
  ComparativeStrategy,
  DefinitenessConfig,
  DefinitenessStrategy,
  DegreeConfig,
  GrammarSystemId,
  GrammarSystemRecord,
  NegativeVerbConfig,
  NegativeVerbForm,
  NegativeVerbStrategy,
  PossessionConfig,
  PossessionStrategy,
  SuperlativeStrategy,
  VerbMarkingConfig,
  VerbMarkingStrategy,
} from "./types.ts";

export const STRATEGY_SYSTEM_IDS = [
  "nouns.definiteness",
  "nouns.possession",
  "verbs.marking-strategy",
  "verbs.negative-forms",
  "modifiers.adjective-behavior",
  "modifiers.comparative",
  "modifiers.superlative",
] as const satisfies readonly GrammarSystemId[];

export type StrategySystemId = (typeof STRATEGY_SYSTEM_IDS)[number];

type StrategyOption<T extends string = string> = {
  value: T;
  label: string;
  expansion?: string;
  example?: string;
};

export const DEFINITENESS_OPTIONS: StrategyOption<DefinitenessStrategy>[] = [
  { value: "definite-article", label: "Definite article", expansion: "A form that marks known or specific reference." },
  {
    value: "indefinite-article",
    label: "Indefinite article",
    expansion: "A form that marks new or nonspecific reference.",
  },
  { value: "both", label: "Both articles" },
  { value: "affixes", label: "Affixes" },
  { value: "demonstratives", label: "Demonstratives" },
  { value: "context", label: "Context only" },
  { value: "other", label: "Other" },
];

export const POSSESSION_OPTIONS: StrategyOption<PossessionStrategy>[] = [
  { value: "possessive-pronouns", label: "Possessive pronouns" },
  { value: "genitive", label: "Genitive marking" },
  { value: "possessor-marking", label: "Possessor marking" },
  { value: "possessed-marking", label: "Possessed-noun marking" },
  { value: "linking-particle", label: "Linking particle" },
  { value: "word-order", label: "Word order only" },
  { value: "multiple", label: "Multiple strategies" },
];

export const VERB_MARKING_OPTIONS: StrategyOption<VerbMarkingStrategy>[] = [
  { value: "invariant", label: "Verb usually does not change" },
  { value: "prefixes", label: "Prefixes" },
  { value: "suffixes", label: "Suffixes" },
  { value: "other-affixes", label: "Other affixes" },
  { value: "stem-changes", label: "Stem changes" },
  { value: "auxiliaries", label: "Auxiliary verbs" },
  { value: "particles", label: "Particles" },
  { value: "multiple", label: "Multiple strategies" },
  { value: "custom", label: "Custom" },
];

export const NEGATIVE_VERB_OPTIONS: StrategyOption<NegativeVerbStrategy>[] = [
  { value: "affix", label: "Affix" },
  { value: "negative-auxiliary", label: "Negative auxiliary" },
  { value: "special-verb", label: "Special negative verb" },
  { value: "stem-change", label: "Stem change" },
  { value: "none", label: "No special verb form" },
  { value: "multiple", label: "Multiple strategies" },
  { value: "custom", label: "Custom" },
];

export const ADJECTIVE_BEHAVIOR_OPTIONS: StrategyOption<AdjectiveBehaviorKind>[] = [
  { value: "invariant", label: "Invariant" },
  { value: "agree-with-noun", label: "Agree with noun", expansion: "Configure the actual agreement under Agreement." },
  { value: "verb-like", label: "Behave like verbs" },
  { value: "noun-like", label: "Behave like nouns" },
  { value: "multiple-classes", label: "Multiple classes" },
  { value: "custom", label: "Custom" },
];

export const COMPARATIVE_OPTIONS: StrategyOption<ComparativeStrategy>[] = [
  { value: "synthetic", label: "Synthetic form", example: "tall → taller" },
  { value: "particle", label: "Comparative particle", example: "more + tall" },
  { value: "affix", label: "Comparative affix" },
  { value: "exceed", label: 'Verb meaning "exceed"', example: "A exceeds B in height" },
  { value: "special", label: "Special construction" },
  { value: "multiple", label: "Multiple strategies" },
  { value: "custom", label: "Custom" },
];

export const SUPERLATIVE_OPTIONS: StrategyOption<SuperlativeStrategy>[] = [
  { value: "dedicated", label: "Dedicated superlative morphology" },
  { value: "intensifier", label: "Intensifier" },
  { value: "comparative", label: "Comparative construction" },
  { value: "definite", label: "Definite construction" },
  { value: "none", label: "No dedicated superlative" },
  { value: "custom", label: "Custom" },
];

export function isStrategySystem(systemId: GrammarSystemId): systemId is StrategySystemId {
  return (STRATEGY_SYSTEM_IDS as readonly string[]).includes(systemId);
}

export function toggleDefinitenessStrategy(
  draft: GrammarSystemRecord,
  strategy: DefinitenessStrategy,
): GrammarSystemRecord {
  if (draft.systemId !== "nouns.definiteness") return draft;
  const config = definitenessConfig(draft);
  return setDefiniteness(draft, { ...config, strategies: toggle(config.strategies, strategy) });
}

export function addArticle(draft: GrammarSystemRecord, form = ""): GrammarSystemRecord {
  if (draft.systemId !== "nouns.definiteness") return draft;
  const config = definitenessConfig(draft);
  if (config.articles.length >= MAX_ARTICLES) return draft;
  return setDefiniteness(draft, { ...config, articles: [...config.articles, { id: newId(), form }] });
}

export function updateArticle(
  draft: GrammarSystemRecord,
  id: string,
  patch: Partial<Omit<ArticleForm, "id">>,
): GrammarSystemRecord {
  if (draft.systemId !== "nouns.definiteness") return draft;
  const config = definitenessConfig(draft);
  return setDefiniteness(draft, {
    ...config,
    articles: config.articles.map((item) => (item.id === id ? { ...item, ...patch, id: item.id } : item)),
  });
}

export function moveArticle(draft: GrammarSystemRecord, id: string, delta: number): GrammarSystemRecord {
  if (draft.systemId !== "nouns.definiteness") return draft;
  const config = definitenessConfig(draft);
  return setDefiniteness(draft, { ...config, articles: move(config.articles, id, delta) });
}

export function removeArticle(draft: GrammarSystemRecord, id: string): GrammarSystemRecord {
  if (draft.systemId !== "nouns.definiteness") return draft;
  const config = definitenessConfig(draft);
  return setDefiniteness(draft, { ...config, articles: config.articles.filter((item) => item.id !== id) });
}

export function togglePossessionStrategy(
  draft: GrammarSystemRecord,
  strategy: PossessionStrategy,
): GrammarSystemRecord {
  if (draft.systemId !== "nouns.possession") return draft;
  const config = possessionConfig(draft);
  return setPossession(draft, { ...config, strategies: toggle(config.strategies, strategy) });
}

export function setAlienability(draft: GrammarSystemRecord, alienability: boolean): GrammarSystemRecord {
  if (draft.systemId !== "nouns.possession") return draft;
  const config = possessionConfig(draft);
  return setPossession(draft, {
    ...config,
    alienability,
    alienabilityNotes: alienability ? config.alienabilityNotes : undefined,
  });
}

export function setAlienabilityNotes(draft: GrammarSystemRecord, alienabilityNotes: string): GrammarSystemRecord {
  if (draft.systemId !== "nouns.possession") return draft;
  const config = possessionConfig(draft);
  return setPossession(draft, { ...config, alienabilityNotes });
}

export function toggleVerbMarking(draft: GrammarSystemRecord, strategy: VerbMarkingStrategy): GrammarSystemRecord {
  if (draft.systemId !== "verbs.marking-strategy") return draft;
  const config = verbMarkingConfig(draft);
  const strategies = toggle(config.strategies, strategy);
  return setVerbMarking(draft, {
    strategies,
    customStrategy: strategies.includes("custom") ? config.customStrategy : undefined,
  });
}

export function setCustomVerbMarking(draft: GrammarSystemRecord, customStrategy: string): GrammarSystemRecord {
  if (draft.systemId !== "verbs.marking-strategy") return draft;
  const config = verbMarkingConfig(draft);
  return setVerbMarking(draft, { ...config, customStrategy });
}

export function toggleNegativeStrategy(
  draft: GrammarSystemRecord,
  strategy: NegativeVerbStrategy,
): GrammarSystemRecord {
  if (draft.systemId !== "verbs.negative-forms") return draft;
  const config = negativeConfig(draft);
  return setNegative(draft, { ...config, strategies: toggle(config.strategies, strategy) });
}

export function addNegativeForm(draft: GrammarSystemRecord, form = ""): GrammarSystemRecord {
  if (draft.systemId !== "verbs.negative-forms") return draft;
  const config = negativeConfig(draft);
  if (config.forms.length >= MAX_CATEGORIES) return draft;
  return setNegative(draft, { ...config, forms: [...config.forms, { id: newId(), form }] });
}

export function updateNegativeForm(
  draft: GrammarSystemRecord,
  id: string,
  patch: Partial<Omit<NegativeVerbForm, "id">>,
): GrammarSystemRecord {
  if (draft.systemId !== "verbs.negative-forms") return draft;
  const config = negativeConfig(draft);
  return setNegative(draft, {
    ...config,
    forms: config.forms.map((item) => (item.id === id ? { ...item, ...patch, id: item.id } : item)),
  });
}

export function moveNegativeForm(draft: GrammarSystemRecord, id: string, delta: number): GrammarSystemRecord {
  if (draft.systemId !== "verbs.negative-forms") return draft;
  const config = negativeConfig(draft);
  return setNegative(draft, { ...config, forms: move(config.forms, id, delta) });
}

export function removeNegativeForm(draft: GrammarSystemRecord, id: string): GrammarSystemRecord {
  if (draft.systemId !== "verbs.negative-forms") return draft;
  const config = negativeConfig(draft);
  return setNegative(draft, { ...config, forms: config.forms.filter((item) => item.id !== id) });
}

export function toggleAdjectiveBehavior(
  draft: GrammarSystemRecord,
  behavior: AdjectiveBehaviorKind,
): GrammarSystemRecord {
  if (draft.systemId !== "modifiers.adjective-behavior") return draft;
  const config = adjectiveConfig(draft);
  const behaviors = toggle(config.behaviors, behavior);
  return setAdjective(draft, {
    behaviors,
    customBehavior: behaviors.includes("custom") ? config.customBehavior : undefined,
    agreementRecordIds: behaviors.includes("agree-with-noun") ? config.agreementRecordIds : [],
  });
}

export function setCustomAdjectiveBehavior(draft: GrammarSystemRecord, customBehavior: string): GrammarSystemRecord {
  if (draft.systemId !== "modifiers.adjective-behavior") return draft;
  const config = adjectiveConfig(draft);
  return setAdjective(draft, { ...config, customBehavior });
}

export function toggleAgreementRecord(draft: GrammarSystemRecord, recordId: string): GrammarSystemRecord {
  if (draft.systemId !== "modifiers.adjective-behavior") return draft;
  const config = adjectiveConfig(draft);
  const agreementRecordIds = config.agreementRecordIds.includes(recordId)
    ? config.agreementRecordIds.filter((item) => item !== recordId)
    : config.agreementRecordIds.length >= MAX_FEATURES
      ? config.agreementRecordIds
      : [...config.agreementRecordIds, recordId];
  return setAdjective(draft, { ...config, agreementRecordIds });
}

export function toggleDegreeStrategy(draft: GrammarSystemRecord, strategy: string): GrammarSystemRecord {
  if (draft.systemId !== "modifiers.comparative" && draft.systemId !== "modifiers.superlative") return draft;
  const config = degreeConfig(draft);
  return setDegree(draft, { ...config, strategies: toggle(config.strategies, strategy) });
}

export function setDegreeMarker(draft: GrammarSystemRecord, marker: string): GrammarSystemRecord {
  if (draft.systemId !== "modifiers.comparative" && draft.systemId !== "modifiers.superlative") return draft;
  return setDegree(draft, { ...degreeConfig(draft), marker });
}

export function setDegreeConstruction(draft: GrammarSystemRecord, construction: string): GrammarSystemRecord {
  if (draft.systemId !== "modifiers.comparative" && draft.systemId !== "modifiers.superlative") return draft;
  return setDegree(draft, { ...degreeConfig(draft), construction });
}

export function summarizeStrategy(
  systemId: GrammarSystemId,
  config: GrammarSystemRecord["config"],
): string | undefined {
  switch (systemId) {
    case "nouns.definiteness":
      return joinLabels(DEFINITENESS_OPTIONS, (config as DefinitenessConfig).strategies);
    case "nouns.possession":
      return joinLabels(POSSESSION_OPTIONS, (config as PossessionConfig).strategies);
    case "verbs.marking-strategy": {
      const value = config as VerbMarkingConfig;
      return joinLabels(VERB_MARKING_OPTIONS, value.strategies, value.customStrategy);
    }
    case "verbs.negative-forms":
      return joinLabels(NEGATIVE_VERB_OPTIONS, (config as NegativeVerbConfig).strategies);
    case "modifiers.adjective-behavior": {
      const value = config as AdjectiveBehaviorConfig;
      return joinLabels(ADJECTIVE_BEHAVIOR_OPTIONS, value.behaviors, value.customBehavior);
    }
    case "modifiers.comparative":
      return joinLabels(COMPARATIVE_OPTIONS, (config as DegreeConfig).strategies);
    case "modifiers.superlative":
      return joinLabels(SUPERLATIVE_OPTIONS, (config as DegreeConfig).strategies);
    default:
      return undefined;
  }
}

function toggle<T>(items: T[], item: T): T[] {
  if (items.includes(item)) return items.filter((value) => value !== item);
  return items.length >= MAX_STRATEGIES ? items : [...items, item];
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

function joinLabels(options: StrategyOption[], values: string[] | undefined, extra?: string) {
  if (!values?.length) return undefined;
  const labels = values.map((value) => {
    if (value === "custom" && extra?.trim()) return extra.trim();
    return options.find((option) => option.value === value)?.label ?? value.replaceAll("-", " ");
  });
  return labels.join(" / ");
}

function definitenessConfig(draft: GrammarSystemRecord): DefinitenessConfig {
  const config = draft.config as DefinitenessConfig;
  return Array.isArray(config.strategies)
    ? { strategies: config.strategies, articles: config.articles ?? [] }
    : { strategies: [], articles: [] };
}

function possessionConfig(draft: GrammarSystemRecord): PossessionConfig {
  const config = draft.config as PossessionConfig;
  return Array.isArray(config.strategies) ? config : { strategies: [] };
}

function verbMarkingConfig(draft: GrammarSystemRecord): VerbMarkingConfig {
  const config = draft.config as VerbMarkingConfig;
  return Array.isArray(config.strategies) ? config : { strategies: [] };
}

function negativeConfig(draft: GrammarSystemRecord): NegativeVerbConfig {
  const config = draft.config as NegativeVerbConfig;
  return Array.isArray(config.strategies)
    ? { strategies: config.strategies, forms: config.forms ?? [] }
    : { strategies: [], forms: [] };
}

function adjectiveConfig(draft: GrammarSystemRecord): AdjectiveBehaviorConfig {
  const config = draft.config as AdjectiveBehaviorConfig;
  return Array.isArray(config.behaviors)
    ? {
        behaviors: config.behaviors,
        customBehavior: config.customBehavior,
        agreementRecordIds: config.agreementRecordIds ?? [],
      }
    : { behaviors: [], agreementRecordIds: [] };
}

function degreeConfig(draft: GrammarSystemRecord): DegreeConfig {
  const config = draft.config as DegreeConfig;
  return Array.isArray(config.strategies) ? config : { strategies: [] };
}

function setDefiniteness(draft: GrammarSystemRecord, config: DefinitenessConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setPossession(draft: GrammarSystemRecord, config: PossessionConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setVerbMarking(draft: GrammarSystemRecord, config: VerbMarkingConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setNegative(draft: GrammarSystemRecord, config: NegativeVerbConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setAdjective(draft: GrammarSystemRecord, config: AdjectiveBehaviorConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setDegree(draft: GrammarSystemRecord, config: DegreeConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}

function newId() {
  return crypto.randomUUID();
}
