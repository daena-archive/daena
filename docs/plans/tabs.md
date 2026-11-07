# Workspace tabs

Lets the user keep several places open at once: workspaces, entities, plugin
views and settings, each in its own tab, managed from the topbar. Side-by-side
viewing comes later; this plan keeps its data model ready for it but does not
build it.

Status: slice 1 (chrome restyle) done and checked in the real app; slice 2
(tab model, `src/lib/navigation/tabs.ts`) done, not yet wired into the shell. Layout option A was chosen (tabs replace
the breadcrumb in the topbar, workspace header compacted). All open decisions are settled (see Decisions).

## Layout

The chrome above the panes is about 290px today: topbar (58px), the workspace
view row (Library / Wiki / Graph), then the large `WorkspaceHeader` (overline,
serif title, description). A separate tab strip would add another row, and the
breadcrumb would repeat the tab title. So the restyle is part of this work.

- **Topbar, one row:** Back / Forward, the tab strip, then a compact Quick Open
  (icon and `⌘K`) and the project status as an icon with its tooltip. The
  breadcrumb goes: the tab title carries the entity or plugin name, and the
  view is shown by the header's view switcher.
- **Workspace header, one row:** overline and serif title on the left, the view
  switcher (`WorkspaceViewNav`) as a segmented control beside them, then the
  guide button, Layout menu and New on the right. The description moves into
  the guide or a tooltip. This frees about 130px on every workspace.
- **Unchanged:** the left rail, the three-pane workbench, the inspector, and
  the native title bar. Tabs in the macOS title bar (overlay title bar beside
  the traffic lights) is deferred.
- **Narrow windows** (the existing 760px breakpoint): the strip collapses to one
  button showing the active tab's title, which opens the tab list.

### Tab anatomy

- Icon and title. Entity tabs use the entity type glyph and colour
  (`EntityGlyph`); workspace and plugin tabs use the rail icon. Titles truncate
  with a tooltip for the full name.
- The active tab is filled. Close (×) shows on the active and hovered tabs;
  middle-click also closes.
- Pinned tabs show the icon only, sit at the left, and are skipped by Close
  others and Close to the right.
- When tabs no longer fit at their minimum width, the strip scrolls and a
  chevron opens a list of all tabs.
- The `+` button and `⌘T` open a new tab on Home.
- Context menu: Duplicate tab (disabled until side-by-side, see below), Pin /
  Unpin, Close, Close others, Close to the right, Reopen closed tab.
- No unsaved-changes dot on inactive tabs: switching away flushes or asks, so
  only the active tab can be dirty, and it already shows the shell's save
  state.

## Behaviour

- **A tab is a location plus its own history.** Back / Forward move within the
  active tab. Each tab keeps its own back and forward stacks.
- **Navigation stays in the current tab** (browser model). A new tab opens only
  when asked:
  - `⌘`-click (Ctrl-click elsewhere) or middle-click on rail items, collection
    rows, Quick Open results, wiki links, graph nodes and hover cards;
  - `⌘↵` in Quick Open;
  - "Open in new tab" in entity context menus and the hover card;
  - in rich-text editors, `⌘`-click already follows a reference in the current
    tab, so a new tab is `⌘⇧`-click.
- **Any number of tabs per plugin.** Two Writing tabs on different
  manuscripts, or Lore library next to Lore graph, are fine: an inactive tab is
  only a saved location. Only the active tab is live, so switching between two
  tabs of the same plugin reloads its webview with the other tab's state.
- **One tab per entity view.** Opening something that is already open in a tab
  switches to that tab. Duplicates arrive with side-by-side.
- **Left rail (plain click):** if a tab for that workspace is already open,
  switch to it (the most recently used one if there are several), and leave
  the current tab alone. Otherwise the current tab navigates there, like any
  other click. `⌘`-click always opens a new tab.
- **Settings and the project center** open in their own tab, or switch to it if
  it is already open, like a singleton.
- **No preview tabs** in v1. A single click does not open a
  temporary tab that the next click replaces.
- **Closing:** the active tab hands over to its right neighbour, or the left one
  if it was last. Closing the last tab leaves one Home tab; there are never
  zero tabs. Closed tabs go on a stack of 10 for Reopen closed tab.
- **Archived or deleted entities:** tabs pointing to an entity archived in this
  session close and go on the closed stack. A tab restored later whose entity
  is gone falls back to its workspace, the same way history restore does now.
- **Disabled plugins:** tabs for a plugin that is disabled or uninstalled close.
- **Per project:** each project keeps its own tabs. Opening a project restores
  its tabs and activates the last active one. Inactive tabs are not restored
  until they are opened.

