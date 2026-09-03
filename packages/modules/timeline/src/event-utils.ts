import { type EntitySummary, type ModuleContext } from "../../../module-api/src/index";
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
  type TimelineContribution,
  type TimelineFieldRole,
  type TimelineLayer,
} from "./projection";

export type EventColors = { fill: string; border: string; text: string };

export type TimelineEvent = {
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
  eraIds: string[];
  start: Date;
  end: Date | null;
  colors: EventColors;
};

export type TimelineGroupId = "events" | "lifelines" | "dates";

export type LoadedTimelineEntry = {
  entity: EntitySummary;
  fields: Record<string, unknown>;
  locationName?: string;
  participantNames: string[];
  eraIds: string[];
  calendarIds: string[];
  relativeYear: number | null;
  contributions: TimelineContribution[];
};

export type TimelineSourceSnapshot = {
  calendarOptions: CalendarOption[];
  loaded: LoadedTimelineEntry[];
  typeLabels: Map<string, string>;
};

export type CalendarOption = {
  id: string;
  name: string;
  definition: CalendarDefinition | null;
};

export type UndatedEvent = {
  entity: { id: string; name: string };
  fields: Record<string, unknown>;
  locationName?: string;
  participantNames: string[];
  eraIds: string[];
  relativeYear?: number;
};

export type PhysicalChronology = {
  contractVersion: 1;
  kind: "physical-offset-years";
  reference: "accepted-source";
  startOffsetYears: number;
  endOffsetYears: number;
};

export function physicalOffset(value: unknown): number | null {
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

export function relativeOffsetLabel(year: number): string {
  return `${year >= 0 ? "+" : ""}${year} years from accepted source`;
}

export function hslToHex(hue: number, saturation: number, lightness: number) {
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

export function colorsForHue(hue: number): EventColors {
  return {
    fill: hslToHex(hue, 0.42, 0.82),
    border: hslToHex(hue, 0.44, 0.42),
    text: hslToHex(hue, 0.38, 0.22),
  };
}

export function colorsForLayer(layer: TimelineEvent["layer"], entityId: string): EventColors {
  if (layer === "lifelines") return { fill: "#dfeae2", border: "#4f705a", text: "#284234" };
  if (layer === "dates") return { fill: "#f3e4cf", border: "#a56d32", text: "#5f3d20" };
  return colorsForHue(hueForId(entityId));
}

export function groupForEvent(event: TimelineEvent): TimelineGroupId {
  if (event.layer === "lifelines") return "lifelines";
  if (event.layer === "dates") return "dates";
  return "events";
}

export function layerLabel(event: TimelineEvent): string {
  if (event.layer === "lifelines") return "Lifeline";
  if (event.layer === "dates") return "Project date";
  if (event.entity.type === "daena.timeline:era") return "Era";
  return "Timeline event";
}

export function matchesTimelineQuery(event: TimelineEvent, query: string): boolean {
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
}

export function eraEnd(era: TimelineEvent): Date {
  if (era.end) return era.end;
  const next = new Date(era.start.getTime());
  next.setUTCFullYear(next.getUTCFullYear() + 1);
  return next;
}

export function eraAtTime(eras: TimelineEvent[], time: Date): TimelineEvent | undefined {
  const stamp = time.getTime();
  const containing = eras.filter((era) => era.start.getTime() <= stamp && stamp <= eraEnd(era).getTime());
  return containing.sort(
    (left, right) => eraEnd(left).getTime() - left.start.getTime() - (eraEnd(right).getTime() - right.start.getTime()),
  )[0];
}

export function hueForId(id: string) {
  let hash = 2166136261;
  for (let index = 0; index < id.length; index += 1) {
    hash ^= id.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) % 360;
}

export function asJsDate(value: unknown): Date | null {
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

export function precisionForScale(scale?: string): CalendarDate["precision"] {
  if (scale === "year") return "year";
  if (scale === "month") return "month";
  if (scale === "day" || scale === "weekday" || scale === "week") return "day";
  if (scale === "hour") return "hour";
  if (scale === "minute") return "minute";
  return "second";
}

export function calendarFromJsDate(value: Date, precision: CalendarDate["precision"] = "day"): CalendarDate {
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

export function formatAxisDate(value: unknown, definition: CalendarDefinition | null, scale?: string) {
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

export function definitionForValue(value: unknown, definitions: ReadonlyMap<string, CalendarDefinition>) {
  const calendarId = parseCalendarDate(value)?.calendar;
  return calendarId ? (definitions.get(calendarId) ?? null) : null;
}

export function formatStoredDate(value: unknown, definitions: ReadonlyMap<string, CalendarDefinition>) {
  return formatWithCalendar(value, definitionForValue(value, definitions));
}

export function rangeLabel(event: TimelineEvent, definitions: ReadonlyMap<string, CalendarDefinition>) {
  const start = formatStoredDate(event.startValue, definitions);
  const end = event.endValue ? formatStoredDate(event.endValue, definitions) : "";
  if (!event.endValue || end === "Undated" || end === start)
    return event.startLabel ? `${event.startLabel}: ${start}` : start;
  const startText = event.startLabel ? `${event.startLabel}: ${start}` : start;
  const endText = event.endLabel ? `${event.endLabel}: ${end}` : end;
  return `${startText} – ${endText}`;
}

export function eventYear(event: TimelineEvent, definition: CalendarDefinition | null): number {
  return calendarDateToParts(calendarFromJsDate(event.start, "day"), definition)?.year ?? event.start.getUTCFullYear();
}

export async function loadCalendarOptions(
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

export function contextLabel(event: TimelineEvent | UndatedEvent): string {
  const labels = [event.locationName, event.participantNames.length ? event.participantNames.join(", ") : ""].filter(
    Boolean,
  );
  return labels.join(" · ");
}
