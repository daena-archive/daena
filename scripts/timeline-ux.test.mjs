import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../packages/modules/timeline/src/index.ts", import.meta.url), "utf8");

for (const group of ['id: "events"', 'id: "lifelines"', 'id: "dates"']) {
  assert.match(source, new RegExp(group.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")), `${group} lane is declared`);
}

assert.match(source, /group:\s*groupForEvent\(event\)/, "timeline items are assigned to semantic lanes");
assert.match(source, /const groups = timelineGroups\(visible\)/, "visible lane definitions are built at render time");
assert.match(
  source,
  /new TimelineCtor\([\s\S]*?groups,\s*options,\s*\)/,
  "vis-timeline receives semantic groups before its options",
);
assert.match(source, /createLayerChip\("Lifelines", layerCounts\.lifelines/, "lifelines expose a counted layer chip");
assert.match(
  source,
  /createLayerChip\("Project dates", layerCounts\.dates/,
  "project dates expose a counted layer chip",
);
assert.match(
  source,
  /buildTypeLabelMap\(\[context\.module, \.\.\.enabledManifests\]\)/,
  "timeline labels types from effective manifests",
);
assert.match(source, /resolveEntityTypeLabel\(type, typeLabels\)/, "timeline resolves schema names for type filters");
assert.match(source, /search\.placeholder = "Name, type, place…"/, "timeline exposes contextual search");
assert.match(source, /All history/, "timeline can scope to one era or all history");
assert.match(source, /Unplaced in this era/, "era-only events stay unplaced inside an era scope");
assert.match(source, /timeline-details timeline-inspector/, "selected items render in a dedicated inspector");
assert.match(source, /timeline-lifeline \.vis-item-content::before/, "lifeline ranges expose distinct endpoints");
assert.match(
  source,
  /reportSurfaceMeta\?\.\(\{[\s\S]*calendarName/,
  "the active display calendar remains visible in the workspace topbar",
);
assert.match(
  source,
  /listLocations\(\{ entityId \}\)[\s\S]*Show on map/,
  "Show on map appears only when the event has map locations",
);

console.log("timeline UX contracts passed");
