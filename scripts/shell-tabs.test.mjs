import assert from "node:assert/strict";
import {
  TAB_GROUP_LIMIT,
  activateRelativeTab,
  activateTab,
  activateTabAt,
  activeGroup,
  activeTab,
  assertTabsInvariants,
  closeOtherTabs,
  closeTab,
  closeTabsToRight,
  createTabsState,
  discardTab,
  findTab,
  findTabByIdentity,
  mostRecentTabForSection,
  moveTab,
  openTab,
  parseTabsState,
  pruneTabs,
  reopenClosedTab,
  serializeTabsState,
  setTabPinned,
  tabIdentity,
  tabsStorageKey,
  updateTab,
} from "../src/lib/navigation/tabs.ts";
import { emptyShellNavigationHistory, recordShellLocation } from "../src/lib/navigation/history.ts";

function deepFreeze(value) {
  if (value !== null && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const key of Object.keys(value)) deepFreeze(value[key]);
  }
  return value;
}

function check(state) {
  assertTabsInvariants(state);
  return state;
}

function tabIds(state) {
  return activeGroup(state).tabs.map((tab) => tab.id);
}

function tabFor(id, location, extra = {}) {
  return { id, location, title: id, icon: homeIcon, ...extra };
}

function shellTab(id, location, extra = {}) {
  return {
    id,
    location,
    history: { back: [], forward: [] },
    pinned: false,
    title: id,
    icon: homeIcon,
    lastActiveAt: 0,
    ...extra,
  };
}

function stripOf(tabs, activeTabId, closed = []) {
  return { version: 1, groups: [{ id: "main", tabs, activeTabId }], activeGroupId: "main", closed };
}

const home = { kind: "home" };
const homeIcon = { kind: "home" };
const loreIcon = { kind: "section", section: "lore" };
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
const panes = {
  collectionWidth: 245,
  contentWidth: 640,
  inspectorWidth: 270,
  viewportWidth: 1440,
};
function loreAt(entityId, view = "library") {
  return {
    kind: "workspace",
    section: "lore",
    view,
    entityId,
    writingView: "manuscripts",
    timelineView: "events",
    collection,
    panes,
    surfaceScrollTop: 0,
  };
}
const loreAria = loreAt("aria-venn");
const loreBran = loreAt("bran-hollis");
const loreGraph = loreAt(null, "graph");
const writing = {
  kind: "workspace",
  section: "writing",
  view: "manuscripts",
  entityId: null,
  writingView: "manuscripts",
  timelineView: "events",
  collection,
  panes,
  surfaceScrollTop: 0,
};
const plugin = {
  kind: "plugin",
  key: "daena.example:main",
  section: "lore",
  entityId: null,
  surfaceScrollTop: 0,
};
const settingsGeneral = { kind: "settings", section: "general" };
const settingsAdvanced = { kind: "settings", section: "advanced" };
const project = { kind: "project", section: "snapshots" };
const replacement = tabFor("replacement", home);

// create

let state = check(createTabsState(tabFor("home", home), "main", 1000));
assert.equal(state.groups.length, 1, "create yields one group");
assert.equal(state.groups[0].tabs.length, 1, "create yields one tab");
assert.equal(activeTab(state).id, "home", "the first tab is active");
assert.equal(activeTab(state).lastActiveAt, 1000, "the first tab records its activation time");
assert.equal(state.activeGroupId, "main");

// openTab placement and activation

state = check(openTab(deepFreeze(state), tabFor("aria", loreAria), 2000));
assert.deepEqual(tabIds(state), ["home", "aria"]);
assert.equal(activeTab(state).id, "aria", "an opened tab is activated by default");
assert.equal(activeTab(state).lastActiveAt, 2000);

state = check(activateTab(deepFreeze(state), "home", 2500));
state = check(openTab(deepFreeze(state), tabFor("bran", loreBran), 3000));
assert.deepEqual(tabIds(state), ["home", "bran", "aria"], "a new tab opens directly after the active tab");
assert.equal(activeTab(state).id, "bran");

const kept = check(openTab(deepFreeze(state), tabFor("graph", loreGraph), 3500, { activate: false }));
assert.deepEqual(tabIds(kept), ["home", "bran", "graph", "aria"], "activate false still inserts after the active tab");
assert.equal(activeTab(kept).id, "bran", "activate false keeps the active tab");
assert.equal(findTab(kept, "graph").tab.lastActiveAt, 0, "an inactive new tab has lastActiveAt 0");
assert.equal(openTab(kept, tabFor("aria", loreAria), 4000), kept, "a duplicate id returns the same state");

// openTab with pinned tabs

let pinned = check(createTabsState(tabFor("home", home, { pinned: true }), "main", 1000));
pinned = check(openTab(deepFreeze(pinned), tabFor("aria", loreAria), 2000));
assert.deepEqual(
  tabIds(pinned),
  ["home", "aria"],
  "an unpinned tab opened from a pinned tab starts the unpinned region",
);
pinned = check(openTab(deepFreeze(pinned), tabFor("pin-bran", loreBran, { pinned: true }), 2100));
assert.deepEqual(tabIds(pinned), ["home", "pin-bran", "aria"], "a pinned tab goes to the end of the pinned region");
pinned = check(activateTab(deepFreeze(pinned), "home", 2200));
pinned = check(openTab(deepFreeze(pinned), tabFor("writing", writing), 2300));
assert.deepEqual(
  tabIds(pinned),
  ["home", "pin-bran", "writing", "aria"],
  "with a pinned tab active, an unpinned tab goes after the whole pinned region",
);
pinned = check(activateTab(deepFreeze(pinned), "aria", 2400));
pinned = check(openTab(deepFreeze(pinned), tabFor("pin-graph", loreGraph, { pinned: true }), 2500));
assert.deepEqual(
  tabIds(pinned),
  ["home", "pin-bran", "pin-graph", "writing", "aria"],
  "a pinned tab opened from an unpinned tab still goes to the end of the pinned region",
);

