import assert from "node:assert/strict";
import { breadcrumbViewLabel, shellBreadcrumbs } from "../src/lib/shell/breadcrumbs.ts";

assert.equal(breadcrumbViewLabel({ section: "lore", view: "library" }), null);
assert.equal(breadcrumbViewLabel({ section: "lore", view: "wiki" }), "Wiki");
assert.equal(breadcrumbViewLabel({ section: "lore", view: "graph" }), "Graph");
assert.equal(breadcrumbViewLabel({ section: "houses", view: "houses" }), null);
assert.equal(breadcrumbViewLabel({ section: "houses", view: "tree" }), "Tree");
assert.equal(breadcrumbViewLabel({ section: "writing", view: "manuscripts", tabLabel: "Manuscripts" }), "Manuscripts");
assert.equal(breadcrumbViewLabel({ section: "timeline", view: "timeline", tabLabel: "Events" }), null);
assert.equal(breadcrumbViewLabel({ section: "timeline", view: "events", tabLabel: "Events" }), "Events");
assert.equal(breadcrumbViewLabel({ section: "maps", view: "default" }), null);

assert.deepEqual(shellBreadcrumbs({ home: true, sectionLabel: "Lore library" }), [{ key: "home", label: "Home" }]);
assert.deepEqual(shellBreadcrumbs({ settingsLabel: "Project · Extensions", sectionLabel: "Lore library" }), [
  { key: "settings", label: "Project · Extensions" },
]);

const loreEntity = shellBreadcrumbs({
  sectionLabel: "Lore library",
  viewLabel: null,
  entityName: "Jon Doe",
});
assert.deepEqual(
  loreEntity.map((item) => item.label),
  ["Lore library", "Jon Doe"],
);

const wiki = shellBreadcrumbs({
  sectionLabel: "Lore library",
  viewLabel: "Wiki",
});
assert.deepEqual(
  wiki.map((item) => item.label),
  ["Lore library", "Wiki"],
);

const afterModuleChange = shellBreadcrumbs({
  sectionLabel: "Timeline",
  viewLabel: "Events",
  entityName: null,
});
assert.deepEqual(
  afterModuleChange.map((item) => item.label),
  ["Timeline", "Events"],
);
assert.equal(
  afterModuleChange.some((item) => item.label === "Lore library" || item.label === "Jon Doe"),
  false,
  "previous module trail is not kept",
);

const plugin = shellBreadcrumbs({
  sectionLabel: "Lore library",
  pluginLabel: "Dice · Roller",
  entityName: "Jon Doe",
});
assert.deepEqual(
  plugin.map((item) => item.label),
  ["Lore library", "Dice · Roller"],
);

console.log("shell breadcrumbs passed");
