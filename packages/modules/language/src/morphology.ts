import type { LexemeForm } from "./lexeme";

export type ParadigmKind = "inflection" | "derivation";

export type MorphOperationKind = "suffix" | "prefix" | "replace-suffix" | "identity";

export type ParadigmSlot = {
  id: string;
  label: string;
  features?: string;
};

export type MorphOperation = {
  id: string;
  slotId: string;
  op: MorphOperationKind;
  value?: string;
  from?: string;
};

export type MorphRule = {
  id: string;
  name: string;
  kind: ParadigmKind;
  match?: string;
  notes?: string;
  operations: MorphOperation[];
};

export type Paradigm = {
  name: string;
  kind: ParadigmKind;
  partOfSpeech?: string;
  notes?: string;
  slots: ParadigmSlot[];
  rules: MorphRule[];
};

export type FormProvenance = "generated" | "authored" | "missing";

export type ParadigmPreviewCell = {
  slot: ParadigmSlot;
  form: string;
  provenance: FormProvenance;
  generated?: string;
  authoredFormId?: string;
  ruleId?: string;
  ruleName?: string;
};

export const PARADIGM_KINDS: { id: ParadigmKind; label: string }[] = [
  { id: "inflection", label: "Inflection" },
  { id: "derivation", label: "Derivation" },
];

export const OPERATION_KINDS: { id: MorphOperationKind; label: string }[] = [
  { id: "suffix", label: "Add suffix" },
  { id: "prefix", label: "Add prefix" },
  { id: "replace-suffix", label: "Replace suffix" },
  { id: "identity", label: "Keep stem" },
];

const TEXT = 500;
const LONG = 2000;
const MAX_SLOTS = 48;
const MAX_RULES = 32;
const MAX_OPERATIONS = 48;
const KIND_IDS = new Set(PARADIGM_KINDS.map((item) => item.id));
const OP_IDS = new Set(OPERATION_KINDS.map((item) => item.id));

function id() {
  return crypto.randomUUID();
}

function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

function kind(value: unknown): ParadigmKind {
  return KIND_IDS.has(value as ParadigmKind) ? (value as ParadigmKind) : "inflection";
}

function operationKind(value: unknown): MorphOperationKind {
  return OP_IDS.has(value as MorphOperationKind) ? (value as MorphOperationKind) : "suffix";
}

export function emptyParadigm(kind: ParadigmKind = "inflection"): Paradigm {
  return { name: "", kind, slots: [], rules: [] };
}

export function emptySlot(): ParadigmSlot {
  return { id: id(), label: "" };
}

export function emptyRule(kind: ParadigmKind = "inflection"): MorphRule {
  return { id: id(), name: "", kind, operations: [] };
}

export function emptyOperation(slotId = ""): MorphOperation {
  return { id: id(), slotId, op: "suffix" };
}

export function normalizeParadigm(value: unknown): Paradigm {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const slots: ParadigmSlot[] = Array.isArray(record.slots)
    ? record.slots
        .map((item): ParadigmSlot | null => {
          const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
          const label = text(entry.label);
          return label ? { id: text(entry.id) || id(), label, features: optional(entry.features) } : null;
        })
        .filter((item): item is ParadigmSlot => item !== null)
        .slice(0, MAX_SLOTS)
    : [];
  const slotIds = new Set(slots.map((item) => item.id));
  const rules: MorphRule[] = Array.isArray(record.rules)
    ? record.rules
        .map((item): MorphRule => {
          const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
          const operations = Array.isArray(entry.operations)
            ? entry.operations
                .map((operation): MorphOperation | null => {
                  const op = operation && typeof operation === "object" ? (operation as Record<string, unknown>) : {};
                  const slotId = text(op.slotId);
                  if (!slotId || !slotIds.has(slotId)) return null;
                  return {
                    id: text(op.id) || id(),
                    slotId,
                    op: operationKind(op.op),
                    value: optional(op.value),
                    from: optional(op.from),
                  };
                })
                .filter((item): item is MorphOperation => item !== null)
                .slice(0, MAX_OPERATIONS)
            : [];
          return {
            id: text(entry.id) || id(),
            name: text(entry.name) || "Untitled rule",
            kind: kind(entry.kind ?? record.kind),
            match: optional(entry.match),
            notes: optional(entry.notes, LONG),
            operations,
          };
        })
        .slice(0, MAX_RULES)
    : [];
  return {
    name: text(record.name),
    kind: kind(record.kind),
    partOfSpeech: optional(record.partOfSpeech),
    notes: optional(record.notes, LONG),
    slots,
    rules,
  };
}