// activateTab

state = check(activateTab(deepFreeze(kept), "graph", 5000));
assert.equal(activeTab(state).id, "graph");
assert.equal(findTab(state, "graph").tab.lastActiveAt, 5000, "activation sets lastActiveAt");
assert.equal(activateTab(state, "missing", 6000), state, "an unknown tab id returns the same state");

// closeTab

const strip = stripOf(
  [shellTab("a", loreAria), shellTab("b", loreBran), shellTab("c", writing), shellTab("d", loreGraph)],
  "b",
);
let closedMiddle = check(closeTab(deepFreeze(strip), "b", 7000, replacement));
assert.deepEqual(tabIds(closedMiddle), ["a", "c", "d"]);
assert.equal(activeTab(closedMiddle).id, "c", "closing the active middle tab activates its right neighbour");
assert.equal(findTab(closedMiddle, "c").tab.lastActiveAt, 7000);
assert.deepEqual(
  closedMiddle.closed.map((tab) => tab.id),
  ["b"],
);

let closedLast = check(closeTab(deepFreeze(stripOf(strip.groups[0].tabs, "d")), "d", 7000, replacement));
assert.equal(activeTab(closedLast).id, "c", "closing the active last tab activates its left neighbour");
assert.deepEqual(
  closedLast.closed.map((tab) => tab.id),
  ["d"],
);

let closedInactive = check(closeTab(deepFreeze(strip), "d", 7000, replacement));
assert.equal(activeTab(closedInactive).id, "b", "closing an inactive tab keeps the active tab");
assert.deepEqual(tabIds(closedInactive), ["a", "b", "c"]);

const onlyTab = check(stripOf([shellTab("a", loreAria)], "a"));
const replaced = check(closeTab(deepFreeze(onlyTab), "a", 8000, replacement));
assert.deepEqual(tabIds(replaced), ["replacement"], "closing the only tab yields the replacement");
assert.equal(activeTab(replaced).id, "replacement");
assert.equal(activeTab(replaced).lastActiveAt, 8000);
assert.deepEqual(
  replaced.closed.map((tab) => tab.id),
  ["a"],
);

let capped = check(
  stripOf(
    [shellTab("keep", home), ...Array.from({ length: 12 }, (_, index) => shellTab(`t${index}`, loreAt(String(index))))],
    "keep",
  ),
);
for (let index = 0; index < 12; index += 1) {
  capped = check(closeTab(deepFreeze(capped), `t${index}`, 9000 + index, replacement));
}
assert.equal(capped.closed.length, 10, "the closed stack is capped at ten");
assert.deepEqual(
  capped.closed.map((tab) => tab.id),
  Array.from({ length: 10 }, (_, index) => `t${index + 2}`),
  "the two oldest closed tabs are dropped and the newest is last",
);
assert.equal(activeTab(capped).id, "keep", "closing inactive tabs keeps the active tab");

const twoGroups = deepFreeze({
  version: 1,
  groups: [
    { id: "main", tabs: [shellTab("a", loreAria), shellTab("b", loreBran)], activeTabId: "a" },
    { id: "side", tabs: [shellTab("c", writing)], activeTabId: "c" },
  ],
  activeGroupId: "main",
  closed: [],
});
const sideRemoved = check(closeTab(twoGroups, "c", 1000, replacement));
assert.deepEqual(
  sideRemoved.groups.map((group) => group.id),
  ["main"],
  "an emptied inactive group is removed",
);
assert.equal(sideRemoved.activeGroupId, "main");
const mainTabClosed = check(closeTab(twoGroups, "a", 1000, replacement));
assert.equal(mainTabClosed.groups.length, 2, "closing a tab that leaves its group non-empty keeps the group");
assert.equal(activeTab(mainTabClosed).id, "b");
const activeGroupEmptied = check(
  closeTab(
    deepFreeze({
      version: 1,
      groups: [
        { id: "main", tabs: [shellTab("a", loreAria)], activeTabId: "a" },
        { id: "side", tabs: [shellTab("c", writing), shellTab("d", loreGraph)], activeTabId: "c" },
      ],
      activeGroupId: "main",
      closed: [],
    }),
    "a",
    1000,
    replacement,
  ),
);
assert.deepEqual(
  activeGroupEmptied.groups.map((group) => group.id),
  ["side"],
);
assert.equal(activeGroupEmptied.activeGroupId, "side", "emptying the active group activates the first remaining group");
assert.equal(activeTab(activeGroupEmptied).id, "c");
assert.equal(activeTab(activeGroupEmptied).lastActiveAt, 1000);

// closeOtherTabs and closeTabsToRight

