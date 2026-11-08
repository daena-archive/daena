# Keep-alive plugin webviews

Keeps the last few plugin webviews hidden, so returning to a tab that showed
one is instant and keeps its in-editor state. Side-by-side tabs come later.
This plan gives each webview its own label and session so two of the same
plugin can exist, and it does not build side-by-side.

Status: slice 1 done (SDK renews an expired, revoked, or stale session once). Slices 2–4 not started. Workspace tabs are in place
(`docs/plans/tabs.md`). An inactive tab is a saved `ShellLocation`. One
embedded webview exists at a time, label `plugin:<sanitized id>`, and
leaving a tab destroys it.

Found while planning: a plugin session expires after 15 minutes. Slice 1
renews an expired, revoked, or stale session once and retries the call.
Before that, a Writing or Language view left open longer than the TTL failed
every RPC, including the drafts it hands to the shell.

## Goal

Returning to a Writing or Language tab shows the webview that was already
there, with its editor undo and scroll. The shell stays the only writer of
the open document. A hidden webview cannot take focus, receive input, or
change the active tab's state. At most N embedded webviews stay alive.
Closing a tab, archiving or deleting its entity, disabling its plugin, or
closing or switching the project destroys that tab's webview.

## Behaviour

- One instance per tab and plugin. The instance id is the tab id; the label
  also names the plugin, so a tab that goes from Writing to Language and
  back has two labels, one per plugin. Navigating inside the tab keeps the
  instance. A second tab, including a copy of the current entry, gets its
  own instance. The pool is keyed by (tab id, plugin id).
- The pool holds Writing and Language only. World graph, Chronology, and the
  map editor stay as they are (see Sandbox views).
- At most 3 embedded webviews are alive, including the one on screen. A
  hidden instance that was used longest ago is closed when a fourth would be
  created. Closing it flushes first, the same way leaving a tab does.
- Only the active tab's instance is shown. Overlays hide that instance.
  Hidden instances stay hidden.
- Showing an instance sets its bounds, then shows it. A new instance stays
  at 1×1 until its first document has painted, as `plugin_mount_webview`
  does today.
- A Writing instance keeps its editor when the shell's next `writing.load`
  names the same entity and the same revision the editor is showing, and the
  editor has no unsent text. Any other load replaces the editor, as today.
  Unsent text is not a reason to replace (see Writing).
- A Language instance, on show, re-reads the language it was opened on.
  Lists and records another tab edited are not left on screen.
- Plugin state events carry the instance id taken from the broker session's
  origin. The shell drops events from any instance that is not the one it
  is showing. A plugin cannot name an instance in the payload.
- The macOS menu `shell-shortcut` event stays a main-window event. It has
  no instance. Tab chords that a Writing or Language webview forwards still
  arrive as `writing-state` or `language-state`, and those do carry the
  instance.
- Window close, project close, and project switch still flush the active
  editor, then destroy every pooled webview.

## Approach

### Label

`plugin_window_label` (`crates/daena-plugin-host/src/runtime.rs:64`) keeps
ASCII letters, digits, `-`, `/`, `_`, and `:`, and turns every other
character into `-`. `daena.writing` becomes `plugin:daena-writing`. A plugin
id may already contain `:`, so `plugin:<sanitized id>:<instance>` cannot be
split. `plugin:foo:bar:inst` is plugin `foo:bar` or plugin `foo` with
instance `bar:inst`.

Tauri labels allow the same set (`[A-Za-z0-9\-/:_]`). No separator character
is reserved.

Use a length prefix. The length is the length of the sanitized id, in
decimal, with no extra characters:

```text
plugin:<length>:<sanitized id>:<instance>
```

`daena.writing` with instance `4f2a9c1b-…` (the tab id) is
`plugin:13:daena-writing:4f2a9c1b-…`. Parse by reading the length, taking
that many characters, and requiring a `:` and a non-empty instance after
them. The instance is a tab id: hex, hyphens, and nothing else. Tab ids come
from `crypto.randomUUID()`, but tabs are also restored from `localStorage`,
so the host validates the instance itself (`[0-9a-f-]`, 1 to 64 characters)
and rejects anything else; the shell never builds a label. The sanitized id
must equal the body of `plugin_window_label(plugin_id)`.

