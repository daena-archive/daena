import type { Timeline, DataItem, TimelineOptions } from "vis-timeline";
import type { ModuleContext, DaenaModule } from "../../../module-api/src/index";
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
  start: Date;
  end: Date | null;
  colors: EventColors;
};

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
  const utc = Date.UTC(date.year, month, day);
  return Number.isFinite(utc) ? new Date(utc) : null;
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
    precision: "day",
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

function createTimelineStyles(): HTMLStyleElement {
  const style = document.createElement("style");
  style.textContent = `
    .timeline-shell { display: grid; gap: 0; }
    .timeline-toolbar { display: flex; align-items: center; justify-content: flex-end; gap: 12px; padding: 10px 14px; border-bottom: 1px solid #e9e1d4; background: #fffefa; }
    .timeline-toolbar-actions { display: flex; gap: 7px; }
    .timeline-toolbar button { border: 1px solid #d9cdbd; border-radius: 7px; padding: 6px 9px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .timeline-toolbar button:hover, .timeline-toolbar button:focus-visible { border-color: #b4773f; color: #55351f; outline: none; }
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
    .timeline-undated { padding: 10px 15px; border-top: 1px solid #e9e1d4; background: #fffefa; }
    .timeline-error { color: #9a4d3f; }
    @media (max-width: 760px) {
      .timeline-toolbar { justify-content: stretch; }
      .timeline-toolbar-actions { flex-wrap: wrap; }
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
  const mapButton = document.createElement("button");
  mapButton.type = "button";
  mapButton.className = "timeline-map-button";
  mapButton.textContent = "Show on map";
  mapButton.onclick = () => void showOnMap(context, event.entity.id);
  details.append(name, range, mapButton);
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
        const style = createTimelineStyles();
        const render = async () => {
          try {
            const entities = await context.entities.list({ type: "event" });
            const loaded = await Promise.all(
              entities.map(async (entity) => ({ entity, fields: await context.fields.list(entity.id) })),
            );
            const dated: TimelineEvent[] = [];
            const undated: { entity: { id: string; name: string }; fields: Record<string, unknown> }[] = [];
            for (const entry of loaded) {
              const start = toJsDate(entry.fields.startsAt) ?? toJsDate(entry.fields.endsAt);
              if (!start) {
                undated.push(entry);
                continue;
              }
              const end = entry.fields.endsAt ? toJsDate(entry.fields.endsAt) : null;
              dated.push({
                entity: entry.entity,
                fields: entry.fields,
                start,
                end: end && end.getTime() >= start.getTime() ? end : null,
                colors: colorsForHue(hueForId(entry.entity.id)),
              });
            }
            dated.sort((left, right) => compareCalendarDates(left.fields.startsAt, right.fields.startsAt));
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
              undated.length > 0 ? `${dated.length} dated · ${undated.length} undated` : `${dated.length} events`;
            header.append(heading, summary);
            shell.append(style, header);

            if (dated.length === 0 && undated.length === 0) {
              const empty = document.createElement("p");
              empty.className = "timeline-empty";
              empty.textContent = "No timeline events yet.";
              shell.append(empty);
              element.append(shell);
              return;
            }

            if (dated.length === 0) {
              const empty = document.createElement("p");
              empty.className = "timeline-empty";
              empty.textContent = "No dated events to plot yet.";
              shell.append(empty);
            } else {
              const toolbar = document.createElement("div");
              toolbar.className = "timeline-toolbar";
              const actions = document.createElement("div");
              actions.className = "timeline-toolbar-actions";
              const zoomInButton = createToolbarButton("Zoom in");
              const zoomOutButton = createToolbarButton("Zoom out");
              const fitButton = createToolbarButton("Fit view");
              actions.append(zoomOutButton, zoomInButton, fitButton);
              toolbar.append(actions);

              const canvas = document.createElement("div");
              canvas.className = "timeline-canvas";
              const details = document.createElement("div");
              details.className = "timeline-details";
              const hint = document.createElement("small");
              hint.textContent = "Select an event to inspect its range.";
              details.append(hint);
              shell.append(toolbar, canvas, details);
              element.append(shell);

              await import("vis-timeline/styles/vis-timeline-graph2d.min.css");
              const { Timeline: TimelineCtor } = await import("vis-timeline/standalone");
              if (cancelled) return;

              const eventsById = new Map(dated.map((event) => [event.entity.id, event]));
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
              if (dated.length >= 40) {
                options.cluster = { titleTemplate: "{count} events", showStipes: true, maxItems: 8 };
              }
              chart = new TimelineCtor(canvas, dated.map(toDataItem), options);
              chart.fit({ animation: false });

              zoomInButton.onclick = () => chart?.zoomIn(0.4);
              zoomOutButton.onclick = () => chart?.zoomOut(0.4);
              fitButton.onclick = () => chart?.fit({ animation: { duration: 320, easingFunction: "easeInOutQuad" } });

              chart.on("select", (properties) => {
                const selectedId = properties.items[0];
                const event = selectedId != null ? eventsById.get(String(selectedId)) : undefined;
                if (!event) {
                  details.replaceChildren(hint);
                  return;
                }
                renderSelection(details, event, context);
              });
              chart.on("click", (properties) => {
                if (properties.item != null) return;
                chart?.setSelection([]);
                details.replaceChildren(hint);
              });

              resizeObserver = new ResizeObserver(() => chart?.redraw());
              resizeObserver.observe(canvas);
            }

            if (undated.length > 0) {
              const note = document.createElement("p");
              note.className = "timeline-undated";
              note.textContent = `Not plotted (no date): ${undated.map((entry) => entry.entity.name).join(", ")}`;
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
          chart?.destroy();
          chart = null;
          element.replaceChildren();
        };
      },
    },
  ],
};
