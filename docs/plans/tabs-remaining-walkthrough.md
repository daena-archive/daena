# Tabs: walkthrough for the remaining work

A self-contained brief for an agent to finish the workspace-tabs feature on
its own: keep-alive webviews (slices 3 and 4) and side-by-side tabs (slices 1
to 6). The owner reviews and merges the branch afterwards, so the work must be
reviewable one commit at a time, and every commit must leave the app building
and the checks green.

Read this whole file before you start. Where it disagrees with an older plan
document, this file wins. Update the older document to match what you build.

## 0. Ground rules

- **Branch.** Start from `main` at or after `125dfb4` ("Plugin instance labels
  and routing for embedded webviews"). Create `tabs-remaining`:

  ```bash
  rtk git switch -c tabs-remaining
  ```

  Commit there, one commit per step below, with messages in the style of the
  history (`Keep-alive webviews, slice 3: pool Writing webviews.`). Never
  commit to `main`, never push, never merge, never rebase `main`.
- **Leave alone:** the World engine commits and files (`9db8aa3` and its
  parents), and the untracked `docs/plans/world-engine-handoff.md`. Never stage
  that file. Stage paths explicitly (`git add <paths>`), not `git add -A`.
- **First commit:** this walkthrough file itself, unchanged, so the review can
  diff your report against it.
- **Repo rules** (`AGENTS.md`): prefix every shell command with `rtk`. Prefer
  the codebase-memory graph tools (`search_graph`, `trace_path`,
  `get_code_snippet`) for code discovery, and `rg` for literals.
  `src/routes/+page.svelte` is about 14,000 lines: never read it whole; find
  functions with `rg -n` or the graph and read those ranges.
- **Formatting:** run `rtk npx prettier --write` on every changed TS, JS and
  Svelte file. Docs (`.md`) are not prettier-governed. `rtk cargo fmt --all`
  also reformats files you did not touch (for example
  `crates/daena-plugin-api/src/theme.rs`): revert those with
  `rtk git checkout -- <file>` before committing.
- **Plugin bundles:** `src-tauri/plugin-assets/*` are built from
  `packages/modules/*`. After any change under `packages/modules/writing`,
  `language`, `lore`, `timeline` or `packages/plugin-sdk`, rebuild
  (`rtk npm run build:writing-ui`, `build:language-ui`, `build:lore-ui`,
  `build:timeline-ui`, `build:plugin-sdk`) and commit the rebuilt bundles in
  the same commit. Drift checks fail otherwise.
- **You cannot run the desktop app.** It is a Tauri app; browser automation
  cannot drive it. Prove what you can with unit, source and Rust tests. List
  everything that needs a person in the app in the report (section 6), per
  step.
- **When stuck, stop.** If a step needs a decision this file does not settle,
  or touches the plugin security boundary in a way not described here, do not
  guess: write the question in the report, leave the step uncommitted or on a
  clearly marked WIP commit, and stop.

### Checks (the "full suite")

Run all of these before every commit. All must pass, with 0 errors and 0
warnings from `check`.

```bash
rtk npm run check
```

```bash
rtk npm run test:shell
```

```bash
rtk npm run test:plugins
```

```bash
rtk npm run test:ui-ux
```

```bash
rtk npm run test:entity-lifecycle
```

```bash
rtk node --experimental-strip-types scripts/language-bridge.test.mjs
```

```bash
rtk npm run build
```

```bash
rtk cargo test -p daena-plugin-host -p daena --locked --offline
```

`npm run build` prints warnings about unused OpenLayers imports in
`src/lib/maps/openlayers/interaction-manager.ts`; they predate this work.
Add every new `scripts/*.test.mjs` to the matching `package.json` script
(`test:shell` for shell code, `test:plugins` or `test:plugin-transport` for
plugin code), so the suite runs it.

## 1. Read first

In this order:

1. `docs/plans/tabs.md`: the shipped tabs feature, its decisions and shortcut
   rules.
2. `docs/plans/keep-alive-webviews.md`: the keep-alive design. Slices 1 and 2
   are done. Read Behaviour, Pool, Writing, Language, Visibility and bounds,
   Leave and close, and slices 3 and 4.
3. `docs/plans/side-by-side.md`: the side-by-side design (layout A chosen).
4. `docs/plans/side-by-side-inventory.md`: the per-location state of
   `+page.svelte`, with line numbers from an earlier tree (they have drifted
   by up to about 60 lines; search by name).
5. `docs/plugins/PLUGIN_PLATFORM.md`, sections on plugin webviews, sessions and
   the Writing and Language surfaces.
6. Code: `src/lib/navigation/tabs.ts`, `src/lib/plugins/SandboxView.svelte`,
   `src-tauri/src/plugin_webview.rs`, `src/lib/modules/writing-bridge.ts`,
   `src/lib/modules/language-bridge.ts`,
   `packages/modules/writing/src/WritingWorkspace.svelte` (`startLoad` near
   line 117, the message switch near line 292),
   `packages/modules/language/src/plugin-main.ts`.

### Where things stand

- Tabs: one group of tabs in the topbar; only the active tab is live. Switching
  runs `flushAutoSave`, then `canLeaveLanguageSection`, then makes the target
  active (`shellTabs = activateTab(...)`), then `restoreShellLocation`, with
  `shellNavigationBusy` true during the restore (`switchToTab`,
  `applyTabsChange`).
- Plugin webviews: each embedded webview's label is
  `plugin:<plugin id with . as />:<tab id>` (`plugin_instance_label` in
  `crates/daena-plugin-host/src/runtime.rs`). Every identity check uses
  `label_belongs_to_plugin`. Forwarded `writing-state`, `language-state`,
  `maps-state` and `maps-selection` events carry `instanceId` taken from the
  session's label. The shell drops events from instances it is not showing
  (`foreignWebviewInstance`). The instance on screen is
  `shownWebviewInstance = activeTab(shellTabs).id`.
- `SandboxView` takes `instanceId` and `holdMount`. The shell passes
  `holdMount={shellNavigationBusy}`, so no webview is created for a tab while
  a tab switch is still restoring it. Every embedded view sits in
  `{#key shownWebviewInstance}`. `trackWritingMount` and `trackLanguageMount`
  use a generation counter, because Svelte 5 runs the new `{#key}` branch's
  setup before the old branch's cleanup.
- The Rust mount (`plugin_mount_webview`) still closes every other embedded
  `plugin:` webview and always recreates its own label. So at most one
  embedded webview exists, and leaving a tab destroys it.
- Plugin sessions expire after 15 minutes; the SDK renews them once per failed
  call (`packages/plugin-sdk/src/index.ts`).

## 2. Decisions settled for this branch

The plans list open questions. Build these answers; the owner may revisit
them at review.

Keep-alive:

1. Pool size N = 3 embedded webviews, including the one on screen.
2. Slice 3 pools **Writing only**. Language joins in slice 4, together with
   `language.resume`, so a Language view is never shown stale. World graph,
   Chronology and Maps never pool in this branch.
3. No new Writing "hold" message. Hiding a Writing instance happens only
   after a successful flush (the leave guards), so it has no unsent text and
   no armed retry timer. On hide the host cancels that label's AI requests.
4. Before hiding the webview that was on screen, the host focuses the `main`
   webview, so a hidden WKWebView cannot keep keyboard focus.

Side-by-side:

5. At most two groups, side by side. The same entry may not be live on both
   sides: opening it to the side focuses the other side's tab, and a switch or
   move that would make both live tabs show one entry focuses the existing
   one instead. Inactive tabs may show the same entry.
6. Minimum pane width 480px. Below `2 × 480px + rail width` the split folds:
   only the focused group is shown, with a toggle in the topbar (a two-pane
   icon plus the other group's tab count) that switches groups. Groups never
   merge on their own.
7. Dragging a tab onto the right half of the content area creates the right
   group. While a tab is dragged over it, the right half shows a drop
   highlight. Dragging onto the other strip moves the tab there.
8. `⌘\` (Ctrl+\\) moves the focused tab to the other side.
   `⌘⌥←` / `⌘⌥→` (Ctrl+Alt+←/→) focus the left or right group.
   `⌥`-click (Alt-click) and `⌥↵` in Quick Open open to the side.

## 3. Keep-alive slice 3: pool Writing webviews

**Goal.** Leaving a Writing tab hides its webview instead of destroying it.
Returning shows the same webview. Up to 3 embedded webviews stay alive. The
editor still reloads its document on return (`writing.load` replaces the
editor), so undo is not kept yet; slice 4 does that.

### Rust (`src-tauri/src/plugin_webview.rs`)

1. `EmbeddedPluginWebviewState` gains `mount_key: String`: the plugin, view
   id and every mount param (`map_entity_id`, `link_id`, `entity_id`,
   `module_state`), joined unambiguously (for example a JSON array).
2. `plugin_mount_webview` gains two parameters:
   - `reuse: bool`. When true, the label already exists and its stored
     `mount_key` equals the new one, set its bounds, show it, and return
     `MountOutcome::Reused`. Otherwise (re)create it as today and return
     `MountOutcome::Created`. Return the outcome to the shell as
     `"reused" | "created"`.
   - `retain: Vec<WebviewInstanceRef>`, where each item is
     `{ pluginId, instanceId }`. Build each label with
     `plugin_instance_label`, which validates it. The loop that closes other
     embedded webviews now skips labels in `retain`, and cancels AI for every
     label it closes, as it does now. So the shell decides which instances
     survive, and Rust closes everything else on every mount: no leaks.
3. `plugin_set_webview_visible(..., visible: false)`: when that webview had
   focus or was the last shown one, focus the `main` webview first. Use
   `app.get_webview("main")` then `set_focus()`, ignoring errors. Hiding
   must not close anything.
4. New command `plugin_suspend_webview(plugin_id, instance_id)`: hide it,
   cancel its label's AI requests (`cancel_plugin_webview_ai`), and keep it
   alive. Register it in the invoke handler list (`src-tauri/src/lib.rs`)
   and in the capability or permission files, wherever
   `plugin_set_webview_visible` is registered. Find them with
   `rg -n plugin_set_webview_visible src-tauri`.
5. Tests in `src-tauri/src/tests.rs`. The `AppHandle` parts may not be
   testable, so factor the decisions into pure functions and test those:
   - mount-key equality (a different `entity_id` means recreate);
   - `retain` filtering (labels kept, labels closed);
   - a bad `instanceId` in `retain` rejected.

### Shell

6. New pure module `src/lib/plugins/webview-pool.ts`, with unit tests in
   `scripts/webview-pool.test.mjs` (add to `test:shell`).
   - Entries: `{ pluginId, instanceId, mountKey, ready, lastUsedAt }`, keyed
     by `(instanceId, pluginId)`.
   - Functions:
     - `touch(pool, ref, mountKey, now)`;
     - `markReady(pool, ref, ready)`;
     - `retainSet(pool, shownRef, limit)`: the shown entry plus the most
       recently used others, up to `limit`;
     - `evicted(pool, retain)`;
     - `pruneTabs(pool, liveTabIds)`;
     - `prunePlugins(pool, enabledPluginIds)`.

   Keep it immutable, like `tabs.ts`.
7. `SandboxView.svelte` gains `pooled?: boolean` and `retain?: () => WebviewInstanceRef[]`,
   plus an `onmounted?: (outcome: "reused" | "created") => void` callback.
   When `pooled`:
   - the mount does not unmount its own label first;
   - it calls mount with `reuse: true` and the current retain list;
   - on `"reused"` it sets `hostMounted` and calls `onmounted("reused")`;
   - teardown calls `plugin_suspend_webview` instead of unmount.

   Unpooled views behave exactly as today.
8. `+page.svelte`:
   - A `webviewPool` `$state`.
   - The Writing `SandboxView` is `pooled`, with `retain` = the pool's
     `retainSet` (N = 3) and `onmounted`, which:
     - on `"reused"`, marks the shell's `writingWebview.ready` from the
       pool's `ready` flag;
     - bumps `writingReadyCount`;
     - leaves `writingLoad = null`, so the existing load effect posts
       `writing.load` to the reused instance.
   - The Writing `ready` state event marks the pool entry ready. Read
     `instanceId` from the raw payload before parsing, as
     `foreignWebviewInstance` does.
   - An `$effect` on the set of live tab ids prunes the pool and unmounts
     pruned instances (`unmountPluginWebview(pluginId, instanceId)`). This
     one effect covers every way a tab disappears: close, close others, close
     to the right, close all, `closeTabsForEntity` (archive),
     `pruneUnavailableShellTabs` (plugin disabled) and project switch.
   - `resetProjectSessionState` clears the pool. Project close already calls
     `closeAllPluginWebviews`.
   - Fullscreen: the effect that ends Writing fullscreen when the Writing
     view goes away posts `writing.fullscreen` false. Make it post to the
     instance that is being hidden (capture it), not to
     `shownWebviewInstance`, which already names the new tab.
   - The visibility effect (`setPluginWebviewVisibleQuietly`) addresses only
     the shown instance. Hidden pool members are never shown by it, so
     closing an overlay cannot reveal one.
9. `scripts/window-close.test.mjs`: add a source check that quit still calls
   `closeAllPluginWebviews` and project close still clears the pool.
10. `docs/plans/keep-alive-webviews.md`: status line (slice 3 done, Writing
    only) and the Pool section updated to the `reuse` / `retain` /
    `suspend` design.

**Hard cases.**

- A tab switch away from Writing while Writing's leave flush fails: the
  switch is refused, nothing is suspended.
- Closing the shown Writing tab: its instance is unmounted, not suspended
  forever.
- Closing a hidden Writing tab: its instance is unmounted.
- A fourth Writing tab: the least recently used hidden one is closed by the
  next mount's `retain`.
- Disabling Writing: `close_plugin_webview` closes every label, and the pool
  prunes.
- Reopening a closed tab: a fresh instance, not a stale reuse.
- Two Writing tabs on the same manuscript each have their own instance.

**Exit criteria.**

- The full suite passes.
- New tests cover the pool module and the Rust pure functions.
- A source test asserts that the Writing `SandboxView` is `pooled` and the
  Language, graph, Chronology and sandbox views are not.
- One commit: `Keep-alive webviews, slice 3: pool Writing webviews.`

**For the owner (manual, in the app).**

- Switch between two Writing tabs and a Lore tab, many times: the right
  text each time, the bounds right, no flash.
- A tab menu over Writing hides it; closing the menu shows the shown one
  only.
- Typing after a switch goes to the visible editor only.
- Open four Writing tabs: the oldest reloads when revisited.
- Disable Writing: no webview remains.
- Quit from a Writing tab: nothing lost.

## 4. Keep-alive slice 4: keep the editor, pool Language

**Goal.** Returning to a Writing tab keeps the editor and its undo history
when the document has not changed meanwhile. Language joins the pool and
re-reads its data when shown.

### Writing (`packages/modules/writing/src/`)

1. The editor keeps `shownRevision`: set from each applied `writing.load`
   (`message.revision`), and updated on `writing.saved` for its entity when
   nothing is unsent. Today `writing.saved` (`WritingWorkspace.svelte` near
   line 305) clears the conflict banner and ignores the revision.
2. Keep rule in `startLoad`: if the new load names the same entity and
   `revision === shownRevision`, keep the editor, its undo history and its
   selection. Adopt the new load token, re-apply `editable`, `placeholder`
   and `mode` from the message, and do not call `sync.reset()`. Otherwise
   replace the editor as today.
3. Unsent text must never be discarded by a load. Same entity, same revision
   and unsent text: keep the editor and publish the text again. Different
   entity or revision with unsent text, which should not happen because the
   leave flush ran: keep the editor, publish the text, and report it through
   the existing save-error path. Do not reset.
4. Tests in `scripts/writing-plugin-source.test.mjs`, which is source-level;
   add pure helpers if that makes it testable:
   - keep on the same entity and revision;
   - replace on a different revision;
   - replace on a different entity;
   - `writing.saved` updates `shownRevision`;
   - unsent text survives every path.

   Then rebuild the bundle (`build:writing-ui`) and run `check:writing-ui`.

### Language (`packages/modules/language/src/`)

5. New host message `language.resume` with `{ entityId, pane }`. The webview
   re-runs `loadLanguage(entityId)` and the active pane's load. If
   `entityId` differs from what it shows, it loads that entity. Add the type
   to `plugin-protocol.ts`, the handler to `plugin-main.ts` or
   `LanguageWorkspace.svelte`, and a source test in
   `scripts/language-plugin-source.test.mjs`. Rebuild with
   `build:language-ui`, then run `check:language-ui`.
6. Shell: the Language `SandboxView` becomes `pooled`. On `"reused"`, mark
   `languageWebview.ready` from the pool and post `language.resume` with the
   selected id and `languagePane`. Language's `{#key selected?.id}` stays: a
   different language in the same tab is a different `mountKey`, so that
   instance is recreated.
7. Document `language.resume` in `docs/plugins/PLUGIN_PLATFORM.md` beside the
   other Language host messages. Also document the Writing keep rule beside
   `writing.load`, and in `docs/plans/writing-isolation.md` if it lists host
   messages.
8. `docs/plans/keep-alive-webviews.md`: status (all slices done), and close
   or answer its open questions. Questions 2 and 3 stay open for the owner,
   to check in the app. `docs/plans/tabs.md`: mark "Keep-alive webviews"
   under Later as done, and update the "Cost" paragraph in Approach.

**Hard cases.** These are the slice 4 manual cases in
`keep-alive-webviews.md`. In tests:

- the same manuscript in two tabs, edited in one: the other is replaced on
  return;
- an external edit: replaced;
- an AI rewrite running when the tab was left: the suspend cancelled it, and
  the dialog shows the cancellation, with no surprise insert;
- fullscreen does not follow to the next tab.

**Exit criteria.**

- The full suite passes, including the rebuilt bundles' drift checks.
- New tests cover the keep rule, unsent text and `language.resume`.
- One commit: `Keep-alive webviews, slice 4: keep the editor, pool Language.`

**For the owner.**

- Type, undo a few steps, switch to Language and back, then undo again: it
  works.
- Same manuscript in two tabs: an edit in one shows in the other with a
  fresh editor.
- Edit a lexicon in one Language tab: another Language tab shows it on
  return.
- Cancel a Language leave dialog: you stay on the tab.
- Activity Monitor: memory with three live webviews (this answers open
  question 1).

## 5. Side-by-side

Build in the order of `side-by-side.md` → Slices. Steps 5.1 to 5.3 change no
behaviour; they are refactors that must keep every existing test passing
untouched. Only update a test whose assertion is about where code lives (a
source test that greps `+page.svelte`), never one about what it does. Say in
the commit which tests you re-pointed and why.

### 5.1 Location store (slice 1)

**Goal.** All per-location state and the functions that load, save, navigate
and tear it down live in one `LocationStore`. The page creates exactly one.
No visible change.

**Approach.**

- Create `src/lib/location/location-store.svelte.ts`: a class with
  `$state` fields for the per-location state. Section 1 of the inventory
  lists it, category (a): section and views, selection and load tokens,
  document body, fields, revisions, dirty tracking, autosave timers and save
  queue, conflict, selection AI dialog state, collection query, expanded
  groups, collection scroll, pane visibility and widths, the map editor
  session, and the Writing and Language bridge state (`writingWebview`,
  `writingLoad`, `writingFlush`, `languageWebview`, `languageLeave`, …).
- Shell-global state stays on the page: the project, entity cache, modules,
  settings, theme, rail, dialogs, Quick Open, the create dialog, toasts and
  tabs.
- Dependencies come in through one `LocationContext` object of getters and
  callbacks that the page builds: the project client, `entities`, `modules`,
  `projectInfo`, `projectDiagnostics`, `confirmDialog`, `setError`,
  `recordShellDeparture`, …. The store never imports `+page.svelte`.
- Effects. The store's `$effect`s (the collection query reload, the
  workbench layout reload, the Writing load, editable, conflict and AI-state
  effects, collection fetch) must run inside a component's effect scope.
  Construct the store during the page script's initialisation, so effects
  created in the constructor attach to the page. Alternatively use
  `$effect.root` with an explicit `destroy()` that the page calls on
  teardown. Pick one and say which.
- The template keeps working through `loc.<field>`. `bind:` works on class
  `$state` fields.
- Move in sub-steps, keeping the suite green at each: the selection and
  document (with `selectEntity`, `loadSelectedState`, `clearSelection`, the
  dirty, autosave and save functions and the conflict functions), then the
  collection and panes, then the views and surfaces (`switchSection`,
  `openPluginView`, `leavePluginView`, `currentShellLocation`,
  `restoreShellLocation`), then the Writing and Language bridges, then the
  map editor session and the selection AI. A sub-step may be its own commit
  (`Side-by-side, slice 1 (selection): …`).
- Keep function names. A moved function becomes a method; leave a one-line
  page wrapper only where many template call sites would otherwise change.
- Plain `let` tokens (`selectedLoadToken`, `saveInFlight`, `writingLoad`, …)
  become plain class fields, not `$state`, matching today's reactivity.
  Changing what is reactive changes behaviour.

**Hard cases.**

- `selectEntity`'s same-id early return.
- The autosave timer and `saveInFlight` serialisation.
- `persistDocumentSnapshot`'s stale-result guards (`documentRevision`,
  `selected.id`).
- The generation guards in `trackWritingMount` and `trackLanguageMount`.
- `beforeunload` reads `hasUnsavedChanges` and `editorRef`; it must read
  them through the store.

**Exit criteria.**

- The full suite passes with behaviour tests unchanged.
- `rg` shows no per-location `$state` left on the page: list any you kept
  and why in the report.
- A new `scripts/location-store.test.mjs` unit-tests the store's pure parts:
  dirty marking, the save queue, conflict state and location capture and
  restore round trip. Use fakes for the context. If the store cannot be
  constructed outside a component, test the extracted pure helpers.

### 5.2 Location view (slice 2)

**Goal.** The template chain under the topbar moves into
`src/lib/location/LocationView.svelte`, which takes a store. It covers
settings, project center, home, host view, wiki, tree, sandbox views and the
workbench, from the first workspace branch to the end of the workbench
(inventory, section 3). The page renders one `LocationView` inside
`.app-main`, after `GlobalToolbar`.

**Details.**

- The `LocationView` root element is the tab panel: `role="tabpanel"`,
  `id="shell-tabpanel-<group id>"`, labelled by its group's active tab.
  `TabStrip`'s `panelId` follows. Remove `id`, `role` and `aria-labelledby`
  from `.app-main`. This fixes the panel that contains the topbar. Check
  every CSS rule that targets `.app-main >` children: the old attempt broke
  the flex column in the `sandbox-active`, `language-surface`,
  `writing-surface` and `map-surface-open` modes. Give the root the same
  flex column there, and never `display: contents`.
- `scripts/writing-shell-guards.test.mjs` matches template branch markers at
  a fixed indent (`/^ {8}\{(?:#|:else )if/`). Re-point it at the new file and
  make it indent-agnostic.
- Make document-level ids and radio names inside the view unique per view
  (`sortField`, `pageSize`, `ai-rewrite-title`, and the others the inventory
  lists in its hazards section).
- Quick Open, the create dialog, `DialogHost`, toasts, `EntityHoverCard` and
  the `HostGuide`s stay on the page.

**Exit criteria.**

- The full suite passes.
- `+page.svelte` loses about 1,900 template lines.
- One tab panel exists, and it no longer contains the topbar (source test).

### 5.3 Store-local slots (slice 3)

**Goal.** Nothing a store keeps is shared through a section-keyed global
slot, so a second store cannot reset the first.

- `collectionQuery` and `expandedGroups`, the collection scroll
  (`collectionScrollBySection`), the specialized-surface scroll, and pane
  visibility and widths become store fields.
- The section-keyed localStorage records (`daena:collection-query:<section>`,
  `daena:workbench-layout:<section>`) become defaults: a store reads them
  only when it enters a section, and writes them as the last-used value.
- The effects that reload them on every `section` change move into the store
  and key off its own section.

**Exit criteria.**

- The full suite passes.
- A unit test constructs two stores (or their slot helpers) in one section
  and shows that changing one does not change the other.

### 5.4 Two groups (slice 4)

**Goal.** The user can split: two groups side by side, each with its own
strip, live location and history. This step covers shell-editor locations
(lore, timeline, houses, settings, home). Plugin webviews remain single:
while one side shows a webview, the other side may not open one; it shows a
short notice with a button to move that tab to the focused side.

**Tab model (`src/lib/navigation/tabs.ts`, plus tests in
`scripts/shell-tabs.test.mjs`).**

- New functions:
  - `moveTabToGroup(state, tabId, targetGroupId | "new-right", now)`;
  - `focusGroup(state, groupId, now)`;
  - `setSplitRatio(state, ratio)`.
- Existing behaviour that must stay:
  - closing the last tab of a group removes the group (it already does when
    other groups exist);
  - at most two groups;
  - invariants in `assertTabsInvariants`.
- Storage version 2 adds `splitRatio` and keeps `activeGroupId` as the
  focused group. `parseTabsState` reads v1 as one group.
- The "same entry live on both sides" rule (decision 5) is a pure function
  over the state plus each group's active tab identity, and is tested.

**Shell.**

- One `LocationStore` per visible group (at most two), keyed by group id.
  Creating the right group creates a second store and restores the moved
  tab's location into it. Removing a group destroys its store, after
  flushing it.
- `shownWebviewInstance` becomes per store: that group's active tab id.
- Focus. `pointerdown` (capture) inside a `LocationView` focuses its group.
  The focused group's active tab gets the accent outline in its strip, as in
  layout A. Every global entry point uses the focused store:
  - Back and Forward;
  - `shellNavigationHistory` (the focused group's active tab);
  - the rail, Quick Open, `setShellEntityOpener`;
  - `handleHistoryKey` and `handleTabKey`;
  - tab shortcuts, which act within the focused group.
- Topbar (layout A). `GlobalToolbar` renders the left group's strip from
  after Back and Forward up to the divider's x position, and the right
  group's strip from the divider to before Quick Open. Measure the divider
  position from the content area so the two line up. With one group the
  topbar is unchanged.
- Divider. A vertical resize handle between the views (reuse
  `PaneResizeHandle`). It sets `splitRatio`, clamped by the 480px minimum,
  and double-click resets it to 0.5.
- Narrow fold (decision 6). Fold when the content width is under
  `2 × 480px`, measured with a `ResizeObserver` on `.app-main`.
- Entry points:
  - tab context menu: "Move to other side" and "Close other side" (closes
    the other group's unpinned tabs);
  - drag to split (decision 7), extending `tab-drag.ts`;
  - "Open to the side" in the rail menu (`RailContextMenu`) and the row menu
    (`EntityRowActions`);
  - `OpenIntent` gains `"side"`; `openIntent()` maps Alt-click to it, and
    its tests cover that;
  - `openInNewTab` gains `{ group: "same" | "other" }`.
- Every mutation fans out. `refreshAfterEntityMutation` runs per store whose
  selection the mutation touches. Rename refreshes both strips' titles.
- Close and unload:
  - `closeProject`, the window-close handshake and `persistShellTabs`
    iterate the stores: flush each, run each leave guard, stop on the first
    refusal;
  - `beforeunload` blocks if any store is dirty.

  Update `scripts/window-close.test.mjs`.
- Popover overlap. `measureAnchoredPopovers` measures every
  `.sandbox-view-container`. When any overlaps, it hides only the instances
  whose container it overlaps. Tag containers with
  `data-plugin-id` / `data-instance-id`.

**Exit criteria.**

- The full suite passes.
- The tabs reducer tests cover moving, creating and removing a group, v1 to
  v2 storage, focus and the same-entry rule.
- Source tests show both strips rendered from `shellTabs.groups`, global
  handlers reading the focused store, and the close paths iterating stores.
- `docs/ui/UI_UX.md` "Shell chrome" describes the split, and `tabs.md`
  shortcuts list the new ones.

**For the owner.**

- Split, edit an entry on each side: both save. Quit, reopen: both groups,
  the divider and the focused side come back.
- A leave dialog on one side leaves the other alone.
- Narrow the window and widen it again: nothing is lost.
- Renaming on one side updates the other.

### 5.5 Webviews on both sides (slice 5)

**Goal.** Each side may show its own plugin webview: Writing beside Lore,
two different manuscripts, Writing beside Language.

- Each store owns at most one shown instance. The pool's `retainSet` keeps
  every store's shown instance plus the most recent hidden ones. N becomes 3
  hidden plus the shown ones, with a hard cap of 5.
- Routing. Plugin events go to the store whose shown instance matches
  `instanceId`. Events with a `null` `instanceId` go to the focused store.
  Each store has its own Writing and Language bridge state (moved in 5.1).
- Overlays. `modalOverlayOpen` hides every shown instance. Anchored popovers
  hide only the instances they overlap (5.4).
- The same document never shows on both sides (decision 5), so two Writing
  webviews never edit one entry.
- Remove the 5.4 "one webview side" notice.

**Exit criteria.**

- The full suite passes.
- Tests cover event routing by instance across two stores, and the
  retain set with two shown instances.

**For the owner.**

- Two manuscripts side by side: typing in each saves its own document.
- A modal hides both webviews. A tab menu over the left side hides only the
  left one.
- Switching tabs on one side leaves the other side's editor and undo
  untouched.

### 5.6 Shortcuts and polish (slice 6)

- Shortcuts (decision 8): `⌘\`, `⌘⌥←` / `⌘⌥→`, `⌥`-click and `⌥↵`.
  - Add them to `src/lib/navigation/tab-shortcuts.ts` and its checked copies
    in the Writing, Language, Lore and Timeline packages, and to
    `TAB_SHORTCUT_IDS` in `src-tauri/src/transfer.rs`. The test in
    `scripts/tab-shortcuts.test.mjs` keeps all six lists in step.
  - Add the macOS menu items to the Window menu in `src-tauri/src/app_menu.rs`,
    with Rust tests, following the existing tab items.
  - Rebuild all four plugin bundles.
- Docs:
  - `docs/plans/side-by-side.md`: status line, the decisions from section 2,
    open questions closed or moved to the report;
  - `docs/plans/tabs.md`: the "Later" section marked done, plus the
    shortcuts table;
  - `docs/ui/UI_UX.md`.

**Exit criteria.**

- The full suite passes.
- The shortcut lists agree (test).
- The menu items are tested.

## 6. Report

When you finish, or stop, append a section `## Branch report` to the end of
this file and commit it as the last commit. Include:

- each commit, with one line on what it did;
- deviations from this walkthrough, and why;
- tests added, and tests re-pointed in the refactor steps;
- the full-suite results at the last commit;
- anything unfinished or uncertain, and every question from "When stuck,
  stop";
- the combined manual checklist for the owner: every "For the owner" item
  above, plus anything your changes added, ordered by risk.
