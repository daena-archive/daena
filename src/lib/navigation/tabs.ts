import type { WorkspaceSection } from "$lib/modules/workspace";
import type { ShellLocation, ShellNavigationHistory } from "./history.ts";
import { SHELL_HISTORY_LIMIT, emptyShellNavigationHistory } from "./history.ts";

export type TabIcon =
  | { kind: "home" }
  | { kind: "settings" }
  | { kind: "project" }
  | { kind: "section"; section: WorkspaceSection }
  | { kind: "entity"; section: WorkspaceSection; entityType: string }
  | { kind: "plugin"; key: string };

export interface ShellTab {
  id: string;
  location: ShellLocation;
  history: ShellNavigationHistory;
  pinned: boolean;
  title: string; // last known title, shown before restore
  icon: TabIcon;
  lastActiveAt: number; // ms timestamp supplied by caller
}

export interface ShellTabGroup {
  id: string;
  tabs: ShellTab[];
  activeTabId: string;
}

export interface ShellTabsState {
  version: 1;
  groups: ShellTabGroup[]; // exactly one group today; code must not assume groups.length === 1 except where noted
  activeGroupId: string;
  closed: ShellTab[]; // most recently closed LAST, at most CLOSED_TAB_LIMIT
}

export interface NewTab {
  id: string;
  location: ShellLocation;
  title: string;
  icon: TabIcon;
  history?: ShellNavigationHistory; // default emptyShellNavigationHistory()
  pinned?: boolean; // default false
}

export const TABS_STATE_VERSION = 1;
export const CLOSED_TAB_LIMIT = 10;
export const TAB_GROUP_LIMIT = 100;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function createTabsState(first: NewTab, groupId: string, now: number): ShellTabsState {
  const tab = buildTab(first, now);
  return {
    version: TABS_STATE_VERSION,
    groups: [{ id: groupId, tabs: [tab], activeTabId: tab.id }],
    activeGroupId: groupId,
    closed: [],
  };
}

export function activeGroup(state: ShellTabsState): ShellTabGroup {
  const group = state.groups.find((candidate) => candidate.id === state.activeGroupId);
  if (!group) throw new Error(`Active tab group "${state.activeGroupId}" is missing`);
  return group;
}

export function activeTab(state: ShellTabsState): ShellTab {
  const group = activeGroup(state);
  const tab = group.tabs.find((candidate) => candidate.id === group.activeTabId);
  if (!tab) throw new Error(`Active tab "${group.activeTabId}" is missing from group "${group.id}"`);
  return tab;
}

export function findTab(
  state: ShellTabsState,
  tabId: string,
): { group: ShellTabGroup; tab: ShellTab; index: number } | null {
  for (const group of state.groups) {
    const index = group.tabs.findIndex((candidate) => candidate.id === tabId);
    if (index !== -1) return { group, tab: group.tabs[index], index };
  }
  return null;
}

export function tabIdentity(location: ShellLocation): string {
  switch (location.kind) {
    case "home":
      return "home";
    case "settings":
      return "settings";
    case "project":
      return "project";
    case "plugin":
      return `plugin:${location.key}:${location.entityId ?? ""}`;
    case "workspace":
      return `workspace:${location.section}:${location.view}:${location.entityId ?? ""}`;
  }
}

export function findTabByIdentity(state: ShellTabsState, identity: string): ShellTab | null {
  let best: ShellTab | null = null;
  for (const group of state.groups) {
    for (const tab of group.tabs) {
      if (tabIdentity(tab.location) !== identity) continue;
      if (!best || tab.lastActiveAt > best.lastActiveAt) best = tab;
    }
  }
  return best;
}

export function mostRecentTabForSection(state: ShellTabsState, section: WorkspaceSection): ShellTab | null {
  let best: ShellTab | null = null;
  for (const group of state.groups) {
    for (const tab of group.tabs) {
      const location = tab.location;
      if (location.kind !== "workspace" && location.kind !== "plugin") continue;
      if (location.section !== section) continue;
      if (!best || tab.lastActiveAt > best.lastActiveAt) best = tab;
    }
  }
  return best;
}

// ---------------------------------------------------------------------------
// Mutations. Each returns a new state, or the same object when nothing changes.
// ---------------------------------------------------------------------------

