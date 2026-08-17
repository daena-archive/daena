import type {
  AdpositionStrategy,
  AdpositionsConfig,
  BasicWordOrderConfig,
  GrammarSystemId,
  GrammarSystemRecord,
  PositionChoice,
  PositionConfig,
  PossessivePositionChoice,
  PossessivePositionConfig,
  RelativeClausePositionChoice,
  RelativeClausePositionConfig,
  WordOrderInfluence,
  WordOrderPattern,
  WordOrderStrength,
} from "./types.ts";

export const CHOICE_SYSTEM_IDS = [
  "syntax.basic-word-order",
  "syntax.adjective-position",
  "syntax.adpositions",
  "syntax.possessive-position",
  "syntax.relative-clause-position",
] as const satisfies readonly GrammarSystemId[];

export type ChoiceSystemId = (typeof CHOICE_SYSTEM_IDS)[number];

export type ChoiceOption<T extends string> = {
  value: T;
  label: string;
  expansion?: string;
  example?: string;
};

export const WORD_ORDER_OPTIONS: ChoiceOption<WordOrderPattern>[] = [
  { value: "sov", label: "SOV", expansion: "Subject → Object → Verb", example: '"The hunter the deer sees."' },
  { value: "svo", label: "SVO", expansion: "Subject → Verb → Object", example: '"The hunter sees the deer."' },
  { value: "vso", label: "VSO", expansion: "Verb → Subject → Object", example: '"Sees the hunter the deer."' },
  { value: "vos", label: "VOS", expansion: "Verb → Object → Subject", example: '"Sees the deer the hunter."' },
  { value: "ovs", label: "OVS", expansion: "Object → Verb → Subject", example: '"The deer sees the hunter."' },
  { value: "osv", label: "OSV", expansion: "Object → Subject → Verb", example: '"The deer the hunter sees."' },
  { value: "flexible", label: "Flexible", expansion: "Order is not fixed in simple statements." },
  { value: "custom", label: "Custom", expansion: "Describe a pattern that is not one of the six basic orders." },
];

export const WORD_ORDER_STRENGTH_OPTIONS: ChoiceOption<WordOrderStrength>[] = [
  { value: "strict", label: "Strict" },
  { value: "strongly-preferred", label: "Strongly preferred" },
  { value: "default-flexible", label: "Default, but flexible" },
  { value: "context", label: "Mostly determined by context" },
];

export const WORD_ORDER_INFLUENCE_OPTIONS: ChoiceOption<WordOrderInfluence>[] = [
  { value: "topic", label: "Topic" },
  { value: "focus", label: "Focus" },
  { value: "emphasis", label: "Emphasis" },
  { value: "definiteness", label: "Definiteness" },
  { value: "animacy", label: "Animacy" },
  { value: "discourse", label: "Discourse context" },
  { value: "custom", label: "Custom" },
];

export const ADJECTIVE_POSITION_OPTIONS: ChoiceOption<PositionChoice>[] = [
  { value: "before", label: "Before noun", example: '"red house"' },
  { value: "after", label: "After noun", example: '"house red"' },
  { value: "either", label: "Either position" },
  { value: "meaning-changes", label: "Position changes meaning" },
  { value: "custom", label: "Custom" },
];

export const POSSESSIVE_POSITION_OPTIONS: ChoiceOption<PossessivePositionChoice>[] = [
  { value: "possessor-before", label: "Possessor before noun", example: '"the king\'s sword"' },
  { value: "possessor-after", label: "Possessor after noun", example: '"the sword of the king"' },
  { value: "either", label: "Either" },
  { value: "morphological", label: "Encoded morphologically" },
  { value: "multiple", label: "Multiple strategies" },
  { value: "custom", label: "Custom" },
];

export const RELATIVE_CLAUSE_POSITION_OPTIONS: ChoiceOption<RelativeClausePositionChoice>[] = [
  { value: "before", label: "Before noun" },
  { value: "after", label: "After noun" },
  { value: "internally-headed", label: "Internally headed" },
  { value: "multiple", label: "Multiple strategies" },
  { value: "custom", label: "Custom" },
];

