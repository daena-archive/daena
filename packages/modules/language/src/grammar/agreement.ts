import { emptyMessage, field, input, textarea } from "../ui.ts";
import { grammarSystemDescriptor } from "./catalog.ts";
import { MAX_FEATURES } from "./normalize.ts";
import type {
  AgreementBehavior,
  AgreementControllerKind,
  AgreementEndpoint,
  AgreementFeature,
  AgreementTargetKind,
  CaseConfig,
  GrammarAgreementRecord,
  GrammarSystemId,
  IndexedGrammar,
  NounClassesConfig,
  NumberConfig,
  ParadigmConfig,
} from "./types.ts";

export const CONTROLLER_OPTIONS: { value: AgreementControllerKind; label: string }[] = [
  { value: "subject", label: "Subject" },
  { value: "object", label: "Object" },
  { value: "noun", label: "Noun" },
  { value: "possessor", label: "Possessor" },
  { value: "custom", label: "Custom" },
];

export const TARGET_OPTIONS: { value: AgreementTargetKind; label: string }[] = [
  { value: "verb", label: "Verb" },
  { value: "adjective", label: "Adjective" },
  { value: "article", label: "Article" },
  { value: "pronoun", label: "Pronoun" },
  { value: "participle", label: "Participle" },
  { value: "custom", label: "Custom" },
];

export const BEHAVIOR_OPTIONS: { value: AgreementBehavior; label: string; expansion?: string }[] = [
  { value: "full", label: "Full agreement" },
  { value: "partial", label: "Partial agreement", expansion: "Only some forms or features agree." },
  { value: "conditional", label: "Conditional agreement", expansion: "Agreement depends on tense, person, or similar conditions." },
];

export type OfferedAgreementGroup = {
  id: string;
  label: string;
  sourceSystemId: GrammarSystemId;
  features: { categoryId?: string; label: string }[];
};

export function endpointLabel(endpoint: AgreementEndpoint) {
  if (endpoint.kind === "custom") return endpoint.customLabel?.trim() || "Custom";
  return (
    CONTROLLER_OPTIONS.find((item) => item.value === endpoint.kind)?.label
    ?? TARGET_OPTIONS.find((item) => item.value === endpoint.kind)?.label
    ?? endpoint.kind
  );
}

export function agreementTitleFromEndpoints(controller: AgreementEndpoint, target: AgreementEndpoint) {
  return `${endpointLabel(controller)} → ${endpointLabel(target)}`;
}

export function summarizeAgreement(record: GrammarAgreementRecord) {
  const features = record.features.map((item) => item.label).filter(Boolean).join(", ");
  return [agreementTitleFromEndpoints(record.controller, record.target), features].filter(Boolean).join(" · ");
}

export function offeredAgreementGroups(index: IndexedGrammar): OfferedAgreementGroup[] {
  const groups: OfferedAgreementGroup[] = [];
  const personal = paradigmAxes(index, "pronouns.personal");
  addAxisGroup(groups, "pronouns.personal", personal, "person", "Person");
  const number = inventoryItems(index, "nouns.number");
  if (number.length) {
    groups.push({ id: "nouns.number", label: "Number", sourceSystemId: "nouns.number", features: number });
  } else {
    addAxisGroup(groups, "pronouns.personal", personal, "number", "Number");
  }
  const classes = classItems(index);
  if (classes) groups.push(classes);
  else addAxisGroup(groups, "pronouns.personal", personal, "gender", "Gender");
  const cases = caseItems(index);
  if (cases.length) groups.push({ id: "nouns.case", label: "Case", sourceSystemId: "nouns.case", features: cases });
  else addAxisGroup(groups, "pronouns.personal", personal, "case", "Case");
  addAxisGroup(groups, "pronouns.personal", personal, "noun-class", "Noun class");
  addAxisGroup(groups, "pronouns.personal", personal, "animacy", "Animacy");
  if (configured(index, "nouns.definiteness")) {
    groups.push({
      id: "nouns.definiteness",
      label: "Definiteness",
      sourceSystemId: "nouns.definiteness",
      features: [{ label: "Definiteness" }],
    });
  }
  return groups;
}

