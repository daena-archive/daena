import type { Timeline, DataGroup, DataItem, TimelineOptions } from "vis-timeline";
import type { FieldRecord, ModuleContext, DaenaModule, EntitySummary } from "../../../module-api/src/index";
import type { ModuleManifest } from "../../../module-api/src/index";
import { parseCalendarDate, type CalendarDate } from "../../../../src/lib/date";
import {
  CALENDAR_DEFINITION_COLLECTION,
  calendarDateToParts,
  formatCalendarParts,
  formatWithCalendar,
  normalizeCalendarDefinition,
  type CalendarDefinition,
} from "./calendar";
import {
  buildFieldContributions,
  discoverTimelineFieldSpecs,
  timelineDateAnchor,
  type TimelineContribution,
  type TimelineFieldRole,
  type TimelineLayer,
} from "./projection";
import manifestJson from "../manifest.json";

const manifest = manifestJson as unknown as ModuleManifest;

type EventColors = { fill: string; border: string; text: string };

type TimelineEvent = {
  id: string;
  entity: { id: string; name: string; type?: string | null };
  startValue: unknown;
  endValue?: unknown;
  startLabel?: string;
  endLabel?: string;
  pointRole?: TimelineFieldRole;
  layer: "timeline" | TimelineLayer;
  locationName?: string;
  participantNames: string[];
  start: Date;
  end: Date | null;
  colors: EventColors;
};

type TimelineGroupId = "events" | "lifelines" | "dates";

type LoadedTimelineEntry = {
  entity: EntitySummary;
  fields: Record<string, unknown>;
  locationName?: string;
  participantNames: string[];
  relativeYear: number | null;
  contributions: TimelineContribution[];
};

type TimelineSourceSnapshot = {
  calendarOptions: CalendarOption[];
  loaded: LoadedTimelineEntry[];
};

type CalendarOption = {
  id: string;
  name: string;
  definition: CalendarDefinition | null;
};

type UndatedEvent = {
  entity: { id: string; name: string };
  fields: Record<string, unknown>;
  locationName?: string;
  participantNames: string[];
  relativeYear?: number;
};

type PhysicalChronology = {
  contractVersion: 1;
  kind: "physical-offset-years";
  reference: "accepted-source";
  startOffsetYears: number;
  endOffsetYears: number;
};

function physicalOffset(value: unknown): number | null {
  if (!value || typeof value !== "object") return null;
  const chronology = value as Partial<PhysicalChronology>;
  if (
    chronology.contractVersion !== 1 ||
    chronology.kind !== "physical-offset-years" ||
    chronology.reference !== "accepted-source" ||
    typeof chronology.startOffsetYears !== "number" ||
    !Number.isSafeInteger(chronology.startOffsetYears) ||
    typeof chronology.endOffsetYears !== "number" ||
    !Number.isSafeInteger(chronology.endOffsetYears) ||
    chronology.startOffsetYears < -100_000 ||
    chronology.endOffsetYears > 100_000 ||
    chronology.startOffsetYears > chronology.endOffsetYears
  )
    return null;
  return chronology.startOffsetYears;
}

function relativeOffsetLabel(year: number): string {
  return `${year >= 0 ? "+" : ""}${year} years from accepted source`;
}