const pinnedStrip = stripOf(
  [
    shellTab("p1", home, { pinned: true }),
    shellTab("p2", loreAt("pinned-two"), { pinned: true }),
    shellTab("a", loreAria),
    shellTab("b", loreBran),
    shellTab("c", writing),
    shellTab("d", loreGraph),
  ],
  "c",
);
const others = check(closeOtherTabs(deepFreeze(pinnedStrip), "b", 9000));
assert.deepEqual(tabIds(others), ["p1", "p2", "b"], "close others keeps pinned tabs and the chosen tab");
assert.equal(activeTab(others).id, "b");
assert.equal(activeTab(others).lastActiveAt, 9000);
assert.deepEqual(
  others.closed.map((tab) => tab.id),
  ["a", "c", "d"],
  "close others pushes closed tabs left to right",
);

const toRight = check(closeTabsToRight(deepFreeze(pinnedStrip), "b", 9000));
assert.deepEqual(tabIds(toRight), ["p1", "p2", "a", "b"], "close to the right keeps tabs on the left and pinned tabs");
assert.equal(activeTab(toRight).id, "b", "closing the active tab to the right activates the chosen tab");
assert.deepEqual(
  toRight.closed.map((tab) => tab.id),
  ["c", "d"],
  "close to the right pushes closed tabs left to right",
);

const toRightInactive = check(closeTabsToRight(deepFreeze(stripOf(pinnedStrip.groups[0].tabs, "a")), "b", 9000));
assert.equal(activeTab(toRightInactive).id, "a", "closing to the right keeps an active tab on the left");
assert.deepEqual(
  toRightInactive.closed.map((tab) => tab.id),
  ["c", "d"],
);
assert.equal(
  closeTabsToRight(deepFreeze(pinnedStrip), "d", 9000),
  pinnedStrip,
  "nothing to the right is the same state",
);

// reopenClosedTab

const history = recordShellLocation(emptyShellNavigationHistory(), home);
const withPinned = check(
  stripOf(
    [
      shellTab("p", loreAria, { pinned: true, history, lastActiveAt: 100 }),
      shellTab("a", loreBran),
      shellTab("b", writing),
    ],
    "a",
  ),
);
const pinnedClosed = check(closeTab(deepFreeze(withPinned), "p", 1000, replacement));
const reopened = check(reopenClosedTab(deepFreeze(pinnedClosed), 2000));
assert.deepEqual(tabIds(reopened), ["p", "a", "b"], "a reopened pinned tab returns to the pinned region");
assert.equal(activeTab(reopened).id, "p", "a reopened tab is activated");
assert.equal(findTab(reopened, "p").tab.pinned, true, "reopening keeps the pinned flag");
assert.deepEqual(findTab(reopened, "p").tab.history, history, "reopening keeps the tab's history");
assert.deepEqual(reopened.closed, [], "reopening removes the entry from the stack");

const aria = stripOf([shellTab("home", home), shellTab("aria", loreAria)], "aria");
const ariaClosed = check(closeTab(deepFreeze(aria), "aria", 1000, replacement));
const ariaAgain = check(openTab(deepFreeze(ariaClosed), tabFor("aria-2", loreAria), 1500));
const ariaReopened = check(reopenClosedTab(deepFreeze(ariaAgain), 2000));
assert.deepEqual(tabIds(ariaReopened), ["home", "aria-2"], "reopening an open identity does not duplicate it");
assert.equal(activeTab(ariaReopened).id, "aria-2", "reopening an open identity activates that tab");
assert.deepEqual(ariaReopened.closed, [], "the reopened entry is dropped");

const settingsOpen = check(
  stripOf([shellTab("s1", settingsGeneral), shellTab("h", home)], "h", [shellTab("s0", settingsAdvanced)]),
);
const settingsReopened = check(reopenClosedTab(deepFreeze(settingsOpen), 3000));
assert.equal(
  activeTab(settingsReopened).id,
  "s1",
  "settings sections share one identity, so the open tab is activated",
);
assert.equal(settingsReopened.groups[0].tabs.length, 2);

const clashing = check(stripOf([shellTab("dup", home)], "dup", [shellTab("dup", plugin)]));
const clashReopened = check(reopenClosedTab(deepFreeze(clashing), 3000));
assert.deepEqual(tabIds(clashReopened), ["dup"], "a closed entry whose id is open is dropped");
assert.equal(activeTab(clashReopened).location.kind, "home", "the open tab is not replaced");
assert.deepEqual(clashReopened.closed, []);

const emptyStack = createTabsState(tabFor("home", home), "main", 1000);
assert.equal(reopenClosedTab(emptyStack, 2000), emptyStack, "an empty closed stack returns the same state");

// setTabPinned and moveTab

const regions = check(
  stripOf(
    [
      shellTab("p1", home, { pinned: true }),
      shellTab("p2", loreAt("pinned-two"), { pinned: true }),
      shellTab("a", loreAria),
      shellTab("b", loreBran),
      shellTab("c", writing),
    ],
    "a",
  ),
);
const pinB = check(setTabPinned(deepFreeze(regions), "b", true));
assert.deepEqual(tabIds(pinB), ["p1", "p2", "b", "a", "c"], "pinning goes to the end of the pinned region");
const unpinP1 = check(setTabPinned(deepFreeze(regions), "p1", false));
assert.deepEqual(tabIds(unpinP1), ["p2", "p1", "a", "b", "c"], "unpinning goes to the start of the unpinned region");
assert.equal(
  setTabPinned(deepFreeze(regions), "p1", true),
  regions,
  "pinning an already pinned tab returns the same state",
);
assert.equal(setTabPinned(regions, "missing", true), regions, "an unknown tab id returns the same state");

