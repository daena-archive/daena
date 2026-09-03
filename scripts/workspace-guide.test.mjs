import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { languageGuideSteps } from "../packages/modules/language/src/guide.ts";
import { loreGuideSteps } from "../src/lib/guides/lore.ts";
import { timelineGuideSteps } from "../src/lib/guides/timeline.ts";
import { dismissGuide, guideDismissedKey, isGuideDismissed } from "../src/lib/guides/persist.ts";

const store = new Map();
globalThis.localStorage = {
  getItem: (key) => store.get(key) ?? null,
  setItem: (key, value) => {
    store.set(key, String(value));
  },
  removeItem: (key) => {
    store.delete(key);
  },
};

assert.equal(guideDismissedKey("language"), "daena-guide:language");
assert.equal(guideDismissedKey("language", "proj-1"), "daena-guide:language:proj-1");
assert.equal(isGuideDismissed("language"), false);
store.set("daena-language-tour-completed", "true");
assert.equal(isGuideDismissed("language"), true, "legacy language tour key still counts as dismissed");
store.clear();
dismissGuide("language");
assert.equal(isGuideDismissed("language"), true);

const empty = languageGuideSteps({ hasLanguage: false, pane: "overview", mode: "tour" });
assert.equal(empty.length, 1);
assert.equal(empty[0].id, "create");
assert.equal(empty[0].waitForTarget, true);
assert.equal(empty[0].target, '[data-guide="workspace-new"]');
assert.equal(empty[0].action, "pause");

const tour = languageGuideSteps({ hasLanguage: true, pane: "overview", mode: "tour" });
assert.deepEqual(
  tour.map((step) => step.id),
  ["overview", "add-word"],
);
assert.equal(tour[0].action, "lexicon");
assert.equal(tour[1].waitForTarget, true);
assert.equal(tour[1].action, "complete");

assert.equal(languageGuideSteps({ hasLanguage: true, pane: "lexicon", mode: "hint" })[0].id, "add-word");
assert.equal(languageGuideSteps({ hasLanguage: true, pane: "sounds", mode: "hint" })[0].id, "add-sound");
assert.equal(languageGuideSteps({ hasLanguage: true, pane: "grammar", mode: "hint" })[0].id, "starter");

const workspace = await readFile(
  new URL("../packages/modules/language/src/LanguageWorkspace.svelte", import.meta.url),
  "utf8",
);
assert.match(workspace, /WorkspaceGuide/, "language uses the shared coach");
assert.match(workspace, /Show language guide/, "help control reopens the guide");
assert.doesNotMatch(workspace, /WelcomeTour/, "slideshow tour is gone");

const emptyLore = loreGuideSteps({ hasCollection: false, hasSelection: false, view: "library", mode: "tour" });
assert.equal(emptyLore[0].id, "create");
assert.equal(emptyLore[0].action, "pause");
assert.deepEqual(
  loreGuideSteps({ hasCollection: true, hasSelection: true, view: "library", mode: "tour" }).map((step) => step.id),
  ["library", "inspector", "wiki", "graph"],
);
assert.equal(loreGuideSteps({ hasCollection: true, hasSelection: false, view: "wiki", mode: "hint" })[0].id, "wiki");

assert.equal(timelineGuideSteps({ hasCollection: false, view: "events", mode: "tour" })[0].id, "create");
assert.deepEqual(
  timelineGuideSteps({ hasCollection: true, view: "events", mode: "tour" }).map((step) => step.id),
  ["events", "calendars", "timeline"],
);
assert.equal(timelineGuideSteps({ hasCollection: true, view: "calendars", mode: "hint" })[0].id, "calendars");

const page = await readFile(new URL("../src/routes/+page.svelte", import.meta.url), "utf8");
assert.match(page, /data-guide="workspace-new"/, "New is a coach target");
assert.match(page, /HostGuide/, "host mounts lore and timeline coaches");
assert.match(page, /Show lore guide/, "lore header reopens the guide");
assert.match(page, /Show timeline guide/, "timeline header reopens the guide");

const viewNav = await readFile(new URL("../src/lib/shell/WorkspaceViewNav.svelte", import.meta.url), "utf8");
assert.match(viewNav, /data-guide=\{`workspace-view-\$\{view.id\}`\}/, "view tabs are coach targets");

const inspector = await readFile(new URL("../src/lib/shell/InspectorPane.svelte", import.meta.url), "utf8");
assert.match(inspector, /data-guide="workspace-inspector"/, "inspector is a coach target");

const lexicon = await readFile(
  new URL("../packages/modules/language/src/panes/Lexicon.svelte", import.meta.url),
  "utf8",
);
assert.match(lexicon, /data-guide="language-add-word"/, "Add word is a coach target");

const grammar = await readFile(
  new URL("../packages/modules/language/src/panes/Grammar.svelte", import.meta.url),
  "utf8",
);
assert.match(grammar, /data-guide="language-grammar-starter"/, "grammar starter is a coach target");

console.log("workspace guide checks passed");