External plugin windows keep the bare label from `plugin_window_label`
(`open_plugin_webview` in `plugin_webview.rs`). The matcher accepts either
form, and only for that plugin. A Writing instance does not match
`daena.maps`. A bare `plugin:` prefix is not enough.

`plugin_window_label` itself stays the bare label. A new function next to
it builds and parses instance labels. Two plugin ids that sanitize to the
same string already collide today (`daena.writing` and `daena-writing`).
The prefix does not make that worse.

### Identity checks

Every comparison below must accept an instance label for that plugin and
reject one for any other plugin. Line numbers are the current tree.

| Site | What it does today | With instances |
| --- | --- | --- |
| `transfer.rs:1252` | `plugin_protocol_response` returns 403 unless the label equals `plugin_window_label` of the URI host. | The parsed plugin must be that host. This is the boundary that stops one webview reading another plugin's package or calling its RPC. |
| `transfer.rs:1210` | A POST `Origin` must be `plugin://<id>`, or be absent. | Unchanged. Every instance of one plugin shares that document origin, so Origin does not tell them apart. The label and the session do. |
| `transfer.rs:1322` | Protocol bootstrap stores the webview label as the session origin. | Store the full instance label. The requested plugin id must still equal the URI host. |
| `transfer.rs:1370` | `authorize_rpc(webview_label, request)`. | Unchanged call. The session's origin must be this exact label (`lib.rs` `valid`, around line 613), so instance A cannot use instance B's session. |
| `transfer.rs:1552` and `:1639` | `forward_plugin_state_event` emits `writing-state`, `language-state`, `maps-state`, and `maps-selection` with no instance. The publisher id is the session's plugin, not the payload (`:1580` requires the event namespace to be that plugin). | Add `instanceId` from the session origin. Overwrite any `instanceId` in the payload. |
| `host.rs:31` and `:558` | `plugin_bootstrap` requires the label to equal `plugin_window_label(plugin_id)`. | Accept an instance label whose plugin is `plugin_id`. The command origin stored at `:42` is the full label. |
| `host.rs:90` and `:558` | `plugin_rpc` accepts any non-empty `plugin:` label, then `authorize_rpc` binds the session to that label. The session's `plugin_id` is whatever bootstrap stored. | Reject a label that does not parse as some plugin. Authorization still requires the session origin to be this label, so a Writing webview cannot present a Language session. |
| `crates/daena-plugin-host/src/lib.rs:3256` `authorize_rpc` | Checks the session id against the origin argument. | Pass the full label, as today. Do not pass the bare plugin label. |
| `crates/daena-plugin-host/src/lib.rs:489` `SessionRegistry::issue` | A new session revokes earlier sessions of the same project, plugin, and origin. Other origins stay valid (`:502`). | Each instance is its own origin. Two instances of Writing must not share a session: a shared origin would revoke the hidden one on the next bootstrap. They do share grants and the per-plugin rate limit, which are keyed by project and plugin, not by session. |
| `crates/daena-plugin-host/src/lib.rs:576` `ids_for_origin`, `src-tauri/src/lib.rs:227` `cancel_plugin_webview_ai` | AI cancel collects every session id for that origin, including revoked ones, and cancels only those requests. | Pass the instance label. Closing A does not cancel B. `ai_request_ids_for_session` (`crates/daena-plugin-host/src/lib.rs:3224`) is already per session. |
| `crates/daena-plugin-host/src/lib.rs:543` `get_active`, used at `:2934` | `importer_runnable` is true when any live session exists for the plugin. | Unchanged. Either instance is enough. Hiding an instance must not revoke its session, or this flips off while the view is merely hidden. |
| `plugin_webview.rs:79` `close_plugin_webview` | Cancels AI and closes the bare label. | Close every embedded label for that plugin, and the bare external window. `project_commands.rs:1508` (project close) and `module_commands.rs:533` (disable) call this. |
| `plugin_webview.rs:303` | Mount closes every other embedded `plugin:` webview. | Stop closing pooled instances. Still refuse a second instance with the same label. |
| `plugin_webview.rs:114` `embedded_webview_states` | Bounds keyed by label. | Key by the full instance label. |
| `plugin_webview.rs:382`, `:416`, `:446`, `:462` | Resize, unmount, post, and show/hide look up the bare label. | Take an instance id, build the label, and reject it when it is not that plugin. |
| `plugin_webview.rs:727` | `plugin_close_all_webviews` already closes every `plugin:` label (`:731`). | Keep that. It is the project-close and window-close sweep. Cancel AI per label before close, as it does now. |
| `src-tauri/src/lib.rs:357` `push_appearance_to_plugin_webviews` | Evaluates in every `plugin:` webview (`:366`). | Keep that. Appearance is not privileged per instance. |
| `src-tauri/src/lib.rs:659` | The `plugin` scheme handler passes `ctx.webview_label()` into `plugin_protocol_response`. | No change at the call. The check at `transfer.rs:1252` is the gate. |
| `src-tauri/src/lib.rs:669` `plugin_icon_response` (`transfer.rs:980`) | Only the `main` webview may read icons. | Unchanged. Not an instance check. |
| `scripts/check-plugin-isolation.mjs:161` | Asserts the source still contains `plugin_window_label(plugin_id) != webview_label`. | Point it at the new predicate. The check must still fail the build if a plugin webview can satisfy another plugin's host. |
| `src-tauri/src/tests.rs:1971`, `:2293`, `:2419`, `:2621` and `crates/daena-plugin-host/src/tests.rs:3053` | Build the bare label and bootstrap with it. | Add instance-label cases beside these. The bare-label cases stay for the external window. |
| `runtime.rs:86` `webview_policy` | The policy label is the bare label. CSP and `plugin://<id>` origin stay per plugin (`runtime/tests.rs` around line 48). | Embedded views do not use `policy.label` as their window label. Navigation still allows only that plugin's origin (`transfer.rs:1191`). |