assert.equal(moveTab(deepFreeze(regions), "a", 0), regions, "an unpinned tab cannot move into the pinned region");
assert.deepEqual(
  tabIds(moveTab(deepFreeze(regions), "a", 99)),
  ["p1", "p2", "b", "c", "a"],
  "a move past the end clamps to the last tab",
);
assert.equal(moveTab(deepFreeze(regions), "p2", 4), regions, "a pinned tab cannot move out of the pinned region");
assert.deepEqual(
  tabIds(moveTab(deepFreeze(regions), "p1", 3)),
  ["p2", "p1", "a", "b", "c"],
  "a pinned tab stays in the pinned region",
);
assert.deepEqual(
  tabIds(moveTab(deepFreeze(regions), "c", 0)),
  ["p1", "p2", "c", "a", "b"],
  "an unpinned tab moves to the start of the unpinned region",
);
assert.deepEqual(
  tabIds(moveTab(deepFreeze(regions), "c", 3)),
  ["p1", "p2", "a", "c", "b"],
  "a move inside the unpinned region is applied",
);
assert.equal(moveTab(deepFreeze(regions), "b", 3), regions, "a move to the same index returns the same state");
assert.equal(moveTab(regions, "b", Number.NaN), regions, "a non-finite index returns the same state");

// activateRelativeTab and activateTabAt

const cycle = check(stripOf([shellTab("a", loreAria), shellTab("b", loreBran), shellTab("c", writing)], "a"));
assert.equal(
  activeTab(deepFreeze(activateRelativeTab(cycle, -1, 3000))).id,
  "c",
  "ctrl-shift-tab wraps to the last tab",
);
assert.equal(
  activeTab(activateRelativeTab(activateRelativeTab(cycle, -1, 3000), 1, 3100)).id,
  "a",
  "ctrl-tab wraps to the first tab",
);
assert.equal(activeTab(activateRelativeTab(cycle, 1, 3000)).id, "b");
assert.equal(activeTab(activateRelativeTab(cycle, -4, 3000)).id, "c", "a large negative step wraps");
assert.equal(findTab(activateRelativeTab(cycle, 1, 3000), "b").tab.lastActiveAt, 3000);

const middle = check(stripOf([shellTab("a", loreAria), shellTab("b", loreBran), shellTab("c", writing)], "b"));
assert.equal(activeTab(activateTabAt(deepFreeze(middle), 0, 4000)).id, "a", "index 0 is the first tab");
assert.equal(activeTab(activateTabAt(deepFreeze(middle), -1, 4000)).id, "c", "index -1 is the last tab");
assert.equal(activateTabAt(deepFreeze(middle), 3, 4000), middle, "an index past the end returns the same state");
assert.equal(
  activateTabAt(deepFreeze(middle), -2, 4000),
  middle,
  "a negative index other than -1 returns the same state",
);
assert.equal(activateTabAt(deepFreeze(middle), 1.5, 4000), middle, "a fractional index returns the same state");

// identity and lookups

assert.equal(tabIdentity(loreAria), "workspace:lore:library:aria-venn");
const scrolled = {
  ...loreAria,
  surfaceScrollTop: 320,
  panes: { ...panes, contentWidth: 500 },
  collection: { ...collection, scrollTop: 77, query: { ...collection.query, textSearch: "harbor" } },
};
assert.equal(tabIdentity(scrolled), tabIdentity(loreAria), "scroll, collection and pane changes keep the identity");
assert.notEqual(
  tabIdentity(loreAt("aria-venn", "graph")),
  tabIdentity(loreAria),
  "a different view is a different identity",
);
assert.notEqual(tabIdentity(loreBran), tabIdentity(loreAria), "a different entity is a different identity");
assert.equal(tabIdentity(settingsGeneral), tabIdentity(settingsAdvanced), "settings sections share an identity");
assert.notEqual(tabIdentity(settingsGeneral), tabIdentity(project), "settings and project are distinct");
assert.equal(tabIdentity(home), "home");
assert.equal(tabIdentity(plugin), "plugin:daena.example:main:");
assert.equal(tabIdentity({ ...plugin, entityId: "x" }), "plugin:daena.example:main:x");

const duplicates = stripOf(
  [
    shellTab("early", loreAria, { lastActiveAt: 100 }),
    shellTab("late", scrolled, { lastActiveAt: 300 }),
    shellTab("other", loreBran, { lastActiveAt: 900 }),
  ],
  "other",
);
assert.equal(
  findTabByIdentity(duplicates, tabIdentity(loreAria)).id,
  "late",
  "the most recently active matching tab wins",
);
assert.equal(findTabByIdentity(duplicates, "workspace:lore:library:none"), null);
const acrossGroups = deepFreeze({
  version: 1,
  groups: [
    { id: "main", tabs: [shellTab("early", loreAria, { lastActiveAt: 100 })], activeTabId: "early" },
    { id: "side", tabs: [shellTab("late", loreAria, { lastActiveAt: 300 })], activeTabId: "late" },
  ],
  activeGroupId: "main",
  closed: [],
});
assert.equal(findTabByIdentity(acrossGroups, tabIdentity(loreAria)).id, "late", "identity lookup searches every group");
const tied = stripOf(
  [shellTab("first", loreAria, { lastActiveAt: 50 }), shellTab("second", scrolled, { lastActiveAt: 50 })],
  "first",
);
assert.equal(findTabByIdentity(tied, tabIdentity(loreAria)).id, "first", "a tie keeps the earlier tab");

