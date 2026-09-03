import {
  type GrammarExample,
  type GrammarLink,
  type ParadigmAxis,
  type ParadigmCell,
  type ParadigmConfig,
} from "./types.ts";

import {
  CELL_FORM,
  CELL_STATES,
  LINK_KINDS,
  MAX_ALTERNATES,
  MAX_AXES,
  MAX_AXIS_VALUES,
  MAX_CELLS,
  MAX_EXAMPLES,
  MAX_LINKS,
  NOTES,
  id,
  obj,
  optional,
  text,
} from "./normalize-primitives.ts";

export function normalizeExamples(value: unknown): GrammarExample[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item): GrammarExample | null => {
      const entry = obj(item);
      const exampleText = text(entry.text);
      if (!exampleText) return null;
      return {
        id: text(entry.id) || id(),
        text: exampleText,
        translation: optional(entry.translation),
        gloss: optional(entry.gloss, NOTES),
        notes: optional(entry.notes, NOTES),
      };
    })
    .filter((item): item is GrammarExample => item !== null)
    .slice(0, MAX_EXAMPLES);
}

export function normalizeLinks(value: unknown): GrammarLink[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item): GrammarLink | null => {
      const entry = obj(item);
      const targetId = text(entry.targetId);
      const kind = text(entry.kind);
      if (!targetId || !LINK_KINDS.has(kind)) return null;
      return {
        id: text(entry.id) || id(),
        kind: kind as GrammarLink["kind"],
        targetId,
        secondaryId: kind === "lexeme-example" ? optional(entry.secondaryId) : optional(entry.secondaryId),
        label: optional(entry.label),
      };
    })
    .filter((item): item is GrammarLink => item !== null)
    .slice(0, MAX_LINKS);
}

export function normalizeAxes(value: unknown): ParadigmAxis[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item): ParadigmAxis | null => {
      const entry = obj(item);
      const label = text(entry.label);
      const values = Array.isArray(entry.values)
        ? entry.values
            .map((raw): ParadigmAxis["values"][number] | null => {
              const valueEntry = obj(raw);
              const valueLabel = text(valueEntry.label);
              if (!valueLabel) return null;
              return {
                id: text(valueEntry.id) || id(),
                label: valueLabel,
                description: optional(valueEntry.description),
              };
            })
            .filter((item): item is ParadigmAxis["values"][number] => item !== null)
            .slice(0, MAX_AXIS_VALUES)
        : [];
      if (!label || values.length === 0) return null;
      return { id: text(entry.id) || id(), label, values };
    })
    .filter((item): item is ParadigmAxis => item !== null)
    .slice(0, MAX_AXES);
}

export function normalizeCells(value: unknown, axes: ParadigmAxis[], examples: GrammarExample[]): ParadigmCell[] {
  const axisIds = new Set(axes.map((axis) => axis.id));
  const valueIds = new Map(axes.map((axis) => [axis.id, new Set(axis.values.map((item) => item.id))]));
  const exampleIds = new Set(examples.map((item) => item.id));
  if (!Array.isArray(value)) return [];
  const cells = value
    .map((item): ParadigmCell | null => {
      const entry = obj(item);
      const coordinates = obj(entry.coordinates);
      const next: Record<string, string> = {};
      for (const [axisId, raw] of Object.entries(coordinates)) {
        const valueId = text(raw);
        if (!axisIds.has(axisId) || !valueIds.get(axisId)?.has(valueId)) continue;
        next[axisId] = valueId;
      }
      if (Object.keys(next).length !== axes.length) return null;
      const state = CELL_STATES.has(text(entry.state)) ? (text(entry.state) as ParadigmCell["state"]) : "form";
      const exampleId = optional(entry.exampleId);
      return {
        id: text(entry.id) || id(),
        coordinates: next,
        state,
        form: state === "form" ? optional(entry.form, CELL_FORM) : undefined,
        alternateForms: Array.isArray(entry.alternateForms)
          ? entry.alternateForms
              .map((item) => text(item, CELL_FORM))
              .filter(Boolean)
              .slice(0, MAX_ALTERNATES)
          : undefined,
        sameAsCellId: state === "same-as" ? optional(entry.sameAsCellId) : undefined,
        notes: optional(entry.notes, NOTES),
        exampleId: exampleId && exampleIds.has(exampleId) ? exampleId : undefined,
      };
    })
    .filter((item): item is ParadigmCell => item !== null)
    .slice(0, MAX_CELLS);
  const ids = new Set(cells.map((cell) => cell.id));
  return cells.map((cell) =>
    cell.sameAsCellId && !ids.has(cell.sameAsCellId) ? { ...cell, sameAsCellId: undefined } : cell,
  );
}

export function paradigm(raw: Record<string, unknown>, examples: GrammarExample[]): ParadigmConfig {
  const axes = normalizeAxes(raw.axes);
  return { axes, cells: normalizeCells(raw.cells, axes, examples) };
}