export function openTab(
  state: ShellTabsState,
  tab: NewTab,
  now: number,
  options?: { activate?: boolean },
): ShellTabsState {
  if (findTab(state, tab.id)) return state;
  const activate = options?.activate ?? true;
  const group = activeGroup(state);
  const activeIndex = group.tabs.findIndex((candidate) => candidate.id === group.activeTabId);
  const pinnedCount = countPinned(group.tabs);
  const pinned = tab.pinned ?? false;
  let insertAt: number;
  if (pinned) {
    insertAt = pinnedCount;
  } else if (group.tabs[activeIndex].pinned) {
    insertAt = pinnedCount;
  } else {
    insertAt = activeIndex + 1;
  }
  const created = buildTab(tab, activate ? now : 0);
  const tabs = [...group.tabs.slice(0, insertAt), created, ...group.tabs.slice(insertAt)];
  return replaceGroup(state, {
    ...group,
    tabs,
    activeTabId: activate ? created.id : group.activeTabId,
  });
}

export function activateTab(state: ShellTabsState, tabId: string, now: number): ShellTabsState {
  const found = findTab(state, tabId);
  if (!found) return state;
  const tabs = found.group.tabs.map((tab) => (tab.id === tabId ? { ...tab, lastActiveAt: now } : tab));
  return {
    ...replaceGroup(state, { ...found.group, tabs, activeTabId: tabId }),
    activeGroupId: found.group.id,
  };
}

export function updateTab(
  state: ShellTabsState,
  tabId: string,
  patch: Partial<Pick<ShellTab, "location" | "history" | "title" | "icon">>,
): ShellTabsState {
  const found = findTab(state, tabId);
  if (!found) return state;
  const next: ShellTab = { ...found.tab };
  if (patch.location !== undefined) next.location = patch.location;
  if (patch.history !== undefined) next.history = copyHistory(patch.history);
  if (patch.title !== undefined) next.title = patch.title;
  if (patch.icon !== undefined) next.icon = patch.icon;
  const tabs = found.group.tabs.map((tab) => (tab.id === tabId ? next : tab));
  return replaceGroup(state, { ...found.group, tabs });
}

export function closeTab(state: ShellTabsState, tabId: string, now: number, replacement: NewTab): ShellTabsState {
  const found = findTab(state, tabId);
  if (!found) return state;
  const closed = pushClosed(state.closed, [found.tab]);

  if (totalTabCount(state) === 1) {
    const tab = buildTab(replacement, now);
    return {
      ...state,
      groups: [{ id: found.group.id, tabs: [tab], activeTabId: tab.id }],
      activeGroupId: found.group.id,
      closed,
    };
  }

  const { group, index } = found;
  if (group.tabs.length === 1) {
    // Other groups exist (more than one tab overall), so the empty group goes.
    const groups = state.groups.filter((candidate) => candidate.id !== group.id);
    if (state.activeGroupId !== group.id) return { ...state, groups, closed };
    const next = groups[0];
    const tabs = next.tabs.map((tab) => (tab.id === next.activeTabId ? { ...tab, lastActiveAt: now } : tab));
    return {
      ...state,
      groups: groups.map((candidate) => (candidate.id === next.id ? { ...next, tabs } : candidate)),
      activeGroupId: next.id,
      closed,
    };
  }

  const isActiveGroup = state.activeGroupId === group.id;
  let tabs = group.tabs.filter((tab) => tab.id !== tabId);
  let activeTabId = group.activeTabId;
  if (group.activeTabId === tabId) {
    const neighbour = group.tabs[index + 1] ?? group.tabs[index - 1];
    activeTabId = neighbour.id;
    if (isActiveGroup) {
      tabs = tabs.map((tab) => (tab.id === neighbour.id ? { ...tab, lastActiveAt: now } : tab));
    }
  }
  return {
    ...state,
    groups: state.groups.map((candidate) => (candidate.id === group.id ? { ...group, tabs, activeTabId } : candidate)),
    closed,
  };
}

export function closeOtherTabs(state: ShellTabsState, tabId: string, now: number): ShellTabsState {
  const found = findTab(state, tabId);
  if (!found) return state;
  const { group } = found;
  const closing = group.tabs.filter((tab) => !tab.pinned && tab.id !== tabId);
  const tabs = group.tabs
    .filter((tab) => tab.pinned || tab.id === tabId)
    .map((tab) => (tab.id === tabId ? { ...tab, lastActiveAt: now } : tab));
  return {
    ...replaceGroup(state, { ...group, tabs, activeTabId: tabId }),
    activeGroupId: group.id,
    closed: pushClosed(state.closed, closing),
  };
}