const sectioned = stripOf(
  [
    shellTab("home", home, { lastActiveAt: 500 }),
    shellTab("aria", loreAria, { lastActiveAt: 100 }),
    shellTab("bran", loreBran, { lastActiveAt: 300 }),
    shellTab("writing", writing, { lastActiveAt: 900 }),
    shellTab("plugin", plugin, { lastActiveAt: 700 }),
    shellTab("settings", { kind: "settings", section: "lore" }, { lastActiveAt: 999 }),
    shellTab("project", { kind: "project", section: "lore" }, { lastActiveAt: 1000 }),
  ],
  "home",
);
assert.equal(
  mostRecentTabForSection(sectioned, "lore").id,
  "plugin",
  "the most recent workspace or plugin tab of the section wins",
);
assert.equal(mostRecentTabForSection(sectioned, "writing").id, "writing");
assert.equal(mostRecentTabForSection(sectioned, "maps"), null, "no tab in the section gives null");
assert.equal(
  mostRecentTabForSection(
    stripOf([shellTab("set", { kind: "settings", section: "houses" }, { lastActiveAt: 5 }), shellTab("h", home)], "h"),
    "houses",
  ),
  null,
  "settings tabs are ignored by the section rule",
);
assert.equal(
  mostRecentTabForSection(
    stripOf([shellTab("x", loreAria, { lastActiveAt: 50 }), shellTab("y", loreBran, { lastActiveAt: 50 })], "x"),
    "lore",
  ).id,
  "x",
  "a tie keeps the earlier tab",
);

// updateTab

const renamed = check(
  updateTab(deepFreeze(stripOf([shellTab("a", loreAria, { lastActiveAt: 42 })], "a")), "a", {
    id: "zzz",
    pinned: true,
    title: "Aria Venn",
    location: loreBran,
    icon: loreIcon,
  }),
);
const renamedTab = findTab(renamed, "a").tab;
assert.equal(renamedTab.id, "a", "updateTab cannot change the id");
assert.equal(renamedTab.pinned, false, "updateTab cannot change pinned");
assert.equal(renamedTab.title, "Aria Venn");
assert.deepEqual(renamedTab.location, loreBran);
assert.deepEqual(renamedTab.icon, loreIcon);
assert.equal(renamedTab.lastActiveAt, 42, "updateTab cannot change lastActiveAt");
const untouched = deepFreeze(stripOf([shellTab("a", loreAria)], "a"));
assert.equal(updateTab(untouched, "missing", { title: "x" }), untouched, "updateTab with an unknown id is a no-op");
assert.equal(closeTab(untouched, "missing", 1, replacement), untouched, "closeTab with an unknown id is a no-op");
assert.equal(closeOtherTabs(untouched, "missing", 1), untouched, "closeOtherTabs with an unknown id is a no-op");
assert.equal(closeTabsToRight(untouched, "missing", 1), untouched, "closeTabsToRight with an unknown id is a no-op");

// each tab keeps its own history

const historyA = { back: [home], forward: [] };
const historyB = { back: [loreAria], forward: [writing] };
let histories = check(stripOf([shellTab("a", loreAria), shellTab("b", loreBran)], "a"));
histories = check(updateTab(deepFreeze(histories), "a", { history: historyA }));
histories = check(updateTab(deepFreeze(histories), "b", { history: historyB }));
histories = check(activateTab(deepFreeze(histories), "b", 100));
histories = check(activateTab(deepFreeze(histories), "a", 200));
histories = check(activateTab(deepFreeze(histories), "b", 300));
assert.deepEqual(findTab(histories, "a").tab.history, historyA, "tab a keeps its history across switches");
assert.deepEqual(findTab(histories, "b").tab.history, historyB, "tab b keeps its history across switches");
assert.notEqual(
  findTab(histories, "a").tab.history.back,
  findTab(histories, "b").tab.history.back,
  "histories are not shared",
);

const sharedHistory = { back: [home], forward: [] };
const sharedStart = check(stripOf([shellTab("base", home)], "base"));
const sharedOpened = check(openTab(deepFreeze(sharedStart), tabFor("x", loreAria, { history: sharedHistory }), 1));
const sharedBoth = check(
  openTab(deepFreeze(sharedOpened), tabFor("y", loreBran, { history: sharedHistory }), 2, { activate: false }),
);
assert.notEqual(
  findTab(sharedBoth, "x").tab.history.back,
  findTab(sharedBoth, "y").tab.history.back,
  "tabs opened with the same history object do not share arrays",
);

// persistence

assert.equal(tabsStorageKey("proj-1"), "daena:tabs:proj-1");

const full = check(
  stripOf(
    [
      shellTab("p", loreAria, {
        pinned: true,
        title: "Aria",
        icon: loreIcon,
        lastActiveAt: 10,
        history: { back: [home], forward: [loreGraph] },
      }),
      shellTab("w", writing, { lastActiveAt: 20, icon: { kind: "entity", section: "lore", entityType: "person" } }),
      shellTab("pl", plugin, { icon: { kind: "plugin", key: "daena.example:main" } }),
      shellTab("set", settingsGeneral, { icon: { kind: "settings" } }),
    ],
    "w",
    [shellTab("old", project, { icon: { kind: "project" } })],
  ),
);
const raw = serializeTabsState(full);
assert.deepEqual(parseTabsState(raw), full, "a serialized state parses back to an equal state");
assert.deepEqual(parseTabsState(JSON.parse(raw)), full, "an already parsed value is accepted");

