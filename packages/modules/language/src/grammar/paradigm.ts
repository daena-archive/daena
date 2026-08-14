import { button, emptyMessage, field, input, textarea } from "../ui.ts";
import { MAX_AXES, MAX_AXIS_VALUES, MAX_CELLS } from "./normalize.ts";
import type {
  ArgumentIndexingConfig,
  ArgumentParticipants,
  ArgumentRepresentation,
  DemonstrativeConfig,
  GrammarSystemId,
  GrammarSystemRecord,
  ParadigmAxis,
  ParadigmCell,
  ParadigmCellState,
  ParadigmConfig,
} from "./types.ts";

export const PARADIGM_SYSTEM_IDS = [
  "pronouns.personal",
  "pronouns.demonstratives",
  "verbs.argument-indexing",
] as const satisfies readonly GrammarSystemId[];

export type ParadigmSystemId = (typeof PARADIGM_SYSTEM_IDS)[number];

export type AxisValueTemplate = { id: string; label: string };

export const PERSON_VALUES: AxisValueTemplate[] = [
  { id: "person-1", label: "1st" },
  { id: "person-2", label: "2nd" },
  { id: "person-3", label: "3rd" },
];

export const NUMBER_VALUES: AxisValueTemplate[] = [
  { id: "number-sg", label: "Singular" },
  { id: "number-pl", label: "Plural" },
  { id: "number-du", label: "Dual" },
];

export const DISTANCE_VALUES: AxisValueTemplate[] = [
  { id: "distance-proximal", label: "Proximal" },
  { id: "distance-distal", label: "Distal" },
  { id: "distance-medial", label: "Medial" },
  { id: "distance-remote", label: "Very distant" },
];

export const EXTRA_PRONOUN_AXES: { id: string; label: string; values: AxisValueTemplate[] }[] = [
  { id: "clusivity", label: "Inclusive / exclusive", values: [{ id: "clusivity-in", label: "Inclusive" }, { id: "clusivity-ex", label: "Exclusive" }] },
  { id: "gender", label: "Gender", values: [{ id: "gender-m", label: "Masculine" }, { id: "gender-f", label: "Feminine" }, { id: "gender-n", label: "Neuter" }] },
  { id: "noun-class", label: "Noun class", values: [{ id: "class-a", label: "Class A" }, { id: "class-b", label: "Class B" }] },
  { id: "case", label: "Case", values: [{ id: "case-nom", label: "Nominative" }, { id: "case-acc", label: "Accusative" }] },
  { id: "animacy", label: "Animacy", values: [{ id: "animacy-anim", label: "Animate" }, { id: "animacy-inan", label: "Inanimate" }] },
  { id: "formality", label: "Formality", values: [{ id: "formality-fam", label: "Familiar" }, { id: "formality-form", label: "Formal" }] },
  { id: "proximity", label: "Proximity", values: [{ id: "proximity-near", label: "Near" }, { id: "proximity-far", label: "Far" }] },
];

export const EXTRA_DEMONSTRATIVE_AXES: { id: string; label: string; values: AxisValueTemplate[] }[] = [
  { id: "number", label: "Number", values: NUMBER_VALUES },
  { id: "gender", label: "Gender / class", values: [{ id: "gender-m", label: "Masculine" }, { id: "gender-f", label: "Feminine" }] },
  { id: "visibility", label: "Visibility", values: [{ id: "vis-seen", label: "Visible" }, { id: "vis-unseen", label: "Not visible" }] },
  { id: "elevation", label: "Elevation", values: [{ id: "elev-up", label: "Uphill" }, { id: "elev-down", label: "Downhill" }] },
  { id: "direction", label: "Direction", values: [{ id: "dir-towards", label: "Towards" }, { id: "dir-away", label: "Away" }] },
  { id: "discourse", label: "Discourse status", values: [{ id: "disc-new", label: "New" }, { id: "disc-given", label: "Given" }] },
];