export function closeTabsToRight(state: ShellTabsState, tabId: string, now: number): ShellTabsState {
  const found = findTab(state, tabId);
  if (!found) return state;
  const { group, index } = found;
  const closing = group.tabs.filter((tab, position) => position > index && !tab.pinned);
  if (closing.length === 0) return state;

  const isActiveGroup = state.activeGroupId === group.id;
  const closesActive = closing.some((tab) => tab.id === group.activeTabId);
  const tabs = group.tabs
    .filter((tab, position) => position <= index || tab.pinned)
    .map((tab) => (isActiveGroup && closesActive && tab.id === tabId ? { ...tab, lastActiveAt: now } : tab));
  return {
    ...replaceGroup(state, { ...group, tabs, activeTabId: closesActive ? tabId : group.activeTabId }),
    closed: pushClosed(state.closed, closing),
  };
}

export function reopenClosedTab(state: ShellTabsState, now: number): ShellTabsState {
  const entry = state.closed.at(-1);
  if (!entry) return state;
  const withoutEntry: ShellTabsState = { ...state, closed: state.closed.slice(0, -1) };

  const existing = findTabByIdentity(state, tabIdentity(entry.location));
  if (existing) return activateTab(withoutEntry, existing.id, now);
  if (findTab(state, entry.id)) return withoutEntry;

  return openTab(
    withoutEntry,
    {
      id: entry.id,
      location: entry.location,
      title: entry.title,
      icon: entry.icon,
      history: entry.history,
      pinned: entry.pinned,
    },
    now,
  );
}

export function setTabPinned(state: ShellTabsState, tabId: string, pinned: boolean): ShellTabsState {
  const found = findTab(state, tabId);
  const next = Boolean(pinned);
  if (!found || found.tab.pinned === next) return state;
  const { group } = found;
  const remaining = group.tabs.filter((tab) => tab.id !== tabId);
  // Pinning goes to the end of the pinned region; unpinning to the start of the
  // unpinned region. Both are the count of pinned tabs left in the group.
  const insertAt = countPinned(remaining);
  const moved: ShellTab = { ...found.tab, pinned: next };
  const tabs = [...remaining.slice(0, insertAt), moved, ...remaining.slice(insertAt)];
  return replaceGroup(state, { ...group, tabs });
}

export function moveTab(state: ShellTabsState, tabId: string, toIndex: number): ShellTabsState {
  const found = findTab(state, tabId);
  if (!found || !Number.isFinite(toIndex)) return state;
  const { group, index, tab } = found;
  const pinnedCount = countPinned(group.tabs);
  const min = tab.pinned ? 0 : pinnedCount;
  const max = tab.pinned ? pinnedCount - 1 : group.tabs.length - 1;
  const target = Math.min(Math.max(Math.trunc(toIndex), min), max);
  if (target === index) return state;
  const tabs = group.tabs.filter((candidate) => candidate.id !== tabId);
  tabs.splice(target, 0, tab);
  return replaceGroup(state, { ...group, tabs });
}

export function activateRelativeTab(state: ShellTabsState, delta: number, now: number): ShellTabsState {
  if (!Number.isFinite(delta)) return state;
  const group = activeGroup(state);
  const count = group.tabs.length;
  const current = group.tabs.findIndex((tab) => tab.id === group.activeTabId);
  const next = (((current + Math.trunc(delta)) % count) + count) % count;
  return activateTab(state, group.tabs[next].id, now);
}

