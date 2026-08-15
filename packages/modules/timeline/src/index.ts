import type { Timeline, DataItem, TimelineOptions } from "vis-timeline";
import type { FieldRecord, ModuleContext, DaenaModule } from "../../../module-api/src/index";
import type { ModuleManifest } from "../../../module-api/src/index";
import {
  compareCalendarDates,
  formatCalendarDate,
  parseCalendarDate,
  type CalendarDate,
} from "../../../../src/lib/date";
import manifestJson from "../manifest.json";

const manifest = manifestJson as unknown as ModuleManifest;

type EventColors = { fill: string; border: string; text: string };

type TimelineEvent = {
  entity: { id: string; name: string };
  fields: Record<string, unknown>;
  locationName?: string;
  participantNames: string[];
  start: Date;
  end: Date | null;
  colors: EventColors;
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

function hueForId(id: string) {
  let hash = 2166136261;
  for (let index = 0; index < id.length; index += 1) {
    hash ^= id.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) % 360;
}

function toJsDate(value: unknown): Date | null {
  const date = parseCalendarDate(value);
  if (!date) return null;
  const month = (date.month ?? 1) - 1;
  const day = date.day ?? 1;
  // Date.UTC treats years 0–99 as 1900–1999. Set the full UTC year after
  // construction so authored fictional years retain their literal value.
  const result = new Date(0);
  result.setUTCFullYear(date.year, month, day);
  result.setUTCHours(date.hour ?? 0, date.minute ?? 0, date.second ?? 0, 0);
  return Number.isFinite(result.getTime()) ? result : null;
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

function calendarFromJsDate(value: Date): CalendarDate {
  return {
    calendar: "gregorian",
    era: "CE",
    year: value.getUTCFullYear(),
    month: value.getUTCMonth() + 1,
    day: value.getUTCDate(),
    hour: value.getUTCHours(),
    minute: value.getUTCMinutes(),
    second: value.getUTCSeconds(),
    precision: "second",
  };
}

function formatAxisDate(value: unknown, _scale?: string, _step?: number) {
  const date = asJsDate(value);
  return date ? formatCalendarDate(calendarFromJsDate(date)) : "";
}

function rangeLabel(startsAt: unknown, endsAt: unknown) {
  const start = formatCalendarDate(startsAt);
  const end = endsAt ? formatCalendarDate(endsAt) : "";
  if (!endsAt || end === "Undated" || end === start) return start;
  return `${start} – ${end}`;
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
    .timeline-shell { display: grid; gap: 0; }
    .timeline-toolbar { display: flex; align-items: center; justify-content: flex-end; gap: 12px; padding: 10px 14px; border-bottom: 1px solid #e9e1d4; background: #fffefa; }
    .timeline-toolbar-actions { display: flex; gap: 7px; }
    .timeline-toolbar button { border: 1px solid #d9cdbd; border-radius: 7px; padding: 6px 9px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-toolbar button:hover, .timeline-toolbar button:focus-visible { border-color: #b4773f; color: #55351f; outline: none; }
    .timeline-scope { min-width: 130px; border: 1px solid #d9cdbd; border-radius: 7px; padding: 6px 9px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-workspace { display: grid; grid-template-columns: var(--timeline-outline-width, 300px) 8px minmax(0, 1fr); min-height: 380px; }
    .timeline-outline { overflow: auto; max-height: min(58vh, 560px); padding: 10px; border-right: 1px solid #e9e1d4; background: #fffefa; }
    .timeline-outline-resize { cursor: col-resize; background: #f4eee3; touch-action: none; }
    .timeline-outline-resize:hover, .timeline-outline-resize:focus-visible { background: #dfcfb8; outline: none; }
    .timeline-outline-heading { display: block; margin: 4px 5px 8px; color: #8f897e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; letter-spacing: .04em; text-transform: uppercase; }
    .timeline-outline-year { margin: 12px 5px 5px; color: #62594e; font: 600 12px var(--font-display, Georgia, serif); }
    .timeline-event-card { display: grid; width: 100%; gap: 3px; margin: 2px 0; padding: 8px 9px; border: 1px solid transparent; border-radius: 8px; background: transparent; color: inherit; text-align: left; cursor: pointer; }
    .timeline-event-card:hover, .timeline-event-card:focus-visible, .timeline-event-card.is-selected { border-color: #d9cdbd; background: #f7f1e7; outline: none; }
    .timeline-event-card strong { color: #302c26; font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-event-card small { color: #8f897e; font: 10px/1.35 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-canvas { position: relative; height: min(58vh, 560px); min-height: 380px; background: radial-gradient(circle at 50% 42%, #fffdf7 0, #fbf8f0 52%, #f4eee3 100%); }
    .timeline-canvas .vis-timeline { border: 0; background: transparent; }
    .timeline-canvas .vis-panel.vis-background, .timeline-canvas .vis-panel.vis-center, .timeline-canvas .vis-panel.vis-left, .timeline-canvas .vis-panel.vis-right, .timeline-canvas .vis-panel.vis-top, .timeline-canvas .vis-panel.vis-bottom { border-color: #e9e1d4; }
    .timeline-canvas .vis-time-axis .vis-text { color: #8f897e; font: 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-canvas .vis-time-axis .vis-grid.vis-minor { border-color: #efe7db; }
    .timeline-canvas .vis-time-axis .vis-grid.vis-major { border-color: #e0d5c4; }
    .timeline-canvas .vis-item { border-width: 1px; border-radius: 7px; font: 600 11px Inter, ui-sans-serif, system-ui, sans-serif; box-shadow: 0 0 0 1px rgba(48, 44, 38, 0.06); }
    .timeline-canvas .vis-item.vis-selected { box-shadow: 0 0 0 2px rgba(139, 92, 46, 0.28); }
    .timeline-canvas .vis-item.vis-point .vis-dot { border-width: 2px; }
    .timeline-canvas .vis-item .vis-item-content { padding: 3px 8px; }
    .timeline-canvas .vis-labelset .vis-label, .timeline-canvas .vis-foreground .vis-group { border-color: #e9e1d4; }
    .timeline-details { display: grid; gap: 7px; min-height: 55px; padding: 12px 15px; border-top: 1px solid #e9e1d4; background: #fffefa; color: #62594e; font: 11px/1.45 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-details strong { color: #302c26; font: 500 16px/1.1 var(--font-display, Georgia, serif); }
    .timeline-details small { color: #8f897e; }
    .timeline-map-button { width: fit-content; padding: 5px 9px; border: 1px solid #d9cdbd; border-radius: 7px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-map-button:hover, .timeline-map-button:focus-visible { border-color: #b4773f; color: #55351f; outline: none; }
    .timeline-empty, .timeline-undated, .timeline-error { margin: 0; padding: 28px 18px; color: #8f897e; font: 12px/1.5 Inter, ui-sans-serif, system-ui, sans-serif; }
    .timeline-undated { padding: 12px 15px; border-top: 1px solid #e9e1d4; background: #fffefa; }
    .timeline-undated-list { display: flex; flex-wrap: wrap; gap: 7px; margin-top: 8px; }
    .timeline-undated-list button { border: 1px solid #d9cdbd; border-radius: 999px; padding: 5px 8px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-undated-list button:hover, .timeline-undated-list button:focus-visible { border-color: #b4773f; color: #55351f; outline: none; }
    .timeline-error { color: #9a4d3f; }
    @media (max-width: 760px) {
      .timeline-toolbar { justify-content: stretch; }
      .timeline-toolbar-actions { flex-wrap: wrap; }
      .timeline-workspace { grid-template-columns: 1fr !important; }
      .timeline-outline { max-height: 260px; border-right: 0; border-bottom: 1px solid #e9e1d4; }
      .timeline-outline-resize { display: none; }
      .timeline-canvas { height: 420px; min-height: 320px; }
    }
  `;
  return style;
}

function createToolbarButton(label: string) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  return button;
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

function renderSelection(details: HTMLElement, event: TimelineEvent, context: ModuleContext) {
  details.replaceChildren();
  const name = document.createElement("strong");
  name.textContent = event.entity.name;
  const range = document.createElement("small");
  range.textContent = rangeLabel(event.fields.startsAt, event.fields.endsAt);
  const contextText = contextLabel(event);
  if (contextText) {
    const contextLine = document.createElement("small");
    contextLine.textContent = contextText;
    details.append(name, range, contextLine);
  } else {
    details.append(name, range);
  }
  if (context.services.isAvailable("daena.maps/navigation", 1)) {
    const mapButton = document.createElement("button");
    mapButton.type = "button";
    mapButton.className = "timeline-map-button";
    mapButton.textContent = "Show on map";
    mapButton.onclick = () => void showOnMap(context, event.entity.id);
    details.append(mapButton);
  }
}

function renderUndatedSelection(details: HTMLElement, event: UndatedEvent, context: ModuleContext) {
  details.replaceChildren();
  const name = document.createElement("strong");
  name.textContent = event.entity.name;
  const status = document.createElement("small");
  status.textContent =
    event.relativeYear === undefined
      ? "No date yet — add a start or end date to place this event in the chronology."
      : `Relative chronology: ${relativeOffsetLabel(event.relativeYear)}. It is kept relative rather than converted to a Gregorian date.`;
  const contextText = contextLabel(event);
  if (contextText) {
    const contextLine = document.createElement("small");
    contextLine.textContent = contextText;
    details.append(name, status, contextLine);
  } else {
    details.append(name, status);
  }
  if (context.services.isAvailable("daena.maps/navigation", 1)) {
    const mapButton = document.createElement("button");
    mapButton.type = "button";
    mapButton.className = "timeline-map-button";
    mapButton.textContent = "Show on map";
    mapButton.onclick = () => void showOnMap(context, event.entity.id);
    details.append(mapButton);
  }
}

function eventYear(event: TimelineEvent): number {
  return event.start.getUTCFullYear();
}

function renderOutline(outline: HTMLElement, events: TimelineEvent[], onSelect: (event: TimelineEvent) => void): void {
  const heading = document.createElement("span");
  heading.className = "timeline-outline-heading";
  heading.textContent = "Chronological outline";
  outline.append(heading);
  let currentYear: number | null = null;
  for (const event of events) {
    const year = eventYear(event);
    if (year !== currentYear) {
      currentYear = year;
      const yearHeading = document.createElement("div");
      yearHeading.className = "timeline-outline-year";
      yearHeading.textContent = String(year);
      outline.append(yearHeading);
    }
    const card = document.createElement("button");
    card.type = "button";
    card.className = "timeline-event-card";
    const name = document.createElement("strong");
    name.textContent = event.entity.name;
    const range = document.createElement("small");
    range.textContent = rangeLabel(event.fields.startsAt, event.fields.endsAt);
    card.append(name, range);
    card.onclick = () => onSelect(event);
    outline.append(card);
  }
}

function toDataItem(event: TimelineEvent): DataItem {
  const item: DataItem = {
    id: event.entity.id,
    content: event.entity.name,
    start: event.start,
    title: `${event.entity.name}\n${rangeLabel(event.fields.startsAt, event.fields.endsAt)}`,
    style: `background-color:${event.colors.fill};border-color:${event.colors.border};color:${event.colors.text};`,
  };
  if (event.end && event.end.getTime() !== event.start.getTime()) {
    item.end = event.end;
    item.type = "range";
  } else {
    item.type = "box";
  }
  return item;
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
            const entityTypes = new Set(context.module.schemas.flatMap((schema) => schema.entityTypes));
            const entities = await context.entities.list();
            const loaded = await Promise.all(
              entities.map(async (entity) => {
                const fields = await context.fields.list(entity.id);
                let sharedMapsFields: FieldRecord[] = [];
                try {
                  // Shared chronology remains readable when Maps is disabled;
                  // only the navigation service should disappear with it.
                  sharedMapsFields = await context.fields.listShared(entity.id, "maps");
                } catch {
                  // A project without the optional Maps contract still renders
                  // ordinary Timeline items instead of failing the projection.
                }
                const relativeYear = physicalOffset(
                  sharedMapsFields.find((field) => field.key === "physicalChronology")?.value,
                );
                const relationships = (await context.relationships.list(entity.id)).filter(
                  (relationship) => relationship.type === "occurred_at" || relationship.type === "involves",
                );
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
                    .map((relationship, index) => (relationship.type === "involves" ? targets[index]?.name : undefined))
                    .filter((name): name is string => Boolean(name)),
                  relativeYear,
                };
              }),
            );
            const dated: TimelineEvent[] = [];
            const undated: UndatedEvent[] = [];
            for (const entry of loaded) {
              if (!entityTypes.has(entry.entity.type ?? "") && entry.relativeYear === null) continue;
              const start = toJsDate(entry.fields.startsAt) ?? toJsDate(entry.fields.endsAt);
              if (!start) {
                undated.push({ ...entry, relativeYear: entry.relativeYear ?? undefined });
                continue;
              }
              const end = entry.fields.endsAt ? toJsDate(entry.fields.endsAt) : null;
              dated.push({
                entity: entry.entity,
                fields: entry.fields,
                locationName: entry.locationName,
                participantNames: entry.participantNames,
                start,
                end: end && end.getTime() >= start.getTime() ? end : null,
                colors: colorsForHue(hueForId(entry.entity.id)),
              });
            }
            dated.sort((left, right) => compareCalendarDates(left.fields.startsAt, right.fields.startsAt));
            const years = [...new Set(dated.map(eventYear))];
            if (activeYear !== null && !years.includes(activeYear)) activeYear = null;
            const visible = activeYear === null ? dated : dated.filter((event) => eventYear(event) === activeYear);
            if (cancelled) return;

            element.replaceChildren();
            element.className = "timeline-projection";
            const shell = document.createElement("div");
            shell.className = "timeline-shell";
            const header = document.createElement("div");
            header.className = "projection-header";
            const heading = document.createElement("h3");
            heading.textContent = "Chronology";
            const summary = document.createElement("small");
            summary.textContent =
              undated.length > 0
                ? `${dated.length} placed · ${undated.length} unplaced or relative`
                : `${dated.length} items`;
            header.append(heading, summary);
            shell.append(style, header);
            let details: HTMLElement | null = null;

            if (dated.length === 0 && undated.length === 0) {
              const empty = document.createElement("p");
              empty.className = "timeline-empty";
              empty.textContent = "No timeline items yet.";
              shell.append(empty);
              element.append(shell);
              return;
            }

            if (dated.length === 0) {
              const empty = document.createElement("p");
              empty.className = "timeline-empty";
              empty.textContent = "No dated timeline items to plot yet.";
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
                void render();
              };
              const actions = document.createElement("div");
              actions.className = "timeline-toolbar-actions";
              const zoomInButton = createToolbarButton("Zoom in");
              const zoomOutButton = createToolbarButton("Zoom out");
              const fitButton = createToolbarButton("Fit view");
              const outlineButton = createToolbarButton(outlineCollapsed ? "Show outline" : "Hide outline");
              outlineButton.onclick = () => {
                outlineCollapsed = !outlineCollapsed;
                void render();
              };
              actions.append(zoomOutButton, zoomInButton, fitButton, outlineButton);
              toolbar.append(scope, actions);

              const workspace = document.createElement("div");
              workspace.className = "timeline-workspace";
              if (outlineCollapsed) workspace.style.gridTemplateColumns = "minmax(0, 1fr)";
              else workspace.style.setProperty("--timeline-outline-width", `${outlineWidth}px`);
              const outline = document.createElement("aside");
              outline.className = "timeline-outline";
              const canvas = document.createElement("div");
              canvas.className = "timeline-canvas";
              const detailPanel = document.createElement("div");
              details = detailPanel;
              detailPanel.className = "timeline-details";
              const hint = document.createElement("small");
              hint.textContent = "Choose an item from the outline or timeline to inspect it.";
              detailPanel.append(hint);
              const selectEvent = (event: TimelineEvent) => {
                chart?.setSelection([event.entity.id], {
                  focus: true,
                  animation: {},
                });
                chart?.focus(event.entity.id, { animation: { duration: 220, easingFunction: "easeInOutQuad" } });
                renderSelection(detailPanel, event, context);
              };
              if (visible.length > 0) renderOutline(outline, visible, selectEvent);
              else {
                const empty = document.createElement("p");
                empty.className = "timeline-empty";
                empty.textContent = "No items in this year.";
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
                workspace.append(outline, resizeHandle, canvas);
              } else {
                workspace.append(canvas);
              }
              shell.append(toolbar, workspace, detailPanel);
              element.append(shell);

              await import("vis-timeline/styles/vis-timeline-graph2d.min.css");
              const { Timeline: TimelineCtor } = await import("vis-timeline/standalone");
              if (cancelled) return;

              const eventsById = new Map(visible.map((event) => [event.entity.id, event]));
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
                  minorLabels: formatAxisDate,
                  majorLabels: formatAxisDate,
                },
              };
              if (visible.length >= 40) {
                options.cluster = { titleTemplate: "{count} events", showStipes: true, maxItems: 8 };
              }
              chart = new TimelineCtor(canvas, visible.map(toDataItem), options);
              chart.fit({ animation: false });

              zoomInButton.onclick = () => chart?.zoomIn(0.4);
              zoomOutButton.onclick = () => chart?.zoomOut(0.4);
              fitButton.onclick = () => chart?.fit({ animation: { duration: 320, easingFunction: "easeInOutQuad" } });

              chart.on("select", (properties) => {
                const selectedId = properties.items[0];
                const event = selectedId != null ? eventsById.get(String(selectedId)) : undefined;
                if (!event) {
                  detailPanel.replaceChildren(hint);
                  return;
                }
                selectEvent(event);
              });
              chart.on("click", (properties) => {
                if (properties.item != null) return;
                chart?.setSelection([]);
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
            const header = document.createElement("div");
            header.className = "projection-header";
            const heading = document.createElement("h3");
            heading.textContent = "Chronology";
            header.append(heading);
            shell.append(style, header);
            showError(shell, cause);
            element.append(shell);
          }
        };
        void render();
        return () => {
          cancelled = true;
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