export const PARTICIPANT_OPTIONS: { value: ArgumentParticipants; label: string }[] = [
  { value: "none", label: "No" },
  { value: "subject", label: "Subject only" },
  { value: "object", label: "Object only" },
  { value: "subject-object", label: "Subject and object" },
  { value: "other", label: "Other" },
];

export const REPRESENTATION_OPTIONS: { value: ArgumentRepresentation; label: string }[] = [
  { value: "endings", label: "Endings" },
  { value: "prefixes", label: "Prefixes" },
  { value: "full-forms", label: "Full forms" },
  { value: "auxiliaries", label: "Auxiliary forms" },
  { value: "flexible-table", label: "Flexible table" },
  { value: "custom", label: "Custom" },
];

export const CELL_STATE_OPTIONS: { value: ParadigmCellState; label: string }[] = [
  { value: "form", label: "Form" },
  { value: "same-as", label: "Same as another form" },
  { value: "zero", label: "Omitted / zero" },
  { value: "not-applicable", label: "Not applicable" },
];

export type ParadigmMutation = {
  draft: GrammarSystemRecord;
  blocked?: { label: string; populated: number; references: number };
  retry?: () => GrammarSystemRecord;
};

export function isParadigmSystem(systemId: GrammarSystemId): systemId is ParadigmSystemId {
  return (PARADIGM_SYSTEM_IDS as readonly string[]).includes(systemId);
}

export function coordKey(coordinates: Record<string, string>) {
  return Object.keys(coordinates)
    .sort()
    .map((key) => `${key}=${coordinates[key]}`)
    .join("|");
}

export function cartesianCoordinates(axes: ParadigmAxis[]): Record<string, string>[] {
  if (!axes.length || axes.some((axis) => axis.values.length === 0)) return [];
  return axes.reduce<Record<string, string>[]>((rows, axis) => {
    if (!rows.length) return axis.values.map((value) => ({ [axis.id]: value.id }));
    return rows.flatMap((row) => axis.values.map((value) => ({ ...row, [axis.id]: value.id })));
  }, []);
}

export function isPopulatedCell(cell: ParadigmCell) {
  if (cell.state === "zero" || cell.state === "not-applicable" || cell.state === "same-as") return true;
  return Boolean(cell.form?.trim() || cell.notes?.trim() || cell.alternateForms?.length);
}

export function syncParadigm(axes: ParadigmAxis[], cells: ParadigmCell[]): ParadigmCell[] {
  const combos = cartesianCoordinates(axes).slice(0, MAX_CELLS);
  const previous = new Map(cells.map((cell) => [coordKey(cell.coordinates), cell]));
  const next = combos.map((coordinates) => {
    const prior = previous.get(coordKey(coordinates));
    return prior ? { ...prior, coordinates } : { id: newId(), coordinates, state: "form" as const };
  });
  const ids = new Set(next.map((cell) => cell.id));
  return next.map((cell) =>
    cell.sameAsCellId && !ids.has(cell.sameAsCellId) ? { ...cell, sameAsCellId: undefined } : cell,
  );
}

export function toggleAxisValue(
  draft: GrammarSystemRecord,
  axisId: string,
  value: AxisValueTemplate,
  options?: { force?: boolean; referenced?: Set<string> },
): ParadigmMutation {
  const axes = paradigmAxes(draft);
  const axis = axes.find((item) => item.id === axisId);
  const present = axis?.values.some((item) => item.id === value.id);
  if (present) return removeAxisValue(draft, axisId, value.id, options);
  const nextAxis = axis
    ? { ...axis, values: axis.values.length >= MAX_AXIS_VALUES ? axis.values : [...axis.values, { ...value }] }
    : { id: axisId, label: axisLabel(axisId), values: [{ ...value }] };
  if (!axis && axes.length >= MAX_AXES) return { draft };
  const nextAxes = axis ? axes.map((item) => (item.id === axisId ? nextAxis : item)) : [...axes, nextAxis];
  return { draft: setParadigm(draft, nextAxes, paradigmCells(draft)) };
}

