const pluginId = document.body.dataset.plugin;
const projectId = new URLSearchParams(window.location.search).get("project");
const status = document.querySelector("#status");
const projection = document.querySelector("#projection");
let sequence = 0;
let sessionId;

async function protocol(body) {
  const response = await fetch("/__rpc", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  const value = await response.json();
  if (!response.ok || value.error) throw new Error(value.error?.message ?? value.error ?? "Plugin protocol request failed");
  return value;
}

function rpc(method, payload) {
  return protocol({ op: "rpc", request: {
    rpcVersion: 1, sessionId, requestId: `${pluginId}-${++sequence}`, method, payload,
  }}).then((response) => {
    if (!response.ok) throw new Error(response.error?.message ?? "Plugin request failed");
    return response.result;
  });
}

function render(entities, relationships) {
  projection.replaceChildren();
  const summary = document.createElement("p");
  summary.className = "summary";
  summary.textContent = `${entities.length} entries · ${relationships.length} links`;
  projection.append(summary);
  if (!entities.length) return;
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 900 350");
  const positions = new Map(entities.map((entity, index) => [entity.id, { x: 90 + (index % 6) * 145, y: 100 + Math.floor(index / 6) * 100 }]));
  for (const relationship of relationships) {
    const source = positions.get(relationship.source_id), target = positions.get(relationship.target_id);
    if (!source || !target) continue;
    const edge = document.createElementNS("http://www.w3.org/2000/svg", "line");
    edge.classList.add("edge"); edge.setAttribute("x1", source.x); edge.setAttribute("y1", source.y); edge.setAttribute("x2", target.x); edge.setAttribute("y2", target.y); svg.append(edge);
  }
  for (const entity of entities) {
    const position = positions.get(entity.id);
    const node = document.createElementNS("http://www.w3.org/2000/svg", "circle"); node.classList.add("node"); node.setAttribute("cx", position.x); node.setAttribute("cy", position.y); node.setAttribute("r", "23"); svg.append(node);
    const label = document.createElementNS("http://www.w3.org/2000/svg", "text"); label.classList.add("label"); label.setAttribute("x", position.x); label.setAttribute("y", position.y + 42); label.textContent = entity.name.slice(0, 20); svg.append(label);
    const type = document.createElementNS("http://www.w3.org/2000/svg", "text"); type.classList.add("type"); type.setAttribute("x", position.x); type.setAttribute("y", position.y + 57); type.textContent = entity.entity_type ?? "entry"; svg.append(type);
  }
  projection.append(svg);
}

function renderTimeline(events) {
  projection.replaceChildren();
  const summary = document.createElement("p");
  summary.className = "summary";
  summary.textContent = `${events.length} events`;
  projection.append(summary);
  if (!events.length) return;
  const track = document.createElement("div");
  track.className = "timeline";
  const calendarValue = (value) => {
    if (value && typeof value === "object" && Number.isFinite(value.year)) {
      return [value.year, value.month, value.day].filter((part) => Number.isFinite(part)).join("-");
    }
    const text = String(value ?? "").trim();
    const match = /^(\d+)(?:-(\d+)(?:-(\d+))?)?$/.exec(text);
    return match ? match.slice(1).filter(Boolean).join("-") : "";
  };
  const timestamp = (value) => {
    const parts = calendarValue(value).split("-").map(Number);
    if (!parts.length || parts.some((part) => !Number.isFinite(part))) return Number.POSITIVE_INFINITY;
    return parts[0] * 1000000 + (parts[1] ?? 0) * 1000 + (parts[2] ?? 0);
  };
  const displayDate = (value) => calendarValue(value) || "Undated";
  for (const event of [...events].sort((left, right) => {
    const leftDate = left.startsAt || left.endsAt;
    const rightDate = right.startsAt || right.endsAt;
    return timestamp(leftDate) - timestamp(rightDate);
  })) {
    const item = document.createElement("article");
    item.className = "timeline-event";
    const date = document.createElement("time");
    const start = displayDate(event.startsAt);
    const end = event.endsAt ? displayDate(event.endsAt) : "";
    date.textContent = end && end !== start ? `${start} – ${end}` : start;
    const card = document.createElement("div");
    card.className = "timeline-card";
    const title = document.createElement("strong");
    title.textContent = event.name;
    const type = document.createElement("small");
    type.textContent = "Event";
    card.append(title, type);
    item.append(date, card);
    track.append(item);
  }
  projection.append(track);
}

async function start() {
  if (!projectId) throw new Error("Plugin project is missing");
  const bootstrap = await protocol({ op: "bootstrap", pluginId, projectId });
  sessionId = bootstrap.sessionId;
  const entities = await rpc("entity.list", {});
  const relationships = (await Promise.all(entities.map((entity) => rpc("relationship.list", { entityId: entity.id })))).flat();
  status.textContent = "Ready to explore.";
  if (pluginId === "worldbuilder.timeline") {
    const events = await Promise.all(entities
      .filter((entity) => entity.entity_type === "event")
      .map(async (entity) => {
        const fields = await rpc("field.list", { entityId: entity.id, namespace: "timeline" });
        const values = Object.fromEntries(fields.map((field) => [field.key, field.value]));
        return { ...entity, startsAt: values.startsAt, endsAt: values.endsAt };
      }));
    renderTimeline(events);
  } else {
    render(entities.filter((entity) => entity.entity_type !== "event"), relationships);
  }
}

start().catch((error) => { status.className = "error"; status.textContent = error instanceof Error ? error.message : String(error); });
