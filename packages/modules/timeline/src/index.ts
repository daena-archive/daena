import type { ModuleContext, DaenaModule } from "../../../module-api/src/index";
import type { ModuleManifest } from "../../../module-api/src/index";
import { compareCalendarDates, formatCalendarDate } from "../../../../src/lib/date";
import manifestJson from "../manifest.json";

const manifest = manifestJson as unknown as ModuleManifest;

async function showOnMap(context: ModuleContext, entityId: string) {
  try {
    const result = await context.maps.focusEntity({ entityId });
    if (result.status === "multiple-links" && result.locations.length > 0) {
      const location = result.locations[0];
      await context.maps.openMap({ mapEntityId: location.mapEntityId, linkId: location.id });
    }
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    if (message.includes("not-on-map") || message.includes("link-unresolved") || message.includes("map-unavailable") || message.includes("forbidden") || message.includes("denied")) return;
    console.error("show on map failed", cause);
  }
}

export const timeline: DaenaModule = {
  manifest,
  views: [{
    id: "timeline-events",
    title: "Timeline Events",
    mount: (element: HTMLElement, context: ModuleContext) => {
      let cancelled = false;
      const render = async () => {
        const entities = await context.entities.list({ type: "event" });
        const events = await Promise.all(entities.map(async (entity) => ({ entity, fields: await context.fields.list(entity.id) })));
        events.sort((left, right) => compareCalendarDates(left.fields.startsAt, right.fields.startsAt));
        if (cancelled) return;
        element.replaceChildren();
        element.className = "timeline-projection";
        const header = document.createElement("div");
        header.className = "projection-header";
        const heading = document.createElement("h3");
        heading.textContent = "Chronology";
        const summary = document.createElement("small");
        summary.textContent = `${events.length} events`;
        header.append(heading, summary);
        element.append(header);
        if (events.length === 0) {
          const empty = document.createElement("p");
          empty.textContent = "No timeline events yet.";
          element.append(empty);
          return;
        }
        const track = document.createElement("div");
        track.className = "timeline-track";
        for (const { entity, fields } of events) {
          const item = document.createElement("div");
          item.className = "timeline-event";
          const date = document.createElement("time");
          date.className = "timeline-date";
          date.textContent = formatCalendarDate(fields.startsAt);
          const card = document.createElement("div");
          card.className = "timeline-card";
          const name = document.createElement("strong");
          name.textContent = entity.name;
          const range = document.createElement("small");
          range.textContent = fields.endsAt ? `Through ${formatCalendarDate(fields.endsAt)}` : "Single event";
          const mapButton = document.createElement("button");
          mapButton.type = "button";
          mapButton.className = "timeline-map-button";
          mapButton.textContent = "Show on map";
          mapButton.onclick = () => void showOnMap(context, entity.id);
          card.append(name, range, mapButton);
          item.append(date, card); track.append(item);
        }
        element.append(track);
      };
      void render();
      return () => {
        cancelled = true;
        element.replaceChildren();
      };
    },
  }],
};