export function removeAxisValue(
  draft: GrammarSystemRecord,
  axisId: string,
  valueId: string,
  options?: { force?: boolean; referenced?: Set<string> },
): ParadigmMutation {
  const axes = paradigmAxes(draft);
  const cells = paradigmCells(draft);
  const axis = axes.find((item) => item.id === axisId);
  if (!axis) return { draft };
  const nextAxes = axes
    .map((item) =>
      item.id === axisId ? { ...item, values: item.values.filter((value) => value.id !== valueId) } : item,
    )
    .filter((item) => item.values.length > 0);
  const blocked = removalBlock(axis.label, axes, cells, nextAxes, options);
  if (blocked) {
    return {
      draft,
      blocked,
      retry: () => removeAxisValue(draft, axisId, valueId, { ...options, force: true }).draft,
    };
  }
  return { draft: setParadigm(draft, nextAxes, cells) };
}

export function addParadigmAxis(
  draft: GrammarSystemRecord,
  axis: { id: string; label: string; values: AxisValueTemplate[] },
): GrammarSystemRecord {
  const axes = paradigmAxes(draft);
  if (axes.some((item) => item.id === axis.id) || axes.length >= MAX_AXES) return draft;
  return setParadigm(draft, [...axes, { id: axis.id, label: axis.label, values: axis.values.map((value) => ({ ...value })) }], paradigmCells(draft));
}

export function addCustomAxis(draft: GrammarSystemRecord): GrammarSystemRecord {
  return addParadigmAxis(draft, {
    id: newId(),
    label: "Custom",
    values: [{ id: newId(), label: "Value" }],
  });
}

export function addCustomAxisValue(draft: GrammarSystemRecord, axisId: string): GrammarSystemRecord {
  const axes = paradigmAxes(draft);
  const next = axes.map((axis) =>
    axis.id === axisId && axis.values.length < MAX_AXIS_VALUES
      ? { ...axis, values: [...axis.values, { id: newId(), label: "Custom" }] }
      : axis,
  );
  return setParadigm(draft, next, paradigmCells(draft));
}

export function renameAxisValue(draft: GrammarSystemRecord, axisId: string, valueId: string, label: string): GrammarSystemRecord {
  const axes = paradigmAxes(draft).map((axis) =>
    axis.id === axisId
      ? { ...axis, values: axis.values.map((value) => (value.id === valueId ? { ...value, label } : value)) }
      : axis,
  );
  return setParadigm(draft, axes, paradigmCells(draft), { relabelOnly: true });
}

export function removeParadigmAxis(
  draft: GrammarSystemRecord,
  axisId: string,
  options?: { force?: boolean; referenced?: Set<string> },
): ParadigmMutation {
  const axes = paradigmAxes(draft);
  const axis = axes.find((item) => item.id === axisId);
  const nextAxes = axes.filter((item) => item.id !== axisId);
  const blocked = removalBlock(axis?.label ?? "dimension", axes, paradigmCells(draft), nextAxes, options);
  if (blocked) {
    return {
      draft,
      blocked,
      retry: () => removeParadigmAxis(draft, axisId, { ...options, force: true }).draft,
    };
  }
  return { draft: setParadigm(draft, nextAxes, paradigmCells(draft)) };
}

export function updateParadigmCell(
  draft: GrammarSystemRecord,
  cellId: string,
  patch: Partial<Omit<ParadigmCell, "id" | "coordinates">>,
): GrammarSystemRecord {
  const cells = paradigmCells(draft).map((cell) => (cell.id === cellId ? { ...cell, ...patch, id: cell.id, coordinates: cell.coordinates } : cell));
  return setParadigm(draft, paradigmAxes(draft), cells, { relabelOnly: true });
}

