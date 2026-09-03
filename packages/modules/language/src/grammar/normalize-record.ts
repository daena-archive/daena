import {
  GRAMMAR_SCHEMA_VERSION,
  type AgreementEndpoint,
  type GrammarAgreementRecord,
  type GrammarCustomRuleRecord,
  type GrammarDiagnostic,
  type GrammarDuplicateRecord,
  type GrammarIssue,
  type GrammarRecord,
  type GrammarSectionStateRecord,
  type GrammarStatus,
  type GrammarSystemConfig,
  type GrammarSystemId,
  type GrammarSystemRecord,
  type IndexedGrammar,
  type LoadedGrammarRecord,
  type NormalizeResult,
} from "./types.ts";
import { grammarSystemDescriptor } from "./catalog.ts";

import {
  BODY,
  MAX_FEATURES,
  MAX_TAGS,
  NOTES,
  STATUSES,
  SYSTEM_IDS,
  compact,
  emptyConfig,
  id,
  issue,
  lines,
  obj,
  optional,
  pick,
  text,
} from "./normalize-primitives.ts";
import {
  normalizeExamples,
  normalizeLinks,
} from "./normalize-paradigm.ts";
import {
  BEHAVIORS,
  CONTROLLERS,
  TARGETS,
  configuredMinimum,
  normalizeSystemConfig,
} from "./normalize-systems.ts";

export function commonFields(record: Record<string, unknown>) {
  return {
    notes: lines(record.notes, NOTES),
    examples: normalizeExamples(record.examples),
    links: normalizeLinks(record.links),
  };
}

export function endpoint(value: unknown, kinds: readonly string[]): AgreementEndpoint | null {
  const entry = obj(value);
  const kind = pick(entry.kind, kinds as readonly AgreementEndpoint["kind"][]);
  if (!kind) return null;
  return { kind, customLabel: kind === "custom" ? optional(entry.customLabel) : undefined };
}

export function normalizeGrammarRecord(value: unknown): NormalizeResult {
  const record = obj(value);
  if ("section" in record && "body" in record && !("recordKind" in record)) {
    return { ok: false, issues: [issue("legacy-topic", "Legacy grammar topics are not accepted.")] };
  }
  const schemaVersion = record.schemaVersion;
  if (schemaVersion !== GRAMMAR_SCHEMA_VERSION) {
    return { ok: false, issues: [issue("invalid-schema-version", "schemaVersion must be 1.")] };
  }
  const kind = text(record.recordKind);
  if (kind === "system") {
    const systemId = text(record.systemId);
    if (!SYSTEM_IDS.has(systemId)) {
      return { ok: false, issues: [issue("unknown-system", `Unknown systemId: ${systemId || "(missing)"}.`)] };
    }
    const status = text(record.status) as GrammarStatus;
    if (!STATUSES.has(status)) {
      return { ok: false, issues: [issue("invalid-status", "status must be unconfigured, configured, or not-used.")] };
    }
    const common = commonFields(record);
    const rawConfig = obj(record.config);
    const issues: GrammarIssue[] = [];
    let config: GrammarSystemConfig = emptyConfig();
    if (status === "not-used" || status === "unconfigured") {
      if (Object.keys(rawConfig).length > 0)
        issues.push(issue("empty-config-required", "Unconfigured and not-used records keep config empty."));
      config = emptyConfig();
    } else {
      config = normalizeSystemConfig(systemId as GrammarSystemId, rawConfig, common.examples);
      if (!configuredMinimum(systemId as GrammarSystemId, config)) {
        issues.push(issue("configured-minimum", "Configured systems need meaningful data.", "config"));
      }
    }
    const next: GrammarSystemRecord = {
      recordKind: "system",
      schemaVersion: 1,
      systemId: systemId as GrammarSystemId,
      status,
      config,
      ...common,
    };
    return { ok: true, record: compact(next), issues };
  }
  if (kind === "agreement") {
    const controller = endpoint(record.controller, CONTROLLERS);
    const target = endpoint(record.target, TARGETS);
    const title = text(record.title);
    if (!controller || !target || !title) {
      return { ok: false, issues: [issue("malformed", "Agreement records need title, controller, and target.")] };
    }
    const features = Array.isArray(record.features)
      ? record.features
          .map((item): GrammarAgreementRecord["features"][number] | null => {
            const entry = obj(item);
            const label = text(entry.label);
            if (!label) return null;
            const sourceSystemId = optional(entry.sourceSystemId);
            return {
              sourceSystemId:
                sourceSystemId && SYSTEM_IDS.has(sourceSystemId) ? (sourceSystemId as GrammarSystemId) : undefined,
              categoryId: optional(entry.categoryId),
              label,
            };
          })
          .filter((item): item is GrammarAgreementRecord["features"][number] => item !== null)
          .slice(0, MAX_FEATURES)
      : [];
    const next: GrammarAgreementRecord = {
      recordKind: "agreement",
      schemaVersion: 1,
      title,
      controller,
      target,
      features,
      behavior: pick(record.behavior, BEHAVIORS) ?? "full",
      defaultForm: optional(record.defaultForm),
      conditions: optional(record.conditions, NOTES),
      exceptions: optional(record.exceptions, NOTES),
      ...commonFields(record),
    };
    return { ok: true, record: compact(next), issues: [] };
  }
  if (kind === "custom-rule") {
    const title = text(record.title);
    if (!title) return { ok: false, issues: [issue("malformed", "Custom rules need a title.")] };
    const next: GrammarCustomRuleRecord = {
      recordKind: "custom-rule",
      schemaVersion: 1,
      title,
      tags: Array.isArray(record.tags)
        ? record.tags
            .map((item) => text(item))
            .filter(Boolean)
            .slice(0, MAX_TAGS)
        : [],
      body: lines(record.body, BODY),
      examples: normalizeExamples(record.examples),
      links: normalizeLinks(record.links),
    };
    return { ok: true, record: compact(next), issues: [] };
  }
  if (kind === "section-state") {
    if (text(record.sectionId) !== "agreement" || text(record.status) !== "not-used") {
      return { ok: false, issues: [issue("malformed", "Section state currently supports only agreement not-used.")] };
    }
    const next: GrammarSectionStateRecord = {
      recordKind: "section-state",
      schemaVersion: 1,
      sectionId: "agreement",
      status: "not-used",
      note: optional(record.note, NOTES),
    };
    return { ok: true, record: compact(next), issues: [] };
  }
  if (!kind) return { ok: false, issues: [issue("unknown-kind", "recordKind is required.")] };
  return { ok: false, issues: [issue("unknown-kind", `Unknown recordKind: ${kind}.`)] };
}

