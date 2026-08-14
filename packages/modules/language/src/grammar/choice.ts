import { emptyMessage, field, input, textarea } from "../ui.ts";
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

export function renderChoiceEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
): HTMLElement | null {
  if (!isChoiceSystem(draft.systemId)) return null;
  const section = document.createElement("section");
  section.className = "language-group grammar-choice-editor";
  if (draft.systemId === "syntax.basic-word-order") {
    section.append(wordOrderEditor(draft, locked, onChange));
  } else if (draft.systemId === "syntax.adjective-position") {
    section.append(
      positionEditor(draft, locked, ADJECTIVE_POSITION_OPTIONS, applyAdjectivePosition, onChange, {
        legend: "Usual adjective position",
        conditionsLabel: "Does adjective position change in special situations?",
      }),
    );
  } else if (draft.systemId === "syntax.possessive-position") {
    section.append(
      positionEditor(draft, locked, POSSESSIVE_POSITION_OPTIONS, applyPossessivePosition, onChange, {
        legend: "Usual possessive position",
        conditionsLabel: "When does this change?",
      }),
    );
  } else if (draft.systemId === "syntax.relative-clause-position") {
    section.append(
      positionEditor(draft, locked, RELATIVE_CLAUSE_POSITION_OPTIONS, applyRelativeClausePosition, onChange, {
        legend: "Usual relative-clause position",
        conditionsLabel: "When does this change?",
        note: "Detailed relative-clause behavior belongs under Clause Types.",
      }),
    );
  } else {
    section.append(adpositionsEditor(draft, locked, onChange));
  }
  return section;
}

function wordOrderEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = isWordOrder(draft.config) ? draft.config : undefined;
  wrap.append(
    choiceCards("order", "Usual order", WORD_ORDER_OPTIONS, config?.order, locked, (order) => {
      onChange(applyBasicWordOrder(draft, { order }), true);
    }),
  );
  if (config?.order === "custom") {
    const custom = input("customOrder", config.customOrder ?? "");
    custom.disabled = locked;
    custom.placeholder = "Describe the usual order.";
    custom.oninput = () => onChange(applyBasicWordOrder(draft, { customOrder: custom.value }), false);
    wrap.append(field("Custom order", custom));
  }
  if (config?.order) {
    wrap.append(
      radioRow(
        "strength",
        "How strong is this ordering?",
        WORD_ORDER_STRENGTH_OPTIONS,
        config.strength,
        locked,
        (strength) => {
          onChange(applyBasicWordOrder(draft, { strength }), true);
        },
      ),
    );
    const notes = textarea("changeNotes", config.changeNotes ?? "", 3);
    notes.disabled = locked;
    notes.oninput = () => onChange(applyBasicWordOrder(draft, { changeNotes: notes.value }), false);
    wrap.append(field("What can cause the order to change?", notes));
  }
  if (config?.order === "flexible") {
    wrap.append(
      checkRow(
        "influences",
        "What can influence the order?",
        WORD_ORDER_INFLUENCE_OPTIONS,
        config.influences,
        locked,
        (influence) => onChange(applyBasicWordOrder(draft, { toggleInfluence: influence as WordOrderInfluence }), true),
      ),
    );
    if (config.influences.includes("custom")) {
      const custom = input("customInfluence", config.customInfluence ?? "");
      custom.disabled = locked;
      custom.oninput = () => onChange(applyBasicWordOrder(draft, { customInfluence: custom.value }), false);
      wrap.append(field("Custom influence", custom));
    }
  }
  return wrap;
}

function positionEditor<T extends string>(
  draft: GrammarSystemRecord,
  locked: boolean,
  options: ChoiceOption<T>[],
  apply: (draft: GrammarSystemRecord, patch: PositionPatch<T>) => GrammarSystemRecord,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
  copy: { legend: string; conditionsLabel: string; note?: string },
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = asPosition(draft.config);
  wrap.append(
    choiceCards("position", copy.legend, options, config?.position as T | undefined, locked, (position) => {
      onChange(apply(draft, { position }), true);
    }),
  );
  if (copy.note) wrap.append(emptyMessage(copy.note));
  if (config?.position === "custom") {
    const custom = input("customPosition", config.customPosition ?? "");
    custom.disabled = locked;
    custom.oninput = () => onChange(apply(draft, { customPosition: custom.value }), false);
    wrap.append(field("Custom position", custom));
  }
  if (config?.position) {
    const alternates = options.filter((option) => option.value !== config.position);
    wrap.append(
      checkRow(
        "alternatePositions",
        "Other positions that also occur (optional)",
        alternates,
        config.alternatePositions,
        locked,
        (value) => onChange(apply(draft, { toggleAlternate: value as T }), true),
      ),
    );
    const conditions = textarea("conditions", config.conditions ?? "", 3);
    conditions.disabled = locked;
    conditions.oninput = () => onChange(apply(draft, { conditions: conditions.value }), false);
    wrap.append(field(copy.conditionsLabel, conditions));
  }
  return wrap;
}

function adpositionsEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = isAdpositions(draft.config) ? draft.config : undefined;
  wrap.append(
    choiceCards("strategy", "Adposition strategy", ADPOSITION_OPTIONS, config?.strategy, locked, (strategy) => {
      onChange(applyAdpositions(draft, { strategy }), true);
    }),
    emptyMessage(
      "If this language does not use adpositions, mark the system as not used. Case is configured under Nouns.",
    ),
  );
  if (config?.strategy === "both" || config?.strategy === "other") {
    const notes = textarea("distributionNotes", config.distributionNotes ?? "", 3);
    notes.disabled = locked;
    notes.placeholder = config.strategy === "both" ? "When does each appear?" : "Describe the strategy.";
    notes.oninput = () => onChange(applyAdpositions(draft, { distributionNotes: notes.value }), false);
    wrap.append(field(config.strategy === "both" ? "When does each appear?" : "Describe the strategy", notes));
  }
  return wrap;
}

function choiceCards<T extends string>(
  name: string,
  legendText: string,
  options: ChoiceOption<T>[],
  selected: T | undefined,
  locked: boolean,
  onChange: (value: T) => void,
) {
  const group = document.createElement("fieldset");
  group.className = "grammar-choices";
  const legend = document.createElement("legend");
  legend.textContent = legendText;
  group.append(legend);
  for (const option of options) {
    const card = document.createElement("label");
    card.className = "grammar-choice";
    if (option.value === selected) card.classList.add("is-selected");
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = name;
    radio.value = option.value;
    radio.checked = option.value === selected;
    radio.disabled = locked;
    radio.onchange = () => onChange(option.value);
    const title = document.createElement("strong");
    title.textContent = option.label;
    card.append(radio, title);
    if (option.expansion) {
      const expansion = document.createElement("span");
      expansion.textContent = option.expansion;
      card.append(expansion);
    }
    if (option.example) {
      const example = document.createElement("em");
      example.textContent = option.example;
      card.append(example);
    }
    group.append(card);
  }
  return group;
}

function radioRow<T extends string>(
  name: string,
  legendText: string,
  options: ChoiceOption<T>[],
  selected: T | undefined,
  locked: boolean,
  onChange: (value: T) => void,
) {
  const group = document.createElement("fieldset");
  group.className = "grammar-status";
  const legend = document.createElement("legend");
  legend.textContent = legendText;
  group.append(legend);
  for (const option of options) {
    const label = document.createElement("label");
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = name;
    radio.value = option.value;
    radio.checked = option.value === selected;
    radio.disabled = locked;
    radio.onchange = () => onChange(option.value);
    label.append(radio, ` ${option.label}`);
    group.append(label);
  }
  return group;
}

function checkRow<T extends string>(
  name: string,
  legendText: string,
  options: ChoiceOption<T>[],
  selected: string[],
  locked: boolean,
  onToggle: (value: T) => void,
) {
  const group = document.createElement("fieldset");
  group.className = "grammar-checks";
  const legend = document.createElement("legend");
  legend.textContent = legendText;
  group.append(legend);
  for (const option of options) {
    const label = document.createElement("label");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.name = name;
    box.value = option.value;
    box.checked = selected.includes(option.value);
    box.disabled = locked;
    box.onchange = () => onToggle(option.value);
    label.append(box, ` ${option.label}`);
    group.append(label);
  }
  return group;
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

function asPosition(config: GrammarSystemRecord["config"]) {
  if (isPosition(config) || isPossessive(config) || isRelative(config)) return config;
  return undefined;
}