export function toggleDistance(draft: GrammarSystemRecord, value: AxisValueTemplate, options?: { force?: boolean; referenced?: Set<string> }): ParadigmMutation {
  if (draft.systemId !== "pronouns.demonstratives") return { draft };
  const result = toggleAxisValue(draft, "distance", value, options);
  if (result.blocked) return result;
  const axes = paradigmAxes(result.draft);
  const distance = axes.find((axis) => axis.id === "distance");
  return {
    draft: setDemonstrative(result.draft, {
      distances: distance?.values.map((item) => item.id) ?? [],
      axes,
      cells: paradigmCells(result.draft),
    }),
  };
}

export function setArgumentParticipants(draft: GrammarSystemRecord, participants: ArgumentParticipants, seed?: ParadigmAxis[]): GrammarSystemRecord {
  if (draft.systemId !== "verbs.argument-indexing") return draft;
  const current = argumentConfig(draft);
  if (participants === "none") {
    return setArgument(draft, { participants, representation: current.representation, axes: [], cells: [], agreementRecordId: current.agreementRecordId });
  }
  const axes = current.axes.length ? current.axes : seedPersonNumber(seed);
  return setArgument(draft, { ...current, participants, axes, cells: current.cells });
}

export function setArgumentRepresentation(draft: GrammarSystemRecord, representation: ArgumentRepresentation): GrammarSystemRecord {
  if (draft.systemId !== "verbs.argument-indexing") return draft;
  return setArgument(draft, { ...argumentConfig(draft), representation });
}

export function setArgumentFlexibleNotes(draft: GrammarSystemRecord, flexibleNotes: string): GrammarSystemRecord {
  if (draft.systemId !== "verbs.argument-indexing") return draft;
  return setArgument(draft, { ...argumentConfig(draft), flexibleNotes });
}

export function setArgumentAgreement(draft: GrammarSystemRecord, agreementRecordId: string | undefined): GrammarSystemRecord {
  if (draft.systemId !== "verbs.argument-indexing") return draft;
  return setArgument(draft, { ...argumentConfig(draft), agreementRecordId });
}

export function summarizeParadigm(systemId: GrammarSystemId, config: GrammarSystemRecord["config"]): string | undefined {
  if (systemId === "pronouns.personal") return axisSummary(config as ParadigmConfig);
  if (systemId === "pronouns.demonstratives") {
    const value = config as DemonstrativeConfig;
    if (value.distances?.length) {
      return value.distances
        .map((id) => DISTANCE_VALUES.find((item) => item.id === id)?.label ?? id.replace("distance-", ""))
        .join(" / ");
    }
    return axisSummary(value);
  }
  if (systemId === "verbs.argument-indexing") {
    const value = config as ArgumentIndexingConfig;
    if (!value.participants) return undefined;
    const participant = PARTICIPANT_OPTIONS.find((item) => item.value === value.participants)?.label ?? value.participants;
    const representation = value.representation
      ? REPRESENTATION_OPTIONS.find((item) => item.value === value.representation)?.label
      : undefined;
    return [participant === "No" ? "Does not index participants" : participant, representation, axisSummary(value)].filter(Boolean).join(" · ");
  }
  return undefined;
}

export type ParadigmEditorContext = {
  confirm: (message: string) => boolean;
  referencedIds: Set<string>;
  pronounAxes?: ParadigmAxis[];
  agreements: { id: string; title: string }[];
};

export function renderParadigmEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ParadigmEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
): HTMLElement | null {
  if (!isParadigmSystem(draft.systemId)) return null;
  const section = document.createElement("section");
  section.className = "language-group grammar-choice-stack";
  if (draft.systemId === "pronouns.personal") section.append(personalEditor(draft, locked, ctx, onChange));
  else if (draft.systemId === "pronouns.demonstratives") section.append(demonstrativeEditor(draft, locked, ctx, onChange));
  else section.append(argumentEditor(draft, locked, ctx, onChange));
  return section;
}

function personalEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ParadigmEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  wrap.append(
    emptyMessage("Start with person and number. Add other distinctions only if the language uses them."),
    valueChecks("Person", "person", PERSON_VALUES, draft, locked, ctx, onChange),
    valueChecks("Number", "number", NUMBER_VALUES, draft, locked, ctx, onChange),
    extraAxisControls(draft, locked, EXTRA_PRONOUN_AXES, ctx, onChange),
    paradigmGrid(draft, locked, onChange),
  );
  return wrap;
}

function demonstrativeEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ParadigmEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  wrap.append(emptyMessage("The grid is generated only from the dimensions you select."));
  const selected = new Set((draft.config as DemonstrativeConfig).distances ?? paradigmAxes(draft).find((axis) => axis.id === "distance")?.values.map((item) => item.id));
  wrap.append(templateChecks("Distance distinctions", DISTANCE_VALUES, selected, locked, (value) => {
    applyMutation(toggleDistance(draft, value, { referenced: ctx.referencedIds }), ctx, onChange);
  }));
  wrap.append(extraAxisControls(draft, locked, EXTRA_DEMONSTRATIVE_AXES.filter((axis) => axis.id !== "distance"), ctx, onChange), paradigmGrid(draft, locked, onChange));
  return wrap;
}

function argumentEditor(
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ParadigmEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const config = argumentConfig(draft);
  wrap.append(
    emptyMessage("Describe whether the verb changes based on who takes part. This is not always the same as Agreement."),
    radios("Do verbs change based on their participants?", PARTICIPANT_OPTIONS, config.participants, locked, (value) => {
      onChange(setArgumentParticipants(draft, value as ArgumentParticipants, ctx.pronounAxes), true);
    }),
  );
  if (config.participants && config.participants !== "none") {
    wrap.append(
      radios("What kind of forms are these?", REPRESENTATION_OPTIONS, config.representation, locked, (value) => {
        onChange(setArgumentRepresentation(draft, value as ArgumentRepresentation), true);
      }),
    );
    if (ctx.agreements.length) {
      const select = document.createElement("select");
      select.disabled = locked;
      select.append(new Option("Do not link an Agreement system", ""));
      for (const agreement of ctx.agreements) select.append(new Option(agreement.title, agreement.id));
      select.value = config.agreementRecordId ?? "";
      select.onchange = () => onChange(setArgumentAgreement(draft, select.value || undefined), true);
      wrap.append(field("Analyze as Agreement (optional)", select));
    }
    if (config.representation === "flexible-table") {
      const notes = textarea("flexibleNotes", config.flexibleNotes ?? "", 4);
      notes.disabled = locked;
      notes.oninput = () => onChange(setArgumentFlexibleNotes(draft, notes.value), false);
      wrap.append(field("Flexible table notes", notes));
    } else if (!config.agreementRecordId) {
      wrap.append(
        valueChecks("Person", "person", PERSON_VALUES, draft, locked, ctx, onChange),
        valueChecks("Number", "number", NUMBER_VALUES, draft, locked, ctx, onChange),
        extraAxisControls(draft, locked, EXTRA_PRONOUN_AXES, ctx, onChange),
        paradigmGrid(draft, locked, onChange),
      );
    } else {
      wrap.append(emptyMessage("This display is linked to an Agreement system. Edit the relationship there instead of copying person/number rules."));
    }
  }
  return wrap;
}

