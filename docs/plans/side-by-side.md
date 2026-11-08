# Side-by-side tabs

Shows two tab groups at once, each with its own tabs, so a writer can keep a
manuscript open beside a character, or the wiki beside the chronology. Builds
on workspace tabs (`docs/plans/tabs.md`) and on keep-alive webviews
(`docs/plans/keep-alive-webviews.md`). The state inventory behind this plan is
in `docs/plans/side-by-side-inventory.md`.

Status: draft. Layout A chosen (the two strips split inside the topbar). Open
questions at the end.

## Layout

- **Two groups, side by side, never more.** No vertical splits, no grids.
  Each group is a pane with its own live location: workspace header,
  collection, content and inspector, or a plugin surface.
- **One topbar, split at the divider.** Back / Forward stay at the far left.
  The left group's strip spans from there to the divider; the right group's
  strip starts at the divider. Quick Open and the project status stay at the
  far right, after the right strip. With one group the topbar is unchanged.
- **Divider.** A vertical resize handle between the panes, like the
  workbench pane handles. Double-click resets it to half. Each pane has a
  minimum width (proposed 480px); below twice that plus the rail the split
  folds (see Narrow windows).
- **Focused group.** One group is focused at a time: the last one clicked or
  typed in. Its active tab gets the accent outline. Back / Forward, the rail,
  Quick Open, `⌘T`, `⌘W` and the tab shortcuts act on the focused group.
- **Workbench inside a pane.** Each pane lays out its workbench as today but
  in half the width, so the inspector and collection collapse sooner. The
  existing workbench breakpoints decide that per pane, not per window.

## Behaviour

