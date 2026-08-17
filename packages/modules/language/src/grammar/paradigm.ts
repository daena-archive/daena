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
  {
    id: "clusivity",
    label: "Inclusive / exclusive",
    values: [
      { id: "clusivity-in", label: "Inclusive" },
      { id: "clusivity-ex", label: "Exclusive" },
    ],
  },
  {
    id: "gender",
    label: "Gender",
    values: [
      { id: "gender-m", label: "Masculine" },
      { id: "gender-f", label: "Feminine" },
      { id: "gender-n", label: "Neuter" },
    ],
  },
  {
    id: "noun-class",
    label: "Noun class",
    values: [
      { id: "class-a", label: "Class A" },
      { id: "class-b", label: "Class B" },
    ],
  },
  {
    id: "case",
    label: "Case",
    values: [
      { id: "case-nom", label: "Nominative" },
      { id: "case-acc", label: "Accusative" },
    ],
  },
  {
    id: "animacy",
    label: "Animacy",
    values: [
      { id: "animacy-anim", label: "Animate" },
      { id: "animacy-inan", label: "Inanimate" },
    ],
  },
  {
    id: "formality",
    label: "Formality",
    values: [
      { id: "formality-fam", label: "Familiar" },
      { id: "formality-form", label: "Formal" },
    ],
  },
  {
    id: "proximity",
    label: "Proximity",
    values: [
      { id: "proximity-near", label: "Near" },
      { id: "proximity-far", label: "Far" },
    ],
  },
];