function extraAxisControls(
  draft: GrammarSystemRecord,
  locked: boolean,
  extras: { id: string; label: string; values: AxisValueTemplate[] }[],
  ctx: ParadigmEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-choice-stack";
  const axes = paradigmAxes(draft);
  if (!locked) {
    const add = document.createElement("select");
    add.setAttribute("aria-label", "Add distinction");
    add.append(new Option("Add distinction…", ""));
    for (const extra of extras) {
      if (!axes.some((axis) => axis.id === extra.id)) add.append(new Option(extra.label, extra.id));
    }
    add.append(new Option("Custom dimension", "custom"));
    add.onchange = () => {
      if (!add.value) return;
      onChange(add.value === "custom" ? addCustomAxis(draft) : addParadigmAxis(draft, extras.find((item) => item.id === add.value)!), true);
    };
    wrap.append(add);
  }
  for (const axis of axes) {
    if (axis.id === "person" || axis.id === "number" || axis.id === "distance") continue;
    const extra = extras.find((item) => item.id === axis.id);
    wrap.append(valueChecks(axis.label, axis.id, extra?.values ?? axis.values, draft, locked, ctx, onChange));
    if (!locked) {
      wrap.append(
        button("Add value", "language-button secondary", () => onChange(addCustomAxisValue(draft, axis.id), true)),
        button("Remove distinction", "language-button secondary language-danger", () => {
          applyMutation(removeParadigmAxis(draft, axis.id, { referenced: ctx.referencedIds }), ctx, onChange);
        }),
      );
    }
  }
  return wrap;
}

function valueChecks(
  legendText: string,
  axisId: string,
  templates: AxisValueTemplate[],
  draft: GrammarSystemRecord,
  locked: boolean,
  ctx: ParadigmEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const axis = paradigmAxes(draft).find((item) => item.id === axisId);
  const selected = new Set(axis?.values.map((item) => item.id));
  const known = new Set(templates.map((item) => item.id));
  const extras = (axis?.values ?? []).filter((item) => !known.has(item.id));
  const group = templateChecks(legendText, templates, selected, locked, (value) => {
    applyMutation(toggleAxisValue(draft, axisId, value, { referenced: ctx.referencedIds }), ctx, onChange);
  });
  for (const value of extras) {
    const row = document.createElement("label");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = true;
    box.disabled = locked;
    box.onchange = () => applyMutation(removeAxisValue(draft, axisId, value.id, { referenced: ctx.referencedIds }), ctx, onChange);
    const name = input("label", value.label);
    name.disabled = locked;
    name.oninput = () => onChange(renameAxisValue(draft, axisId, value.id, name.value), false);
    row.append(box, name);
    group.append(row);
  }
  return group;
}

function paradigmGrid(
  draft: GrammarSystemRecord,
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  const axes = paradigmAxes(draft);
  const cells = paradigmCells(draft);
  if (!axes.length) return emptyMessage("Select at least one distinction to generate the paradigm.");
  const wrap = document.createElement("div");
  wrap.className = "grammar-paradigm";
  wrap.setAttribute("role", "group");
  wrap.setAttribute("aria-label", "Paradigm");
  const table = document.createElement("table");
  table.className = "grammar-paradigm-table";
  const caption = document.createElement("caption");
  caption.className = "visually-hidden";
  caption.textContent = "Paradigm";
  table.append(caption);
  const colAxis = axes.length > 1 ? axes[axes.length - 1] : undefined;
  const rowAxes = axes.length > 1 ? axes.slice(0, -1) : axes;
  const rowCombos = cartesianCoordinates(rowAxes);
  const head = document.createElement("thead");
  const headRow = document.createElement("tr");
  const corner = document.createElement("th");
  corner.scope = "col";
  corner.textContent = rowAxes.map((axis) => axis.label).join(" · ");
  headRow.append(corner);
  const columns = colAxis ? colAxis.values : [{ id: "", label: "" }];
  for (const column of columns) {
    const cell = document.createElement("th");
    cell.scope = "col";
    cell.textContent = column.label || colAxis?.label || "Form";
    headRow.append(cell);
  }
  head.append(headRow);
  table.append(head);
  const body = document.createElement("tbody");
  for (const row of rowCombos) {
    const tr = document.createElement("tr");
    const th = document.createElement("th");
    th.scope = "row";
    th.textContent = rowAxes.map((axis) => axis.values.find((value) => value.id === row[axis.id])?.label ?? "").join(" · ");
    tr.append(th);
    for (const column of columns) {
      const coordinates = colAxis ? { ...row, [colAxis.id]: column.id } : row;
      const cell = cells.find((item) => coordKey(item.coordinates) === coordKey(coordinates));
      const td = document.createElement("td");
      if (cell) td.append(cellEditor(cell, cells, locked, onChange, draft));
      tr.append(td);
    }
    body.append(tr);
  }
  table.append(body);
  wrap.append(table);
  return wrap;
}