export function serializeGrammarRecord(value: GrammarRecord): Record<string, unknown> {
  const result = normalizeGrammarRecord(value);
  if (!result.ok) throw new Error(result.issues.map((item) => item.message).join(" "));
  return JSON.parse(JSON.stringify(result.record)) as Record<string, unknown>;
}

export function indexGrammarRecords(records: { id: string; revision?: string; value: unknown }[]): IndexedGrammar {
  const systems = new Map<GrammarSystemId, LoadedGrammarRecord>();
  const duplicates = new Map<GrammarSystemId, GrammarDuplicateRecord[]>();
  const seen = new Map<GrammarSystemId, GrammarDuplicateRecord[]>();
  const agreements: LoadedGrammarRecord[] = [];
  const customRules: LoadedGrammarRecord[] = [];
  const sectionStates = new Map<string, LoadedGrammarRecord>();
  const rejected: IndexedGrammar["rejected"] = [];
  const diagnostics: IndexedGrammar["diagnostics"] = [];

  for (const record of records) {
    const result = normalizeGrammarRecord(record.value);
    if (!result.ok) {
      rejected.push({ id: record.id, issues: result.issues });
      diagnostics.push({ ...result.issues[0], recordIds: [record.id] });
      continue;
    }
    const loaded: LoadedGrammarRecord = {
      id: record.id,
      revision: record.revision ?? "",
      value: result.record,
    };
    if (result.record.recordKind === "system") {
      const ids = seen.get(result.record.systemId) ?? [];
      ids.push({ id: record.id, revision: record.revision ?? "" });
      seen.set(result.record.systemId, ids);
    } else if (result.record.recordKind === "agreement") agreements.push(loaded);
    else if (result.record.recordKind === "custom-rule") customRules.push(loaded);
    else sectionStates.set(result.record.sectionId, loaded);
  }

  for (const [systemId, ids] of seen) {
    if (ids.length > 1) {
      duplicates.set(systemId, ids);
      diagnostics.push({
        code: "duplicate-system",
        message: `${grammarSystemDescriptor(systemId)?.label ?? systemId} has duplicate records. Edits are disabled until the conflict is resolved.`,
        systemId,
        recordIds: ids.map((item) => item.id),
      });
      continue;
    }
    const first = ids[0];
    const source = records.find((item) => item.id === first?.id);
    const result = source ? normalizeGrammarRecord(source.value) : null;
    if (result?.ok && result.record.recordKind === "system" && first) {
      systems.set(systemId, { id: first.id, revision: first.revision || source?.revision || "", value: result.record });
      for (const item of result.issues)
        diagnostics.push({ ...item, recordIds: ids.map((entry) => entry.id), systemId });
    }
  }

  return { systems, duplicates, agreements, customRules, sectionStates, rejected, diagnostics };
}

export function systemStatus(index: IndexedGrammar, systemId: GrammarSystemId): GrammarStatus {
  if (index.duplicates.has(systemId)) return "unconfigured";
  const record = index.systems.get(systemId)?.value;
  return record?.recordKind === "system" ? record.status : "unconfigured";
}

export function brokenAgreementFeatures(index: IndexedGrammar): GrammarDiagnostic[] {
  const diagnostics: GrammarDiagnostic[] = [];
  for (const record of index.agreements) {
    if (record.value.recordKind !== "agreement") continue;
    for (const feature of record.value.features) {
      if (!feature.sourceSystemId) continue;
      const system = index.systems.get(feature.sourceSystemId);
      if (!system || system.value.recordKind !== "system") {
        diagnostics.push({
          code: "broken-reference",
          message: `Agreement “${record.value.title}” references missing system ${feature.sourceSystemId}.`,
          recordIds: [record.id],
          systemId: feature.sourceSystemId,
        });
        continue;
      }
      if (!feature.categoryId) continue;
      const config = system.value.config as {
        categories?: { id: string }[];
        cases?: { id: string }[];
        classes?: { id: string }[];
        axes?: { id: string; values: { id: string }[] }[];
        cells?: { id: string }[];
      };
      const ids = new Set([
        ...(config.categories ?? []).map((item) => item.id),
        ...(config.cases ?? []).map((item) => item.id),
        ...(config.classes ?? []).map((item) => item.id),
        ...(config.axes ?? []).map((axis) => axis.id),
        ...(config.axes ?? []).flatMap((axis) => axis.values.map((value) => value.id)),
        ...(config.cells ?? []).map((item) => item.id),
      ]);
      if (feature.categoryId && !ids.has(feature.categoryId)) {
        diagnostics.push({
          code: "broken-reference",
          message: `Agreement “${record.value.title}” references a missing category in ${feature.sourceSystemId}.`,
          recordIds: [record.id],
          systemId: feature.sourceSystemId,
        });
      }
    }
  }
  return diagnostics;
}