export const EXTRA_DEMONSTRATIVE_AXES: { id: string; label: string; values: AxisValueTemplate[] }[] = [
  { id: "number", label: "Number", values: NUMBER_VALUES },
  {
    id: "gender",
    label: "Gender / class",
    values: [
      { id: "gender-m", label: "Masculine" },
      { id: "gender-f", label: "Feminine" },
    ],
  },
  {
    id: "visibility",
    label: "Visibility",
    values: [
      { id: "vis-seen", label: "Visible" },
      { id: "vis-unseen", label: "Not visible" },
    ],
  },
  {
    id: "elevation",
    label: "Elevation",
    values: [
      { id: "elev-up", label: "Uphill" },
      { id: "elev-down", label: "Downhill" },
    ],
  },
  {
    id: "direction",
    label: "Direction",
    values: [
      { id: "dir-towards", label: "Towards" },
      { id: "dir-away", label: "Away" },
    ],
  },
  {
    id: "discourse",
    label: "Discourse status",
    values: [
      { id: "disc-new", label: "New" },
      { id: "disc-given", label: "Given" },
    ],
  },
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

export function removalBlock(
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
  const removedValues = previousAxes
    .flatMap((axis) => axis.values.map((value) => value.id))
    .filter((id) => !remainingValues.has(id));
  const references =
    dropped.filter((cell) => options?.referenced?.has(cell.id)).length +
    removedValues.filter((id) => options?.referenced?.has(id)).length;
  if (populated === 0 && references === 0) return undefined;
  return { label, populated, references };
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
  return setParadigm(
    draft,
    [...axes, { id: axis.id, label: axis.label, values: axis.values.map((value) => ({ ...value })) }],
    paradigmCells(draft),
  );
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

export function renameAxisValue(
  draft: GrammarSystemRecord,
  axisId: string,
  valueId: string,
  label: string,
): GrammarSystemRecord {
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
  const cells = paradigmCells(draft).map((cell) =>
    cell.id === cellId ? { ...cell, ...patch, id: cell.id, coordinates: cell.coordinates } : cell,
  );
  return setParadigm(draft, paradigmAxes(draft), cells, { relabelOnly: true });
}

export function toggleDistance(
  draft: GrammarSystemRecord,
  value: AxisValueTemplate,
  options?: { force?: boolean; referenced?: Set<string> },
): ParadigmMutation {
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

export function setArgumentParticipants(
  draft: GrammarSystemRecord,
  participants: ArgumentParticipants,
  seed?: ParadigmAxis[],
): GrammarSystemRecord {
  if (draft.systemId !== "verbs.argument-indexing") return draft;
  const current = argumentConfig(draft);
  if (participants === "none") {
    return setArgument(draft, {
      participants,
      representation: current.representation,
      axes: [],
      cells: [],
      agreementRecordId: current.agreementRecordId,
    });
  }
  const axes = current.axes.length ? current.axes : seedPersonNumber(seed);
  return setArgument(draft, { ...current, participants, axes, cells: current.cells });
}

export function setArgumentRepresentation(
  draft: GrammarSystemRecord,
  representation: ArgumentRepresentation,
): GrammarSystemRecord {
  if (draft.systemId !== "verbs.argument-indexing") return draft;
  return setArgument(draft, { ...argumentConfig(draft), representation });
}

export function setArgumentFlexibleNotes(draft: GrammarSystemRecord, flexibleNotes: string): GrammarSystemRecord {
  if (draft.systemId !== "verbs.argument-indexing") return draft;
  return setArgument(draft, { ...argumentConfig(draft), flexibleNotes });
}

export function setArgumentAgreement(
  draft: GrammarSystemRecord,
  agreementRecordId: string | undefined,
): GrammarSystemRecord {
  if (draft.systemId !== "verbs.argument-indexing") return draft;
  return setArgument(draft, { ...argumentConfig(draft), agreementRecordId });
}

export function summarizeParadigm(
  systemId: GrammarSystemId,
  config: GrammarSystemRecord["config"],
): string | undefined {
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
    const participant =
      PARTICIPANT_OPTIONS.find((item) => item.value === value.participants)?.label ?? value.participants;
    const representation = value.representation
      ? REPRESENTATION_OPTIONS.find((item) => item.value === value.representation)?.label
      : undefined;
    return [participant === "No" ? "Does not index participants" : participant, representation, axisSummary(value)]
      .filter(Boolean)
      .join(" · ");
  }
  return undefined;
}

export type ParadigmEditorContext = {
  confirm: (message: string) => boolean;
  referencedIds: Set<string>;
  pronounAxes?: ParadigmAxis[];
  agreements: { id: string; title: string }[];
};

function axisLabel(axisId: string) {
  if (axisId === "person") return "Person";
  if (axisId === "number") return "Number";
  if (axisId === "distance") return "Distance";
  return (
    EXTRA_PRONOUN_AXES.find((item) => item.id === axisId)?.label ??
    EXTRA_DEMONSTRATIVE_AXES.find((item) => item.id === axisId)?.label ??
    "Distinction"
  );
}

function axisSummary(config: { axes?: ParadigmAxis[] }) {
  if (!config.axes?.length) return undefined;
  return config.axes.map((axis) => `${axis.label}: ${axis.values.map((item) => item.label).join("/")}`).join(" · ");
}

function seedPersonNumber(seed?: ParadigmAxis[]): ParadigmAxis[] {
  const person = seed?.find((axis) => axis.id === "person");
  const number = seed?.find((axis) => axis.id === "number");
  if (person || number)
    return [person, number]
      .filter((axis): axis is ParadigmAxis => Boolean(axis))
      .map((axis) => ({
        ...axis,
        values: axis.values.map((value) => ({ ...value })),
      }));
  return defaultPersonNumberAxes();
}

function defaultPersonNumberAxes(): ParadigmAxis[] {
  return [
    { id: "person", label: "Person", values: PERSON_VALUES.map((item) => ({ ...item })) },
    {
      id: "number",
      label: "Number",
      values: NUMBER_VALUES.filter((item) => item.id !== "number-du").map((item) => ({ ...item })),
    },
  ];
}

export function paradigmAxes(draft: GrammarSystemRecord): ParadigmAxis[] {
  return Array.isArray((draft.config as ParadigmConfig).axes) ? (draft.config as ParadigmConfig).axes : [];
}

export function paradigmCells(draft: GrammarSystemRecord): ParadigmCell[] {
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

function setParadigm(
  draft: GrammarSystemRecord,
  axes: ParadigmAxis[],
  cells: ParadigmCell[],
  options?: { relabelOnly?: boolean },
): GrammarSystemRecord {
  const nextCells = options?.relabelOnly ? cells : syncParadigm(axes, cells);
  if (draft.systemId === "pronouns.demonstratives") {
    const distance = axes.find((axis) => axis.id === "distance");
    return setDemonstrative(draft, {
      distances: distance?.values.map((item) => item.id) ?? [],
      axes,
      cells: nextCells,
    });
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
  return {
    ...draft,
    status: "configured",
    config: { ...config, axes: config.axes, cells: syncParadigm(config.axes, config.cells) },
  };
}

function newId() {
  return crypto.randomUUID();
}