function cellEditor(
  cell: ParadigmCell,
  cells: ParadigmCell[],
  locked: boolean,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
  draft: GrammarSystemRecord,
) {
  const wrap = document.createElement("div");
  wrap.className = "grammar-paradigm-cell";
  const state = document.createElement("select");
  state.setAttribute("aria-label", "Cell state");
  state.disabled = locked;
  for (const option of CELL_STATE_OPTIONS) state.append(new Option(option.label, option.value));
  state.value = cell.state;
  state.onchange = () => onChange(updateParadigmCell(draft, cell.id, { state: state.value as ParadigmCellState }), true);
  wrap.append(state);
  if (cell.state === "form") {
    const form = input("form", cell.form ?? "");
    form.setAttribute("aria-label", "Form");
    form.disabled = locked;
    form.oninput = () => onChange(updateParadigmCell(draft, cell.id, { form: form.value }), false);
    wrap.append(form);
  } else if (cell.state === "same-as") {
    const select = document.createElement("select");
    select.setAttribute("aria-label", "Same as");
    select.disabled = locked;
    select.append(new Option("Choose a form…", ""));
    for (const other of cells) {
      if (other.id === cell.id) continue;
      const label = Object.values(other.coordinates).join(" · ") + (other.form ? ` (${other.form})` : "");
      select.append(new Option(label, other.id));
    }
    select.value = cell.sameAsCellId ?? "";
    select.onchange = () => onChange(updateParadigmCell(draft, cell.id, { sameAsCellId: select.value || undefined }), true);
    wrap.append(select);
  }
  const notes = input("notes", cell.notes ?? "");
  notes.placeholder = "Notes";
  notes.setAttribute("aria-label", "Notes");
  notes.disabled = locked;
  notes.oninput = () => onChange(updateParadigmCell(draft, cell.id, { notes: notes.value }), false);
  wrap.append(notes);
  return wrap;
}

function templateChecks(
  legendText: string,
  templates: AxisValueTemplate[],
  selected: Set<string>,
  locked: boolean,
  onToggle: (value: AxisValueTemplate) => void,
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
    box.onchange = () => onToggle(template);
    label.append(box, ` ${template.label}`);
    group.append(label);
  }
  return group;
}

function radios(
  legendText: string,
  options: { value: string; label: string }[],
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
    group.append(card);
  }
  return group;
}

function applyMutation(
  result: ParadigmMutation,
  ctx: ParadigmEditorContext,
  onChange: (next: GrammarSystemRecord, rerender: boolean) => void,
) {
  if (result.blocked) {
    const extra = result.blocked.references ? ` ${result.blocked.references} agreement reference(s) will break.` : "";
    if (
      !ctx.confirm(
        `Removing ${result.blocked.label} will discard ${result.blocked.populated} filled cell(s).${extra} Continue?`,
      )
    ) {
      return;
    }
    if (result.retry) onChange(result.retry(), true);
    return;
  }
  onChange(result.draft, true);
}

function removalBlock(
  label: string,
  previousAxes: ParadigmAxis[],
  cells: ParadigmCell[],
  nextAxes: ParadigmAxis[],
  options?: { force?: boolean; referenced?: Set<string> },
) {
  if (options?.force) return undefined;
  const kept = new Set(syncParadigm(nextAxes, cells).map((cell) => cell.id));
  const dropped = cells.filter((cell) => !kept.has(cell.id));
  const populated = dropped.filter(isPopulatedCell).length;
  const remainingValues = new Set(nextAxes.flatMap((axis) => axis.values.map((value) => value.id)));
  const removedValues = previousAxes.flatMap((axis) => axis.values.map((value) => value.id)).filter((id) => !remainingValues.has(id));
  const references =
    dropped.filter((cell) => options?.referenced?.has(cell.id)).length
    + removedValues.filter((id) => options?.referenced?.has(id)).length;
  if (populated === 0 && references === 0) return undefined;
  return { label, populated, references };
}