export function featureDisplayLabel(index: IndexedGrammar, feature: AgreementFeature) {
  if (feature.sourceSystemId && feature.categoryId) {
    const live = liveCategoryLabel(index, feature.sourceSystemId, feature.categoryId);
    if (live) return live;
    return feature.label;
  }
  if (feature.sourceSystemId) {
    return grammarSystemDescriptor(feature.sourceSystemId)?.label ?? feature.label;
  }
  return feature.label;
}

export function setAgreementController(draft: GrammarAgreementRecord, kind: AgreementControllerKind): GrammarAgreementRecord {
  const controller: AgreementEndpoint = { kind, customLabel: kind === "custom" ? draft.controller.customLabel : undefined };
  return withAutoTitle(draft, { controller });
}

export function setAgreementTarget(draft: GrammarAgreementRecord, kind: AgreementTargetKind): GrammarAgreementRecord {
  const target: AgreementEndpoint = { kind, customLabel: kind === "custom" ? draft.target.customLabel : undefined };
  return withAutoTitle(draft, { target });
}

export function setAgreementEndpointLabel(
  draft: GrammarAgreementRecord,
  role: "controller" | "target",
  customLabel: string,
): GrammarAgreementRecord {
  const next = { ...draft[role], customLabel };
  return withAutoTitle(draft, { [role]: next });
}

export function setAgreementBehavior(draft: GrammarAgreementRecord, behavior: AgreementBehavior): GrammarAgreementRecord {
  return { ...draft, behavior };
}

export function setAgreementField(
  draft: GrammarAgreementRecord,
  fieldName: "defaultForm" | "conditions" | "exceptions" | "notes" | "title",
  value: string,
): GrammarAgreementRecord {
  return { ...draft, [fieldName]: value };
}

export function toggleAgreementGroup(draft: GrammarAgreementRecord, group: OfferedAgreementGroup): GrammarAgreementRecord {
  const keys = new Set(group.features.map((item) => featureKey(group.sourceSystemId, item.categoryId)));
  const selected = draft.features.filter((item) => keys.has(featureKey(item.sourceSystemId, item.categoryId)));
  if (selected.length === group.features.length) {
    return { ...draft, features: draft.features.filter((item) => !keys.has(featureKey(item.sourceSystemId, item.categoryId))) };
  }
  const next = [...draft.features];
  for (const item of group.features) {
    const key = featureKey(group.sourceSystemId, item.categoryId);
    if (next.some((feature) => featureKey(feature.sourceSystemId, feature.categoryId) === key)) continue;
    if (next.length >= MAX_FEATURES) break;
    next.push({ sourceSystemId: group.sourceSystemId, categoryId: item.categoryId, label: item.label });
  }
  return { ...draft, features: next };
}

export function addCustomAgreementFeature(draft: GrammarAgreementRecord, label = "Custom"): GrammarAgreementRecord {
  if (draft.features.length >= MAX_FEATURES) return draft;
  return { ...draft, features: [...draft.features, { label }] };
}

export function updateAgreementFeature(
  draft: GrammarAgreementRecord,
  index: number,
  patch: Partial<AgreementFeature>,
): GrammarAgreementRecord {
  return {
    ...draft,
    features: draft.features.map((item, itemIndex) => (itemIndex === index ? { ...item, ...patch } : item)),
  };
}

export function removeAgreementFeature(draft: GrammarAgreementRecord, index: number): GrammarAgreementRecord {
  return { ...draft, features: draft.features.filter((_, itemIndex) => itemIndex !== index) };
}

export function groupSelected(draft: GrammarAgreementRecord, group: OfferedAgreementGroup) {
  return (
    group.features.length > 0 &&
    group.features.every((item) =>
      draft.features.some(
        (feature) => featureKey(feature.sourceSystemId, feature.categoryId) === featureKey(group.sourceSystemId, item.categoryId),
      ),
    )
  );
}

