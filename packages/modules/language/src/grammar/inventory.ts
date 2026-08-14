import { button, emptyMessage, field, input, textarea } from "../ui.ts";
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
  { id: "noun-class", label: "Noun class system", meaning: "A larger set of grammatical classes, not necessarily gendered." },
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
  { id: "imperfective", label: "Imperfective", meaning: "Presents an event as ongoing, habitual, or internally structured." },
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
    return { draft: setNumber(draft, { ...config, categories: append(config.categories, numberFromTemplate("custom")) }) };
  }
  const existing = config.categories.find((item) => item.templateId === templateId);
  if (existing) return removeById(draft, existing.id, options);
  return { draft: setNumber(draft, { ...config, categories: append(config.categories, numberFromTemplate(templateId)) }) };
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

export function removeNumberCategory(draft: GrammarSystemRecord, id: string, options?: { force?: boolean; referenced?: Set<string> }) {
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

export function updateCase(draft: GrammarSystemRecord, id: string, patch: Partial<Omit<CaseItem, "id">>): GrammarSystemRecord {
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

export function removeCase(draft: GrammarSystemRecord, id: string, options?: { force?: boolean; referenced?: Set<string> }) {
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

export function removeNounClass(draft: GrammarSystemRecord, id: string, options?: { force?: boolean; referenced?: Set<string> }) {
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

export function removeTamCategory(draft: GrammarSystemRecord, id: string, options?: { force?: boolean; referenced?: Set<string> }) {
  return removeById(draft, id, options);
}

export type InventoryEditorContext = {
  referencedIds: Set<string>;
  confirm: (message: string) => boolean;
};

export function renderInventoryEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: InventoryEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
): HTMLElement | null {
  if (!isInventorySystem(draft.systemId)) return null;
  const section = document.createElement("section");
  section.className = "language-group grammar-inventory";
  if (draft.systemId === "nouns.number") section.append(numberEditor(draft, locked, ctx, onChange));
  else if (draft.systemId === "nouns.case") section.append(caseEditor(draft, locked, ctx, onChange));
  else if (draft.systemId === "nouns.classes") section.append(classEditor(draft, locked, ctx, onChange));
  else section.append(tamEditor(draft, locked, ctx, onChange));
  return section;
}

function numberEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: InventoryEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = numberConfig(draft);
  const selected = new Set(config.categories.map((item) => item.templateId).filter(Boolean));
  wrap.append(
    templateChecks(
      "Number categories",
      NUMBER_TEMPLATES.filter((item) => item.id !== "custom"),
      selected,
      locked,
      (templateId) => applyMutation(toggleNumberTemplate(draft, templateId as NumberCategoryId, { referenced: ctx.referencedIds }), ctx, onChange),
    ),
  );
  if (!locked) {
    wrap.append(
      button("Add custom category", "language-button secondary", () => {
        applyMutation(toggleNumberTemplate(draft, "custom"), ctx, onChange);
      }),
    );
  }
  wrap.append(
    templateChecks("How is number usually expressed?", NUMBER_MARKING_OPTIONS, new Set(config.markingStrategies), locked, (id) => {
      onChange(toggleNumberMarking(draft, id as MarkingStrategy), true);
    }),
  );
  for (const [index, item] of config.categories.entries()) {
    wrap.append(
      itemCard(
        item.label || "Number category",
        index,
        config.categories.length,
        ctx.referencedIds.has(item.id),
        locked,
        [
          namedField("label", "Label", item.label, locked, (value) => onChange(updateNumberCategory(draft, item.id, { label: value }), false)),
          namedField("meaning", "Meaning", item.meaning ?? "", locked, (value) =>
            onChange(updateNumberCategory(draft, item.id, { meaning: value }), false),
          ),
          namedField("marker", "Marker", item.marker ?? "", locked, (value) =>
            onChange(updateNumberCategory(draft, item.id, { marker: value }), false),
          ),
          namedField("position", "Position", item.position ?? "", locked, (value) =>
            onChange(updateNumberCategory(draft, item.id, { position: value }), false),
          ),
          namedArea("notes", "Notes", item.notes ?? "", locked, (value) =>
            onChange(updateNumberCategory(draft, item.id, { notes: value }), false),
          ),
        ],
        {
          move: (delta) => onChange(moveNumberCategory(draft, item.id, delta), true),
          remove: () => applyMutation(removeNumberCategory(draft, item.id, { referenced: ctx.referencedIds }), ctx, onChange),
        },
      ),
    );
  }
  return wrap;
}

function caseEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: InventoryEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  wrap.append(emptyMessage("Case names are convenient labels, not universal exact meanings."));
  const config = caseConfig(draft);
  if (!locked) {
    const add = document.createElement("select");
    add.setAttribute("aria-label", "Add a case");
    add.append(new Option("Add a case…", ""));
    for (const template of CASE_TEMPLATES) add.append(new Option(template.label, template.id));
    add.onchange = () => {
      if (!add.value) return;
      onChange(addCase(draft, add.value as CaseTemplateId), true);
    };
    wrap.append(add);
  }
  for (const [index, item] of config.cases.entries()) {
    wrap.append(
      itemCard(
        item.name || "Case",
        index,
        config.cases.length,
        ctx.referencedIds.has(item.id),
        locked,
        [
          namedField("name", "Name", item.name, locked, (value) => onChange(updateCase(draft, item.id, { name: value }), false)),
          namedField("abbreviation", "Abbreviation", item.abbreviation ?? "", locked, (value) =>
            onChange(updateCase(draft, item.id, { abbreviation: value }), false),
          ),
          namedArea("primaryFunction", "Primary function", item.primaryFunction, locked, (value) =>
            onChange(updateCase(draft, item.id, { primaryFunction: value }), false),
          ),
          namedArea("additionalFunctions", "Additional functions", item.additionalFunctions ?? "", locked, (value) =>
            onChange(updateCase(draft, item.id, { additionalFunctions: value }), false),
          ),
          namedField("marking", "How it is marked", item.marking ?? "", locked, (value) =>
            onChange(updateCase(draft, item.id, { marking: value }), false),
          ),
          namedArea("notes", "Notes", item.notes ?? "", locked, (value) => onChange(updateCase(draft, item.id, { notes: value }), false)),
        ],
        {
          move: (delta) => onChange(moveCase(draft, item.id, delta), true),
          remove: () => applyMutation(removeCase(draft, item.id, { referenced: ctx.referencedIds }), ctx, onChange),
        },
      ),
    );
  }
  return wrap;
}

function classEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: InventoryEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  wrap.append(
    emptyMessage("If this language has no grammatical classes, mark the system as not used. Agreement behavior belongs under Agreement."),
  );
  const config = classConfig(draft);
  wrap.append(
    radioKind(NOUN_CLASS_KIND_OPTIONS, config.kind, locked, (kind) => onChange(setNounClassKind(draft, kind), true)),
  );
  if (!locked) {
    wrap.append(button("Add class", "language-button secondary", () => onChange(addNounClass(draft), true)));
  }
  for (const [index, item] of config.classes.entries()) {
    wrap.append(
      itemCard(
        item.name || "Class",
        index,
        config.classes.length,
        ctx.referencedIds.has(item.id),
        locked,
        [
          namedField("name", "Name", item.name, locked, (value) => onChange(updateNounClass(draft, item.id, { name: value }), false)),
          namedField("abbreviation", "Abbreviation", item.abbreviation ?? "", locked, (value) =>
            onChange(updateNounClass(draft, item.id, { abbreviation: value }), false),
          ),
          namedArea("membership", "Typical membership", item.membership ?? "", locked, (value) =>
            onChange(updateNounClass(draft, item.id, { membership: value }), false),
          ),
          namedArea("exceptions", "Exceptions", item.exceptions ?? "", locked, (value) =>
            onChange(updateNounClass(draft, item.id, { exceptions: value }), false),
          ),
        ],
        {
          move: (delta) => onChange(moveNounClass(draft, item.id, delta), true),
          remove: () => applyMutation(removeNounClass(draft, item.id, { referenced: ctx.referencedIds }), ctx, onChange),
        },
      ),
    );
  }
  return wrap;
}

function tamEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: InventoryEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const templates = tamTemplates(draft.systemId);
  const config = tamConfig(draft);
  const selected = new Set(config.categories.map((item) => item.templateId).filter(Boolean));
  const common = templates.filter((item) => !item.more && item.id !== "custom");
  const extra = templates.filter((item) => item.more);
  wrap.append(
    templateChecks("Categories", common, selected, locked, (templateId) =>
      applyMutation(toggleTamTemplate(draft, templateId, { referenced: ctx.referencedIds }), ctx, onChange),
    ),
  );
  if (extra.length) {
    const more = document.createElement("details");
    more.className = "grammar-learn";
    if (extra.some((item) => selected.has(item.id))) more.open = true;
    const summary = document.createElement("summary");
    summary.textContent = "More";
    more.append(
      summary,
      templateChecks("Additional categories", extra, selected, locked, (templateId) =>
        applyMutation(toggleTamTemplate(draft, templateId, { referenced: ctx.referencedIds }), ctx, onChange),
      ),
    );
    wrap.append(more);
  }
  if (!locked) {
    wrap.append(
      button("Add custom category", "language-button secondary", () => {
        applyMutation(toggleTamTemplate(draft, "custom"), ctx, onChange);
      }),
    );
  }
  for (const [index, item] of config.categories.entries()) {
    wrap.append(
      itemCard(
        item.label || "Category",
        index,
        config.categories.length,
        ctx.referencedIds.has(item.id),
        locked,
        [
          namedField("label", "Label", item.label, locked, (value) => onChange(updateTamCategory(draft, item.id, { label: value }), false)),
          namedArea("meaning", "Meaning", item.meaning ?? "", locked, (value) =>
            onChange(updateTamCategory(draft, item.id, { meaning: value }), false),
          ),
          namedField("marker", "Marker or construction", item.marker ?? "", locked, (value) =>
            onChange(updateTamCategory(draft, item.id, { marker: value }), false),
          ),
          namedArea("interaction", "Interaction notes", item.interaction ?? "", locked, (value) =>
            onChange(updateTamCategory(draft, item.id, { interaction: value }), false),
          ),
          namedArea("notes", "Notes", item.notes ?? "", locked, (value) =>
            onChange(updateTamCategory(draft, item.id, { notes: value }), false),
          ),
        ],
        {
          move: (delta) => onChange(moveTamCategory(draft, item.id, delta), true),
          remove: () => applyMutation(removeTamCategory(draft, item.id, { referenced: ctx.referencedIds }), ctx, onChange),
        },
      ),
    );
  }
  return wrap;
}