`src-tauri/src/broker.rs` does not compare webview labels. Trusted shell RPC
there is the main window. `app_close.rs` does not compare labels either. The
shell flushes, then `leavePluginView({ closeWebviews: true })` calls
`plugin_close_all_webviews`.

Capability tokens are not bound to the label. An asset read token matches
plugin id and session id (`transfer.rs` `take_read`, around line 163). A
text transfer handed to the shell matches plugin id and project id
(`take_text`, around line 364), not the session. The shell must claim a
text transfer only while handling an event from the active instance.
Grants stay per project and plugin.

There is no `bundled:<plugin>` collision. That origin is the shell's
trusted session and is a different string (`SessionRegistry::issue`).

### Shell routing

`plugin_webview_post`, `plugin_resize_webview`, `plugin_set_webview_visible`,
and `plugin_unmount_webview` take `pluginId` and `instanceId`. The shell
(`src/lib/project/client.ts` around lines 700–722) passes the tab id.
`SandboxView` mounts and resizes that instance.

State listeners in `+page.svelte` (`writing-state` around line 8202,
`language-state` around 8185, `maps-state` around 8112, `maps-selection`
around 8177) ignore a payload whose `instanceId` is not the shown instance.
`handleLanguageStateEvent` (line 1773) and the writing handler must not run
shortcuts, pane changes, or drafts from a hidden instance.
`writingMessageCurrent` (`writing-bridge.ts:286`) stays as a second check
for entity and load token. It is not the instance check.

### Pool

The shell keeps a map from tab id to `{ pluginId, instanceId, mountKey,
lastUsed }`. `mountKey` is the plugin, the view id, and the mount params
(`entityId`, `moduleState`, and `mapEntityId` / `linkId` if a caller starts
passing them).

N is 3. This repo has no measurement of a Daena webview. Public write-ups
put a non-trivial WKWebView around 50–150 MB of private memory, and a
WebView2 view adds a renderer of tens of MB plus the page on top of a
shared browser process. Those are not numbers for this app. A Writing view
is the heavy one (the TipTap editor). Cap the pool at 3 until Writing and
Language are measured in Activity Monitor and Task Manager. Evict the least
recently used hidden instance. Never evict the one on screen.

