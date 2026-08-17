import { MAX_CATEGORIES, MAX_STRATEGIES } from "./normalize.ts";
import type {
  ClauseNegationConfig,
  ClauseNegationStrategy,
  ContentQuestionBehavior,
  ContentQuestionsConfig,
  GrammarSystemId,
  GrammarSystemRecord,
  ImperativeStrategy,
  ImperativesConfig,
  InterrogativeItem,
  ParticlePlacement,
  RelativeClausesConfig,
  RelativizationStrategy,
  YesNoQuestionStrategy,
  YesNoQuestionsConfig,
} from "./types.ts";

export const CLAUSE_SYSTEM_IDS = [
  "clauses.yes-no-questions",
  "clauses.content-questions",
  "clauses.imperatives",
  "clauses.negation",
  "clauses.relative-clauses",
] as const satisfies readonly GrammarSystemId[];

export type ClauseSystemId = (typeof CLAUSE_SYSTEM_IDS)[number];

type ClauseOption<T extends string = string> = {
  value: T;
  label: string;
  expansion?: string;
};

export const YES_NO_OPTIONS: ClauseOption<YesNoQuestionStrategy>[] = [
  { value: "intonation", label: "Intonation only" },
  { value: "particle", label: "Question particle" },
  { value: "word-order", label: "Word-order change" },
  { value: "verb-morphology", label: "Verb morphology" },
  { value: "auxiliary", label: "Auxiliary" },
  { value: "multiple", label: "Multiple strategies" },
  { value: "custom", label: "Custom" },
];

export const PLACEMENT_OPTIONS: ClauseOption<ParticlePlacement>[] = [
  { value: "clause-initial", label: "Beginning of clause" },
  { value: "clause-final", label: "End of clause" },
  { value: "before-verb", label: "Before verb" },
  { value: "after-verb", label: "After verb" },
  { value: "other", label: "Other" },
];

export const CONTENT_QUESTION_OPTIONS: ClauseOption<ContentQuestionBehavior>[] = [
  { value: "in-situ", label: "Remain in normal position" },
  { value: "fronted", label: "Move to beginning" },
  { value: "fixed-position", label: "Move to another fixed position" },
  { value: "special-structure", label: "Special clause structure" },
  { value: "mixed", label: "Mixed" },
  { value: "custom", label: "Custom" },
];

export const INTERROGATIVE_TEMPLATES = ["who", "what", "where", "when", "why", "how"] as const;

export const IMPERATIVE_OPTIONS: ClauseOption<ImperativeStrategy>[] = [
  { value: "bare-verb", label: "Bare verb" },
  { value: "special-form", label: "Special verb form" },
  { value: "particle", label: "Particle" },
  { value: "auxiliary", label: "Auxiliary" },
  { value: "word-order", label: "Word-order change" },
  { value: "multiple", label: "Multiple forms based on politeness or number" },
  { value: "custom", label: "Custom" },
];

export const CLAUSE_NEGATION_OPTIONS: ClauseOption<ClauseNegationStrategy>[] = [
  { value: "particle", label: "Particle" },
  { value: "affix", label: "Affix" },
  { value: "auxiliary", label: "Auxiliary" },
  { value: "special-verb", label: "Special negative verb" },
  { value: "multiple", label: "Multiple strategies" },
  { value: "custom", label: "Custom" },
];

export const RELATIVIZATION_OPTIONS: ClauseOption<RelativizationStrategy>[] = [
  { value: "relative-pronoun", label: "Relative pronoun" },
  { value: "complementizer", label: "Complementizer" },
  { value: "gap", label: "Gap" },
  { value: "resumptive", label: "Resumptive pronoun" },
  { value: "internally-headed", label: "Internally headed" },
  { value: "multiple", label: "Multiple strategies" },
  { value: "custom", label: "Custom" },
];

export function isClauseSystem(systemId: GrammarSystemId): systemId is ClauseSystemId {
  return (CLAUSE_SYSTEM_IDS as readonly string[]).includes(systemId);
}

export function toggleYesNoStrategy(draft: GrammarSystemRecord, strategy: YesNoQuestionStrategy): GrammarSystemRecord {
  if (draft.systemId !== "clauses.yes-no-questions") return draft;
  const config = yesNoConfig(draft);
  const strategies = toggle(config.strategies, strategy);
  return setYesNo(draft, {
    strategies,
    particle: strategies.includes("particle") ? config.particle : undefined,
    placement: strategies.includes("particle") ? config.placement : undefined,
  });
}

