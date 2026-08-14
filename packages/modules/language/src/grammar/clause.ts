import { button, emptyMessage, field, input, textarea } from "../ui.ts";
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
    return setContent(draft, { ...config, interrogatives: config.interrogatives.filter((item) => item.id !== existing.id) });
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

export function toggleImperativeStrategy(draft: GrammarSystemRecord, strategy: ImperativeStrategy): GrammarSystemRecord {
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

export function toggleNegationStrategy(draft: GrammarSystemRecord, strategy: ClauseNegationStrategy): GrammarSystemRecord {
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

export function toggleRelativization(draft: GrammarSystemRecord, strategy: RelativizationStrategy): GrammarSystemRecord {
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
          : CONTENT_QUESTION_OPTIONS.find((item) => item.value === value.behavior)?.label ?? value.behavior;
      const words = value.interrogatives?.map((item) => item.meaning).filter(Boolean).join(", ");
      return [behavior, words].filter(Boolean).join(" · ");
    }
    case "clauses.imperatives":
      return joinLabels(IMPERATIVE_OPTIONS, (config as ImperativesConfig).strategies);
    case "clauses.negation": {
      const value = config as ClauseNegationConfig;
      return join([joinLabels(CLAUSE_NEGATION_OPTIONS, value.strategies), value.particle ? `“${value.particle}”` : undefined]);
    }
    case "clauses.relative-clauses":
      return joinLabels(RELATIVIZATION_OPTIONS, (config as RelativeClausesConfig).strategies);
    default:
      return undefined;
  }
}

export type ClauseEditorContext = {
  lexemes: { id: string; lemma: string }[];
  negativeVerbSummary?: string;
  relativePositionSummary?: string;
};

export function renderClauseEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ClauseEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
): HTMLElement | null {
  if (!isClauseSystem(draft.systemId)) return null;
  const section = document.createElement("section");
  section.className = "language-group grammar-choice-stack";
  if (draft.systemId === "clauses.yes-no-questions") section.append(yesNoEditor(draft, locked, onChange));
  else if (draft.systemId === "clauses.content-questions") section.append(contentEditor(draft, locked, ctx, onChange));
  else if (draft.systemId === "clauses.imperatives") section.append(imperativeEditor(draft, locked, onChange));
  else if (draft.systemId === "clauses.negation") section.append(negationEditor(draft, locked, ctx, onChange));
  else section.append(relativeEditor(draft, locked, ctx, onChange));
  return section;
}

function yesNoEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = yesNoConfig(draft);
  wrap.append(
    checks("How are yes/no questions formed?", YES_NO_OPTIONS, config.strategies, locked, (value) =>
      onChange(toggleYesNoStrategy(draft, value as YesNoQuestionStrategy), true),
    ),
  );
  if (config.strategies.includes("particle")) {
    const particle = input("particle", config.particle ?? "");
    particle.disabled = locked;
    particle.oninput = () => onChange(setYesNoParticle(draft, particle.value), false);
    wrap.append(field("Particle", particle), placementRadios(config.placement, locked, (value) => onChange(setYesNoPlacement(draft, value), true)));
  }
  return wrap;
}

function contentEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ClauseEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = contentConfig(draft);
  wrap.append(radios("Where do question words appear?", CONTENT_QUESTION_OPTIONS, config.behavior, locked, (value) =>
    onChange(setContentBehavior(draft, value as ContentQuestionBehavior), true),
  ));
  if (config.behavior === "custom") {
    const custom = input("customBehavior", config.customBehavior ?? "");
    custom.disabled = locked;
    custom.oninput = () => onChange(setContentCustomBehavior(draft, custom.value), false);
    wrap.append(field("Custom behavior", custom));
  }
  const selected = new Set(config.interrogatives.map((item) => item.meaning));
  wrap.append(
    checks(
      "Common interrogatives",
      INTERROGATIVE_TEMPLATES.map((meaning) => ({ value: meaning, label: meaning })),
      [...selected],
      locked,
      (value) => onChange(toggleInterrogative(draft, value), true),
    ),
  );
  if (!locked) wrap.append(button("Add interrogative", "language-button secondary", () => onChange(addInterrogative(draft), true)));
  for (const [index, item] of config.interrogatives.entries()) {
    const lexeme = document.createElement("select");
    lexeme.name = "lexemeId";
    lexeme.disabled = locked;
    lexeme.append(new Option("Not linked to a word", ""));
    for (const choice of ctx.lexemes) lexeme.append(new Option(choice.lemma, choice.id));
    lexeme.value = item.lexemeId ?? "";
    lexeme.onchange = () => onChange(updateInterrogative(draft, item.id, { lexemeId: lexeme.value || undefined }), true);
    wrap.append(
      rowCard(item.meaning || `Interrogative ${index + 1}`, index, config.interrogatives.length, locked, [
        namedField("meaning", "Meaning", item.meaning, locked, (value) =>
          onChange(updateInterrogative(draft, item.id, { meaning: value }), false),
        ),
        namedField("form", "Form", item.form ?? "", locked, (value) =>
          onChange(updateInterrogative(draft, item.id, { form: value }), false),
        ),
        field("Linked word (optional)", lexeme),
      ], {
        move: (delta) => onChange(moveInterrogative(draft, item.id, delta), true),
        remove: () => onChange(removeInterrogative(draft, item.id), true),
      }),
    );
  }
  wrap.append(emptyMessage("Interrogatives do not become lexicon entries unless you link a word."));
  return wrap;
}

function imperativeEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = imperativeConfig(draft);
  wrap.append(
    checks("How are commands formed?", IMPERATIVE_OPTIONS, config.strategies, locked, (value) =>
      onChange(toggleImperativeStrategy(draft, value as ImperativeStrategy), true),
    ),
  );
  const advanced = document.createElement("details");
  advanced.className = "grammar-learn";
  advanced.open = Boolean(config.numberDistinction || config.polarityDistinction || config.politenessDistinction);
  const summary = document.createElement("summary");
  summary.textContent = "Advanced";
  advanced.append(summary);
  for (const [key, label] of [
    ["numberDistinction", "Singular vs plural imperative"],
    ["polarityDistinction", "Positive vs negative imperative"],
    ["politenessDistinction", "Polite imperative"],
  ] as const) {
    const row = document.createElement("label");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = Boolean(config[key]);
    box.disabled = locked;
    box.onchange = () => onChange(setImperativeDistinction(draft, key, box.checked), true);
    row.append(box, ` ${label}`);
    advanced.append(row);
  }
  wrap.append(advanced);
  return wrap;
}

function negationEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ClauseEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = negationConfig(draft);
  wrap.append(
    emptyMessage("This editor owns clause negation. Do not re-enter negative verb morphology configured under Verbs."),
    checks("Primary strategy", CLAUSE_NEGATION_OPTIONS, config.strategies, locked, (value) =>
      onChange(toggleNegationStrategy(draft, value as ClauseNegationStrategy), true),
    ),
  );
  if (ctx.negativeVerbSummary) wrap.append(emptyMessage(`Negative verb forms: ${ctx.negativeVerbSummary}`));
  if (config.strategies.includes("particle")) {
    const particle = input("particle", config.particle ?? "");
    particle.disabled = locked;
    particle.oninput = () => onChange(setNegationParticle(draft, particle.value), false);
    wrap.append(field("Particle", particle), placementRadios(config.placement, locked, (value) => onChange(setNegationPlacement(draft, value), true)));
  }
  const questions = textarea("negativeQuestions", config.negativeQuestions ?? "", 3);
  questions.disabled = locked;
  questions.oninput = () => onChange(setNegationQuestions(draft, questions.value), false);
  const imperatives = textarea("negativeImperatives", config.negativeImperatives ?? "", 3);
  imperatives.disabled = locked;
  imperatives.oninput = () => onChange(setNegationImperatives(draft, imperatives.value), false);
  wrap.append(field("Negative questions", questions), field("Negative imperatives", imperatives));
  return wrap;
}

function relativeEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ClauseEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = relativeConfig(draft);
  wrap.append(
    emptyMessage("Placement relative to the head noun is configured under Syntax → Relative clause position."),
    checks("Relativization strategy", RELATIVIZATION_OPTIONS, config.strategies, locked, (value) =>
      onChange(toggleRelativization(draft, value as RelativizationStrategy), true),
    ),
  );
  if (ctx.relativePositionSummary) wrap.append(emptyMessage(`Relative clause position: ${ctx.relativePositionSummary}`));
  const head = textarea("headBehavior", config.headBehavior ?? "", 3);
  head.disabled = locked;
  head.oninput = () => onChange(setRelativeHeadBehavior(draft, head.value), false);
  const resumptives = textarea("resumptives", config.resumptives ?? "", 3);
  resumptives.disabled = locked;
  resumptives.oninput = () => onChange(setRelativeResumptives(draft, resumptives.value), false);
  wrap.append(field("Head behavior", head), field("Resumptives or gaps", resumptives));
  return wrap;
}

function checks(legendText: string, options: ClauseOption[], selected: string[], locked: boolean, onToggle: (value: string) => void) {
  const group = document.createElement("fieldset");
  group.className = "grammar-checks";
  const legend = document.createElement("legend");
  legend.textContent = legendText;
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
    group.append(label);
  }
  return group;
}

function radios(legendText: string, options: ClauseOption[], selected: string | undefined, locked: boolean, onChange: (value: string) => void) {
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
    radio.name = "behavior";
    radio.value = option.value;
    radio.checked = option.value === selected;
    radio.disabled = locked;
    radio.onchange = () => onChange(option.value);
    const title = document.createElement("strong");
    title.textContent = option.label;
    card.append(radio, title);
    group.append(card);
  }
  return group;
}

function placementRadios(selected: ParticlePlacement | undefined, locked: boolean, onChange: (value: ParticlePlacement) => void) {
  const group = document.createElement("fieldset");
  group.className = "grammar-status";
  const legend = document.createElement("legend");
  legend.textContent = "Position";
  group.append(legend);
  for (const option of PLACEMENT_OPTIONS) {
    const label = document.createElement("label");
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "placement";
    radio.value = option.value;
    radio.checked = option.value === selected;
    radio.disabled = locked;
    radio.onchange = () => onChange(option.value);
    label.append(radio, ` ${option.label}`);
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
  return values.map((value) => options.find((option) => option.value === value)?.label ?? value.replaceAll("-", " ")).join(" / ");
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