export const ADPOSITION_OPTIONS: ChoiceOption<AdpositionStrategy>[] = [
  { value: "prepositions", label: "Prepositions", expansion: "Appear before the noun phrase." },
  { value: "postpositions", label: "Postpositions", expansion: "Appear after the noun phrase." },
  { value: "both", label: "Both", expansion: "Prepositions and postpositions both occur." },
  { value: "other", label: "Other strategy" },
];

export function isChoiceSystem(systemId: GrammarSystemId): systemId is ChoiceSystemId {
  return (CHOICE_SYSTEM_IDS as readonly string[]).includes(systemId);
}

export function choiceLabel(options: ChoiceOption<string>[], value: string | undefined) {
  return options.find((option) => option.value === value)?.label ?? value ?? "";
}

export type BasicWordOrderPatch = {
  order?: WordOrderPattern;
  customOrder?: string;
  strength?: WordOrderStrength | "";
  toggleInfluence?: WordOrderInfluence;
  customInfluence?: string;
  changeNotes?: string;
};

export function applyBasicWordOrder(draft: GrammarSystemRecord, patch: BasicWordOrderPatch): GrammarSystemRecord {
  if (draft.systemId !== "syntax.basic-word-order") return draft;
  const current = isWordOrder(draft.config) ? draft.config : undefined;
  const order = patch.order ?? current?.order;
  if (!order) return { ...draft, status: "configured", config: {} };

  let influences = current?.influences ?? [];
  if (patch.toggleInfluence) {
    influences = toggle(influences, patch.toggleInfluence);
  }
  if (order !== "flexible") influences = [];

  const config: BasicWordOrderConfig = {
    order,
    customOrder:
      order === "custom" ? (patch.customOrder !== undefined ? patch.customOrder : current?.customOrder) : undefined,
    strength: patch.strength === "" ? undefined : patch.strength !== undefined ? patch.strength : current?.strength,
    influences,
    customInfluence:
      order === "flexible" && influences.includes("custom")
        ? patch.customInfluence !== undefined
          ? patch.customInfluence
          : current?.customInfluence
        : undefined,
    changeNotes: patch.changeNotes !== undefined ? patch.changeNotes || undefined : current?.changeNotes,
  };
  return { ...draft, status: "configured", config };
}

export type PositionPatch<T extends string> = {
  position?: T;
  customPosition?: string;
  toggleAlternate?: T;
  conditions?: string;
};

export function applyAdjectivePosition(
  draft: GrammarSystemRecord,
  patch: PositionPatch<PositionChoice>,
): GrammarSystemRecord {
  if (draft.systemId !== "syntax.adjective-position") return draft;
  return applyPositionRecord(draft, ADJECTIVE_POSITION_OPTIONS, isPosition, patch);
}

export function applyPossessivePosition(
  draft: GrammarSystemRecord,
  patch: PositionPatch<PossessivePositionChoice>,
): GrammarSystemRecord {
  if (draft.systemId !== "syntax.possessive-position") return draft;
  return applyPositionRecord(draft, POSSESSIVE_POSITION_OPTIONS, isPossessive, patch);
}

export function applyRelativeClausePosition(
  draft: GrammarSystemRecord,
  patch: PositionPatch<RelativeClausePositionChoice>,
): GrammarSystemRecord {
  if (draft.systemId !== "syntax.relative-clause-position") return draft;
  return applyPositionRecord(draft, RELATIVE_CLAUSE_POSITION_OPTIONS, isRelative, patch);
}

export type AdpositionsPatch = {
  strategy?: AdpositionStrategy;
  distributionNotes?: string;
};

export function applyAdpositions(draft: GrammarSystemRecord, patch: AdpositionsPatch): GrammarSystemRecord {
  if (draft.systemId !== "syntax.adpositions") return draft;
  const current = isAdpositions(draft.config) ? draft.config : undefined;
  const strategy = patch.strategy ?? current?.strategy;
  if (!strategy) return { ...draft, status: "configured", config: {} };
  const notes =
    patch.distributionNotes !== undefined ? patch.distributionNotes || undefined : current?.distributionNotes;
  const config: AdpositionsConfig = {
    strategy,
    distributionNotes: strategy === "both" || strategy === "other" ? notes : undefined,
  };
  return { ...draft, status: "configured", config };
}

