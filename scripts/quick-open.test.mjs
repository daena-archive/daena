import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { groupQuickOpenItems, moveQuickOpenIndex, rankQuickOpenItems } from "../src/lib/quick-open/model.ts";

const items = [
  {
    id: "destination:lore",
    category: "Destinations",
    label: "Lore",
    description: "World reference workspace",
    keywords: ["library"],
    action: { kind: "destination", destination: "navigation:lore" },
  },
  {
    id: "entity:1",
    category: "Results",
    label: "Lorelai",
    description: "Person",
    action: { kind: "entity", entityId: "1" },
  },
  {
    id: "command:settings",
    category: "Commands",
    label: "Open Settings",
    description: "Application preferences",
    action: { kind: "command", command: "settings" },
  },
];

assert.deepEqual(
  rankQuickOpenItems(items, "lore").map((item) => item.id),
  ["destination:lore", "entity:1"],
  "label prefixes rank ahead of partial matches",
);
assert.equal(moveQuickOpenIndex(0, -1, 3), 2, "Arrow Up wraps to the final result");
assert.equal(moveQuickOpenIndex(2, 1, 3), 0, "Arrow Down wraps to the first result");
assert.equal(moveQuickOpenIndex(0, 1, 0), -1, "empty result sets have no active item");
assert.deepEqual(
  groupQuickOpenItems(items).map((group) => group.category),
  ["Results", "Destinations", "Commands"],
  "groups retain the product-defined order",
);

const shell = await readFile(new URL("../src/routes/+page.svelte", import.meta.url), "utf8");
const quickOpen = await readFile(new URL("../src/lib/shell/QuickOpen.svelte", import.meta.url), "utf8");
const toolbar = await readFile(new URL("../src/lib/shell/GlobalToolbar.svelte", import.meta.url), "utf8");

assert.match(toolbar, /Quick Open/, "the toolbar exposes Quick Open as a first-class action");
assert.match(shell, /event\.key\.toLowerCase\(\)/, "global shortcuts normalize keyboard input");
assert.match(shell, /key === "k"/, "Cmd or Ctrl K opens Quick Open");
assert.match(shell, /key === "n"/, "Cmd or Ctrl N opens contextual creation");
assert.match(quickOpen, /ArrowDown/, "Quick Open supports arrow-key navigation");
assert.match(quickOpen, /aria-activedescendant/, "Quick Open exposes its active option accessibly");
assert.match(quickOpen, /trapModalTab/, "Quick Open traps focus while open");
assert.match(shell, /createDialogView/, "creation uses one progressive dialog state");
assert.match(shell, /create-template-tiles/, "the creation menu presents templates as tiles");
assert.match(shell, /handleCreateDialogKeydown/, "the template gallery supports arrow navigation and tab trapping");
assert.match(shell, /"ArrowLeft", "ArrowRight"/, "tile navigation supports both axes");
assert.match(shell, /requiredCreateFields/, "focused creation derives required fields from the template");
assert.match(shell, /optionalCreateFields/, "focused creation derives optional details separately");
assert.match(
  shell,
  /aria-expanded=\{createMoreDetailsOpen\}/,
  "optional fields use an accessible collapsed disclosure",
);
assert.match(shell, /openFocusedCreate\(option\.key\)/, "template tiles open focused creation directly");
assert.doesNotMatch(
  shell,
  /showQuickCreate|requiresGuidedCreation/,
  "creation no longer branches into quick and guided modes",
);
assert.match(shell, /createWithOption/, "all creation routes share one commit path");
assert.match(shell, /context\.entities\.create/, "creation retains the module context atomic write boundary");
assert.match(shell, /returnFocus\?\.focus\(\)/, "creation restores focus when it closes");
assert.match(shell, /collectionQuery\.textSearch/, "collection-scoped search remains separate from Quick Open");
assert.match(shell, /project\.search\(term\)/, "Quick Open project results use the canonical search service");

console.log("Quick Open and creation contracts passed");
