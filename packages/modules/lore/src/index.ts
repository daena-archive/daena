import type { ModuleContext, WorldbuilderModule } from "../../../module-api/src/index";
import type { ModuleManifest } from "../../../module-api/src/index";
import manifestJson from "../manifest.json";

const manifest = manifestJson as unknown as ModuleManifest;

export const lore: WorldbuilderModule = {
  manifest,
  views: [{
    id: "lore-entities",
    title: "Lore Entries",
    mount: (element: HTMLElement, context: ModuleContext) => {
      let cancelled = false;
      const render = async () => {
        const entities = (await context.entities.list()).filter((entity) => entity.type !== "event");
        const relationships = (await Promise.all(entities.map((entity) => context.relationships.list(entity.id))))
          .flat()
          .filter((relationship, index, all) => all.findIndex((candidate) => candidate.id === relationship.id) === index)
          .filter((relationship) => entities.some((entity) => entity.id === relationship.sourceId) && entities.some((entity) => entity.id === relationship.targetId));
        if (cancelled) return;
        element.replaceChildren();
        const header = document.createElement("div");
        header.className = "projection-header";
        const heading = document.createElement("h3");
        heading.textContent = "World graph";
        const summary = document.createElement("small");
        summary.textContent = `${entities.length} entries · ${relationships.length} links`;
        header.append(heading, summary);
        element.className = "projection-graph";
        element.append(header);
        if (entities.length === 0) {
          const empty = document.createElement("p");
          empty.textContent = "No lore entries yet.";
          element.append(empty);
          return;
        }
        const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
        svg.setAttribute("viewBox", "0 0 720 230");
        const positions = new Map(entities.map((entity, index) => [entity.id, { x: 70 + (index % 5) * 145, y: 70 + Math.floor(index / 5) * 85 }]));
        for (const relationship of relationships) {
          const source = positions.get(relationship.sourceId);
          const target = positions.get(relationship.targetId);
          if (!source || !target) continue;
          const edge = document.createElementNS("http://www.w3.org/2000/svg", "line");
          edge.classList.add("projection-edge");
          edge.setAttribute("x1", String(source.x)); edge.setAttribute("y1", String(source.y));
          edge.setAttribute("x2", String(target.x)); edge.setAttribute("y2", String(target.y));
          svg.append(edge);
        }
        for (const entity of entities) {
          const position = positions.get(entity.id)!;
          const group = document.createElementNS("http://www.w3.org/2000/svg", "g");
          const node = document.createElementNS("http://www.w3.org/2000/svg", "circle");
          node.classList.add("projection-node"); node.setAttribute("cx", String(position.x)); node.setAttribute("cy", String(position.y)); node.setAttribute("r", "21");
          const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
          label.classList.add("projection-node-label"); label.setAttribute("x", String(position.x)); label.setAttribute("y", String(position.y + 39)); label.setAttribute("text-anchor", "middle"); label.textContent = entity.name.slice(0, 20);
          const type = document.createElementNS("http://www.w3.org/2000/svg", "text");
          type.classList.add("projection-node-type"); type.setAttribute("x", String(position.x)); type.setAttribute("y", String(position.y + 53)); type.setAttribute("text-anchor", "middle"); type.textContent = entity.type ?? "entry";
          group.append(node, label, type); svg.append(group);
        }
        element.append(svg);
      };
      void render();
      return () => {
        cancelled = true;
        element.replaceChildren();
      };
    },
  }],
};