function applyMutation(
  result: InventoryMutation,
  ctx: InventoryEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  if (result.blocked) {
    if (
      !ctx.confirm(
        `“${result.blocked.label}” is referenced by agreement. Remove it anyway? Agreement will keep the broken reference until you edit it.`,
      )
    ) {
      return;
    }
    const forced = removeById(result.draft, result.blocked.id, { force: true });
    onChange(forced.draft, true);
    return;
  }
  onChange(result.draft, true);
}

function removeById(
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
    return { draft: setClasses(draft, { kind: config.kind ?? "gender", classes: config.classes.filter((item) => item.id !== id) }) };
  }
  if (isTam(draft.systemId)) {
    const config = tamConfig(draft);
    return { draft: setTam(draft, { categories: config.categories.filter((item) => item.id !== id) }) };
  }
  return { draft };
}

function itemLabel(draft: GrammarSystemRecord, id: string) {
  if (draft.systemId === "nouns.number") return numberConfig(draft).categories.find((item) => item.id === id)?.label ?? "category";
  if (draft.systemId === "nouns.case") return caseConfig(draft).cases.find((item) => item.id === id)?.name ?? "case";
  if (draft.systemId === "nouns.classes") return classConfig(draft).classes.find((item) => item.id === id)?.name ?? "class";
  if (isTam(draft.systemId)) return tamConfig(draft).categories.find((item) => item.id === id)?.label ?? "category";
  return "category";
}

function templateChecks(
  legendText: string,
  templates: InventoryTemplate[],
  selected: Set<string | undefined>,
  locked: boolean,
  onToggle: (id: string) => void,
) {
  const group = document.createElement("fieldset");
  group.className = "grammar-checks";
  const legend = document.createElement("legend");
  legend.textContent = legendText;
  group.append(legend);
  for (const template of templates) {
    const label = document.createElement("label");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.value = template.id;
    box.checked = selected.has(template.id);
    box.disabled = locked;
    box.onchange = () => onToggle(template.id);
    label.append(box, ` ${template.label}`);
    if (template.meaning) {
      const hint = document.createElement("span");
      hint.className = "grammar-template-hint";
      hint.textContent = template.meaning;
      label.append(hint);
    }
    group.append(label);
  }
  return group;
}

function radioKind(
  options: InventoryTemplate<NounClassKind>[],
  selected: NounClassKind | undefined,
  locked: boolean,
  onChange: (kind: NounClassKind) => void,
) {
  const group = document.createElement("fieldset");
  group.className = "grammar-choices";
  const legend = document.createElement("legend");
  legend.textContent = "What kind of classification is this?";
  group.append(legend);
  for (const option of options) {
    const card = document.createElement("label");
    card.className = "grammar-choice";
    if (option.id === selected) card.classList.add("is-selected");
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "kind";
    radio.value = option.id;
    radio.checked = option.id === selected;
    radio.disabled = locked;
    radio.onchange = () => onChange(option.id);
    const title = document.createElement("strong");
    title.textContent = option.label;
    card.append(radio, title);
    if (option.meaning) {
      const meaning = document.createElement("span");
      meaning.textContent = option.meaning;
      card.append(meaning);
    }
    group.append(card);
  }
  return group;
}

function itemCard(
  titleText: string,
  index: number,
  total: number,
  referenced: boolean,
  locked: boolean,
  fields: HTMLElement[],
  actions: { move: (delta: number) => void; remove: () => void },
) {
  const card = document.createElement("article");
  card.className = "grammar-inventory-item";
  card.setAttribute("role", "listitem");
  const head = document.createElement("div");
  head.className = "grammar-inventory-toolbar";
  const title = document.createElement("strong");
  title.textContent = titleText;
  head.append(title);
  if (referenced) {
    const badge = document.createElement("span");
    badge.textContent = "Referenced by agreement";
    head.append(badge);
  }
  if (!locked) {
    const up = button("Up", "language-button secondary", () => actions.move(-1));
    const down = button("Down", "language-button secondary", () => actions.move(1));
    up.setAttribute("aria-label", `Move ${titleText} up`);
    down.setAttribute("aria-label", `Move ${titleText} down`);
    up.disabled = index === 0;
    down.disabled = index === total - 1;
    const remove = button("Remove", "language-button secondary language-danger", actions.remove);
    remove.setAttribute("aria-label", `Remove ${titleText}`);
    head.append(up, down, remove);
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

function tamTemplates(systemId: GrammarSystemId) {
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
