import { GRAMMAR_CATALOG, GRAMMAR_SECTIONS, grammarSectionDescriptor, systemsForSection } from "./catalog.ts";
import { systemStatus } from "./normalize.ts";
import type {
  AdjectiveBehaviorConfig,
  AdpositionsConfig,
  ArgumentIndexingConfig,
  BasicWordOrderConfig,
  CaseConfig,
  ClauseNegationConfig,
  ContentQuestionsConfig,
  DefinitenessConfig,
  DegreeConfig,
  DemonstrativeConfig,
  GrammarSearchHit,
  GrammarStatus,
  GrammarSystemId,
  GrammarSystemRecord,
  ImperativesConfig,
  IndexedGrammar,
  NegativeVerbConfig,
  NounClassesConfig,
  NumberConfig,
  ParadigmConfig,
  PositionConfig,
  PossessivePositionConfig,
  PossessionConfig,
  RelativeClausePositionConfig,
  RelativeClausesConfig,
  TamConfig,
  VerbMarkingConfig,
  YesNoQuestionsConfig,
} from "./types.ts";

const STATUS_LABEL: Record<GrammarStatus, string> = {
  unconfigured: "Not configured",
  configured: "Configured",
  "not-used": "Not used",
};

export function grammarStatusLabel(status: GrammarStatus) {
  return STATUS_LABEL[status];
}

function labels(items: { label?: string; name?: string }[]) {
  return items.map((item) => item.label || item.name).filter(Boolean).join(", ");
}

function join(parts: (string | undefined)[]) {
  return parts.filter(Boolean).join(" · ");
}

const ORDER_LABEL: Record<string, string> = {
  sov: "SOV",
  svo: "SVO",
  vso: "VSO",
  vos: "VOS",
  ovs: "OVS",
  osv: "OSV",
  flexible: "Flexible",
  custom: "Custom",
};

const STRENGTH_LABEL: Record<string, string> = {
  strict: "Strict",
  "strongly-preferred": "Strongly preferred",
  "default-flexible": "Default, but flexible",
  context: "Mostly determined by context",
};

export function summarizeSystem(systemId: GrammarSystemId, record: GrammarSystemRecord | undefined): string {
  if (!record || record.status === "unconfigured") return STATUS_LABEL.unconfigured;
  if (record.status === "not-used") {
    return record.notes ? `${STATUS_LABEL["not-used"]} · ${record.notes}` : STATUS_LABEL["not-used"];
  }
  const config = record.config;
  switch (systemId) {
    case "syntax.basic-word-order": {
      const value = config as BasicWordOrderConfig;
      return join([ORDER_LABEL[value.order] ?? value.order, value.strength ? STRENGTH_LABEL[value.strength] : undefined]);
    }
    case "syntax.adjective-position":
      return (config as PositionConfig).position.replaceAll("-", " ");
    case "syntax.possessive-position":
      return (config as PossessivePositionConfig).position.replaceAll("-", " ");
    case "syntax.relative-clause-position":
      return (config as RelativeClausePositionConfig).position.replaceAll("-", " ");
    case "syntax.adpositions":
      return (config as AdpositionsConfig).strategy.replaceAll("-", " ");
    case "nouns.number": {
      const value = config as NumberConfig;
      return join([labels(value.categories), value.markingStrategies[0]?.replaceAll("-", " ")]);
    }
    case "nouns.case": {
      const value = config as CaseConfig;
      return `${value.cases.length} case${value.cases.length === 1 ? "" : "s"}`;
    }
    case "nouns.classes": {
      const value = config as NounClassesConfig;
      return join([value.kind.replace("-", " "), labels(value.classes.map((item) => ({ label: item.name })))]);
    }
    case "nouns.definiteness":
      return (config as DefinitenessConfig).strategies.join(" / ").replaceAll("-", " ");
    case "nouns.possession":
      return (config as PossessionConfig).strategies.join(" / ").replaceAll("-", " ");
    case "pronouns.personal":
      return paradigmSummary(config as ParadigmConfig);
    case "pronouns.demonstratives": {
      const value = config as DemonstrativeConfig;
      return value.distances.length ? value.distances.join(" / ") : paradigmSummary(value);
    }
    case "verbs.marking-strategy":
      return (config as VerbMarkingConfig).strategies.join(" / ").replaceAll("-", " ");
    case "verbs.tense":
    case "verbs.aspect":
    case "verbs.mood":
      return labels((config as TamConfig).categories);
    case "verbs.argument-indexing": {
      const value = config as ArgumentIndexingConfig;
      return join([value.participants.replace("-", " and "), value.representation?.replaceAll("-", " ")]);
    }
    case "verbs.negative-forms":
      return (config as NegativeVerbConfig).strategies.join(" / ").replaceAll("-", " ");
    case "modifiers.adjective-behavior":
      return (config as AdjectiveBehaviorConfig).behaviors.join(" / ").replaceAll("-", " ");
    case "modifiers.comparative":
    case "modifiers.superlative":
      return (config as DegreeConfig).strategies.join(" / ").replaceAll("-", " ");
    case "clauses.yes-no-questions": {
      const value = config as YesNoQuestionsConfig;
      return join([value.strategies.join(" / ").replaceAll("-", " "), value.particle ? `“${value.particle}”` : undefined]);
    }
    case "clauses.content-questions":
      return (config as ContentQuestionsConfig).behavior.replaceAll("-", " ");
    case "clauses.imperatives":
      return (config as ImperativesConfig).strategies.join(" / ").replaceAll("-", " ");
    case "clauses.negation": {
      const value = config as ClauseNegationConfig;
      return join([value.strategies.join(" / ").replaceAll("-", " "), value.particle ? `“${value.particle}”` : undefined]);
    }
    case "clauses.relative-clauses":
      return (config as RelativeClausesConfig).strategies.join(" / ").replaceAll("-", " ");
  }
}