export function renderAgreementEditor(
  draft: GrammarAgreementRecord,
  locked: boolean,
  index: IndexedGrammar,
  onChange: (next: GrammarAgreementRecord, rerender: boolean) => void,
): HTMLElement {
  const section = document.createElement("section");
  section.className = "language-group grammar-choice-stack";
  section.append(emptyMessage("Which element determines the grammatical features? Which element changes to match it?"));
  const titleField = input("title", draft.title);
  titleField.disabled = locked;
  titleField.oninput = () => onChange(setAgreementField(draft, "title", titleField.value), false);
  section.append(field("Title", titleField));
  section.append(
    radios("Controller", CONTROLLER_OPTIONS, draft.controller.kind, locked, (value) => {
      onChange(setAgreementController(draft, value as AgreementControllerKind), true);
    }),
  );
  if (draft.controller.kind === "custom") {
    const custom = input("controllerCustom", draft.controller.customLabel ?? "");
    custom.disabled = locked;
    custom.oninput = () => onChange(setAgreementEndpointLabel(draft, "controller", custom.value), false);
    section.append(field("Custom controller", custom));
  }
  section.append(
    radios("Target", TARGET_OPTIONS, draft.target.kind, locked, (value) => {
      onChange(setAgreementTarget(draft, value as AgreementTargetKind), true);
    }),
  );
  if (draft.target.kind === "custom") {
    const custom = input("targetCustom", draft.target.customLabel ?? "");
    custom.disabled = locked;
    custom.oninput = () => onChange(setAgreementEndpointLabel(draft, "target", custom.value), false);
    section.append(field("Custom target", custom));
  }
  const groups = offeredAgreementGroups(index);
  const features = document.createElement("fieldset");
  features.className = "grammar-checks";
  const legend = document.createElement("legend");
  legend.textContent = `${endpointLabel(draft.target)} agrees with ${endpointLabel(draft.controller)} in`;
  features.append(legend);
  if (groups.length === 0) features.append(emptyMessage("Configure number, case, classes, or pronouns first to reuse those categories here."));
  for (const group of groups) {
    const label = document.createElement("label");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = groupSelected(draft, group);
    box.disabled = locked;
    box.onchange = () => onChange(toggleAgreementGroup(draft, group), true);
    label.append(box, ` ${group.label}`);
    features.append(label);
  }
  section.append(features);
  const customFeatures = draft.features.filter((item) => !item.sourceSystemId);
  for (const feature of customFeatures) {
    const featureIndex = draft.features.indexOf(feature);
    const row = document.createElement("div");
    row.className = "grammar-inventory-toolbar";
    const name = input("customFeature", feature.label);
    name.disabled = locked;
    name.oninput = () => onChange(updateAgreementFeature(draft, featureIndex, { label: name.value }), false);
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "language-button secondary language-danger";
    remove.textContent = "Remove";
    remove.disabled = locked;
    remove.onclick = () => onChange(removeAgreementFeature(draft, featureIndex), true);
    row.append(name, remove);
    section.append(field("Custom feature", row));
  }
  if (!locked) {
    const add = document.createElement("button");
    add.type = "button";
    add.className = "language-button secondary";
    add.textContent = "Add custom feature";
    add.onclick = () => onChange(addCustomAgreementFeature(draft), true);
    section.append(add);
  }
  const broken = draft.features.filter((item) => item.sourceSystemId && item.categoryId && !liveCategoryLabel(index, item.sourceSystemId, item.categoryId)
    || (item.sourceSystemId && !item.categoryId && !configured(index, item.sourceSystemId)));
  if (broken.length) {
    section.append(emptyMessage(`Broken references: ${broken.map((item) => item.label).join(", ")}. Edit the owning system to restore them, or remove the feature.`));
  }
  section.append(
    radios("Behavior", BEHAVIOR_OPTIONS, draft.behavior, locked, (value) => {
      onChange(setAgreementBehavior(draft, value as AgreementBehavior), true);
    }),
  );
  const defaultForm = input("defaultForm", draft.defaultForm ?? "");
  defaultForm.disabled = locked;
  defaultForm.oninput = () => onChange(setAgreementField(draft, "defaultForm", defaultForm.value), false);
  const conditions = textarea("conditions", draft.conditions ?? "", 3);
  conditions.disabled = locked;
  conditions.oninput = () => onChange(setAgreementField(draft, "conditions", conditions.value), false);
  const exceptions = textarea("exceptions", draft.exceptions ?? "", 3);
  exceptions.disabled = locked;
  exceptions.oninput = () => onChange(setAgreementField(draft, "exceptions", exceptions.value), false);
  section.append(
    field("Default form (optional)", defaultForm),
    field("Conditions (optional)", conditions),
    field("Exceptions (optional)", exceptions),
  );
  return section;
}