Destroy an instance when:

- its tab closes, or the closed-tab stack drops it;
- the entity it shows is archived or deleted in this session;
- its plugin is disabled or uninstalled (the tab closes too, per the tabs plan);
- the project closes or another project opens (`close_plugin_webview` on every plugin, after the shell flush).

Hidden instances call `hide` and are never given focus. `open_plugin_webview`
is the only `set_focus` on a plugin view today, and it is the external
window. Do not add `set_focus` for a hidden embedded view. Before hiding
the view that has been on screen, move focus back to `main`. Confirm on
macOS that a hidden WKWebView is not left as first responder (open question).

`SandboxView` (`src/lib/plugins/SandboxView.svelte`) today closes and
recreates on every mount (`:98`–`:104`) and closes again on destroy
(`:123`). For a pooled plugin it adopts the live instance: set bounds, show,
and on placeholder destroy it hides. It closes only when the pool says the
instance is gone. `plugin_mount_webview` must not recreate an instance that
is already loaded with the same `mountKey`. A different `mountKey` on the
same tab closes that instance and mounts another. Query params are still
read only at document load (`plugin_webview.rs` comment around line 311).

### Writing

The shell is the only writer. `flushAutoSave` (`+page.svelte:5468`) calls
`flushWritingWebview` before `saveDocument`. The webview never saves. It
publishes drafts (`draft-sync.ts`, idle 500 ms, retry 2 s) and the shell
applies them in `applyWritingDraft` (`:1874`).

Today the Writing webview already survives entity changes while the section
stays Writing. `writingWebviewShown` (`:1605`) keeps one `SandboxView`, with
no `{#key}` on the entity. `trackWritingMount` (`:1831`) clears shell load
state only when that placeholder leaves the document. The effect at `:1932`
posts `writing.load` when `entityId:selectedLoadToken` changes.
`postWritingLoad` (`:1908`) skips the load only when the entity is the same
and the flush applied text. Otherwise `WritingWorkspace.startLoad`
(`WritingWorkspace.svelte:117`) bumps the load sequence, calls
`sync.reset()` (drops unsent text), closes the AI dialog, and builds a new
editor. Undo is lost on every real load, including a Writing-tab to
Writing-tab switch that never destroys the webview.

What changes for a Writing to Writing tab switch: each tab has its own
webview. The switch hides one and shows the other. It does not load the
other manuscript into the webview that was just hidden. The shell still
reloads the entry from disk on restore (`loadSelectedState` bumps
`selectedLoadToken` at `:5479`) and posts `writing.load` to the instance
being shown. That load is the one the keep rule applies to.

The keep rule lives in `startLoad`. The revision the editor is showing is
the revision from the last applied `writing.load`, updated when
`writing.saved` arrives for this entity and nothing is unsent.
`writing.saved` today (`:305`) clears the conflict banner and does not
store `message.revision`. Without that update, the first autosave makes
the shell's revision newer than the editor's, and every return replaces
the editor and drops undo. The load token stays per load
(`writing-bridge.ts` comment around line 163). Do not use the token as the
revision.

Cases:

- Same document in two tabs. Leaving flushes and saves, or the switch is
  refused. `switchToTab` (`:3867`) returns when `flushAutoSave` returns
  false, and a revision conflict makes `saveDocument` return false
  (`:6480`). The hidden instance's shown revision is the one it saved.
  Editing the other tab saves a newer revision. Returning posts that newer
  revision, so the hidden editor is replaced. Its undo belonged to text
  that is no longer the document. If the other tab did not change the
  revision, the editor and its undo stay.
- External edit. The shell reloads a new revision. The load does not match.
  The editor is replaced.
- Conflict banner. The switch does not happen while the shell is in
  conflict, because save returns false. The banner is shell state
  (`documentConflict`), not per tab. There is no path that leaves a tab
  with the banner up and the save uncommitted.