function paradigmSummary(config: ParadigmConfig) {
  return config.axes.map((axis) => `${axis.label}: ${axis.values.map((item) => item.label).join("/")}`).join(" · ");
}

export type SectionCardSummary = {
  id: string;
  label: string;
  detail: string;
  configured: number;
  notUsed: number;
  total: number;
};

export function sectionCardSummary(
  index: IndexedGrammar,
  sectionId: (typeof GRAMMAR_SECTIONS)[number]["id"],
): SectionCardSummary {
  const section = grammarSectionDescriptor(sectionId)!;
  if (sectionId === "agreement") {
    if (index.sectionStates.get("agreement")?.value.recordKind === "section-state") {
      return { id: sectionId, label: section.label, detail: "Not used", configured: 0, notUsed: 1, total: 0 };
    }
    const count = index.agreements.length;
    return {
      id: sectionId,
      label: section.label,
      detail: count === 0 ? "None configured" : `${count} system${count === 1 ? "" : "s"} configured`,
      configured: count,
      notUsed: 0,
      total: count,
    };
  }
  if (sectionId === "other") {
    const count = index.customRules.length;
    return {
      id: sectionId,
      label: section.label,
      detail: count === 0 ? "No custom rules" : `${count} custom rule${count === 1 ? "" : "s"}`,
      configured: count,
      notUsed: 0,
      total: count,
    };
  }
  const systems = systemsForSection(sectionId);
  let configured = 0;
  let notUsed = 0;
  for (const system of systems) {
    const status = systemStatus(index, system.id);
    if (status === "configured") configured += 1;
    if (status === "not-used") notUsed += 1;
  }
  return {
    id: sectionId,
    label: section.label,
    detail:
      configured === 0 && notUsed === 0
        ? "None configured"
        : configured === 0
          ? `${notUsed} not used`
          : `${configured} system${configured === 1 ? "" : "s"} configured`,
    configured,
    notUsed,
    total: systems.length,
  };
}

export type GlanceRow = { label: string; value: string };

export function grammarGlance(index: IndexedGrammar): GlanceRow[] {
  const row = (systemId: GrammarSystemId, label: string): GlanceRow => {
    const record = index.systems.get(systemId)?.value;
    const value =
      record?.recordKind === "system" ? summarizeSystem(systemId, record) : grammarStatusLabel("unconfigured");
    return { label, value };
  };
  return [
    row("syntax.basic-word-order", "Basic word order"),
    row("syntax.adjective-position", "Adjective position"),
    row("nouns.case", "Case system"),
    row("nouns.number", "Number"),
    row("verbs.tense", "Verb tense"),
    row("clauses.yes-no-questions", "Questions"),
    row("clauses.negation", "Negation"),
  ];
}

function matches(haystack: string, needle: string) {
  return haystack.toLowerCase().includes(needle);
}

export function searchGrammar(query: string, index: IndexedGrammar): GrammarSearchHit[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  const hits: GrammarSearchHit[] = [];
  for (const system of GRAMMAR_CATALOG) {
    const record = index.systems.get(system.id)?.value;
    const status = systemStatus(index, system.id);
    const summary = record?.recordKind === "system" ? summarizeSystem(system.id, record) : grammarStatusLabel(status);
    const blob = [system.label, system.hint, system.searchAliases.join(" "), summary].join("\n");
    if (matches(blob, needle)) {
      hits.push({
        kind: "system",
        systemId: system.id,
        sectionId: system.sectionId,
        label: system.label,
        status,
        summary,
        recordId: index.systems.get(system.id)?.id,
      });
    }
  }
  for (const record of index.agreements) {
    if (record.value.recordKind !== "agreement") continue;
    const blob = [record.value.title, record.value.notes, record.value.features.map((item) => item.label).join(" ")].join(
      "\n",
    );
    if (matches(blob, needle)) {
      hits.push({
        kind: "agreement",
        sectionId: "agreement",
        label: record.value.title,
        summary: `${record.value.controller.kind} → ${record.value.target.kind}`,
        recordId: record.id,
      });
    }
  }
  for (const record of index.customRules) {
    if (record.value.recordKind !== "custom-rule") continue;
    const blob = [record.value.title, record.value.tags.join(" "), record.value.body].join("\n");
    if (matches(blob, needle)) {
      hits.push({
        kind: "custom-rule",
        sectionId: "other",
        label: record.value.title,
        summary: record.value.tags.join(", ") || record.value.body.split("\n")[0] || "Custom rule",
        recordId: record.id,
      });
    }
  }
  return hits;
}
