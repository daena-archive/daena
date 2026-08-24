import assert from "node:assert/strict";
import {
  SHELL_HISTORY_LIMIT,
  emptyShellNavigationHistory,
  recordShellLocation,
  sameShellLocation,
  shellHistoryBack,
  shellHistoryForward,
} from "../src/lib/navigation/history.ts";

const home = { kind: "home" };
const collection = {
  query: {
    textSearch: "",
    sortField: "updated_at",
    sortDir: "desc",
    pageSize: 25,
    page: 0,
    excludedTypes: [],
    viewMode: "grouped",
  },
  expandedGroups: [],
  scrollTop: 0,
};
const lore = {
  kind: "workspace",
  section: "lore",
  view: "library",
  entityId: null,
  writingView: "manuscripts",
  timelineView: "events",
  collection,
};
const graph = { ...lore, view: "graph" };
const settings = { kind: "settings", section: "git" };

let history = emptyShellNavigationHistory();
history = recordShellLocation(history, home);
history = recordShellLocation(history, lore);
history = recordShellLocation(history, lore);
assert.deepEqual(history.back, [home, lore], "recording the same departure twice does not add a dead history step");

const filteredLore = {
  ...lore,
  collection: {
    ...collection,
    query: { ...collection.query, textSearch: "harbor", excludedTypes: ["person"] },
    expandedGroups: ["place"],
    scrollTop: 184,
  },
};
assert.equal(
  sameShellLocation(lore, filteredLore),
  false,
  "filter and scroll context are part of a workspace location",
);

const back = shellHistoryBack(history, graph);
assert.ok(back, "Back is available after locations have been recorded");
assert.ok(sameShellLocation(back.target, lore), "Back restores the most recent departure");
assert.deepEqual(back.history.back, [home]);
assert.deepEqual(back.history.forward, [graph]);

const forward = shellHistoryForward(back.history, lore);
assert.ok(forward, "Forward is available after going back");
assert.ok(sameShellLocation(forward.target, graph), "Forward restores the location that was left by Back");
assert.deepEqual(forward.history.back, [home, lore]);
assert.deepEqual(forward.history.forward, []);

const branched = recordShellLocation(back.history, settings);
assert.deepEqual(branched.forward, [], "ordinary navigation after Back clears the forward branch");

history = emptyShellNavigationHistory();
for (let index = 0; index < SHELL_HISTORY_LIMIT + 5; index += 1) {
  history = recordShellLocation(history, { ...lore, entityId: String(index) });
}
assert.equal(history.back.length, SHELL_HISTORY_LIMIT, "history remains bounded");
assert.equal(history.back[0].entityId, "5", "bounded history retains the newest locations");

console.log("shell navigation history contracts passed");
