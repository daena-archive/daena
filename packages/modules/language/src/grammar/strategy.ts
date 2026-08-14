import { button, emptyMessage, field, input, textarea } from "../ui.ts";
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

export type StrategyEditorContext = {
  agreements: { id: string; title: string }[];
};

export function renderStrategyEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: StrategyEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
): HTMLElement | null {
  if (!isStrategySystem(draft.systemId)) return null;
  const section = document.createElement("section");
  section.className = "language-group grammar-choice-stack";
  if (draft.systemId === "nouns.definiteness") section.append(definitenessEditor(draft, locked, onChange));
  else if (draft.systemId === "nouns.possession") section.append(possessionEditor(draft, locked, onChange));
  else if (draft.systemId === "verbs.marking-strategy") section.append(verbMarkingEditor(draft, locked, onChange));
  else if (draft.systemId === "verbs.negative-forms") section.append(negativeEditor(draft, locked, onChange));
  else if (draft.systemId === "modifiers.adjective-behavior")
    section.append(adjectiveEditor(draft, locked, ctx, onChange));
  else section.append(degreeEditor(draft, locked, onChange));
  return section;
}

function definitenessEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = definitenessConfig(draft);
  wrap.append(
    emptyMessage("If the language has no grammatical definiteness distinction, mark this system as not used."),
    strategyChecks(DEFINITENESS_OPTIONS, config.strategies, locked, (value) =>
      onChange(toggleDefinitenessStrategy(draft, value as DefinitenessStrategy), true),
    ),
    emptyMessage("Article agreement belongs under Agreement. Record article forms here only."),
  );
  const usesArticles = config.strategies.some(
    (item) => item === "definite-article" || item === "indefinite-article" || item === "both" || item === "affixes",
  );
  if (usesArticles) {
    if (!locked)
      wrap.append(button("Add article form", "language-button secondary", () => onChange(addArticle(draft), true)));
    for (const [index, item] of config.articles.entries()) {
      wrap.append(
        rowCard(
          `Article ${index + 1}`,
          index,
          config.articles.length,
          locked,
          [
            namedField("form", "Form", item.form, locked, (value) =>
              onChange(updateArticle(draft, item.id, { form: value }), false),
            ),
            namedField("position", "Position", item.position ?? "", locked, (value) =>
              onChange(updateArticle(draft, item.id, { position: value }), false),
            ),
            namedArea("notes", "Notes", item.notes ?? "", locked, (value) =>
              onChange(updateArticle(draft, item.id, { notes: value }), false),
            ),
          ],
          {
            move: (delta) => onChange(moveArticle(draft, item.id, delta), true),
            remove: () => onChange(removeArticle(draft, item.id), true),
          },
        ),
      );
    }
  }
  return wrap;
}

function possessionEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = possessionConfig(draft);
  wrap.append(
    emptyMessage("Ordering of possessor and noun belongs under Syntax → Possessive position."),
    strategyChecks(POSSESSION_OPTIONS, config.strategies, locked, (value) =>
      onChange(togglePossessionStrategy(draft, value as PossessionStrategy), true),
    ),
  );
  const advanced = document.createElement("details");
  advanced.className = "grammar-learn";
  advanced.open = Boolean(config.alienability);
  const summary = document.createElement("summary");
  summary.textContent = "Advanced";
  advanced.append(summary);
  const yesNo = document.createElement("fieldset");
  yesNo.className = "grammar-status";
  const legend = document.createElement("legend");
  legend.textContent = "Does the language distinguish alienable and inalienable possession?";
  yesNo.append(legend);
  for (const [label, value] of [
    ["No", false],
    ["Yes", true],
  ] as const) {
    const row = document.createElement("label");
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "alienability";
    radio.checked = config.alienability === value;
    radio.disabled = locked;
    radio.onchange = () => onChange(setAlienability(draft, value), true);
    row.append(radio, ` ${label}`);
    yesNo.append(row);
  }
  advanced.append(yesNo);
  if (config.alienability) {
    const notes = textarea("alienabilityNotes", config.alienabilityNotes ?? "", 3);
    notes.disabled = locked;
    notes.oninput = () => onChange(setAlienabilityNotes(draft, notes.value), false);
    advanced.append(field("How does the distinction work?", notes));
  }
  wrap.append(advanced);
  return wrap;
}

function verbMarkingEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = verbMarkingConfig(draft);
  wrap.append(
    strategyChecks(VERB_MARKING_OPTIONS, config.strategies, locked, (value) =>
      onChange(toggleVerbMarking(draft, value as VerbMarkingStrategy), true),
    ),
  );
  if (config.strategies.includes("custom")) {
    const custom = input("customStrategy", config.customStrategy ?? "");
    custom.disabled = locked;
    custom.oninput = () => onChange(setCustomVerbMarking(draft, custom.value), false);
    wrap.append(field("Custom strategy", custom));
  }
  return wrap;
}

function negativeEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = negativeConfig(draft);
  wrap.append(
    emptyMessage("Clause Types → Negation owns particles and clause behavior. Do not enter the same marker twice."),
    strategyChecks(NEGATIVE_VERB_OPTIONS, config.strategies, locked, (value) =>
      onChange(toggleNegativeStrategy(draft, value as NegativeVerbStrategy), true),
    ),
  );
  if (!locked)
    wrap.append(button("Add negative form", "language-button secondary", () => onChange(addNegativeForm(draft), true)));
  for (const [index, item] of config.forms.entries()) {
    wrap.append(
      rowCard(
        item.form || `Form ${index + 1}`,
        index,
        config.forms.length,
        locked,
        [
          namedField("form", "Marker or form", item.form, locked, (value) =>
            onChange(updateNegativeForm(draft, item.id, { form: value }), false),
          ),
          namedArea("conditions", "Changes by tense or mood", item.conditions ?? "", locked, (value) =>
            onChange(updateNegativeForm(draft, item.id, { conditions: value }), false),
          ),
          namedArea("notes", "Notes", item.notes ?? "", locked, (value) =>
            onChange(updateNegativeForm(draft, item.id, { notes: value }), false),
          ),
        ],
        {
          move: (delta) => onChange(moveNegativeForm(draft, item.id, delta), true),
          remove: () => onChange(removeNegativeForm(draft, item.id), true),
        },
      ),
    );
  }
  return wrap;
}

function adjectiveEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: StrategyEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = adjectiveConfig(draft);
  wrap.append(
    emptyMessage("Placement is configured under Syntax → Adjective position."),
    strategyChecks(ADJECTIVE_BEHAVIOR_OPTIONS, config.behaviors, locked, (value) =>
      onChange(toggleAdjectiveBehavior(draft, value as AdjectiveBehaviorKind), true),
    ),
  );
  if (config.behaviors.includes("custom")) {
    const custom = input("customBehavior", config.customBehavior ?? "");
    custom.disabled = locked;
    custom.oninput = () => onChange(setCustomAdjectiveBehavior(draft, custom.value), false);
    wrap.append(field("Custom behavior", custom));
  }
  if (config.behaviors.includes("agree-with-noun")) {
    wrap.append(
      emptyMessage("Link the Agreement system that describes adjective agreement. Do not copy those rules here."),
    );
    if (ctx.agreements.length === 0) wrap.append(emptyMessage("No agreement systems are configured yet."));
    else {
      wrap.append(
        strategyChecks(
          ctx.agreements.map((item) => ({ value: item.id, label: item.title })),
          config.agreementRecordIds,
          locked,
          (value) => onChange(toggleAgreementRecord(draft, value), true),
        ),
      );
    }
  }
  return wrap;
}

function degreeEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = degreeConfig(draft);
  const options = draft.systemId === "modifiers.superlative" ? SUPERLATIVE_OPTIONS : COMPARATIVE_OPTIONS;
  wrap.append(
    strategyChecks(options, config.strategies, locked, (value) => onChange(toggleDegreeStrategy(draft, value), true)),
  );
  const marker = input("marker", config.marker ?? "");
  marker.disabled = locked;
  marker.oninput = () => onChange(setDegreeMarker(draft, marker.value), false);
  const construction = textarea("construction", config.construction ?? "", 3);
  construction.disabled = locked;
  construction.oninput = () => onChange(setDegreeConstruction(draft, construction.value), false);
  wrap.append(
    field("Marker", marker),
    field("Construction", construction),
    emptyMessage("Irregular forms can be recorded as examples."),
  );
  return wrap;
}

function strategyChecks(
  options: StrategyOption[],
  selected: string[],
  locked: boolean,
  onToggle: (value: string) => void,
) {
  const group = document.createElement("fieldset");
  group.className = "grammar-checks";
  const legend = document.createElement("legend");
  legend.textContent = "Strategies";
  group.append(legend);
  for (const option of options) {
    const label = document.createElement("label");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.value = option.value;
    box.checked = selected.includes(option.value);
    box.disabled = locked;
    box.onchange = () => onToggle(option.value);
    label.append(box, ` ${option.label}`);
    if (option.expansion) {
      const hint = document.createElement("span");
      hint.className = "grammar-template-hint";
      hint.textContent = option.expansion;
      label.append(hint);
    }
    if (option.example) {
      const example = document.createElement("em");
      example.className = "grammar-template-hint";
      example.textContent = option.example;
      label.append(example);
    }
    group.append(label);
  }
  return group;
}

function rowCard(
  titleText: string,
  index: number,
  total: number,
  locked: boolean,
  fields: HTMLElement[],
  actions: { move: (delta: number) => void; remove: () => void },
) {
  const card = document.createElement("article");
  card.className = "grammar-inventory-item";
  const head = document.createElement("div");
  head.className = "grammar-inventory-toolbar";
  const title = document.createElement("strong");
  title.textContent = titleText;
  head.append(title);
  if (!locked) {
    const up = button("Up", "language-button secondary", () => actions.move(-1));
    const down = button("Down", "language-button secondary", () => actions.move(1));
    up.disabled = index === 0;
    down.disabled = index === total - 1;
    head.append(up, down, button("Remove", "language-button secondary language-danger", actions.remove));
  }
  card.append(head, ...fields);
  return card;
}

function namedField(name: string, labelText: string, value: string, locked: boolean, onInput: (value: string) => void) {
  const control = input(name, value);
  control.disabled = locked;
  control.oninput = () => onInput(control.value);
  return field(labelText, control);
}

function namedArea(name: string, labelText: string, value: string, locked: boolean, onInput: (value: string) => void) {
  const control = textarea(name, value, 2);
  control.disabled = locked;
  control.oninput = () => onInput(control.value);
  return field(labelText, control);
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