function axisLabel(axisId: string) {
  if (axisId === "person") return "Person";
  if (axisId === "number") return "Number";
  if (axisId === "distance") return "Distance";
  return EXTRA_PRONOUN_AXES.find((item) => item.id === axisId)?.label
    ?? EXTRA_DEMONSTRATIVE_AXES.find((item) => item.id === axisId)?.label
    ?? "Distinction";
}

function axisSummary(config: { axes?: ParadigmAxis[] }) {
  if (!config.axes?.length) return undefined;
  return config.axes.map((axis) => `${axis.label}: ${axis.values.map((item) => item.label).join("/")}`).join(" · ");
}

function seedPersonNumber(seed?: ParadigmAxis[]): ParadigmAxis[] {
  const person = seed?.find((axis) => axis.id === "person");
  const number = seed?.find((axis) => axis.id === "number");
  if (person || number) return [person, number].filter((axis): axis is ParadigmAxis => Boolean(axis)).map((axis) => ({
    ...axis,
    values: axis.values.map((value) => ({ ...value })),
  }));
  return defaultPersonNumberAxes();
}

function defaultPersonNumberAxes(): ParadigmAxis[] {
  return [
    { id: "person", label: "Person", values: PERSON_VALUES.map((item) => ({ ...item })) },
    { id: "number", label: "Number", values: NUMBER_VALUES.filter((item) => item.id !== "number-du").map((item) => ({ ...item })) },
  ];
}

function paradigmAxes(draft: GrammarSystemRecord): ParadigmAxis[] {
  return Array.isArray((draft.config as ParadigmConfig).axes) ? (draft.config as ParadigmConfig).axes : [];
}

function paradigmCells(draft: GrammarSystemRecord): ParadigmCell[] {
  return Array.isArray((draft.config as ParadigmConfig).cells) ? (draft.config as ParadigmConfig).cells : [];
}

function argumentConfig(draft: GrammarSystemRecord): ArgumentIndexingConfig {
  const config = draft.config as ArgumentIndexingConfig;
  return {
    participants: config.participants,
    representation: config.representation,
    axes: config.axes ?? [],
    cells: config.cells ?? [],
    flexibleNotes: config.flexibleNotes,
    agreementRecordId: config.agreementRecordId,
  };
}

function setParadigm(draft: GrammarSystemRecord, axes: ParadigmAxis[], cells: ParadigmCell[], options?: { relabelOnly?: boolean }): GrammarSystemRecord {
  const nextCells = options?.relabelOnly ? cells : syncParadigm(axes, cells);
  if (draft.systemId === "pronouns.demonstratives") {
    const distance = axes.find((axis) => axis.id === "distance");
    return setDemonstrative(draft, { distances: distance?.values.map((item) => item.id) ?? [], axes, cells: nextCells });
  }
  if (draft.systemId === "verbs.argument-indexing") {
    return setArgument(draft, { ...argumentConfig(draft), axes, cells: nextCells });
  }
  return { ...draft, status: "configured", config: { axes, cells: nextCells } };
}

function setDemonstrative(draft: GrammarSystemRecord, config: DemonstrativeConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config };
}

function setArgument(draft: GrammarSystemRecord, config: ArgumentIndexingConfig): GrammarSystemRecord {
  return { ...draft, status: "configured", config: { ...config, axes: config.axes, cells: syncParadigm(config.axes, config.cells) } };
}

function newId() {
  return crypto.randomUUID();
}