export function setYesNoParticle(draft: GrammarSystemRecord, particle: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.yes-no-questions") return draft;
  return setYesNo(draft, { ...yesNoConfig(draft), particle });
}

export function setYesNoPlacement(draft: GrammarSystemRecord, placement: ParticlePlacement): GrammarSystemRecord {
  if (draft.systemId !== "clauses.yes-no-questions") return draft;
  return setYesNo(draft, { ...yesNoConfig(draft), placement });
}

export function setContentBehavior(draft: GrammarSystemRecord, behavior: ContentQuestionBehavior): GrammarSystemRecord {
  if (draft.systemId !== "clauses.content-questions") return draft;
  const config = contentConfig(draft);
  return setContent(draft, {
    ...config,
    behavior,
    customBehavior: behavior === "custom" ? config.customBehavior : undefined,
  });
}

export function setContentCustomBehavior(draft: GrammarSystemRecord, customBehavior: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.content-questions") return draft;
  return setContent(draft, { ...contentConfig(draft), customBehavior });
}

export function toggleInterrogative(draft: GrammarSystemRecord, meaning: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.content-questions") return draft;
  const config = contentConfig(draft);
  const existing = config.interrogatives.find((item) => item.meaning === meaning);
  if (existing) {
    return setContent(draft, {
      ...config,
      interrogatives: config.interrogatives.filter((item) => item.id !== existing.id),
    });
  }
  return setContent(draft, {
    ...config,
    interrogatives: append(config.interrogatives, { id: newId(), meaning }),
  });
}

export function addInterrogative(draft: GrammarSystemRecord, meaning = ""): GrammarSystemRecord {
  if (draft.systemId !== "clauses.content-questions") return draft;
  const config = contentConfig(draft);
  return setContent(draft, { ...config, interrogatives: append(config.interrogatives, { id: newId(), meaning }) });
}

export function updateInterrogative(
  draft: GrammarSystemRecord,
  id: string,
  patch: Partial<Omit<InterrogativeItem, "id">>,
): GrammarSystemRecord {
  if (draft.systemId !== "clauses.content-questions") return draft;
  const config = contentConfig(draft);
  return setContent(draft, {
    ...config,
    interrogatives: config.interrogatives.map((item) => (item.id === id ? { ...item, ...patch, id: item.id } : item)),
  });
}

export function moveInterrogative(draft: GrammarSystemRecord, id: string, delta: number): GrammarSystemRecord {
  if (draft.systemId !== "clauses.content-questions") return draft;
  const config = contentConfig(draft);
  return setContent(draft, { ...config, interrogatives: move(config.interrogatives, id, delta) });
}

export function removeInterrogative(draft: GrammarSystemRecord, id: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.content-questions") return draft;
  const config = contentConfig(draft);
  return setContent(draft, { ...config, interrogatives: config.interrogatives.filter((item) => item.id !== id) });
}

export function toggleImperativeStrategy(
  draft: GrammarSystemRecord,
  strategy: ImperativeStrategy,
): GrammarSystemRecord {
  if (draft.systemId !== "clauses.imperatives") return draft;
  const config = imperativeConfig(draft);
  return setImperative(draft, { ...config, strategies: toggle(config.strategies, strategy) });
}

export function setImperativeDistinction(
  draft: GrammarSystemRecord,
  key: "numberDistinction" | "polarityDistinction" | "politenessDistinction",
  value: boolean,
): GrammarSystemRecord {
  if (draft.systemId !== "clauses.imperatives") return draft;
  return setImperative(draft, { ...imperativeConfig(draft), [key]: value });
}

export function toggleNegationStrategy(
  draft: GrammarSystemRecord,
  strategy: ClauseNegationStrategy,
): GrammarSystemRecord {
  if (draft.systemId !== "clauses.negation") return draft;
  const config = negationConfig(draft);
  const strategies = toggle(config.strategies, strategy);
  return setNegation(draft, {
    ...config,
    strategies,
    particle: strategies.includes("particle") ? config.particle : undefined,
    placement: strategies.includes("particle") ? config.placement : undefined,
  });
}

export function setNegationParticle(draft: GrammarSystemRecord, particle: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.negation") return draft;
  return setNegation(draft, { ...negationConfig(draft), particle });
}

export function setNegationPlacement(draft: GrammarSystemRecord, placement: ParticlePlacement): GrammarSystemRecord {
  if (draft.systemId !== "clauses.negation") return draft;
  return setNegation(draft, { ...negationConfig(draft), placement });
}

export function setNegationQuestions(draft: GrammarSystemRecord, negativeQuestions: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.negation") return draft;
  return setNegation(draft, { ...negationConfig(draft), negativeQuestions });
}