assert.equal(parseTabsState("{not json"), null, "invalid JSON gives null");
assert.equal(parseTabsState({ ...full, version: 2 }), null, "a wrong version gives null");
assert.equal(parseTabsState(JSON.stringify({ ...full, version: 2 })), null, "a wrong version in JSON gives null");
assert.equal(parseTabsState(null), null);
assert.equal(parseTabsState("[]"), null);
assert.equal(parseTabsState(42), null);
assert.equal(parseTabsState({ ...full, groups: "nope" }), null, "groups that are not an array give null");

const goodAndBad = {
  version: 1,
  groups: [
    {
      id: "main",
      activeTabId: "a",
      tabs: [
        shellTab("a", loreAria),
        { id: "bad", location: { kind: "workspace", section: "lore" } },
        shellTab("b", loreBran),
      ],
    },
  ],
  activeGroupId: "main",
  closed: [],
};
const keptGood = check(parseTabsState(goodAndBad));
assert.deepEqual(
  keptGood.groups[0].tabs.map((tab) => tab.id),
  ["a", "b"],
  "one bad tab does not drop the good ones",
);

const duplicateTabs = {
  version: 1,
  groups: [
    {
      id: "main",
      activeTabId: "a",
      tabs: [
        shellTab("a", loreAria, { title: "first" }),
        shellTab("a", loreBran, { title: "second" }),
        shellTab("c", home),
      ],
    },
  ],
  activeGroupId: "main",
  closed: [],
};
const dedupedTabs = check(parseTabsState(duplicateTabs));
assert.deepEqual(
  dedupedTabs.groups[0].tabs.map((tab) => tab.id),
  ["a", "c"],
  "a duplicate tab id is dropped",
);
assert.equal(dedupedTabs.groups[0].tabs[0].title, "first", "the first occurrence wins");

const duplicateAcrossGroups = {
  version: 1,
  groups: [
    { id: "main", activeTabId: "a", tabs: [shellTab("a", loreAria)] },
    { id: "side", activeTabId: "a", tabs: [shellTab("a", loreBran), shellTab("c", writing)] },
    { id: "main", activeTabId: "z", tabs: [shellTab("z", home)] },
  ],
  activeGroupId: "side",
  closed: [],
};
const dedupedGroups = check(parseTabsState(duplicateAcrossGroups));
assert.deepEqual(
  dedupedGroups.groups.map((group) => group.id),
  ["main", "side"],
  "a duplicate group id is dropped",
);
assert.deepEqual(
  dedupedGroups.groups[1].tabs.map((tab) => tab.id),
  ["c"],
  "a tab id seen in an earlier group is dropped",
);
assert.equal(dedupedGroups.groups[1].activeTabId, "c", "an active tab that was dropped falls back to the first tab");

const badActive = {
  version: 1,
  groups: [
    { id: "main", activeTabId: "missing", tabs: [shellTab("a", loreAria), shellTab("b", loreBran)] },
    { id: "side", activeTabId: "c", tabs: [shellTab("c", writing)] },
  ],
  activeGroupId: "nope",
  closed: [],
};
const fixedActive = check(parseTabsState(badActive));
assert.equal(fixedActive.groups[0].activeTabId, "a", "an invalid activeTabId falls back to the first tab");
assert.equal(fixedActive.activeGroupId, "main", "an invalid activeGroupId falls back to the first group");

const unordered = {
  version: 1,
  groups: [
    { id: "main", activeTabId: "u", tabs: [shellTab("u", loreAria), shellTab("p", loreBran, { pinned: true })] },
  ],
  activeGroupId: "main",
  closed: [],
};
const partitioned = check(parseTabsState(unordered));
assert.deepEqual(
  partitioned.groups[0].tabs.map((tab) => tab.id),
  ["p", "u"],
  "pinned tabs move to the start",
);
assert.equal(partitioned.groups[0].activeTabId, "u", "a valid active tab is kept after partitioning");

const invalidHistory = {
  version: 1,
  groups: [
    {
      id: "main",
      activeTabId: "a",
      tabs: [
        {
          id: "a",
          location: loreAria,
          title: "a",
          icon: { kind: "home" },
          pinned: false,
          lastActiveAt: 0,
          history: {
            back: [
              null,
              ...Array.from({ length: 20 }, (_, index) => loreAt(String(index))),
              "nope",
              ...Array.from({ length: 25 }, (_, index) => loreAt(String(index + 20))),
            ],
            forward: [...Array.from({ length: 45 }, (_, index) => loreAt(String(index))), null],
          },
        },
      ],
    },
  ],
  activeGroupId: "main",
  closed: [],
};
const cappedHistory = check(parseTabsState(invalidHistory));
const cappedTab = cappedHistory.groups[0].tabs[0];
assert.equal(cappedTab.history.back.length, 40, "back keeps the last forty valid entries");
assert.equal(cappedTab.history.back[0].entityId, "5", "back drops the oldest entries first");
assert.equal(cappedTab.history.back.at(-1).entityId, "44");
assert.equal(cappedTab.history.forward.length, 40, "forward keeps the first forty valid entries");
assert.equal(cappedTab.history.forward[0].entityId, "0");
assert.equal(cappedTab.history.forward.at(-1).entityId, "39");

const badIcon = {
  version: 1,
  groups: [{ id: "main", activeTabId: "a", tabs: [{ ...shellTab("a", loreAria), icon: { kind: "bogus" } }] }],
  activeGroupId: "main",
  closed: [],
};
assert.deepEqual(
  check(parseTabsState(badIcon)).groups[0].tabs[0].icon,
  { kind: "home" },
  "an invalid icon falls back to home",
);