export function summarizeChoice(systemId: GrammarSystemId, config: GrammarSystemRecord["config"]): string | undefined {
  switch (systemId) {
    case "syntax.basic-word-order": {
      if (!isWordOrder(config)) return undefined;
      const parts = [choiceLabel(WORD_ORDER_OPTIONS, config.order)];
      if (config.order === "custom" && config.customOrder?.trim()) parts[0] = config.customOrder.trim();
      if (config.strength) parts.push(choiceLabel(WORD_ORDER_STRENGTH_OPTIONS, config.strength));
      return parts.filter(Boolean).join(" · ");
    }
    case "syntax.adjective-position":
      return positionSummary(config, ADJECTIVE_POSITION_OPTIONS, isPosition);
    case "syntax.possessive-position":
      return positionSummary(config, POSSESSIVE_POSITION_OPTIONS, isPossessive);
    case "syntax.relative-clause-position":
      return positionSummary(config, RELATIVE_CLAUSE_POSITION_OPTIONS, isRelative);
    case "syntax.adpositions":
      return isAdpositions(config) ? choiceLabel(ADPOSITION_OPTIONS, config.strategy) : undefined;
    default:
      return undefined;
  }
}

function toggle<T>(values: T[], item: T) {
  return values.includes(item) ? values.filter((value) => value !== item) : [...values, item];
}

function applyPositionRecord<T extends string>(
  draft: GrammarSystemRecord,
  options: ChoiceOption<T>[],
  guard: (config: GrammarSystemRecord["config"]) => boolean,
  patch: PositionPatch<T>,
): GrammarSystemRecord {
  const allowed = new Set(options.map((option) => option.value));
  type PositionRecord = { position: T; alternatePositions: T[]; customPosition?: string; conditions?: string };
  const current = guard(draft.config) ? (draft.config as PositionRecord) : undefined;
  const position = patch.position ?? current?.position;
  if (!position || !allowed.has(position)) return { ...draft, status: "configured", config: {} };
  let alternates = (current?.alternatePositions ?? []).filter((item) => item !== position && allowed.has(item));
  if (patch.toggleAlternate && patch.toggleAlternate !== position && allowed.has(patch.toggleAlternate)) {
    alternates = toggle(alternates, patch.toggleAlternate);
  }
  const config = {
    position,
    customPosition:
      position === "custom"
        ? patch.customPosition !== undefined
          ? patch.customPosition
          : current?.customPosition
        : undefined,
    alternatePositions: alternates,
    conditions: patch.conditions !== undefined ? patch.conditions || undefined : current?.conditions,
  };
  return { ...draft, status: "configured", config: config as GrammarSystemRecord["config"] };
}

function positionSummary(
  config: GrammarSystemRecord["config"],
  options: ChoiceOption<string>[],
  guard: (value: GrammarSystemRecord["config"]) => boolean,
) {
  if (!guard(config) || !("position" in config)) return undefined;
  const value = config as { position: string; customPosition?: string };
  if (value.position === "custom" && value.customPosition?.trim()) return value.customPosition.trim();
  return choiceLabel(options, value.position);
}

function isWordOrder(config: GrammarSystemRecord["config"]): config is BasicWordOrderConfig {
  return "order" in config;
}

function isPosition(config: GrammarSystemRecord["config"]): config is PositionConfig {
  return (
    "position" in config &&
    ADJECTIVE_POSITION_OPTIONS.some((option) => option.value === (config as PositionConfig).position)
  );
}

function isPossessive(config: GrammarSystemRecord["config"]): config is PossessivePositionConfig {
  return (
    "position" in config &&
    POSSESSIVE_POSITION_OPTIONS.some((option) => option.value === (config as PossessivePositionConfig).position)
  );
}

function isRelative(config: GrammarSystemRecord["config"]): config is RelativeClausePositionConfig {
  return (
    "position" in config &&
    RELATIVE_CLAUSE_POSITION_OPTIONS.some(
      (option) => option.value === (config as RelativeClausePositionConfig).position,
    )
  );
}

function isAdpositions(config: GrammarSystemRecord["config"]): config is AdpositionsConfig {
  return "strategy" in config;
}