export function activateTabAt(state: ShellTabsState, index: number, now: number): ShellTabsState {
  const group = activeGroup(state);
  const resolved = index === -1 ? group.tabs.length - 1 : index;
  if (!Number.isInteger(resolved) || resolved < 0 || resolved >= group.tabs.length) return state;
  return activateTab(state, group.tabs[resolved].id, now);
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

export function tabsStorageKey(projectKey: string): string {
  return `daena:tabs:${projectKey}`;
}

export function serializeTabsState(state: ShellTabsState): string {
  return JSON.stringify(state);
}

export function parseTabsState(raw: unknown): ShellTabsState | null {
  try {
    const value = typeof raw === "string" ? JSON.parse(raw) : raw;
    return parseStateValue(value);
  } catch {
    return null;
  }
}

export function assertTabsInvariants(state: ShellTabsState): void {
  if (state.version !== TABS_STATE_VERSION) {
    throw new Error(`Tabs state version must be ${TABS_STATE_VERSION}, got ${String(state.version)}`);
  }
  if (state.groups.length === 0) throw new Error("Tabs state must have at least one group");
  if (state.closed.length > CLOSED_TAB_LIMIT) {
    throw new Error(`Closed tabs exceed the limit of ${CLOSED_TAB_LIMIT}`);
  }

  const groupIds = new Set<string>();
  const tabIds = new Set<string>();
  for (const group of state.groups) {
    if (groupIds.has(group.id)) throw new Error(`Duplicate tab group id "${group.id}"`);
    groupIds.add(group.id);
    if (group.tabs.length === 0) throw new Error(`Tab group "${group.id}" has no tabs`);
    if (!group.tabs.some((tab) => tab.id === group.activeTabId)) {
      throw new Error(`Tab group "${group.id}" active tab "${group.activeTabId}" is not in the group`);
    }
    let sawUnpinned = false;
    for (const tab of group.tabs) {
      if (tabIds.has(tab.id)) throw new Error(`Duplicate tab id "${tab.id}"`);
      tabIds.add(tab.id);
      if (!tab.pinned) {
        sawUnpinned = true;
      } else if (sawUnpinned) {
        throw new Error(`Pinned tab "${tab.id}" follows an unpinned tab in group "${group.id}"`);
      }
    }
  }
  if (!groupIds.has(state.activeGroupId)) {
    throw new Error(`Active tab group "${state.activeGroupId}" does not exist`);
  }
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

function buildTab(tab: NewTab, lastActiveAt: number): ShellTab {
  return {
    id: tab.id,
    location: tab.location,
    history: copyHistory(tab.history ?? emptyShellNavigationHistory()),
    pinned: tab.pinned ?? false,
    title: tab.title,
    icon: tab.icon,
    lastActiveAt,
  };
}

function copyHistory(history: ShellNavigationHistory): ShellNavigationHistory {
  return { back: [...history.back], forward: [...history.forward] };
}

function replaceGroup(state: ShellTabsState, group: ShellTabGroup): ShellTabsState {
  return {
    ...state,
    groups: state.groups.map((candidate) => (candidate.id === group.id ? group : candidate)),
  };
}

function countPinned(tabs: ShellTab[]): number {
  return tabs.filter((tab) => tab.pinned).length;
}

function totalTabCount(state: ShellTabsState): number {
  return state.groups.reduce((count, group) => count + group.tabs.length, 0);
}

function pushClosed(closed: ShellTab[], tabs: ShellTab[]): ShellTab[] {
  return [...closed, ...tabs].slice(-CLOSED_TAB_LIMIT);
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return false;
  const proto = Object.getPrototypeOf(value);
  return proto === Object.prototype || proto === null;
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isStringOrNull(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}

function isOptionalModuleState(value: unknown): boolean {
  return value === undefined || value === null || isPlainObject(value);
}

function isValidCollection(value: unknown): boolean {
  return (
    isPlainObject(value) &&
    isPlainObject(value.query) &&
    isStringArray(value.expandedGroups) &&
    isFiniteNumber(value.scrollTop)
  );
}

function isValidPanes(value: unknown): boolean {
  return (
    isPlainObject(value) &&
    isFiniteNumber(value.collectionWidth) &&
    isFiniteNumber(value.contentWidth) &&
    isFiniteNumber(value.inspectorWidth) &&
    isFiniteNumber(value.viewportWidth)
  );
}

function parseLocation(value: unknown): ShellLocation | null {
  if (!isPlainObject(value)) return null;
  const kind = value.kind;
  if (kind === "home") return { ...value } as ShellLocation;
  if (kind === "settings" || kind === "project") {
    return isNonEmptyString(value.section) ? ({ ...value } as ShellLocation) : null;
  }
  if (kind === "plugin") {
    const valid =
      isNonEmptyString(value.key) &&
      isNonEmptyString(value.section) &&
      isStringOrNull(value.entityId) &&
      isFiniteNumber(value.surfaceScrollTop) &&
      isOptionalModuleState(value.moduleState);
    return valid ? ({ ...value } as ShellLocation) : null;
  }
  if (kind === "workspace") {
    const valid =
      isNonEmptyString(value.section) &&
      typeof value.view === "string" &&
      isStringOrNull(value.entityId) &&
      typeof value.writingView === "string" &&
      typeof value.timelineView === "string" &&
      isFiniteNumber(value.surfaceScrollTop) &&
      isValidCollection(value.collection) &&
      isValidPanes(value.panes) &&
      isOptionalModuleState(value.moduleState);
    return valid ? ({ ...value } as ShellLocation) : null;
  }
  return null;
}

function parseIcon(value: unknown): TabIcon | null {
  if (!isPlainObject(value)) return null;
  const kind = value.kind;
  if (kind === "home" || kind === "settings" || kind === "project") return { kind };
  if (kind === "section") {
    return isNonEmptyString(value.section) ? { kind, section: value.section as WorkspaceSection } : null;
  }
  if (kind === "entity") {
    return isNonEmptyString(value.section) && typeof value.entityType === "string"
      ? { kind, section: value.section as WorkspaceSection, entityType: value.entityType }
      : null;
  }
  if (kind === "plugin") {
    return isNonEmptyString(value.key) ? { kind, key: value.key } : null;
  }
  return null;
}

function parseHistory(value: unknown): ShellNavigationHistory {
  if (!isPlainObject(value) || !Array.isArray(value.back) || !Array.isArray(value.forward)) {
    return emptyShellNavigationHistory();
  }
  return {
    back: validLocations(value.back).slice(-SHELL_HISTORY_LIMIT),
    forward: validLocations(value.forward).slice(0, SHELL_HISTORY_LIMIT),
  };
}

function validLocations(entries: unknown[]): ShellLocation[] {
  const locations: ShellLocation[] = [];
  for (const entry of entries) {
    const location = parseLocation(entry);
    if (location) locations.push(location);
  }
  return locations;
}

function parseTab(value: unknown): ShellTab | null {
  if (!isPlainObject(value) || !isNonEmptyString(value.id)) return null;
  const location = parseLocation(value.location);
  if (!location) return null;
  return {
    id: value.id,
    location,
    history: parseHistory(value.history),
    pinned: typeof value.pinned === "boolean" ? value.pinned : false,
    title: typeof value.title === "string" ? value.title : "",
    icon: parseIcon(value.icon) ?? { kind: "home" },
    lastActiveAt: isFiniteNumber(value.lastActiveAt) ? value.lastActiveAt : 0,
  };
}

function parseGroupTabs(rawTabs: unknown, acceptedTabIds: Set<string>): ShellTab[] {
  const tabs: ShellTab[] = [];
  if (!Array.isArray(rawTabs)) return tabs;
  for (const raw of rawTabs) {
    if (tabs.length >= TAB_GROUP_LIMIT) break;
    const tab = parseTab(raw);
    if (!tab || acceptedTabIds.has(tab.id)) continue;
    acceptedTabIds.add(tab.id);
    tabs.push(tab);
  }
  return tabs;
}

function partitionPinned(tabs: ShellTab[]): ShellTab[] {
  return [...tabs.filter((tab) => tab.pinned), ...tabs.filter((tab) => !tab.pinned)];
}

function parseStateValue(value: unknown): ShellTabsState | null {
  if (!isPlainObject(value) || value.version !== TABS_STATE_VERSION || !Array.isArray(value.groups)) {
    return null;
  }

  const groupIds = new Set<string>();
  // Filled as tabs are accepted, so it ends up holding exactly the open tab ids.
  const openTabIds = new Set<string>();
  const groups: ShellTabGroup[] = [];

  for (const rawGroup of value.groups) {
    if (!isPlainObject(rawGroup)) continue;
    const id = rawGroup.id;
    if (typeof id !== "string" || groupIds.has(id)) continue;
    const tabs = parseGroupTabs(rawGroup.tabs, openTabIds);
    if (tabs.length === 0) continue;
    groupIds.add(id);
    const ordered = partitionPinned(tabs);
    const rawActive = rawGroup.activeTabId;
    const activeTabId =
      typeof rawActive === "string" && ordered.some((tab) => tab.id === rawActive) ? rawActive : ordered[0].id;
    groups.push({ id, tabs: ordered, activeTabId });
  }
  if (groups.length === 0) return null;

  const rawActiveGroup = value.activeGroupId;
  const activeGroupId =
    typeof rawActiveGroup === "string" && groups.some((group) => group.id === rawActiveGroup)
      ? rawActiveGroup
      : groups[0].id;

  const closed: ShellTab[] = [];
  if (Array.isArray(value.closed)) {
    for (const raw of value.closed) {
      const tab = parseTab(raw);
      if (tab && !openTabIds.has(tab.id)) closed.push(tab);
    }
  }

  return {
    version: TABS_STATE_VERSION,
    groups,
    activeGroupId,
    closed: closed.slice(-CLOSED_TAB_LIMIT),
  };
}