- AI rewrite. The run lives in the webview (`ai-run.ts` polls
  `ai.request.poll`). Hide cancels that instance's AI and closes the dialog
  before the view is hidden. An accepted proposal is flushed with the
  leave. A run still in flight does not finish into a hidden editor.
- Full screen. `writingFullscreen` is shell state. The effect at `:2014`
  ends it when `writingWebviewShown` becomes false, and posts
  `writing.fullscreen` false when the webview is ready. Post that to the
  instance being hidden. A hidden instance is not left fullscreen.
- Unsent text at the next load. `startLoad` calls `sync.reset()`, which
  drops it. The keep rule must not treat "has unsent text" as "replace".
  Same entity, same revision, and unsent text: keep the editor, then flush
  again. A different entity, or a different revision: flush has already
  run on hide; if text is still unsent, keep the editor and surface the
  failure instead of deleting it. Replace is for a load the shell intends,
  after a clean flush.

On hide, after a successful flush: cancel the idle and retry timers
(`draft-sync.ts` `dispose`), cancel this origin's AI, and ignore further
drafts until show. The queue in `draft-sync.ts` already serializes a flush
against an idle publish. A retry armed because a publish failed rearms
unsent text (`:128`). Hide must dispose that timer or the hidden view
publishes into whatever tab is now active. The shell also drops the event
by instance id.

### Language

`{#key selected?.id}` (`+page.svelte` around line 9827) destroys the
Language placeholder when the selected language changes. Mount params are
`entityId` and `moduleState` (`{ pane }`). `plugin-main.ts` reads them once
from `location.search` (`:47`) and loads that entity in
`LanguageWorkspace.loadLanguage` (`:140`). Nothing in the page later tells
the webview that the entity changed. There is no refresh subscription.

While a Language instance is hidden, these go stale: the language record,
overview fields, lexicon rows, and anything another tab or the shell edited.
The webview does not hear those edits. `entity-changed` is published when
Language itself writes (`plugin-main.ts:83` and `:121`), which updates the
shell, not the other way around.

On show, post `language.resume` with the tab's entity id and pane. The
webview re-runs `loadLanguage` and the active pane's load. That is a new
host message. Pane and leave messages stay as they are.

Leave guards run before the tab switch (`canLeaveLanguageSection`,
`:1764`), which posts `language.can-leave`. The active pane saves or asks
(`Overview.tryLeaveOverview`). A save still in flight refuses the leave.
Overview's autosave timer is 800 ms (`Overview.svelte` `scheduleOverviewAutosave`).
The leave path clears it when the pane is clean or the user discards.
Lexicon's search timer only re-queries. Hide does not by itself make a
clean Language view dirty. Resume on show is what fixes staleness.

Language stores IPA recents and a grammar hint in `localStorage`
(`IpaPicker.svelte`, `Grammar.svelte`). Instances share `plugin://daena.language`
as the document origin. If WebKit uses one data store per origin, those keys
are shared. That is preferences, not document text. Do not put per-tab
editor state in `localStorage`.

### Sandbox views

World graph and Chronology mount through `SandboxView` with no `entityId`,
`mapEntityId`, `linkId`, or `moduleState` (`+page.svelte` around line 9318).
No caller passes `mapEntityId` or `linkId` today. Both plugins load their
data once at startup (`lore` `mountLoreGraph` render, timeline projection
source) and do not listen for `daena:host-message`. After an entity edit
the graph and the chronology are stale until the webview is created again.
Camera and scroll live only in that page.

The workspace map editor is not that child webview. It is
`NativeVectorMapEditor` in the main webview (`+page.svelte` around line 9926),
remounted by `{#key mapsEditorKey:mapReloadCounter}`. `maps-state` is still
forwarded for a maps plugin view. Keeping the map editor warm is a separate
change to that Svelte view, and this plan does not do it.

v1 does not pool graph, chronology, or maps. They keep closing on leave, so
the next open reads current data. The pool's `mountKey` is there so a later
slice can keep one when the params match and a refresh message exists.
Pooling them now would show a stale graph.

### Visibility and bounds