// discardTab

const discardStrip = deepFreeze(
  stripOf(
    [
      shellTab("a", loreAria, { lastActiveAt: 1 }),
      shellTab("b", loreBran, { lastActiveAt: 2 }),
      shellTab("c", writing, { lastActiveAt: 3 }),
    ],
    "b",
    [shellTab("old", home)],
  ),
);
assert.equal(discardTab(discardStrip, "missing"), discardStrip, "discarding an unknown id returns the same state");
const onlyDiscard = deepFreeze(stripOf([shellTab("a", loreAria)], "a", [shellTab("old", home)]));
assert.equal(discardTab(onlyDiscard, "a"), onlyDiscard, "discarding the last tab returns the same state");

const discardedMiddle = check(discardTab(discardStrip, "b"));
assert.deepEqual(tabIds(discardedMiddle), ["a", "c"]);
assert.equal(activeTab(discardedMiddle).id, "c", "discarding the active middle tab activates its right neighbour");
assert.equal(findTab(discardedMiddle, "c").tab.lastActiveAt, 3, "discard does not stamp a new activation time");
assert.equal(discardedMiddle.closed, discardStrip.closed, "discard does not touch the closed stack");

const discardedLast = check(discardTab(deepFreeze(stripOf(discardStrip.groups[0].tabs, "c")), "c"));
assert.equal(activeTab(discardedLast).id, "b", "discarding the active last tab activates its left neighbour");
assert.deepEqual(discardedLast.closed, []);

const discardedInactive = check(discardTab(discardStrip, "a"));
assert.equal(activeTab(discardedInactive).id, "b", "discarding an inactive tab keeps the active tab");
assert.deepEqual(tabIds(discardedInactive), ["b", "c"]);
assert.equal(discardedInactive.closed, discardStrip.closed);

const discardGroups = deepFreeze({
  version: 1,
  groups: [
    { id: "main", tabs: [shellTab("a", loreAria, { lastActiveAt: 4 })], activeTabId: "a" },
    {
      id: "side",
      tabs: [shellTab("c", writing, { lastActiveAt: 8 }), shellTab("d", loreGraph, { lastActiveAt: 9 })],
      activeTabId: "c",
    },
  ],
  activeGroupId: "main",
  closed: [shellTab("old", home)],
});
const discardedSide = check(discardTab(discardGroups, "c"));
assert.equal(discardedSide.groups.length, 2, "discarding a tab that leaves its group non-empty keeps the group");
assert.equal(discardedSide.groups[1].activeTabId, "d", "an inactive group's active tab hands over to its neighbour");
assert.equal(discardedSide.activeGroupId, "main");
assert.equal(discardedSide.closed, discardGroups.closed);

const discardedActiveGroup = check(discardTab(discardGroups, "a"));
assert.deepEqual(
  discardedActiveGroup.groups.map((group) => group.id),
  ["side"],
  "discarding the only tab of the active group removes that group",
);
assert.equal(discardedActiveGroup.activeGroupId, "side");
assert.equal(activeTab(discardedActiveGroup).id, "c");
assert.equal(activeTab(discardedActiveGroup).lastActiveAt, 8, "discard does not stamp the group it activates");
assert.equal(discardedActiveGroup.closed, discardGroups.closed);

const discardedInactiveGroup = check(
  discardTab(
    deepFreeze({
      version: 1,
      groups: [
        { id: "main", tabs: [shellTab("a", loreAria), shellTab("b", loreBran)], activeTabId: "a" },
        { id: "side", tabs: [shellTab("c", writing)], activeTabId: "c" },
      ],
      activeGroupId: "main",
      closed: [],
    }),
    "c",
  ),
);
assert.deepEqual(
  discardedInactiveGroup.groups.map((group) => group.id),
  ["main"],
  "discarding the only tab of an inactive group removes that group",
);
assert.equal(activeTab(discardedInactiveGroup).id, "a");

// pruneTabs

const pruneBase = deepFreeze(
  stripOf(
    [
      shellTab("a", loreAria, { lastActiveAt: 1 }),
      shellTab("b", loreBran, { lastActiveAt: 2 }),
      shellTab("c", writing, { lastActiveAt: 3 }),
    ],
    "b",
    [shellTab("old", plugin), shellTab("keep-closed", home)],
  ),
);
assert.equal(
  pruneTabs(pruneBase, () => true, replacement, 50),
  pruneBase,
  "pruning nothing returns the same state",
);

const prunedMiddle = check(pruneTabs(pruneBase, (tab) => tab.id !== "b" && tab.id !== "old", replacement, 50));
assert.deepEqual(tabIds(prunedMiddle), ["a", "c"]);
assert.equal(activeTab(prunedMiddle).id, "c", "pruning the active tab activates the right kept neighbour");
assert.equal(activeTab(prunedMiddle).lastActiveAt, 50, "pruning the active tab stamps the neighbour");
assert.deepEqual(
  prunedMiddle.closed.map((tab) => tab.id),
  ["keep-closed"],
  "pruned closed entries go away and dropped open tabs are not pushed onto closed",
);

const prunedRight = check(pruneTabs(pruneBase, (tab) => tab.id === "a" || tab.id === "keep-closed", replacement, 60));
assert.equal(activeTab(prunedRight).id, "a", "when the right neighbour is also dropped, the left one activates");
assert.equal(activeTab(prunedRight).lastActiveAt, 60);
assert.deepEqual(
  prunedRight.closed.map((tab) => tab.id),
  ["keep-closed"],
);