export function setNegationImperatives(draft: GrammarSystemRecord, negativeImperatives: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.negation") return draft;
  return setNegation(draft, { ...negationConfig(draft), negativeImperatives });
}

export function toggleRelativization(
  draft: GrammarSystemRecord,
  strategy: RelativizationStrategy,
): GrammarSystemRecord {
  if (draft.systemId !== "clauses.relative-clauses") return draft;
  const config = relativeConfig(draft);
  return setRelative(draft, { ...config, strategies: toggle(config.strategies, strategy) });
}

export function setRelativeHeadBehavior(draft: GrammarSystemRecord, headBehavior: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.relative-clauses") return draft;
  return setRelative(draft, { ...relativeConfig(draft), headBehavior });
}

export function setRelativeResumptives(draft: GrammarSystemRecord, resumptives: string): GrammarSystemRecord {
  if (draft.systemId !== "clauses.relative-clauses") return draft;
  return setRelative(draft, { ...relativeConfig(draft), resumptives });
}

export function summarizeClause(systemId: GrammarSystemId, config: GrammarSystemRecord["config"]): string | undefined {
  switch (systemId) {
    case "clauses.yes-no-questions": {
      const value = config as YesNoQuestionsConfig;
      return join([joinLabels(YES_NO_OPTIONS, value.strategies), value.particle ? `“${value.particle}”` : undefined]);
    }
    case "clauses.content-questions": {
      const value = config as ContentQuestionsConfig;
      if (!value.behavior) return undefined;
      const behavior =
        value.behavior === "custom" && value.customBehavior?.trim()
          ? value.customBehavior.trim()
          : (CONTENT_QUESTION_OPTIONS.find((item) => item.value === value.behavior)?.label ?? value.behavior);
      const words = value.interrogatives
        ?.map((item) => item.meaning)
        .filter(Boolean)
        .join(", ");
      return [behavior, words].filter(Boolean).join(" · ");
    }
    case "clauses.imperatives":
      return joinLabels(IMPERATIVE_OPTIONS, (config as ImperativesConfig).strategies);
    case "clauses.negation": {
      const value = config as ClauseNegationConfig;
      return join([
        joinLabels(CLAUSE_NEGATION_OPTIONS, value.strategies),
        value.particle ? `“${value.particle}”` : undefined,
      ]);
    }
    case "clauses.relative-clauses":
      return joinLabels(RELATIVIZATION_OPTIONS, (config as RelativeClausesConfig).strategies);
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

function append<T>(items: T[], item: T): T[] {
  return items.length >= MAX_CATEGORIES ? items : [...items, item];
}

function join(parts: (string | undefined)[]) {
  return parts.filter(Boolean).join(" · ") || undefined;
}

function joinLabels(options: ClauseOption[], values: string[] | undefined) {
  if (!values?.length) return undefined;
  return values
    .map((value) => options.find((option) => option.value === value)?.label ?? value.replaceAll("-", " "))
    .join(" / ");
}

function yesNoConfig(draft: GrammarSystemRecord): YesNoQuestionsConfig {
  const config = draft.config as YesNoQuestionsConfig;
  return Array.isArray(config.strategies) ? config : { strategies: [] };
}

function contentConfig(draft: GrammarSystemRecord): ContentQuestionsConfig {
  const config = draft.config as ContentQuestionsConfig;
  return {
    behavior: config.behavior,
    customBehavior: config.customBehavior,
    interrogatives: config.interrogatives ?? [],
  };
}

function imperativeConfig(draft: GrammarSystemRecord): ImperativesConfig {
  const config = draft.config as ImperativesConfig;
  return Array.isArray(config.strategies) ? config : { strategies: [] };
}

function negationConfig(draft: GrammarSystemRecord): ClauseNegationConfig {
  const config = draft.config as ClauseNegationConfig;
  return Array.isArray(config.strategies) ? config : { strategies: [] };
}

function relativeConfig(draft: GrammarSystemRecord): RelativeClausesConfig {
  const config = draft.config as RelativeClausesConfig;
  return Array.isArray(config.strategies) ? config : { strategies: [] };
}

function setYesNo(draft: GrammarSystemRecord, config: YesNoQuestionsConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setContent(draft: GrammarSystemRecord, config: ContentQuestionsConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setImperative(draft: GrammarSystemRecord, config: ImperativesConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setNegation(draft: GrammarSystemRecord, config: ClauseNegationConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}
function setRelative(draft: GrammarSystemRecord, config: RelativeClausesConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}

function newId() {
  return crypto.randomUUID();
}