function withAutoTitle(draft: GrammarAgreementRecord, patch: Partial<GrammarAgreementRecord>): GrammarAgreementRecord {
  const next = { ...draft, ...patch };
  const previous = agreementTitleFromEndpoints(draft.controller, draft.target);
  if (!draft.title.trim() || draft.title === previous) next.title = agreementTitleFromEndpoints(next.controller, next.target);
  return next;
}

function featureKey(sourceSystemId: string | undefined, categoryId: string | undefined) {
  return `${sourceSystemId ?? ""}:${categoryId ?? ""}`;
}

function configured(index: IndexedGrammar, systemId: GrammarSystemId) {
  const record = index.systems.get(systemId)?.value;
  return record?.recordKind === "system" && record.status === "configured";
}

function paradigmAxes(index: IndexedGrammar, systemId: GrammarSystemId) {
  const record = index.systems.get(systemId)?.value;
  if (record?.recordKind !== "system" || record.status !== "configured") return [];
  return (record.config as ParadigmConfig).axes ?? [];
}

function addAxisGroup(
  groups: OfferedAgreementGroup[],
  systemId: GrammarSystemId,
  axes: ParadigmConfig["axes"],
  axisId: string,
  label: string,
) {
  if (groups.some((group) => group.label === label)) return;
  const axis = axes.find((item) => item.id === axisId);
  if (!axis) return;
  groups.push({
    id: `${systemId}:${axisId}`,
    label,
    sourceSystemId: systemId,
    features: [{ categoryId: axis.id, label: axis.label || label }],
  });
}

function inventoryItems(index: IndexedGrammar, systemId: "nouns.number") {
  const record = index.systems.get(systemId)?.value;
  if (record?.recordKind !== "system" || record.status !== "configured") return [];
  return ((record.config as NumberConfig).categories ?? [])
    .filter((item) => item.id && item.label?.trim())
    .map((item) => ({ categoryId: item.id, label: item.label }));
}

function caseItems(index: IndexedGrammar) {
  const record = index.systems.get("nouns.case")?.value;
  if (record?.recordKind !== "system" || record.status !== "configured") return [];
  return ((record.config as CaseConfig).cases ?? [])
    .filter((item) => item.id && item.name?.trim())
    .map((item) => ({ categoryId: item.id, label: item.name }));
}

function classItems(index: IndexedGrammar): OfferedAgreementGroup | undefined {
  const record = index.systems.get("nouns.classes")?.value;
  if (record?.recordKind !== "system" || record.status !== "configured") return undefined;
  const config = record.config as NounClassesConfig;
  const features = (config.classes ?? [])
    .filter((item) => item.id && item.name?.trim())
    .map((item) => ({ categoryId: item.id, label: item.name }));
  if (!features.length) return undefined;
  return {
    id: "nouns.classes",
    label: config.kind === "noun-class" ? "Noun class" : "Gender",
    sourceSystemId: "nouns.classes",
    features,
  };
}

function liveCategoryLabel(index: IndexedGrammar, systemId: GrammarSystemId, categoryId: string) {
  const record = index.systems.get(systemId)?.value;
  if (record?.recordKind !== "system") return undefined;
  const config = record.config as NumberConfig & CaseConfig & NounClassesConfig & ParadigmConfig;
  const fromInventory = [
    ...(config.categories ?? []),
    ...(config.cases ?? []).map((item) => ({ id: item.id, label: item.name })),
    ...(config.classes ?? []).map((item) => ({ id: item.id, label: item.name })),
  ].find((item) => item.id === categoryId);
  if (fromInventory?.label) return fromInventory.label;
  const axis = (config.axes ?? []).find((item) => item.id === categoryId);
  return axis?.label;
}

function radios(
  legendText: string,
  options: { value: string; label: string; expansion?: string }[],
  selected: string | undefined,
  locked: boolean,
  onChange: (value: string) => void,
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
    radio.name = legendText;
    radio.value = option.value;
    radio.checked = option.value === selected;
    radio.disabled = locked;
    radio.onchange = () => onChange(option.value);
    const title = document.createElement("strong");
    title.textContent = option.label;
    card.append(radio, title);
    if (option.expansion) {
      const hint = document.createElement("span");
      hint.textContent = option.expansion;
      card.append(hint);
    }
    group.append(card);
  }
  return group;
}