- **Splitting.**
  - Tab context menu: "Move to other side" (creates the right group when
    there is one group and the tab is not its only tab).
  - Drag a tab onto the other strip, or onto the right half of the content
    area to create the right group.
  - "Open to the side" in the rail and row menus, and `⌥`-click (Alt-click)
    on rail items, rows, Quick Open results (`⌥↵`), wiki links and hover
    cards. It opens a new tab in the other group, creating it if needed.
  - `⌘\` (Ctrl+\\) moves the focused tab to the other side.
- **Unsplitting.** Closing the last tab of a group removes the group; the
  other group fills the window. A "Close other side" item in the tab menu
  closes every unpinned tab of the other group.
- **The same entry on both sides.** Proposed for v1: not allowed. Opening an
  entry to the side when the other group already shows it focuses that tab.
  Moving a tab, or switching tabs, so that both groups' active tabs would
  show the same entry focuses the other group's tab instead. Inactive tabs
  may show the same entry; only the two live ones may not. See Open
  questions.
- **History** stays per tab, as today.
- **Persistence.** Groups, their order, the divider position and the focused
  group are saved with the tabs (`daena:tabs:<root>`), version 2 of the
  stored state. Version 1 state loads as one group.
- **Project switch, close and window close** flush every live location, not
  only the focused one.

### Narrow windows

Below the split threshold (two minimum panes plus the rail) only the focused
group is shown, full width. The topbar shows that group's strip and a small
toggle (proposed: a two-pane icon with the other group's tab count) that
switches to the other group. Widening the window brings the split back. The
groups themselves never merge on their own.

### Shortcuts

| Action                        | macOS        | Windows / Linux |
| ----------------------------- | ------------ | --------------- |
| Move tab to the other side    | `⌘\`         | `Ctrl+\`        |
| Focus the other side          | `⌘⌥←` / `⌘⌥→` | `Ctrl+Alt+←` / `→` |
| Open to the side (click)      | `⌥`-click    | `Alt`-click     |
| Open to the side (Quick Open) | `⌥↵`         | `Alt+Enter`     |

`⌘1`–`⌘9` and `⌃Tab` act on the focused group. On macOS the new items join
the Window menu. Plugin webviews forward the new chords the same way they
forward the tab chords today (`tab-shortcuts.ts` and its checked copies).

## Approach

Today one live location fills `+page.svelte`. The inventory counts what that
means: about 300 lines of per-location state, about 2,900 lines of functions
and effects that load, save, navigate and tear it down, and about 1,950
lines of template. Side-by-side needs two of them. The order below keeps the
app working after every step, and the first steps change no behaviour.

### Location store

A `LocationStore` (`src/lib/location/`) holds everything the inventory
marks per-location: section and view, selection and load tokens, document
body, fields, revisions, dirty tracking, autosave timer and save queue,
conflicts, the selection's AI dialogs, collection query, expanded groups and
scroll, pane visibility and widths, map editor session, and the Language and
Writing bridge state for its own webview instance. Its methods are today's
page functions: `currentShellLocation`, `restoreShellLocation`,
`selectEntity`, `flushAutoSave`, `saveDocument`, the leave guards, the view
switches. Shell-global state stays on the page: project, entity cache,
modules, settings, theme, rail, dialogs, Quick Open, create dialog, toasts.

The section-keyed effects (collection query and workbench layout reload)
move into the store and key off its own section. Their localStorage keys
stay per section for the last-used defaults, but a store only reads them
when it first enters a section, so a second store cannot reset the first.

### Location view

The template chain under the topbar (settings, project center, home, host
view, wiki, tree, sandbox views, workbench) moves into a `LocationView`
component that takes a store. Each view is its own `role="tabpanel"`,
labelled by its group's active tab, which also fixes the tab panel that
today contains the topbar. Document-level ids and radio-group names inside
it (`sortField`, `pageSize`, `ai-rewrite-title`, the collection list) get a
per-view prefix. `specializedSurfaceElement` and the pane element binds
become store fields.

### Groups in the shell

`ShellTabsState` already has groups. The page keeps one store per visible
group (at most two) and points each at its group's active tab. A tab switch
inside a group runs that group's leave guards only. `shellNavigationHistory`
becomes the focused group's active tab history. `setShellEntityOpener`,
`handleHistoryKey`, `handleTabKey` and Quick Open target the focused group.
`openInNewTab` gains a target group (`"same" | "other"`), and `OpenIntent`
gains `"side"`.

`tabIdentity` dedupe already walks every group. For v1 it also refuses a
second live location on the same entry (see The same entry on both sides).

### Saving and the single writer

Each store saves its own entry with its own `loadedDocumentRevision` and
save queue. Because v1 never has the same entry live twice, two stores never
save the same document, and the Writing rule (the shell is the only writer of
an open document) holds per store. `refreshAfterEntityMutation` fans out to
every store whose selection the mutation touches, so a rename or relationship
change made on one side shows on the other. An external edit reaches each
store through the existing reload-from-disk path.

### Plugin webviews

Two panes can each show a plugin webview only once webviews have instance
labels and instance-addressed messages (`keep-alive-webviews.md`). Each
store owns at most one webview instance. Shell overlays and popovers hide
only the instances they overlap; `measureAnchoredPopovers` already measures
every `.sandbox-view-container`. Two instances of the same plugin (two
Writing documents) are fine; the same Writing document on both sides is not
(v1 rule above).

### Close and unload

`closeProject`, the window-close handshake and `beforeunload` iterate the
stores: flush each, run each leave guard, and block unload if any store is
dirty.

## Slices

1. **Location store, one caller.** Move the per-location state and functions
   into `LocationStore`; the page builds one. No visible change. All shell
   tests pass; add store unit tests for select, dirty, save, conflict and
   restore.
2. **Location view.** Move the template chain into `LocationView`; the page
   renders one. The view is the tab panel. Per-view ids. No visible change,
   except the tab panel no longer contains the topbar.
3. **Store-local slots.** Collection query, expanded groups, scroll and pane
   layout become store fields; section-keyed storage becomes a default only.
   Still one store.
4. **Two groups.** Render two views when the state has two groups; split
   topbar; divider; focus; move to other side; drag to split; open to the
   side; close-group rules; persistence v2; narrow-window fold. Shell-editor
   locations and at most one webview pane until slice 5.
5. **Webviews on both sides.** After keep-alive's instance labels: one
   webview per store, overlap hiding per instance, Writing beside Writing on
   different documents.
6. **Shortcuts and polish.** `⌘\`, focus-other-side, `⌥`-click everywhere,
   menu items, plugin key forwarding, `UI_UX.md` and `tabs.md` updates.

## Verification

- Unit: the store (select, dirty, autosave, conflict, restore), the tabs
  reducers for two groups (move, close last tab of a group, focus, storage
  v1 to v2), the open-intent `"side"` mapping.
- Real macOS app, windowed and fullscreen:
  - edit an entry on the left, switch focus right, edit another entry; both
    save; quitting saves both;
  - rename an entry on one side; the other side's tab title and links update;
  - a leave dialog on one side does not touch the other;
  - opening an entry to the side when the other side shows it focuses it;
  - narrow the window below the threshold and back; nothing is lost;
  - Writing on one side and the wiki on the other; then Writing on both
    (slice 5); overlays hide only the webview they cover;
  - restart restores both groups, the divider and the focused side.

## Open questions

1. **The same entry on both sides.** v1 proposal: not allowed, focus the
   existing tab. Later option: one shared document per entry (both sides
   edit the same buffer, like editor groups in code editors), which works
   for the shell editor but not for two Writing webviews without live
   mirroring.
2. **Narrow windows.** Fold to the focused group with a toggle (proposed), or
   stack the groups vertically?
3. **Minimum pane width.** 480px proposed.
4. **Right group creation by drag.** Drop on the right half of the content
   area (proposed), or only onto a visible drop zone at the right edge?