const prunedNone = check(pruneTabs(pruneBase, () => false, tabFor("fresh", home), 70));
assert.deepEqual(tabIds(prunedNone), ["fresh"], "pruning every tab leaves the replacement");
assert.equal(activeTab(prunedNone).lastActiveAt, 70);
assert.equal(prunedNone.activeGroupId, "main", "the replacement keeps the active group id");
assert.deepEqual(prunedNone.closed, [], "a full prune also clears rejected closed tabs");

const pruneGroups = deepFreeze({
  version: 1,
  groups: [
    { id: "main", tabs: [shellTab("a", loreAria, { lastActiveAt: 1 })], activeTabId: "a" },
    {
      id: "side",
      tabs: [shellTab("c", writing, { lastActiveAt: 2 }), shellTab("d", loreBran, { lastActiveAt: 3 })],
      activeTabId: "c",
    },
  ],
  activeGroupId: "main",
  closed: [shellTab("old", home)],
});
const prunedActiveGroup = check(pruneTabs(pruneGroups, (tab) => tab.id !== "a", replacement, 80));
assert.deepEqual(
  prunedActiveGroup.groups.map((group) => group.id),
  ["side"],
  "an emptied group is removed",
);
assert.equal(prunedActiveGroup.activeGroupId, "side", "emptying the active group activates the first remaining group");
assert.equal(activeTab(prunedActiveGroup).id, "c");
assert.equal(activeTab(prunedActiveGroup).lastActiveAt, 80);
assert.equal(prunedActiveGroup.closed.length, 1, "a kept closed tab stays");

const prunedSideActive = check(pruneTabs(pruneGroups, (tab) => tab.id !== "c", replacement, 90));
assert.equal(prunedSideActive.activeGroupId, "main");
assert.equal(activeTab(prunedSideActive).id, "a");
assert.equal(activeTab(prunedSideActive).lastActiveAt, 1, "keeping the global active tab does not restamp it");
const prunedSide = prunedSideActive.groups.find((group) => group.id === "side");
assert.equal(prunedSide.activeTabId, "d", "a background group activates the neighbour of its dropped active tab");
assert.equal(
  prunedSide.tabs.find((tab) => tab.id === "d").lastActiveAt,
  3,
  "a background group does not get a new activation time",
);

const manyClosed = Array.from({ length: 12 }, (_, index) => shellTab(`old-${index}`, loreAt(String(index))));
const closedFiltered = {
  version: 1,
  groups: [{ id: "main", activeTabId: "a", tabs: [shellTab("a", loreAria)] }],
  activeGroupId: "main",
  closed: [shellTab("a", loreBran), ...manyClosed, null, { id: "" }],
};
const keptClosed = check(parseTabsState(closedFiltered));
assert.deepEqual(
  keptClosed.closed.map((tab) => tab.id),
  Array.from({ length: 10 }, (_, index) => `old-${index + 2}`),
  "closed entries clashing with open ids and invalid entries are dropped, and the last ten are kept",
);
assert.deepEqual(
  check(parseTabsState({ ...full, closed: undefined })).closed,
  [],
  "missing closed gives an empty stack",
);

const bigGroup = {
  version: 1,
  groups: [
    {
      id: "main",
      activeTabId: "t0",
      tabs: Array.from({ length: TAB_GROUP_LIMIT + 5 }, (_, index) => shellTab(`t${index}`, home)),
    },
  ],
  activeGroupId: "main",
  closed: [],
};
assert.equal(
  check(parseTabsState(bigGroup)).groups[0].tabs.length,
  TAB_GROUP_LIMIT,
  "a group keeps at most TAB_GROUP_LIMIT tabs",
);

assert.equal(
  parseTabsState({
    version: 1,
    groups: [{ id: "g", tabs: [{ id: "", location: home }] }],
    activeGroupId: "g",
    closed: [],
  }),
  null,
  "a state with no valid tab gives null",
);
assert.equal(
  parseTabsState({ version: 1, groups: [], activeGroupId: "g", closed: [] }),
  null,
  "a state with no groups gives null",
);
assert.equal(
  parseTabsState({ version: 1, groups: [{ id: 7, tabs: [shellTab("a", home)] }], activeGroupId: "7", closed: [] }),
  null,
  "a state whose only group has a non-string id gives null",
);

const messy = deepFreeze({
  version: 1,
  groups: [
    {
      id: "main",
      activeTabId: "nope",
      tabs: [
        { id: "u", location: home, title: "u", icon: { kind: "bogus" } },
        { id: "p", location: loreAria, title: "p", pinned: true, history: { back: [null, loreBran] } },
      ],
    },
  ],
  activeGroupId: "nope",
  closed: [],
});
const messyBefore = JSON.stringify(messy);
const messyParsed = parseTabsState(messy);
assert.notEqual(messyParsed, null, "a messy state still parses");
assert.equal(JSON.stringify(messy), messyBefore, "parsing does not change the input");
check(messyParsed);
assert.deepEqual(
  messyParsed.groups[0].tabs.map((tab) => tab.id),
  ["p", "u"],
);
assert.equal(
  messyParsed.groups[0].activeTabId,
  "p",
  "an invalid active tab falls back to the first tab after partitioning",
);
assert.deepEqual(
  messyParsed.groups[0].tabs[0].history,
  { back: [], forward: [] },
  "a history without a forward array is empty",
);

console.log("shell tabs contracts ok");
