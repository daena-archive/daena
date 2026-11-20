import { GRAMMAR_CATALOG, GRAMMAR_SECTIONS, grammarSectionDescriptor, systemsForSection } from "./catalog.ts";
import { summarizeChoice } from "./choice.ts";
import { summarizeClause } from "./clause.ts";
import { summarizeParadigm } from "./paradigm.ts";
import { summarizeStrategy } from "./strategy.ts";
import { systemStatus } from "./normalize.ts";
import type {
  CaseConfig,
  GrammarSearchHit,
  GrammarStatus,
  GrammarSystemId,
  GrammarSystemRecord,
  IndexedGrammar,
  NounClassesConfig,
  NumberConfig,
  TamConfig,
} from "./types.ts";

type Translate = (source: string, values?: Record<string, string | number>) => string;
const english: Translate = (source, values) => source.replace(/\{(\w+)\}/g, (match, key) => String(values?.[key] ?? match));

const STATUS_LABEL: Record<GrammarStatus, string> = {
  unconfigured: "Not configured",
  configured: "Configured",
  "not-used": "Not used",
};

export function grammarStatusLabel(status: GrammarStatus) {
  return STATUS_LABEL[status];
}

function labels(items: { label?: string; name?: string }[] | undefined) {
  return (items ?? [])
    .map((item) => item.label || item.name)
    .filter(Boolean)
    .join(", ");
}

function join(parts: (string | undefined)[]) {
  return parts.filter(Boolean).join(" · ");
}

export function summarizeSystem(systemId: GrammarSystemId, record: GrammarSystemRecord | undefined, translate: Translate = english): string {
  if (!record || record.status === "unconfigured") return translate(STATUS_LABEL.unconfigured);
  if (record.status === "not-used") {
    return record.notes ? `${translate(STATUS_LABEL["not-used"])} · ${record.notes}` : translate(STATUS_LABEL["not-used"]);
  }
  const choice = summarizeChoice(systemId, record.config);
  if (choice) return choice;
  const strategy = summarizeStrategy(systemId, record.config);
  if (strategy) return strategy;
  const clause = summarizeClause(systemId, record.config);
  if (clause) return clause;
  const paradigm = summarizeParadigm(systemId, record.config);
  if (paradigm) return paradigm;
  const config = record.config;
  if (!config || Object.keys(config).length === 0) return translate(STATUS_LABEL.configured);
  switch (systemId) {
    case "nouns.number": {
      const value = config as NumberConfig;
      return (
        join([labels(value.categories), value.markingStrategies?.[0]?.replaceAll("-", " ")]) || translate(STATUS_LABEL.configured)
      );
    }
    case "nouns.case": {
      const value = config as CaseConfig;
      const count = value.cases?.length ?? 0;
      return count ? `${count} case${count === 1 ? "" : "s"}` : translate(STATUS_LABEL.configured);
    }
    case "nouns.classes": {
      const value = config as NounClassesConfig;
      return (
        join([value.kind?.replace("-", " "), labels(value.classes?.map((item) => ({ label: item.name })))]) ||
        STATUS_LABEL.configured
      );
    }
    case "verbs.tense":
    case "verbs.aspect":
    case "verbs.mood":
      return labels((config as TamConfig).categories) || translate(STATUS_LABEL.configured);
    default:
      return translate(STATUS_LABEL.configured);
  }
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
  translate: Translate = english,
): SectionCardSummary {
  const section = grammarSectionDescriptor(sectionId)!;
  if (sectionId === "agreement") {
    if (index.sectionStates.get("agreement")?.value.recordKind === "section-state") {
      return { id: sectionId, label: section.label, detail: translate("Not used"), configured: 0, notUsed: 1, total: 0 };
    }
    const count = index.agreements.length;
    return {
      id: sectionId,
      label: section.label,
      detail: count === 0 ? translate("None configured") : translate(count === 1 ? "{count} system configured" : "{count} systems configured", { count }),
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
      detail: count === 0 ? translate("No custom rules") : translate(count === 1 ? "{count} custom rule" : "{count} custom rules", { count }),
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
        ? translate("None configured")
        : configured === 0
          ? translate("{count} not used", { count: notUsed })
          : translate(configured === 1 ? "{count} system configured" : "{count} systems configured", { count: configured }),
    configured,
    notUsed,
    total: systems.length,
  };
}

export type GlanceRow = { label: string; value: string };

export function grammarGlance(index: IndexedGrammar, translate: Translate = english): GlanceRow[] {
  const row = (systemId: GrammarSystemId, label: string): GlanceRow => {
    const record = index.systems.get(systemId)?.value;
    const value =
      record?.recordKind === "system" ? summarizeSystem(systemId, record, translate) : translate(grammarStatusLabel("unconfigured"));
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

export function searchGrammar(query: string, index: IndexedGrammar, translate: Translate = english): GrammarSearchHit[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  const hits: GrammarSearchHit[] = [];
  for (const system of GRAMMAR_CATALOG) {
    const record = index.systems.get(system.id)?.value;
    const status = systemStatus(index, system.id);
    const summary = record?.recordKind === "system" ? summarizeSystem(system.id, record) : grammarStatusLabel(status);
    const blob = [system.label, system.hint, translate(system.label), translate(system.hint), system.searchAliases.join(" "), summary].join("\n");
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
    const blob = [
      record.value.title,
      record.value.notes,
      record.value.features.map((item) => item.label).join(" "),
    ].join("\n");
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