`embeddedPluginWebviewIds` (`+page.svelte:1669`) is the Language id, the
Writing id, or the open sandbox plugin. One is on screen. The effect at
`:1717` shows those ids unless a modal overlay is open or
`measureAnchoredPopovers` (`:1691`) finds an anchored popover over
`.sandbox-view-container`. `setPluginWebviewVisibleQuietly` calls
`plugin_set_webview_visible` with a plugin id.

With a pool, that effect talks to the shown instance only. Hidden instances
are not in the list, so closing an overlay does not show them. A resize
observer in `SandboxView` runs only while its placeholder is mounted, so a
hidden instance does not follow layout. On show: read the placeholder,
`plugin_resize_webview` to those bounds, then `plugin_set_webview_visible`
true. For a document that is not ready yet, keep today's rule: stay 1×1 and
hidden until `PageLoadEvent::Finished`, then apply the stored bounds and
show (`plugin_mount_webview` around line 333). Re-showing a ready instance
does not wait for another load. Set bounds before `show` so the old
rectangle does not flash.

### Leave and close

Tab switch order stays: dismiss menus, `flushAutoSave`, `canLeaveLanguageSection`,
then capture and restore (`switchToTab`). `flushAutoSave` flushes the shown
Writing instance. Language leave runs only when the current section is
Language. Map leave (`resolveDirtyMapSession`) stays on `leavePluginView`
and is not a pooled webview.

After that flush succeeds, hide runs the suspend steps above. A hidden
Writing instance then has no draft timer and no AI poll. A hidden Language
instance has had its leave guard. Neither should publish into the shell.
The shell drops foreign instance events anyway.

Window close (`+page.svelte` `onCloseRequested` around line 8022) still
acks, flushes, runs the Language leave, then `leavePluginView({ closeWebviews: true })`.
Project close and project switch do the same (`:5455`, `:5420`).
`plugin_close_all_webviews` then destroys every `plugin:` label, including
hidden ones. Hidden instances were flushed when they were hidden. The close
flush covers the one on screen. A hidden instance that can still write
after hide is a bug in suspend, not a second flush on quit.

`app_close.rs` is unchanged. Its handshake still waits for the shell.
`scripts/window-close.test.mjs` stays on that handshake. Add a source check
that quit closes every `plugin:` label, not only the bare one.

### Simpler design

Keep one webview per plugin, on the bare label, and cache editor state
inside the Writing webview per entity. No new label, no session change, no
isolation-script edit. Switching back to Writing reuses the one webview.
The cache can restore undo for a manuscript that is not on screen. Memory
is one Writing process, not one per tab.

That misses the reason for this plan. Two Writing tabs share one webview,
so only one editor is alive, and side-by-side of two manuscripts needs
per-instance labels and per-origin sessions anyway. The cache would be
replaced by the pool. Language would have the same limit: one workspace
for every Language tab, unless Language grew its own per-entity cache.

Per-instance labels match the broker. `SessionRegistry::issue` already
keeps other origins alive and revokes only the origin that is bootstrapping.
The work is the closed list of equality checks above, done once in slice 1
while behavior is unchanged.

Recommend per-instance labels, pool size 3, Writing and Language only.
Take the simpler design only if side-by-side is dropped. Do not build both.

### Contract and drift

Plugin payloads do not grow an instance field. The host adds `instanceId`
on the shell event and overwrites one if the plugin sent it. Parsers in
`writing-bridge.ts` and `language-bridge.ts` ignore unknown fields today,
so they will drop `instanceId` until they are taught to read the host's
copy. `scripts/writing-bridge.test.mjs` and the language bridge tests need
a case that a payload `instanceId` does not choose the instance.

New host messages, posted with `plugin_webview_post`:

- `language.resume`, with the entity id and pane. Document it next to the
  Language surface in `docs/plugins/PLUGIN_PLATFORM.md` (the list around
  line 234).
- A Writing hold on hide, if suspend is a message rather than only
  `writing.flush` plus cancel. Prefer dispose-and-cancel inside the
  existing flush path if that can be done without a new message. If a
  message is added, list it beside `writing.load` in `PLUGIN_PLATFORM.md`
  (around line 244) and in `docs/plans/writing-isolation.md`.