function hslToHex(hue: number, saturation: number, lightness: number) {
  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const segment = hue / 60;
  const second = chroma * (1 - Math.abs((segment % 2) - 1));
  const [red, green, blue] =
    segment < 1
      ? [chroma, second, 0]
      : segment < 2
        ? [second, chroma, 0]
        : segment < 3
          ? [0, chroma, second]
          : segment < 4
            ? [0, second, chroma]
            : segment < 5
              ? [second, 0, chroma]
              : [chroma, 0, second];
  const match = lightness - chroma / 2;
  return `#${[red, green, blue]
    .map((channel) =>
      Math.round((channel + match) * 255)
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
}

function colorsForHue(hue: number): EventColors {
  return {
    fill: hslToHex(hue, 0.42, 0.82),
    border: hslToHex(hue, 0.44, 0.42),
    text: hslToHex(hue, 0.38, 0.22),
  };
}

function colorsForLayer(layer: TimelineEvent["layer"], entityId: string): EventColors {
  if (layer === "lifelines") return { fill: "#dfeae2", border: "#4f705a", text: "#284234" };
  if (layer === "dates") return { fill: "#f3e4cf", border: "#a56d32", text: "#5f3d20" };
  return colorsForHue(hueForId(entityId));
}

function groupForEvent(event: TimelineEvent): TimelineGroupId {
  if (event.layer === "lifelines") return "lifelines";
  if (event.layer === "dates") return "dates";
  return "events";
}

function layerLabel(event: TimelineEvent): string {
  if (event.layer === "lifelines") return "Lifeline";
  if (event.layer === "dates") return "Project date";
  return "Timeline event";
}

function entityTypeLabel(type: string | null | undefined): string {
  if (!type) return "Unknown type";
  return type.charAt(0).toUpperCase() + type.slice(1).replace(/[-_]+/g, " ");
}

function hueForId(id: string) {
  let hash = 2166136261;
  for (let index = 0; index < id.length; index += 1) {
    hash ^= id.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) % 360;
}

function asJsDate(value: unknown): Date | null {
  if (value instanceof Date) return Number.isFinite(value.getTime()) ? value : null;
  if (typeof value === "number" && Number.isFinite(value)) {
    const date = new Date(value);
    return Number.isFinite(date.getTime()) ? date : null;
  }
  if (value && typeof value === "object") {
    const candidate = value as { toDate?: () => Date; valueOf?: () => number };
    if (typeof candidate.toDate === "function") {
      const date = candidate.toDate();
      return date instanceof Date && Number.isFinite(date.getTime()) ? date : null;
    }
    if (typeof candidate.valueOf === "function") {
      const date = new Date(candidate.valueOf());
      return Number.isFinite(date.getTime()) ? date : null;
    }
  }
  return null;
}

function precisionForScale(scale?: string): CalendarDate["precision"] {
  if (scale === "year") return "year";
  if (scale === "month") return "month";
  if (scale === "day" || scale === "weekday" || scale === "week") return "day";
  if (scale === "hour") return "hour";
  if (scale === "minute") return "minute";
  return "second";
}

function calendarFromJsDate(value: Date, precision: CalendarDate["precision"] = "day"): CalendarDate {
  return {
    calendar: "gregorian",
    era: "CE",
    year: value.getUTCFullYear(),
    ...(precision !== "year" ? { month: value.getUTCMonth() + 1 } : {}),
    ...(!["year", "month"].includes(precision ?? "day") ? { day: value.getUTCDate() } : {}),
    ...(["hour", "minute", "second"].includes(precision ?? "day") ? { hour: value.getUTCHours() } : {}),
    ...(["minute", "second"].includes(precision ?? "day") ? { minute: value.getUTCMinutes() } : {}),
    ...(precision === "second" ? { second: value.getUTCSeconds() } : {}),
    precision,
  };
}

function formatAxisDate(value: unknown, definition: CalendarDefinition | null, scale?: string) {
  const date = asJsDate(value);
  if (!date) return "";
  const precision = precisionForScale(scale);
  if (!definition) return formatWithCalendar(calendarFromJsDate(date, precision), null);
  const parts = calendarDateToParts(calendarFromJsDate(date, "day"), definition);
  if (!parts) return "";
  return formatCalendarParts(
    {
      year: parts.year,
      ...(precision !== "year" ? { month: parts.month } : {}),
      ...(!["year", "month"].includes(precision ?? "day") ? { day: parts.day } : {}),
      ...(precision !== "year" ? { weekday: parts.weekday, season: parts.season } : {}),
      precision,
    },
    definition,
  );
}

function definitionForValue(value: unknown, definitions: ReadonlyMap<string, CalendarDefinition>) {
  const calendarId = parseCalendarDate(value)?.calendar;
  return calendarId ? (definitions.get(calendarId) ?? null) : null;
}

function formatStoredDate(value: unknown, definitions: ReadonlyMap<string, CalendarDefinition>) {
  return formatWithCalendar(value, definitionForValue(value, definitions));
}

function rangeLabel(event: TimelineEvent, definitions: ReadonlyMap<string, CalendarDefinition>) {
  const start = formatStoredDate(event.startValue, definitions);
  const end = event.endValue ? formatStoredDate(event.endValue, definitions) : "";
  if (!event.endValue || end === "Undated" || end === start)
    return event.startLabel ? `${event.startLabel}: ${start}` : start;
  const startText = event.startLabel ? `${event.startLabel}: ${start}` : start;
  const endText = event.endLabel ? `${event.endLabel}: ${end}` : end;
  return `${startText} – ${endText}`;
}

function eventYear(event: TimelineEvent, definition: CalendarDefinition | null): number {
  return calendarDateToParts(calendarFromJsDate(event.start, "day"), definition)?.year ?? event.start.getUTCFullYear();
}

async function loadCalendarOptions(
  context: ModuleContext,
  entities: readonly EntitySummary[],
): Promise<CalendarOption[]> {
  const calendars = entities.filter((entity) => entity.type === "daena.timeline:calendar");
  const custom: Array<CalendarOption | null> = await Promise.all(
    calendars.map(async (entity): Promise<CalendarOption | null> => {
      try {
        const records = await context.records.list(CALENDAR_DEFINITION_COLLECTION, entity.id, { limit: 1 });
        return {
          id: entity.id,
          name: entity.name,
          definition: normalizeCalendarDefinition(records[0]?.value ?? {}),
        };
      } catch {
        return null;
      }
    }),
  );
  return [
    { id: "gregorian", name: "Gregorian", definition: null },
    ...custom.filter((option): option is CalendarOption => option !== null),
  ];
}

function contextLabel(event: TimelineEvent | UndatedEvent): string {
  const labels = [event.locationName, event.participantNames.length ? event.participantNames.join(", ") : ""].filter(
    Boolean,
  );
  return labels.join(" · ");
}

function createTimelineStyles(): HTMLStyleElement {
  const style = document.createElement("style");
  style.textContent = `
    .timeline-shell { display: grid; gap: 0; background: var(--surface); }
    .timeline-toolbar { display: flex; align-items: flex-end; justify-content: space-between; gap: 12px; padding: 10px 14px; border-bottom: 1px solid var(--line-soft); background: var(--surface); }
    .timeline-toolbar-controls { display: flex; align-items: flex-end; flex: 1; flex-wrap: wrap; gap: 8px; }
    .timeline-toolbar-actions { display: flex; align-items: center; gap: 5px; }
    .timeline-toolbar button { border: 1px solid var(--line-strong); border-radius: 7px; padding: 7px 9px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-toolbar button:hover, .timeline-toolbar button:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .timeline-filter-field { display: grid; gap: 3px; color: var(--ink-muted); font: 600 8px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: .045em; text-transform: uppercase; }
    .timeline-filter-field input, .timeline-filter-field select { height: 30px; border: 1px solid var(--line-strong); border-radius: 7px; padding: 0 9px; background: var(--surface); color: var(--theme-neutral-text-soft, #51483e); font: 500 10px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: 0; text-transform: none; }
    .timeline-filter-field input:focus-visible, .timeline-filter-field select:focus-visible { border-color: var(--accent); box-shadow: 0 0 0 2px rgba(180, 119, 63, .12); outline: none; }
    .timeline-search { width: min(230px, 28vw); }
    .timeline-scope, .timeline-calendar, .timeline-type-filter { min-width: 120px; }
    .timeline-layerbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 14px; border-bottom: 1px solid var(--line-soft); background: var(--theme-warning-bg, #fbf8f0); }
    .timeline-layer-chips, .timeline-legend { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; }
    .timeline-layer-chip { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--line-strong); border-radius: 999px; padding: 5px 9px; background: var(--surface); color: var(--theme-neutral-text-soft, #766c60); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-layer-chip[aria-pressed="true"] { border-color: var(--theme-warning-border, #9c6c3d); background: var(--theme-warning-bg, #f1e4d3); color: var(--theme-warning-text, #51351f); }
    .timeline-layer-count { min-width: 17px; border-radius: 999px; padding: 1px 5px; background: rgba(48, 44, 38, .08); text-align: center; font-size: 9px; }
    .timeline-legend { color: var(--ink-muted); font: 9px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-legend-item { display: inline-flex; align-items: center; gap: 4px; white-space: nowrap; }
    .timeline-legend-dot { width: 7px; height: 7px; border: 2px solid var(--theme-warning-border, #a56d32); border-radius: 50%; background: var(--surface); }
    .timeline-legend-range { width: 18px; height: 6px; border: 1px solid var(--theme-success-border, #4f705a); border-radius: 999px; background: var(--theme-success-bg, #dfeae2); }
    .timeline-legend-approx { width: 18px; height: 6px; border: 1px dashed var(--ink-muted); border-radius: 2px; }
    .timeline-workspace { display: grid; grid-template-columns: var(--timeline-outline-width, 300px) 8px minmax(360px, 1fr) minmax(220px, 270px); min-height: 420px; }
    .timeline-outline { overflow: auto; max-height: min(62vh, 620px); padding: 10px; border-right: 1px solid var(--line-soft); background: var(--surface); }
    .timeline-outline-resize { cursor: col-resize; background: var(--surface-warm); touch-action: none; }
    .timeline-outline-resize: hover, .timeline-outline-resize:focus-visible { background: var(--theme-warning-border, #dfcfb8); outline: none; }
    .timeline-outline-heading { display: block; margin: 4px 5px 8px; color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: .04em; text-transform: uppercase; }
    .timeline-outline-year { margin: 12px 5px 5px; color: var(--ink-muted); font: 600 12px var(--font-display, Georgia, serif); }
    .timeline-event-card { display: grid; width: 100%; gap: 4px; margin: 2px 0; padding: 8px 9px; border: 1px solid transparent; border-radius: 8px; background: transparent; color: inherit; text-align: left; cursor: pointer; }
    .timeline-event-card:hover, .timeline-event-card:focus-visible, .timeline-event-card.is-selected { border-color: var(--line-strong); background: var(--theme-warning-bg, #f7f1e7); outline: none; }
    .timeline-event-card strong { color: var(--theme-neutral-text, #302c26); font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-event-card small { color: var(--ink-muted); font: 10px/1.35 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-card-meta { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
    .timeline-card-kind { border-radius: 999px; padding: 2px 5px; background: var(--theme-warning-bg, #eee6da); color: var(--theme-neutral-text-soft, #786b5d); font: 600 8px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: .025em; text-transform: uppercase; }
    .timeline-canvas { position: relative; height: min(62vh, 620px); min-height: 420px; background: radial-gradient(circle at 50% 42%, var(--theme-warning-bg, #fffdf7) 0, var(--theme-warning-bg, #fbf8f0) 52%, var(--surface-warm) 100%); }
    .timeline-canvas .vis-timeline { border: 0; background: transparent; }
    .timeline-canvas .vis-panel.vis-background, .timeline-canvas .vis-panel.vis-center, .timeline-canvas .vis-panel.vis-left, .timeline-canvas .vis-panel.vis-right, .timeline-canvas .vis-panel.vis-top, .timeline-canvas .vis-panel.vis-bottom { border-color: var(--line-soft); }
    .timeline-canvas .vis-time-axis .vis-text { color: var(--ink-muted); font: 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-canvas .vis-time-axis .vis-grid.vis-minor { border-color: var(--theme-warning-border, #efe7db); }
    .timeline-canvas .vis-time-axis .vis-grid.vis-major { border-color: var(--theme-warning-border, #e0d5c4); }
    .timeline-canvas .vis-item { border-width: 1px; border-radius: 7px; font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif; box-shadow: 0 0 0 1px rgba(48, 44, 38, 0.06); }
    .timeline-canvas .vis-item.vis-selected { box-shadow: 0 0 0 2px rgba(139, 92, 46, 0.28); }
    .timeline-canvas .vis-item.vis-background { background: rgba(180, 119, 63, 0.12); color: var(--ink-muted); border: 0; box-shadow: none; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-canvas .vis-item.timeline-lifeline { border-radius: 999px; }
    .timeline-canvas .vis-item.timeline-imprecise { border-style: dashed; }
    .timeline-canvas .vis-item.vis-point .vis-dot { border-width: 2px; }
    .timeline-canvas .vis-item .vis-item-content { padding: 3px 8px; }
    .timeline-canvas .vis-labelset .vis-label, .timeline-canvas .vis-foreground .vis-group { border-color: var(--line-soft); }
    .timeline-canvas .vis-labelset .vis-label { background: color-mix(in srgb, var(--surface) 88%, transparent); color: var(--ink-soft); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-canvas .vis-labelset .vis-label .vis-inner { padding: 8px 10px; }
    .timeline-canvas .vis-item.timeline-lifeline .vis-item-content { display: inline-flex; align-items: center; gap: 6px; }
    .timeline-canvas .vis-item.timeline-lifeline .vis-item-content::before, .timeline-canvas .vis-item.timeline-lifeline .vis-item-content::after { content: ""; width: 6px; height: 6px; flex: 0 0 auto; border: 1px solid currentColor; border-radius: 50%; background: var(--surface); }
    .timeline-details { display: grid; align-content: start; gap: 9px; min-height: 55px; padding: 16px; background: var(--surface); color: var(--ink-muted); font: 11px/1.45 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-inspector { overflow: auto; max-height: min(62vh, 620px); border-left: 1px solid var(--line-soft); }
    .timeline-details strong { color: var(--theme-neutral-text, #302c26); font: 500 18px/1.15 var(--font-display, Georgia, serif); }
    .timeline-details small { color: var(--ink-muted); }
    .timeline-inspector-kicker { color: var(--theme-warning-text, #9a6a3b) !important; font: 700 8px Inter, ui-sans-serif, system-ui, sans-serif !important; letter-spacing: .08em; text-transform: uppercase; }
    .timeline-inspector-chip { width: fit-content; border-radius: 999px; padding: 3px 7px; background: var(--theme-warning-bg, #f1ebe1); color: var(--theme-neutral-text-soft, #6f6559) !important; font-size: 9px !important; }
    .timeline-inspector-divider { width: 32px; height: 1px; margin: 2px 0; border: 0; background: var(--line-strong); }
    .timeline-map-button { width: fit-content; padding: 5px 9px; border: 1px solid var(--line-strong); border-radius: 7px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-map-button:hover, .timeline-map-button:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .timeline-empty, .timeline-undated, .timeline-error { margin: 0; padding: 28px 18px; color: var(--ink-muted); font: 12px/1.5 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-undated { padding: 12px 15px; border-top: 1px solid var(--line-soft); background: var(--surface); }
    .timeline-undated-list { display: flex; flex-wrap: wrap; gap: 7px; margin-top: 8px; }
    .timeline-undated-list button { border: 1px solid var(--line-strong); border-radius: 999px; padding: 5px 8px; background: var(--surface); color: var(--ink-muted); font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-undated-list button:hover, .timeline-undated-list button:focus-visible { border-color: var(--accent); color: var(--theme-warning-text, #55351f); outline: none; }
    .timeline-error { color: var(--theme-danger-text, #9a4d3f); }
    @media (max-width: 1100px) {
      .timeline-workspace { grid-template-columns: var(--timeline-outline-width, 270px) 8px minmax(340px, 1fr); }
      .timeline-inspector { grid-column: 1 / -1; max-height: none; border-top: 1px solid var(--line-soft); border-left: 0; }
      .timeline-details { grid-template-columns: minmax(160px, .7fr) minmax(220px, 1.3fr); }
    }
    @media (max-width: 760px) {
      .timeline-toolbar { align-items: stretch; flex-direction: column; }
      .timeline-search { width: min(100%, 320px); }
      .timeline-layerbar { align-items: flex-start; flex-direction: column; }
      .timeline-workspace { grid-template-columns: 1fr !important; }
      .timeline-outline { max-height: 260px; border-right: 0; border-bottom: 1px solid var(--line-soft); }
      .timeline-outline-resize { display: none; }
      .timeline-canvas { height: 420px; min-height: 320px; }
      .timeline-inspector { grid-column: auto; }
      .timeline-details { grid-template-columns: 1fr; }
    }
  `;
  return style;
}

function createToolbarButton(label: string, ariaLabel = label) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  button.setAttribute("aria-label", ariaLabel);
  button.title = ariaLabel;
  return button;
}

function createFilterField(label: string, control: HTMLInputElement | HTMLSelectElement) {
  const field = document.createElement("label");
  field.className = "timeline-filter-field";
  const caption = document.createElement("span");
  caption.textContent = label;
  field.append(caption, control);
  return field;
}

function createLayerChip(label: string, count: number, active: boolean, onToggle: () => void) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "timeline-layer-chip";
  button.setAttribute("aria-pressed", String(active));
  const text = document.createElement("span");
  text.textContent = label;
  const badge = document.createElement("span");
  badge.className = "timeline-layer-count";
  badge.textContent = String(count);
  button.append(text, badge);
  button.onclick = onToggle;
  return button;
}

function createLegend() {
  const legend = document.createElement("div");
  legend.className = "timeline-legend";
  legend.setAttribute("aria-label", "Timeline legend");
  for (const [className, label] of [
    ["timeline-legend-dot", "Known date"],
    ["timeline-legend-range", "Date span"],
    ["timeline-legend-approx", "Year or month precision"],
  ]) {
    const item = document.createElement("span");
    item.className = "timeline-legend-item";
    const mark = document.createElement("span");
    mark.className = className;
    item.append(mark, document.createTextNode(label));
    legend.append(item);
  }
  return legend;
}

async function showOnMap(context: ModuleContext, entityId: string) {
  try {
    const result = await context.maps.focusEntity({ entityId });
    if (result.status === "multiple-links" && result.locations.length > 0) {
      const location = result.locations[0];
      await context.maps.openMap({ mapEntityId: location.mapEntityId, linkId: location.id });
    }
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    if (
      message.includes("not-on-map") ||
      message.includes("link-unresolved") ||
      message.includes("map-unavailable") ||
      message.includes("forbidden") ||
      message.includes("denied")
    )
      return;
    console.error("show on map failed", cause);
  }
}

async function appendShowOnMapButton(
  details: HTMLElement,
  context: ModuleContext,
  entityId: string,
  selectionKey: string,
) {
  if (!context.services.isAvailable("daena.maps/navigation", 1)) return;
  try {
    const locations = await context.maps.listLocations({ entityId });
    if (locations.length === 0 || details.dataset.selectionKey !== selectionKey) return;
    const mapButton = document.createElement("button");
    mapButton.type = "button";
    mapButton.className = "timeline-map-button";
    mapButton.textContent = "Show on map";
    mapButton.onclick = () => void showOnMap(context, entityId);
    details.append(mapButton);
  } catch {
    // A project without map locations still shows the inspector without the action.
  }
}

function renderSelection(
  details: HTMLElement,
  event: TimelineEvent,
  context: ModuleContext,
  definitions: ReadonlyMap<string, CalendarDefinition>,
  displayCalendarName: string,
) {
  const selectionKey = event.id;
  details.dataset.selectionKey = selectionKey;
  details.replaceChildren();
  const kicker = document.createElement("small");
  kicker.className = "timeline-inspector-kicker";
  kicker.textContent = layerLabel(event);
  const name = document.createElement("strong");
  name.textContent = event.entity.name;
  const type = document.createElement("small");
  type.className = "timeline-inspector-chip";
  type.textContent = entityTypeLabel(event.entity.type);
  const divider = document.createElement("hr");
  divider.className = "timeline-inspector-divider";
  const range = document.createElement("small");
  range.textContent = rangeLabel(event, definitions);
  const calendar = document.createElement("small");
  calendar.textContent = `Displayed on the ${displayCalendarName} calendar`;
  details.append(kicker, name, type, divider, range, calendar);
  const contextText = contextLabel(event);
  if (contextText) {
    const contextLine = document.createElement("small");
    contextLine.textContent = contextText;
    details.append(contextLine);
  }
  void appendShowOnMapButton(details, context, event.entity.id, selectionKey);
}

function renderUndatedSelection(details: HTMLElement, event: UndatedEvent, context: ModuleContext) {
  const selectionKey = event.entity.id;
  details.dataset.selectionKey = selectionKey;
  details.replaceChildren();
  const kicker = document.createElement("small");
  kicker.className = "timeline-inspector-kicker";
  kicker.textContent = event.relativeYear === undefined ? "Unplaced item" : "Relative chronology";
  const name = document.createElement("strong");
  name.textContent = event.entity.name;
  const status = document.createElement("small");
  status.textContent =
    event.relativeYear === undefined
      ? "No date yet — add a start or end date to place this event in the chronology."
      : `Relative chronology: ${relativeOffsetLabel(event.relativeYear)}. It is kept relative rather than converted to a Gregorian date.`;
  details.append(kicker, name, status);
  const contextText = contextLabel(event);
  if (contextText) {
    const contextLine = document.createElement("small");
    contextLine.textContent = contextText;
    details.append(contextLine);
  }
  void appendShowOnMapButton(details, context, event.entity.id, selectionKey);
}

function renderOutline(
  outline: HTMLElement,
  events: TimelineEvent[],
  definition: CalendarDefinition | null,
  definitions: ReadonlyMap<string, CalendarDefinition>,
  selectedId: string | null,
  onSelect: (event: TimelineEvent) => void,
): void {
  const heading = document.createElement("span");
  heading.className = "timeline-outline-heading";
  heading.textContent = "Chronological outline";
  outline.append(heading);
  let currentYear: number | null = null;
  for (const event of events) {
    const year = eventYear(event, definition);
    if (year !== currentYear) {
      currentYear = year;
      const yearHeading = document.createElement("div");
      yearHeading.className = "timeline-outline-year";
      yearHeading.textContent = String(year);
      outline.append(yearHeading);
    }
    const card = document.createElement("button");
    card.type = "button";
    card.className = `timeline-event-card${event.id === selectedId ? " is-selected" : ""}`;
    card.dataset.eventId = event.id;
    const name = document.createElement("strong");
    name.textContent = event.entity.name;
    const range = document.createElement("small");
    range.textContent = rangeLabel(event, definitions);
    const meta = document.createElement("span");
    meta.className = "timeline-card-meta";
    const kind = document.createElement("span");
    kind.className = "timeline-card-kind";
    kind.textContent = layerLabel(event);
    const type = document.createElement("small");
    type.textContent = entityTypeLabel(event.entity.type);
    meta.append(kind, type);
    card.append(name, range, meta);
    card.onclick = () => onSelect(event);
    outline.append(card);
  }
}

function toDataItem(event: TimelineEvent, definitions: ReadonlyMap<string, CalendarDefinition>): DataItem {
  const partial = [event.startValue, event.endValue]
    .filter((value) => value !== undefined)
    .some((value) => ["year", "month"].includes(parseCalendarDate(value)?.precision ?? "day"));
  const itemLabel =
    event.pointRole && event.pointRole !== "point" && event.startLabel
      ? `${event.entity.name} · ${event.startLabel}`
      : event.entity.name;
  const item: DataItem = {
    id: event.id,
    content: itemLabel,
    group: groupForEvent(event),
    start: event.start,
    title: `${event.entity.name}\n${rangeLabel(event, definitions)}`,
    className: [
      `timeline-${groupForEvent(event)}`,
      event.layer === "lifelines" ? "timeline-lifeline" : "",
      partial ? "timeline-imprecise" : "",
    ]
      .filter(Boolean)
      .join(" "),
    style: `background-color:${event.colors.fill};border-color:${event.colors.border};color:${event.colors.text};`,
  };
  if (event.end && event.end.getTime() !== event.start.getTime()) {
    item.end = event.end;
    item.type = "range";
  } else if (event.pointRole) {
    item.type = "point";
  } else {
    item.type = "box";
  }
  return item;
}

function timelineGroups(events: readonly TimelineEvent[]): DataGroup[] {
  const counts = new Map<TimelineGroupId, number>();
  for (const event of events) {
    const group = groupForEvent(event);
    counts.set(group, (counts.get(group) ?? 0) + 1);
  }
  return [
    { id: "events", label: "Events" },
    { id: "lifelines", label: "Lifelines" },
    { id: "dates", label: "Project dates" },
  ]
    .filter(({ id }) => counts.has(id as TimelineGroupId))
    .map(({ id, label }) => ({
      id,
      content: `${label} (${counts.get(id as TimelineGroupId) ?? 0})`,
      className: `timeline-group-${id}`,
    }));
}

function showError(shell: HTMLElement, cause: unknown) {
  const error = document.createElement("p");
  error.className = "timeline-error";
  error.textContent = cause instanceof Error ? cause.message : String(cause);
  shell.append(error);
}

export const timeline: DaenaModule = {
  manifest,
  views: [
    {
      id: "timeline-events",
      title: "Timeline Events",
      mount: (element: HTMLElement, context: ModuleContext) => {
        let cancelled = false;
        let chart: Timeline | null = null;
        let resizeObserver: ResizeObserver | null = null;
        let activeYear: number | null = null;
        let outlineWidth = 300;
        let outlineCollapsed = false;
        let showTimelineEvents = true;
        let showProjectDates = false;
        let showLifelines = true;
        let searchQuery = "";
        let selectedEntityType = "";
        let selectedEventId: string | null = null;
        let selectedCalendarId = "gregorian";
        let sourceSnapshot: TimelineSourceSnapshot | null = null;
        let searchTimer: number | null = null;
        let removeOutlineResize: (() => void) | null = null;
        const style = createTimelineStyles();
        const render = async () => {
          try {
            resizeObserver?.disconnect();
            resizeObserver = null;
            removeOutlineResize?.();
            removeOutlineResize = null;
            chart?.destroy();
            chart = null;
            let snapshot = sourceSnapshot;
            if (!snapshot) {
              const [entities, enabledManifests] = await Promise.all([context.entities.list(), context.modules.list()]);
              const calendarOptions = await loadCalendarOptions(context, entities);
              const entityTypes = new Set(
                context.module.schemas.flatMap((schema) => schema.entityTypes.map((entityType) => entityType.id)),
              );
              const contributionSpecs = discoverTimelineFieldSpecs(enabledManifests, context.module.id);
              const contributionNamespaces = [...new Set(contributionSpecs.map((spec) => spec.namespace))];
              const loaded: LoadedTimelineEntry[] = await Promise.all(
                entities.map(async (entity) => {
                  const isTimelineEntity = entityTypes.has(entity.type ?? "");
                  const fields = isTimelineEntity ? await context.fields.list(entity.id) : {};
                  const contributedRecords = (
                    await Promise.all(
                      contributionNamespaces.map(async (namespace) => {
                        try {
                          return await context.fields.listShared(entity.id, namespace);
                        } catch {
                          return [] as FieldRecord[];
                        }
                      }),
                    )
                  ).flat();
                  let sharedMapsFields: FieldRecord[] = [];
                  try {
                    // Shared chronology remains readable when Maps is disabled;
                    // only the navigation service should disappear with it.
                    if (isTimelineEntity) sharedMapsFields = await context.fields.listShared(entity.id, "maps");
                  } catch {
                    // A project without the optional Maps contract still renders
                    // ordinary Timeline items instead of failing the projection.
                  }
                  const relativeYear = physicalOffset(
                    sharedMapsFields.find((field) => field.key === "physicalChronology")?.value,
                  );
                  const relationships = isTimelineEntity
                    ? (await context.relationships.list(entity.id)).filter(
                        (relationship) => relationship.type === "occurred_at" || relationship.type === "involves",
                      )
                    : [];
                  const targets = await Promise.all(
                    relationships.map((relationship) => context.entities.get(relationship.targetId)),
                  );
                  return {
                    entity,
                    fields,
                    locationName: relationships
                      .map((relationship, index) =>
                        relationship.type === "occurred_at" ? targets[index]?.name : undefined,
                      )
                      .find((name): name is string => Boolean(name)),
                    participantNames: relationships
                      .map((relationship, index) =>
                        relationship.type === "involves" ? targets[index]?.name : undefined,
                      )
                      .filter((name): name is string => Boolean(name)),
                    relativeYear,
                    contributions: buildFieldContributions(entity, contributedRecords, contributionSpecs),
                  };
                }),
              );
              snapshot = { calendarOptions, loaded };
              sourceSnapshot = snapshot;
            }
            const { calendarOptions, loaded } = snapshot;
            if (!calendarOptions.some((option) => option.id === selectedCalendarId)) selectedCalendarId = "gregorian";
            const selectedCalendarOption =
              calendarOptions.find((option) => option.id === selectedCalendarId) ?? calendarOptions[0];
            const selectedCalendar = selectedCalendarOption?.definition ?? null;
            const calendarDefinitions = new Map(
              calendarOptions
                .filter(
                  (option): option is CalendarOption & { definition: CalendarDefinition } => option.definition !== null,
                )
                .map((option) => [option.id, option.definition]),
            );
            const entityTypes = new Set(
              context.module.schemas.flatMap((schema) => schema.entityTypes.map((entityType) => entityType.id)),
            );
            const dated: TimelineEvent[] = [];
            const contributed: TimelineEvent[] = [];
            const undated: UndatedEvent[] = [];
            const eras: TimelineEvent[] = [];
            for (const entry of loaded) {
              if (
                entry.entity.type !== "daena.timeline:calendar" &&
                (entityTypes.has(entry.entity.type ?? "") || entry.relativeYear !== null)
              ) {
                const startValue = entry.fields.startsAt ?? entry.fields.endsAt;
                const startAnchor = timelineDateAnchor(startValue);
                if (!startAnchor) {
                  if (entry.entity.type !== "daena.timeline:era")
                    undated.push({ ...entry, relativeYear: entry.relativeYear ?? undefined });
                } else {
                  const endValue = entry.fields.endsAt;
                  const endAnchor = endValue ? timelineDateAnchor(endValue) : null;
                  const item: TimelineEvent = {
                    id: entry.entity.id,
                    entity: entry.entity,
                    startValue,
                    endValue,
                    layer: "timeline",
                    locationName: entry.locationName,
                    participantNames: entry.participantNames,
                    start: startAnchor.date,
                    end: endAnchor && endAnchor.date.getTime() >= startAnchor.date.getTime() ? endAnchor.date : null,
                    colors: colorsForLayer("timeline", entry.entity.id),
                  };
                  if (entry.entity.type === "daena.timeline:era") eras.push(item);
                  else dated.push(item);
                }
              }
              for (const contribution of entry.contributions) {
                const startAnchor = timelineDateAnchor(contribution.startValue);
                if (!startAnchor) continue;
                const endAnchor = contribution.endValue ? timelineDateAnchor(contribution.endValue) : null;
                contributed.push({
                  id: `contribution:${contribution.id}`,
                  entity: contribution.entity,
                  startValue: contribution.startValue,
                  endValue: contribution.endValue,
                  startLabel: contribution.startLabel,
                  endLabel: contribution.endLabel,
                  pointRole: contribution.pointRole,
                  layer: contribution.layer,
                  participantNames: [],
                  start: startAnchor.date,
                  end: endAnchor && endAnchor.date.getTime() >= startAnchor.date.getTime() ? endAnchor.date : null,
                  colors: colorsForLayer(contribution.layer, contribution.entity.id),
                });
              }
            }
            const allEvents = [...dated, ...contributed];
            const layerCounts = {
              events: dated.length,
              lifelines: contributed.filter((event) => event.layer === "lifelines").length,
              dates: contributed.filter((event) => event.layer === "dates").length,
            };
            const availableEntityTypes = [
              ...new Set(allEvents.map((event) => event.entity.type).filter((type): type is string => Boolean(type))),
            ].sort((left, right) => left.localeCompare(right));
            if (selectedEntityType && !availableEntityTypes.includes(selectedEntityType)) selectedEntityType = "";
            const query = searchQuery.trim().toLocaleLowerCase();
            const plotted = [
              ...(showTimelineEvents ? dated : []),
              ...contributed.filter(
                (event) =>
                  (event.layer === "dates" && showProjectDates) || (event.layer === "lifelines" && showLifelines),
              ),
            ]
              .filter((event) => !selectedEntityType || event.entity.type === selectedEntityType)
              .filter((event) => {
                if (!query) return true;
                return [
                  event.entity.name,
                  event.entity.type,
                  event.startLabel,
                  event.endLabel,
                  event.locationName,
                  ...event.participantNames,
                ]
                  .filter(Boolean)
                  .some((value) => String(value).toLocaleLowerCase().includes(query));
              })
              .sort((left, right) => left.start.getTime() - right.start.getTime() || left.id.localeCompare(right.id));
            const years = [...new Set(plotted.map((event) => eventYear(event, selectedCalendar)))];
            if (activeYear !== null && !years.includes(activeYear)) activeYear = null;
            const visible =
              activeYear === null
                ? plotted
                : plotted.filter((event) => eventYear(event, selectedCalendar) === activeYear);
            if (selectedEventId && !visible.some((event) => event.id === selectedEventId)) selectedEventId = null;
            if (cancelled) return;

            element.replaceChildren();
            element.className = "timeline-projection";
            const shell = document.createElement("div");
            shell.className = "timeline-shell";
            const calendarName = selectedCalendarOption?.name ?? "Gregorian";
            context.reportSurfaceMeta?.({
              subtitle:
                undated.length > 0
                  ? `${plotted.length} placed · ${undated.length} unplaced or relative · ${calendarName}`
                  : `${plotted.length} items · ${calendarName}`,
            });
            shell.append(style);
            let details: HTMLElement | null = null;

            if (allEvents.length === 0 && undated.length === 0) {
              const empty = document.createElement("p");
              empty.className = "timeline-empty";
              empty.textContent = "No timeline items yet.";
              shell.append(empty);
              element.append(shell);
              return;
            }

            if (allEvents.length === 0) {
              const empty = document.createElement("p");
              empty.className = "timeline-empty";
              empty.textContent =
                contributed.length > 0
                  ? "No items are visible with the current date layers."
                  : "No dated timeline items to plot yet.";
              const detailPanel = document.createElement("div");
              details = detailPanel;
              detailPanel.className = "timeline-details";
              const hint = document.createElement("small");
              hint.textContent = "Choose an unplaced item below to inspect it.";
              detailPanel.append(hint);
              shell.append(empty, detailPanel);
            } else {
              const toolbar = document.createElement("div");
              toolbar.className = "timeline-toolbar";
              const controls = document.createElement("div");
              controls.className = "timeline-toolbar-controls";
              const search = document.createElement("input");
              search.type = "search";
              search.className = "timeline-search";
              search.value = searchQuery;
              search.placeholder = "Name, type, place…";
              search.setAttribute("aria-label", "Search timeline");
              search.oninput = () => {
                searchQuery = search.value;
                const cursor = search.selectionStart ?? search.value.length;
                if (searchTimer !== null) window.clearTimeout(searchTimer);
                searchTimer = window.setTimeout(() => {
                  searchTimer = null;
                  void render().then(() => {
                    const next = element.querySelector<HTMLInputElement>(".timeline-search");
                    next?.focus();
                    next?.setSelectionRange(cursor, cursor);
                  });
                }, 160);
              };
              const scope = document.createElement("select");
              scope.className = "timeline-scope";
              scope.setAttribute("aria-label", "Chronology scope");
              const allDates = document.createElement("option");
              allDates.value = "";
              allDates.textContent = "All dates";
              scope.append(allDates);
              for (const year of years) {
                const option = document.createElement("option");
                option.value = String(year);
                option.textContent = String(year);
                if (year === activeYear) option.selected = true;
                scope.append(option);
              }
              scope.onchange = () => {
                activeYear = scope.value ? Number(scope.value) : null;
                selectedEventId = null;
                void render();
              };
              const calendar = document.createElement("select");
              calendar.className = "timeline-calendar";
              calendar.setAttribute("aria-label", "Displayed calendar");
              for (const optionValue of calendarOptions) {
                const option = document.createElement("option");
                option.value = optionValue.id;
                option.textContent = optionValue.name;
                option.selected = optionValue.id === selectedCalendarId;
                calendar.append(option);
              }
              calendar.onchange = () => {
                selectedCalendarId = calendar.value;
                activeYear = null;
                selectedEventId = null;
                void render();
              };
              const typeFilter = document.createElement("select");
              typeFilter.className = "timeline-type-filter";
              const allTypes = document.createElement("option");
              allTypes.value = "";
              allTypes.textContent = "All entity types";
              typeFilter.append(allTypes);
              for (const entityType of availableEntityTypes) {
                const option = document.createElement("option");
                option.value = entityType;
                option.textContent = entityTypeLabel(entityType);
                option.selected = entityType === selectedEntityType;
                typeFilter.append(option);
              }
              typeFilter.onchange = () => {
                selectedEntityType = typeFilter.value;
                activeYear = null;
                selectedEventId = null;
                void render();
              };
              controls.append(
                createFilterField("Search", search),
                createFilterField("Calendar", calendar),
                createFilterField("Entity type", typeFilter),
                createFilterField("Year", scope),
              );
              const actions = document.createElement("div");
              actions.className = "timeline-toolbar-actions";
              const zoomInButton = createToolbarButton("+", "Zoom in");
              const zoomOutButton = createToolbarButton("−", "Zoom out");
              const fitButton = createToolbarButton("Fit", "Fit timeline to visible items");
              const outlineButton = createToolbarButton(outlineCollapsed ? "Show outline" : "Hide outline");
              outlineButton.onclick = () => {
                outlineCollapsed = !outlineCollapsed;
                void render();
              };
              actions.append(zoomOutButton, zoomInButton, fitButton, outlineButton);
              toolbar.append(controls, actions);

              const layerbar = document.createElement("div");
              layerbar.className = "timeline-layerbar";
              const chips = document.createElement("div");
              chips.className = "timeline-layer-chips";
              chips.append(
                createLayerChip("Events", layerCounts.events, showTimelineEvents, () => {
                  showTimelineEvents = !showTimelineEvents;
                  activeYear = null;
                  selectedEventId = null;
                  void render();
                }),
                createLayerChip("Lifelines", layerCounts.lifelines, showLifelines, () => {
                  showLifelines = !showLifelines;
                  activeYear = null;
                  selectedEventId = null;
                  void render();
                }),
                createLayerChip("Project dates", layerCounts.dates, showProjectDates, () => {
                  showProjectDates = !showProjectDates;
                  activeYear = null;
                  selectedEventId = null;
                  void render();
                }),
              );
              layerbar.append(chips, createLegend());

              const workspace = document.createElement("div");
              workspace.className = "timeline-workspace";
              if (outlineCollapsed) workspace.style.gridTemplateColumns = "minmax(360px, 1fr) minmax(220px, 270px)";
              else workspace.style.setProperty("--timeline-outline-width", `${outlineWidth}px`);
              const outline = document.createElement("aside");
              outline.className = "timeline-outline";
              const canvas = document.createElement("div");
              canvas.className = "timeline-canvas";
              const detailPanel = document.createElement("div");
              details = detailPanel;
              detailPanel.className = "timeline-details timeline-inspector";
              const hint = document.createElement("small");
              hint.textContent = "Select an item to inspect its dates, type, and related context.";
              detailPanel.append(hint);
              const selectEvent = (event: TimelineEvent) => {
                selectedEventId = event.id;
                for (const card of outline.querySelectorAll<HTMLButtonElement>(".timeline-event-card"))
                  card.classList.toggle("is-selected", card.dataset.eventId === event.id);
                chart?.setSelection([event.id], {
                  focus: true,
                  animation: {},
                });
                chart?.focus(event.id, { animation: { duration: 220, easingFunction: "easeInOutQuad" } });
                renderSelection(
                  detailPanel,
                  event,
                  context,
                  calendarDefinitions,
                  selectedCalendarOption?.name ?? "Gregorian",
                );
              };
              if (visible.length > 0)
                renderOutline(outline, visible, selectedCalendar, calendarDefinitions, selectedEventId, selectEvent);
              else {
                const empty = document.createElement("p");
                empty.className = "timeline-empty";
                empty.textContent = "No items match the current filters.";
                outline.append(empty);
              }
              if (!outlineCollapsed) {
                const resizeHandle = document.createElement("div");
                resizeHandle.className = "timeline-outline-resize";
                resizeHandle.setAttribute("role", "separator");
                resizeHandle.setAttribute("aria-label", "Resize chronological outline");
                resizeHandle.tabIndex = 0;
                resizeHandle.onpointerdown = (event) => {
                  event.preventDefault();
                  const startX = event.clientX;
                  const startWidth = outlineWidth;
                  const onMove = (move: PointerEvent) => {
                    outlineWidth = Math.max(210, Math.min(520, startWidth + move.clientX - startX));
                    workspace.style.setProperty("--timeline-outline-width", `${outlineWidth}px`);
                    chart?.redraw();
                  };
                  const onUp = () => {
                    window.removeEventListener("pointermove", onMove);
                    window.removeEventListener("pointerup", onUp);
                    removeOutlineResize = null;
                  };
                  removeOutlineResize = onUp;
                  window.addEventListener("pointermove", onMove);
                  window.addEventListener("pointerup", onUp, { once: true });
                };
                workspace.append(outline, resizeHandle, canvas, detailPanel);
              } else {
                workspace.append(canvas, detailPanel);
              }
              const selectedEvent = selectedEventId ? visible.find((event) => event.id === selectedEventId) : undefined;
              if (selectedEvent)
                renderSelection(
                  detailPanel,
                  selectedEvent,
                  context,
                  calendarDefinitions,
                  selectedCalendarOption?.name ?? "Gregorian",
                );
              shell.append(toolbar, layerbar, workspace);
              element.append(shell);

              await import("vis-timeline/styles/vis-timeline-graph2d.min.css");
              const { Timeline: TimelineCtor } = await import("vis-timeline/standalone");
              if (cancelled) return;

              const eventsById = new Map(visible.map((event) => [event.id, event]));
              const options: TimelineOptions = {
                stack: true,
                stackSubgroups: true,
                selectable: true,
                multiselect: false,
                zoomable: true,
                moveable: true,
                showCurrentTime: false,
                orientation: "top",
                margin: { item: { horizontal: 8, vertical: 6 }, axis: 12 },
                tooltip: { followMouse: true },
                format: {
                  minorLabels: (value, scale) => formatAxisDate(value, selectedCalendar, scale),
                  majorLabels: (value, scale) => formatAxisDate(value, selectedCalendar, scale),
                },
              };
              if (visible.length >= 40) {
                options.cluster = { titleTemplate: "{count} events", showStipes: true, maxItems: 8 };
              }
              const groups = timelineGroups(visible);
              chart = new TimelineCtor(
                canvas,
                [
                  ...eras.map((era) => {
                    const end =
                      era.end ??
                      (() => {
                        const next = new Date(era.start.getTime());
                        next.setUTCFullYear(next.getUTCFullYear() + 1);
                        return next;
                      })();
                    const item: DataItem = {
                      id: `era:${era.entity.id}`,
                      content: era.entity.name,
                      start: era.start,
                      end,
                      type: "background",
                      title: era.entity.name,
                    };
                    return item;
                  }),
                  ...visible.map((event) => toDataItem(event, calendarDefinitions)),
                ],
                groups,
                options,
              );
              chart.fit({ animation: false });
              if (selectedEventId) chart.setSelection([selectedEventId]);

              zoomInButton.onclick = () => chart?.zoomIn(0.4);
              zoomOutButton.onclick = () => chart?.zoomOut(0.4);
              fitButton.onclick = () => chart?.fit({ animation: { duration: 320, easingFunction: "easeInOutQuad" } });

              chart.on("select", (properties) => {
                const selectedId = properties.items[0];
                const event = selectedId != null ? eventsById.get(String(selectedId)) : undefined;
                if (!event) {
                  selectedEventId = null;
                  for (const card of outline.querySelectorAll<HTMLButtonElement>(".timeline-event-card"))
                    card.classList.remove("is-selected");
                  detailPanel.replaceChildren(hint);
                  return;
                }
                selectEvent(event);
              });
              chart.on("click", (properties) => {
                if (properties.item != null) return;
                selectedEventId = null;
                chart?.setSelection([]);
                for (const card of outline.querySelectorAll<HTMLButtonElement>(".timeline-event-card"))
                  card.classList.remove("is-selected");
                detailPanel.replaceChildren(hint);
              });

              resizeObserver = new ResizeObserver(() => chart?.redraw());
              resizeObserver.observe(canvas);
            }

            if (undated.length > 0) {
              const note = document.createElement("div");
              note.className = "timeline-undated";
              const label = document.createElement("small");
              label.textContent = undated.some((entry) => entry.relativeYear !== undefined)
                ? "Unplaced or relative items"
                : "Unplaced items";
              const list = document.createElement("div");
              list.className = "timeline-undated-list";
              for (const entry of undated) {
                const button = document.createElement("button");
                button.type = "button";
                button.textContent =
                  entry.relativeYear === undefined
                    ? entry.entity.name
                    : `${entry.entity.name} · ${relativeOffsetLabel(entry.relativeYear)}`;
                button.onclick = () => {
                  if (details) renderUndatedSelection(details, entry, context);
                };
                list.append(button);
              }
              note.append(label, list);
              shell.append(note);
            }
            if (!element.contains(shell)) element.append(shell);
          } catch (cause) {
            if (cancelled) return;
            console.error("timeline projection failed", cause);
            element.replaceChildren();
            element.className = "timeline-projection";
            const shell = document.createElement("div");
            shell.className = "timeline-shell";
            shell.append(style);
            showError(shell, cause);
            element.append(shell);
          }
        };
        void render();
        return () => {
          cancelled = true;
          if (searchTimer !== null) window.clearTimeout(searchTimer);
          searchTimer = null;
          resizeObserver?.disconnect();
          resizeObserver = null;
          removeOutlineResize?.();
          removeOutlineResize = null;
          chart?.destroy();
          chart = null;
          element.replaceChildren();
        };
      },
    },
  ],
};
