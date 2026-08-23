import type { Core, ElementDefinition, NodeSingular } from "cytoscape";
import {
  type EntitySummary,
  type ModuleContext,
  type DaenaModule,
  type Relationship,
} from "../../../module-api/src/index";
import type { ModuleManifest } from "../../../module-api/src/index";
import manifestJson from "../manifest.json";

const manifest = manifestJson as unknown as ModuleManifest;

type NodeColors = { fill: string; border: string; text: string };

function displayType(type: string | null) {
  return type?.trim() || "entity";
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

function colorsForHue(hue: number): NodeColors {
  return {
    fill: hslToHex(hue, 0.48, 0.87),
    border: hslToHex(hue, 0.46, 0.42),
    text: hslToHex(hue, 0.4, 0.22),
  };
}

function colorsForTypes(types: Iterable<string>): Map<string, NodeColors> {
  const uniqueTypes = [...new Set(types)].sort();
  const count = Math.max(uniqueTypes.length, 1);
  return new Map(uniqueTypes.map((type, index) => [type, colorsForHue((24 + (index * 360) / count) % 360)]));
}

function createGraphStyles(): HTMLStyleElement {
  const style = document.createElement("style");
  style.textContent = `
    .lore-graph-shell { display: grid; gap: 0; }
    .lore-graph-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 14px; border-bottom: 1px solid #e9e1d4; background: #fffefa; }
    .lore-graph-toolbar-actions { display: flex; gap: 7px; }
    .lore-graph-toolbar button { border: 1px solid #d9cdbd; border-radius: 7px; padding: 6px 9px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .lore-graph-toolbar button:hover, .lore-graph-toolbar button:focus-visible { border-color: #b4773f; color: #55351f; outline: none; }
    .lore-graph-legend { display: flex; flex-wrap: wrap; gap: 7px; color: #8f897e; font: 10px Inter, ui-sans-serif, system-ui, sans-serif; }
    .lore-graph-legend-item { display: inline-flex; align-items: center; gap: 5px; margin: 0; padding: 4px 8px; border: 1px solid #e4d9c8; border-radius: 999px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .lore-graph-legend-item:hover, .lore-graph-legend-item:focus-visible { border-color: #b4773f; color: #55351f; outline: none; }
    .lore-graph-legend-item.is-hidden { opacity: 0.45; border-style: dashed; color: #8f897e; }
    .lore-graph-legend-item.is-hidden .lore-graph-legend-swatch { background: transparent !important; }
    .lore-graph-legend-swatch { width: 8px; height: 8px; border: 1px solid currentColor; border-radius: 50%; }
    .lore-graph-canvas { position: relative; height: min(58vh, 560px); min-height: 380px; background: radial-gradient(circle at 50% 42%, #fffdf7 0, #fbf8f0 52%, #f4eee3 100%); }
    .lore-graph-details { display: grid; gap: 7px; min-height: 55px; padding: 12px 15px; border-top: 1px solid #e9e1d4; background: #fffefa; color: #62594e; font: 11px/1.45 Inter, ui-sans-serif, system-ui, sans-serif; }
    .lore-graph-details strong { color: #302c26; font: 500 16px/1.1 var(--font-display, Georgia, serif); }
    .lore-graph-details small { color: #8f897e; }
    .lore-graph-details-list { display: flex; flex-wrap: wrap; gap: 5px 10px; margin: 0; padding: 0; list-style: none; }
    .lore-graph-details-list li { padding: 3px 7px; border-radius: 999px; background: #f4eee3; }
    .lore-graph-map-button { width: fit-content; padding: 5px 9px; border: 1px solid #d9cdbd; border-radius: 7px; background: #fffefa; color: #62594e; font: 600 10px Inter, ui-sans-serif, system-ui, sans-serif; cursor: pointer; }
    .lore-graph-map-button:hover, .lore-graph-map-button:focus-visible { border-color: #b4773f; color: #55351f; outline: none; }
    .lore-graph-empty { margin: 0; padding: 28px 18px; color: #8f897e; font: 12px/1.5 Inter, ui-sans-serif, system-ui, sans-serif; }
    @media (max-width: 760px) {
      .lore-graph-toolbar { align-items: flex-start; flex-direction: column; }
      .lore-graph-canvas { height: 420px; min-height: 320px; }
    }
  `;
  return style;
}

function compareEntities(left: EntitySummary, right: EntitySummary) {
  return left.name.localeCompare(right.name) || left.id.localeCompare(right.id);
}

function stablePositions(entities: EntitySummary[]): Map<string, { x: number; y: number }> {
  const columns = Math.max(3, Math.ceil(Math.sqrt(entities.length)));
  const columnWidth = 360;
  const rowHeight = 270;
  const originX = 220;
  const originY = 170;
  return new Map(
    [...entities].sort(compareEntities).map((entity, index) => [
      entity.id,
      {
        x: originX + (index % columns) * columnWidth,
        y: originY + Math.floor(index / columns) * rowHeight,
      },
    ]),
  );
}

function separateOverlappingNodes(graph: Core) {
  const nodes = graph
    .nodes()
    .toArray()
    .sort((left, right) => left.id().localeCompare(right.id()));
  const gap = 36;

  for (let pass = 0; pass < 32; pass += 1) {
    let moved = false;
    for (let firstIndex = 0; firstIndex < nodes.length; firstIndex += 1) {
      const first = nodes[firstIndex];
      const firstBox = first.boundingBox();
      for (let secondIndex = firstIndex + 1; secondIndex < nodes.length; secondIndex += 1) {
        const second = nodes[secondIndex];
        const secondBox = second.boundingBox();
        const overlapX = Math.min(firstBox.x2, secondBox.x2) - Math.max(firstBox.x1, secondBox.x1) + gap;
        const overlapY = Math.min(firstBox.y2, secondBox.y2) - Math.max(firstBox.y1, secondBox.y1) + gap;
        if (overlapX <= 0 || overlapY <= 0) continue;

        const firstPosition = first.position();
        const secondPosition = second.position();
        const horizontalDirection =
          secondPosition.x > firstPosition.x || (secondPosition.x === firstPosition.x && secondIndex % 2 === 0)
            ? 1
            : -1;
        const verticalDirection =
          secondPosition.y > firstPosition.y || (secondPosition.y === firstPosition.y && secondIndex % 2 === 0)
            ? 1
            : -1;
        if (overlapX <= overlapY) {
          const shift = overlapX / 2;
          first.position({ x: firstPosition.x - horizontalDirection * shift, y: firstPosition.y });
          second.position({ x: secondPosition.x + horizontalDirection * shift, y: secondPosition.y });
        } else {
          const shift = overlapY / 2;
          first.position({ x: firstPosition.x, y: firstPosition.y - verticalDirection * shift });
          second.position({ x: secondPosition.x, y: secondPosition.y + verticalDirection * shift });
        }
        moved = true;
      }
    }
    if (!moved) break;
  }
}

function graphElements(
  entities: EntitySummary[],
  relationships: Relationship[],
  positions: Map<string, { x: number; y: number }>,
  colorsByType: Map<string, NodeColors>,
): ElementDefinition[] {
  const entityIds = new Set(entities.map((entity) => entity.id));
  return [
    ...entities.map((entity) => ({
      group: "nodes" as const,
      position: positions.get(entity.id),
      data: {
        id: entity.id,
        label: entity.name,
        type: entity.type ?? "",
        typeLabel: displayType(entity.type),
        ...colorsByType.get(displayType(entity.type))!,
      },
    })),
    ...relationships
      .filter((relationship) => entityIds.has(relationship.sourceId) && entityIds.has(relationship.targetId))
      .map((relationship) => ({
        group: "edges" as const,
        data: {
          id: relationship.id,
          source: relationship.sourceId,
          target: relationship.targetId,
          label: relationship.type.split("_").join(" "),
        },
      })),
  ];
}

function isGraphNode(value: NodeSingular | EntitySummary): value is NodeSingular {
  return typeof (value as NodeSingular).id === "function";
}

function renderSelection(
  details: HTMLElement,
  nodeOrEntity: NodeSingular | EntitySummary,
  entities: Map<string, EntitySummary>,
  relationships: Relationship[],
  context: ModuleContext,
) {
  const entity = isGraphNode(nodeOrEntity) ? entities.get(nodeOrEntity.id()) : nodeOrEntity;
  if (!entity) return;
  const connected = relationships
    .filter((relationship) => relationship.sourceId === entity.id || relationship.targetId === entity.id)
    .map((relationship) => {
      const otherId = relationship.sourceId === entity.id ? relationship.targetId : relationship.sourceId;
      const other = entities.get(otherId);
      return other ? `${relationship.type.split("_").join(" ")} · ${other.name}` : null;
    })
    .filter((label): label is string => Boolean(label));
  details.replaceChildren();
  const name = document.createElement("strong");
  name.textContent = entity.name;
  const type = document.createElement("small");
  type.textContent = `${displayType(entity.type)} · ${connected.length} connection${connected.length === 1 ? "" : "s"}`;
  const actionRow = document.createElement("div");
  actionRow.className = "lore-graph-toolbar-actions";
  if (context.services.isAvailable("daena.maps/navigation", 1)) {
    const mapButton = document.createElement("button");
    mapButton.type = "button";
    mapButton.className = "lore-graph-map-button";
    mapButton.textContent = "Show on map";
    mapButton.onclick = () => void showOnMap(context, entity.id);
    actionRow.append(mapButton);
  }
  details.append(name, type, actionRow);
  if (connected.length === 0) return;
  const list = document.createElement("ul");
  list.className = "lore-graph-details-list";
  for (const label of connected.slice(0, 8)) {
    const item = document.createElement("li");
    item.textContent = label;
    list.append(item);
  }
  details.append(list);
}

function applyNeighborhoodHighlight(graph: Core, node: NodeSingular | null, showLabels = false) {
  graph.elements().removeClass("faded related hover-related");
  if (!node || node.hasClass("type-hidden")) return;
  const neighborhood = node.closedNeighborhood().not(".type-hidden");
  graph.elements().not(".type-hidden").difference(neighborhood).addClass("faded");
  node.connectedEdges().not(".type-hidden").addClass("related");
  if (showLabels) node.connectedEdges().not(".type-hidden").addClass("hover-related");
}

function applyTypeVisibility(graph: Core, hiddenTypes: Set<string>) {
  graph.nodes().forEach((node) => {
    const hidden = hiddenTypes.has(String(node.data("typeLabel") ?? "entity"));
    node.toggleClass("type-hidden", hidden);
    if (hidden) node.removeClass("hovered selected");
  });
  graph.edges().forEach((edge) => {
    const hidden = edge.source().hasClass("type-hidden") || edge.target().hasClass("type-hidden");
    edge.toggleClass("type-hidden", hidden);
    if (hidden) edge.removeClass("related selected hover-related");
  });
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

export const lore: DaenaModule = {
  manifest,
  views: [
    {
      id: "lore-entities",
      title: "Lore Entries",
      mount: (element: HTMLElement, context: ModuleContext) => {
        let cancelled = false;
        let graph: Core | null = null;
        const style = createGraphStyles();
        const render = async () => {
          const entityTypes = new Set(context.module.schemas.flatMap((schema) => schema.entityTypes));
          const entities = (await context.entities.list())
            .filter((entity) => entity.type !== null && entityTypes.has(entity.type))
            .sort(compareEntities);
          const relationships = (await Promise.all(entities.map((entity) => context.relationships.list(entity.id))))
            .flat()
            .filter(
              (relationship, index, all) => all.findIndex((candidate) => candidate.id === relationship.id) === index,
            )
            .filter(
              (relationship) =>
                entities.some((entity) => entity.id === relationship.sourceId) &&
                entities.some((entity) => entity.id === relationship.targetId),
            )
            .sort((left, right) => left.id.localeCompare(right.id));
          if (cancelled) return;

          element.replaceChildren();
          element.className = "projection-graph";
          const shell = document.createElement("div");
          shell.className = "lore-graph-shell";
          const header = document.createElement("div");
          header.className = "projection-contextbar";
          const summary = document.createElement("small");
          summary.textContent = `${entities.length} entities · ${relationships.length} links`;
          header.append(summary);
          shell.append(style, header);

          if (entities.length === 0) {
            const empty = document.createElement("p");
            empty.className = "lore-graph-empty";
            empty.textContent = "No entities yet.";
            shell.append(empty);
            element.append(shell);
            return;
          }

          const toolbar = document.createElement("div");
          toolbar.className = "lore-graph-toolbar";
          const legend = document.createElement("div");
          legend.className = "lore-graph-legend";
          const typeNames = [...new Set(entities.map((entity) => displayType(entity.type)))].sort();
          const colorsByType = colorsForTypes(typeNames);
          const hiddenTypes = new Set<string>();
          const legendButtons = new Map<string, HTMLButtonElement>();
          for (const type of typeNames) {
            const legendItem = document.createElement("button");
            legendItem.type = "button";
            legendItem.className = "lore-graph-legend-item";
            legendItem.setAttribute("aria-pressed", "true");
            legendItem.title = `Hide ${type}`;
            const swatch = document.createElement("span");
            swatch.className = "lore-graph-legend-swatch";
            const colors = colorsByType.get(type)!;
            swatch.style.color = colors.border;
            swatch.style.background = colors.fill;
            const label = document.createElement("span");
            label.textContent = type;
            legendItem.append(swatch, label);
            legendButtons.set(type, legendItem);
            legend.append(legendItem);
          }
          const actions = document.createElement("div");
          actions.className = "lore-graph-toolbar-actions";
          const fitButton = document.createElement("button");
          fitButton.type = "button";
          fitButton.textContent = "Fit graph";
          const layoutButton = document.createElement("button");
          layoutButton.type = "button";
          layoutButton.textContent = "Rearrange";
          actions.append(fitButton, layoutButton);
          toolbar.append(legend, actions);
          shell.append(toolbar);

          const canvas = document.createElement("div");
          canvas.className = "lore-graph-canvas";
          shell.append(canvas);
          const details = document.createElement("div");
          details.className = "lore-graph-details";
          const hint = document.createElement("small");
          hint.textContent = "Hover or select an entity to inspect its connections. Click a type to show or hide it.";
          details.append(hint);
          shell.append(details);
          element.append(shell);

          const entityMap = new Map(entities.map((entity) => [entity.id, entity]));
          const positions = stablePositions(entities);
          const { default: createCytoscape } = await import("cytoscape");
          if (cancelled) return;
          const layout = {
            name: relationships.length > 0 ? ("cose" as const) : ("grid" as const),
            animate: false,
            fit: false,
            padding: 72,
            randomize: false,
            avoidOverlap: true,
            nodeDimensionsIncludeLabels: true,
            componentSpacing: 88,
            nodeRepulsion: 18000,
            nodeOverlap: 56,
            idealEdgeLength: 340,
            edgeElasticity: 0.18,
            gravity: 0.55,
            numIter: 3000,
          };
          graph = createCytoscape({
            container: canvas,
            elements: graphElements(entities, relationships, positions, colorsByType),
            minZoom: 0.25,
            maxZoom: 3,
            wheelSensitivity: 0.18,
            style: [
              {
                selector: "node",
                style: {
                  "background-color": "data(fill)",
                  "border-color": "data(border)",
                  "border-width": 2,
                  color: "data(text)",
                  label: "data(label)",
                  "font-family": "Inter, ui-sans-serif, system-ui, sans-serif",
                  "font-size": 11,
                  "font-weight": 600,
                  "text-wrap": "ellipsis",
                  "text-max-width": "112px",
                  "text-valign": "center",
                  "text-halign": "center",
                  width: "label",
                  height: 34,
                  padding: "10px",
                  shape: "round-rectangle",
                  "overlay-opacity": 0,
                },
              },
              {
                selector: "edge",
                style: {
                  width: 1.6,
                  "line-color": "#c6b9a7",
                  "target-arrow-color": "#c6b9a7",
                  "target-arrow-shape": "triangle",
                  "curve-style": "bezier",
                  label: "",
                  color: "#8d8273",
                  "font-family": "Inter, ui-sans-serif, system-ui, sans-serif",
                  "font-size": 10,
                  "text-wrap": "ellipsis",
                  "text-max-width": "130px",
                  "text-background-color": "#fffefa",
                  "text-background-opacity": 0.9,
                  "text-background-padding": "2px",
                  "text-rotation": "autorotate",
                  "overlay-opacity": 0,
                },
              },
              {
                selector: "edge.related, edge.selected",
                style: {
                  label: "",
                  width: 2.4,
                  "line-color": "#a87945",
                  "target-arrow-color": "#a87945",
                  color: "#6f4d2f",
                },
              },
              {
                selector: "edge.hover-related",
                style: { label: "data(label)" },
              },
              { selector: ".faded", style: { opacity: 0.16 } },
              { selector: ".hovered", style: { "border-width": 3, "border-color": "#8b5c2e" } },
              { selector: "node.selected", style: { "border-width": 4, "border-color": "#8b5c2e" } },
              { selector: ".type-hidden", style: { display: "none" } },
            ],
          });
          const visibleElements = () => graph!.elements().not(".type-hidden");
          const arrangeGraph = () => {
            if (!graph) return;
            graph.layout(layout).run();
            separateOverlappingNodes(graph);
            const visible = visibleElements();
            if (visible.length > 0) graph.fit(visible, 72);
          };
          arrangeGraph();

          const syncTypeVisibility = () => {
            if (!graph) return;
            applyTypeVisibility(graph, hiddenTypes);
            const selected = graph.nodes("node.selected").not(".type-hidden");
            if (selected.length === 0) {
              graph.elements().removeClass("selected");
              applyNeighborhoodHighlight(graph, null);
              details.replaceChildren(hint);
            } else {
              applyNeighborhoodHighlight(graph, selected[0]);
            }
          };

          for (const [type, legendItem] of legendButtons) {
            legendItem.onclick = () => {
              if (hiddenTypes.has(type)) {
                hiddenTypes.delete(type);
                legendItem.classList.remove("is-hidden");
                legendItem.setAttribute("aria-pressed", "true");
                legendItem.title = `Hide ${type}`;
              } else {
                hiddenTypes.add(type);
                legendItem.classList.add("is-hidden");
                legendItem.setAttribute("aria-pressed", "false");
                legendItem.title = `Show ${type}`;
              }
              syncTypeVisibility();
            };
          }

          fitButton.onclick = () => {
            if (!graph) return;
            const visible = visibleElements();
            if (visible.length > 0) graph.fit(visible, 72);
          };
          layoutButton.onclick = () => {
            if (!graph) return;
            graph.nodes().forEach((node) => {
              const position = positions.get(node.id());
              if (position) node.position(position);
            });
            arrangeGraph();
          };
          graph.on("mouseover", "node", (event) => {
            const node = event.target;
            if (node.hasClass("type-hidden")) return;
            node.addClass("hovered");
            applyNeighborhoodHighlight(graph!, node, true);
          });
          graph.on("mouseout", "node", (event) => {
            event.target.removeClass("hovered");
            const selected = graph!.nodes("node.selected").not(".type-hidden");
            applyNeighborhoodHighlight(graph!, selected.length > 0 ? selected[0] : null);
          });
          graph.on("tap", "node", (event) => {
            const node = event.target;
            if (node.hasClass("type-hidden")) return;
            graph!.elements().removeClass("selected");
            node.addClass("selected");
            node.connectedEdges().not(".type-hidden").addClass("selected");
            renderSelection(details, node, entityMap, relationships, context);
            applyNeighborhoodHighlight(graph!, node);
          });
          graph.on("tap", (event) => {
            if (event.target !== graph) return;
            graph!.elements().removeClass("selected");
            applyNeighborhoodHighlight(graph!, null);
            details.replaceChildren(hint);
          });
        };
        void render();
        return () => {
          cancelled = true;
          graph?.destroy();
          graph = null;
          element.replaceChildren();
        };
      },
    },
  ],
};
