import type { ModuleContext, ModuleId, WorldbuilderModule, ModuleView } from "../../../module-api/src/index";
import { compareCalendarDates, formatCalendarDate } from "../../../../src/lib/date";

const id = "worldbuilder.timeline" as ModuleId;

export const timeline: WorldbuilderModule = {
  manifest: {
    id,
    name: "Timeline",
    version: "0.1.0",
    apiVersion: "1",
    capabilities: ["entity.read", "entity.write", "document.read", "document.write", "relationship.read", "relationship.write", "asset.read", "asset.write", "search.query"],
    schemas: [{
      namespace: "timeline",
      entityTypes: ["event"],
      fields: [
        { key: "startsAt", label: "Starts", type: "date", required: true },
        { key: "endsAt", label: "Ends", type: "date" },
      ],
    }],
    templates: [{ id: "event", name: "Timeline event", entityType: "event", fields: { startsAt: "", endsAt: "" } }],
    migrations: [{ id: "timeline-v1", from: 0, to: 1, recovery: "backup", operations: [{ kind: "create-namespace", namespace: "timeline" }] }],
  },
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
          card.append(name, range);
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