`writing.load`'s keep rule is webview behavior, not a new field. The
revision is already on the message.

`npm run check:plugin-contract` (`package.json`) regenerates nothing by
itself. It runs `scripts/validate-plugin-contract.mjs` and
`scripts/check-plugin-contract-drift.mjs`. A new host message that is not
in the manifest event list does not change the manifest contract. It does
change the source drift checks:

- `scripts/check-plugin-isolation.mjs` (the label predicate, and the
  "one embedded view" assumptions if they are encoded as source matches).
- `scripts/check-writing-ui-drift.mjs` and `scripts/writing-plugin-source.test.mjs`
  when Writing handles `writing.load` or a hold message differently.
- `scripts/check-language-ui-drift.mjs` and `scripts/language-plugin-source.test.mjs`
  when `language.resume` is added.
- `scripts/window-close.test.mjs` for the close sweep.

`gen:plugin-contract` stays as it is unless a manifest event is added.
Do not add a plugin-published event for the instance id.

## Slices

1. **Session lifetime (bug fix, ships first).** TTL is 15 minutes (`crates/daena-plugin-host/src/lib.rs:1712`). The SDK
   bootstraps once and keeps that session id (`packages/plugin-sdk/src/index.ts`
   `ensureSession`, around line 298). A view kept longer than that fails
   RPC. A new bootstrap revokes the old session of that same origin. AI
   polls are bound to the session id, and an expired session cannot serve
   them either way.
   On `session.expired`, `session.revoked`, or `session.stale`, bootstrap
   once and retry the call once. An expired, revoked, or stale session is
   already unusable, so renewing it cannot break a working AI run: a poll
   bound to the old session fails either way. Same origin, so only this
   view's old sessions are revoked. Renewal must not loop: one bootstrap
   per failed call, and a second failure is returned.
   Files: `packages/plugin-sdk/src/index.ts`, plugin-sdk tests, a host test
   beside `revoked_webview_sessions_stay_addressable_by_origin`.
   Manual: leave a Writing tab in the foreground for more than 15 minutes
   and type. Save still works. This slice is worth shipping before the pool,
   because the visible editor has the same cliff today.

2. **Instance labels and routing. Pool size stays 1.** Behavior matches
   today: leave still destroys the webview. Every identity site in the
   table accepts an instance label and rejects the wrong plugin. Commands
   take `instanceId`. Shell events gain `instanceId` from the session.
   Mount still closes other embedded plugin webviews, so a second instance
   cannot stick around yet.
   Files: `crates/daena-plugin-host/src/runtime.rs`, `crates/daena-plugin-host/src/lib.rs`
   (tests), `src-tauri/src/host.rs`, `transfer.rs`, `plugin_webview.rs`,
   `lib.rs`, `src/lib/project/client.ts`, `SandboxView.svelte`, the state
   parsers, `scripts/check-plugin-isolation.mjs`.
   Tests: Rust tests for parse, cross-plugin 403, two origins and two live
   sessions, same origin still revokes, forwarded `instanceId` ignores the
   payload. `scripts/writing-bridge.test.mjs` for the parser. A source test
   that resize, post, show, and unmount take an instance.
   Manual: open Writing, leave it, come back, and confirm it still reloads.
   Open an external plugin window if one is still reachable. Overlay hide
   and show still line up.

3. **Pool, for Writing and Language.** Hide instead of close. N is 3.
   LRU eviction, tab close, archive, disable, and project close destroy the
   right labels. Only the shown instance is visible. Bounds are set before
   show. Suspend on hide (draft timers, AI cancel, focus back to `main`).
   Shell drops other instances' events. Graph, chronology, and the map
   editor still remount.
   Files: `plugin_webview.rs` (stop closing the pool; close by plugin id
   across instance labels), `SandboxView.svelte`, `+page.svelte` (pool map,
   visibility effect, tab close, `switchToTab`), `project_commands.rs` and
   `module_commands.rs` only if slice 1 did not already close every label.
   Tests: a Rust test that mounting instance B does not close instance A
   when the pool allows it, and that disable closes both. A source test
   that the visibility effect names an instance. Extend
   `scripts/window-close.test.mjs` so quit still closes every `plugin:` label.
   Manual, windowed and fullscreen: two Writing tabs, switch, bounds match
   the placeholder, a popover hides only the shown view, the hidden view
   does not take keys. Disable Writing and confirm no `plugin:` webview
   remains. Quit from a Writing tab still saves.