### Shortcuts

| Action                   | macOS            | Windows / Linux      |
| ------------------------ | ---------------- | -------------------- |
| New tab                  | `⌘T`             | `Ctrl+T`             |
| Close tab                | `⌘W`             | `Ctrl+W`             |
| Reopen closed tab        | `⇧⌘T`            | `Ctrl+Shift+T`       |
| Next / previous tab      | `⌃Tab` / `⌃⇧Tab` | `Ctrl+Tab` / `+Shift`|
| Go to tab 1–8, last tab  | `⌘1`–`⌘8`, `⌘9`  | `Ctrl+1`–`8`, `9`    |
| Back / Forward (in tab)  | `⌥←` / `⌥→`      | `Alt+←` / `Alt+→`    |

`⌘W` closes the window today through the menu that `app_menu.rs` builds as a
copy of `Menu::default`. Close Window moves to `⇧⌘W`.

## Approach

### Only the active tab is live

`+page.svelte` holds one live location: one `section`, one `selected` entity,
one set of fields and autosave state, and one embedded plugin webview. Writing
also depends on that: the shell is the only writer of the open document. Making
several locations live at once means taking that state out of the page. That
is the real work for side-by-side, and tabs do not need it.

So in v1 an inactive tab is just a saved `ShellLocation`. Switching tabs works
like Back / Forward does now:

1. dismiss transient menus;
2. `flushAutoSave()`, then the leave guards (`canLeaveLanguageSection`,
   `resolveDirtyMapSession`, the Writing flush). A cancel keeps the current
   tab;
3. capture `currentShellLocation()` into the outgoing tab;
4. `restoreShellLocation(target)`, then swap in the target tab's history.

`ShellLocation` already records section, view, entity, collection query and
scroll, pane widths, surface scroll and module state, and
`restoreShellLocation` already rebuilds from it. Tabs reuse both; restore is
not rewritten.

Cost: returning to a Writing, Language, graph or chronology tab reloads its
webview, as navigation does today. Undo history inside the webview and its
in-editor scroll are lost; drafts are not, because leaving flushes them. If the
reload feels slow, a keep-alive pool (below) is the fix.

### Data model

New pure module `src/lib/navigation/tabs.ts`, next to `history.ts`:

```ts
interface ShellTab {
  id: string;
  location: ShellLocation;
  history: ShellNavigationHistory;
  pinned: boolean;
  // Last known title and icon, shown before the location is restored.
  title: string;
  icon: TabIcon;
  lastActiveAt: number;
}

interface ShellTabGroup {
  id: string;
  tabs: ShellTab[];
  activeTabId: string;
}

interface ShellTabsState {
  version: 1;
  groups: ShellTabGroup[]; // one group until side-by-side
  activeGroupId: string;
  closed: ShellTab[]; // most recent last, at most 10
}
```

Pure functions, with no Svelte or IPC: `openTab`, `activateTab`, `closeTab`,
`closeOthers`, `closeToRight`, `reopenClosed`, `pinTab`, `moveTab`,
`findTabFor(identity)`, `tabIdentity(location)`, `tabTitle(location, lookup)`,
and `parseTabsState(raw)`, which validates stored state and drops malformed
tabs.

`tabIdentity` decides "already open": kind, section, plugin key and entity id,
ignoring scroll, collection query and pane widths.

Groups exist so that side-by-side only adds a second group; one is enough for
now.

### Shell wiring

- `shellNavigationHistory` becomes the active tab's `history`.
  `recordShellLocation`, `shellHistoryBack` and `shellHistoryForward` are
  unchanged and run on that tab.
- Every navigation entry point that can open a new tab (`selectEntity`,
  `openSidebarNavigationItem`, `openPluginView`, `selectQuickOpenItem`,
  `openEntityById`, `openSettings`, `openProjectCenter`) gets a
  `{ target: "current" | "new-tab" }` option. With `new-tab`, the shell creates
  the tab and switches to it through the normal tab switch, then navigates.
- After each successful navigation, the shell updates the active tab's
  `location`, `title` and `icon`. Identity edits (rename, type change) refresh
  the titles of other tabs from the entity cache.
- `GlobalToolbar` takes a `tabs` snippet in place of `breadcrumbs`. The
  breadcrumb helpers become the tab title helpers (`breadcrumbs.ts` →
  `tabTitle`).
- Tabs are saved to `localStorage` per project, debounced, like the workbench
  layout. They are local UI state and never go into project files.

### Plugin webviews