export function serializeParadigm(value: Paradigm): Record<string, unknown> {
  return normalizeParadigm(value);
}

export function applyOperation(stem: string, operation: MorphOperation): string {
  const value = operation.value ?? "";
  switch (operation.op) {
    case "prefix":
      return `${value}${stem}`;
    case "replace-suffix": {
      const from = operation.from ?? "";
      if (from && stem.endsWith(from)) return `${stem.slice(0, -from.length)}${value}`;
      return `${stem}${value}`;
    }
    case "identity":
      return stem;
    default:
      return `${stem}${value}`;
  }
}

export function ruleMatches(rule: MorphRule, stem: string): boolean {
  return !rule.match || stem.endsWith(rule.match);
}

export function generatedForm(paradigm: Paradigm, stem: string, slotId: string) {
  const candidates = paradigm.rules
    .filter((rule) => ruleMatches(rule, stem))
    .flatMap((rule) => {
      const operation = [...rule.operations].reverse().find((item) => item.slotId === slotId);
      return operation ? [{ rule, operation, specificity: rule.match?.length ?? 0 }] : [];
    })
    .sort((left, right) => right.specificity - left.specificity);
  const best = candidates[0];
  if (!best) return null;
  return { form: applyOperation(stem, best.operation), rule: best.rule, operation: best.operation };
}

export function authoredOverride(forms: LexemeForm[], paradigmId: string, slot: ParadigmSlot) {
  return (
    forms.find((item) => item.paradigmId === paradigmId && item.slotId === slot.id) ??
    forms.find((item) => !item.slotId && item.kind === slot.label)
  );
}

export function previewParadigm(
  paradigm: Paradigm,
  stem: string,
  forms: LexemeForm[] = [],
  paradigmId = "",
): ParadigmPreviewCell[] {
  return paradigm.slots.map((slot) => {
    const generated = generatedForm(paradigm, stem, slot.id);
    const authored = paradigmId ? authoredOverride(forms, paradigmId, slot) : undefined;
    if (authored) {
      return {
        slot,
        form: authored.form,
        provenance: "authored" as const,
        generated: generated?.form,
        authoredFormId: authored.id,
        ruleId: generated?.rule.id,
        ruleName: generated?.rule.name,
      };
    }
    if (generated) {
      return {
        slot,
        form: generated.form,
        provenance: "generated" as const,
        generated: generated.form,
        ruleId: generated.rule.id,
        ruleName: generated.rule.name,
      };
    }
    return { slot, form: "", provenance: "missing" as const };
  });
}

export function pinOverride(forms: LexemeForm[], paradigmId: string, slot: ParadigmSlot, form: string): LexemeForm[] {
  const next = forms.map((item) => ({ ...item }));
  const existing = authoredOverride(next, paradigmId, slot);
  if (existing) {
    existing.form = form;
    existing.kind = existing.kind || slot.label;
    existing.paradigmId = paradigmId;
    existing.slotId = slot.id;
    existing.provenance = "override";
    return next;
  }
  return [
    ...next,
    {
      id: id(),
      form,
      kind: slot.label,
      paradigmId,
      slotId: slot.id,
      provenance: "override",
    },
  ];
}

export function clearOverride(forms: LexemeForm[], paradigmId: string, slot: ParadigmSlot): LexemeForm[] {
  const authored = authoredOverride(forms, paradigmId, slot);
  if (!authored) return forms;
  return forms.filter((item) => item.id !== authored.id);
}