4. **Keep the editor, and resume Language.** `startLoad` implements the
   keep rule. `writing.saved` stores the revision when nothing is unsent.
   `language.resume` reloads the open language and the active pane.
   Files: `packages/modules/writing/src/WritingWorkspace.svelte`,
   `draft-sync.ts` if suspend needs a hook, `writing-bridge.ts` if a hold
   message exists, `packages/modules/language/src/plugin-main.ts`,
   `LanguageWorkspace.svelte`, `plugin-protocol.ts`, `PLUGIN_PLATFORM.md`,
   the writing and language drift scripts.
   Tests: `scripts/writing-plugin-source.test.mjs` for keep, replace, and
   "unsent text is not discarded". Language source test that `language.resume`
   calls the load path. Drift checks in `package.json` `check:writing-ui`
   and `check:language-ui`.
   Manual: type in Writing, undo a few times, switch to Language, switch
   back, undo again. Two tabs on the same manuscript: edit one, return to
   the other, and see the edited text with a fresh editor. Edit one tab,
   switch away and back without anyone else writing, and see undo intact.
   External save, then return, and see the new text. Start an AI rewrite,
   switch away before it finishes, return, and see no surprise insert.
   Full screen, switch away, and see the next tab not fullscreen. Language:
   edit a lexicon elsewhere, return, and see the new row. Cancel a Language
   leave dialog and stay on the tab.

Slice 4 is the one that meets the goal. Slices 1–3 are safe to land without
it, and slice 1 is worth landing on its own. Do not turn slice 3 on for users and describe it as keeping undo.

## Verification

- Rust: label parse, protocol 403 for the wrong plugin, two sessions for
  two origins, AI cancel for one origin, close-by-plugin clears every
  instance label. `cargo test -p daena-plugin-host --locked --offline` and
  the `src-tauri` tests that cover `shell_event_for_plugin_event` and the
  protocol bridge.
- Node: `scripts/writing-bridge.test.mjs`, `scripts/writing-plugin-source.test.mjs`,
  `scripts/language-plugin-source.test.mjs`, `scripts/check-plugin-isolation.mjs`,
  `scripts/window-close.test.mjs`.
- Real macOS app, windowed and fullscreen, after slice 4:
  - the manual checks in slice 4;
  - a modal and a tab-menu popover hide the shown webview and do not reveal
    a hidden one when they close;
  - bounds after the topbar and after a window resize while the tab was hidden;
  - `⌘W` in the Writing editor closes the tab and its instance, not the window;
  - quit flushes and leaves no plugin webview;
  - a third hidden Writing or Language tab causes the least recently used
    hidden one to reload next time.
- Windows and Linux, after slice 4: the same tab switch with focus inside
  Writing, and WebView2 memory for three views.

## Open questions

1. N stays 3 until someone measures a live Writing view and a live Language
   view on macOS and on WebView2. Raise it only with those numbers.
2. On macOS, does `webview.hide()` clear first responder, or can a hidden
   WKWebView keep typing? Slice 3 moves focus to `main` either way. Confirm
   in the app.
3. Do two `plugin://daena.language` webviews share one WebKit data store?
   If they do, `localStorage` keys are shared. Confirm before any per-tab
   state is written there.
4. Is suspend a new `writing.hold` message, or can hide be "flush, cancel
   this origin's AI, dispose draft timers" with no protocol change? Prefer
   no new message if the webview can do it from the existing flush.
5. Graph and chronology join the pool only after they accept a refresh
   message and re-query. Until then they remount. The map editor is the
   main-window `NativeVectorMapEditor`, and warming it is out of this plan.