- v1 needs no Rust changes: one embedded webview at a time, rebuilt on mount.
- Plugins asking the shell to open an entity (the Language and Writing
  `open-entity` state messages, `EditorHost.openEntity`) get an optional
  `newTab` flag. This changes the plugin protocol, so the plugin contract and
  its drift checks need regenerating.
- The tab context menu and overflow list are anchored popovers under the
  topbar, over the content area. They register with the popover-overlap
  hiding (`popover-overlap.ts`), so they hide a plugin webview only when they
  cover it.
- The topbar gets shorter, which moves every webview placeholder.
  `SandboxView` already re-syncs when its placeholder moves; check it in the
  real app, windowed and fullscreen.

### Shortcuts while a plugin webview has focus

Key events inside a plugin webview never reach the main webview. On macOS the
tab shortcuts are native menu items in `app_menu.rs` (a Tab section in the
File menu), which fire whatever webview has focus, and send a menu event to the
shell. Windows and Linux have no menu today. There the plugin webview runtime
forwards those key chords to the shell as a host message, and the main webview
handles them in its own keydown handler like `⌘K`.

### Accessibility

- The strip is a `tablist` whose tabs control the main region.
- Activation is manual: arrow keys, Home and End move focus, and Enter or Space
  activates. Activation can open a leave dialog, so focus alone must not
  switch tabs.
- Delete closes the focused tab. The × buttons are outside the tab order and
  have labels ("Close Aria Venn").
- `⇧F10` or the context-menu key opens the tab menu.
- Pinned tabs keep their full title as the accessible name.

## Slices

1. **Chrome restyle, no tabs.** Compact `WorkspaceHeader` with the view
   switcher in it, the description in the guide, the compact topbar. Ships on
   its own and settles the look. Update `docs/ui/UI_UX.md`.
2. **Tab model.** `tabs.ts` with pure functions and persistence parsing, plus
   `scripts/shell-tabs.test.mjs` (open, close, reopen, pin, move, dedupe,
   malformed storage, the history-per-tab hand-off).
3. **One-tab wiring.** The shell runs on the tab store with one tab: history
   moves into the tab, and the tab strip replaces the breadcrumb. Nothing new
   for the user yet; existing navigation and history tests must still pass.
4. **Multiple tabs.** New, close, switch through the leave guards, the `+`
   button, open-in-new-tab entry points, the left-rail rule, singleton
   settings, titles and icons, per-project restore.
5. **Management.** Context menu, pin, drag and keyboard reorder, overflow list,
   reopen closed tab, narrow-window layout, popover-overlap registration.
6. **Shortcuts.** Native menu items on macOS; key forwarding from plugin
   webviews on other platforms; `newTab` in `open-entity`.

### Later

- **Keep-alive webviews.** Keep the last few plugin webviews hidden instead of
  closing them, so returning to a Writing or Language tab is instant and keeps
  its undo history. Needs per-instance labels (`plugin:<id>:<instance>`) in
  `plugin_webview.rs`, host messages addressed to an instance, and an LRU that
  flushes before evicting. Broker sessions are already per origin.
- **Side-by-side.** Two tab groups visible at once, each with its own strip.
  Needs more than one live location, so the selected entity, fields, autosave
  and document state come out of `+page.svelte` into per-location stores.
  Writing's single-writer rule then applies per document, not per shell.
  Duplicate tab arrives here. This gets its own plan.

## Verification

- Unit: the `tabs.ts` reducers and storage parsing; `navigation-history` and
  `shell-breadcrumbs` tests updated.
- Real macOS app, windowed and fullscreen:
  - switching away from a Writing tab mid-typing saves the draft;
  - cancelling a Language or map leave dialog keeps the current tab;
  - Back and Forward stay within their tab;
  - webview bounds are right after the topbar change and while switching tabs;
  - the overflow list and context menu hide a webview only when they cover it;
  - `⌘W` with focus in the Writing editor closes the tab, not the window;
  - quitting and reopening the project restores its tabs;
  - renaming an entity updates every tab that shows it;
  - archiving an entity closes its tabs.
- Windows and Linux: shortcuts with focus in a plugin webview.

## Decisions

1. A plain click on a left-rail workspace switches to an already-open tab for
   it (the most recently used one), and only navigates the current tab when
   none is open. `⌘`-click opens a new tab.
2. No preview tabs in v1.
3. Native title bar unchanged in v1; tabs live in the topbar.
4. In rich-text editors `⌘`-click keeps following a reference in the current
   tab; `⌘⇧`-click opens it in a new tab.
5. Settings and the project center are singleton tabs.
