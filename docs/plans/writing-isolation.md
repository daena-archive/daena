# Writing Studio isolation

Moves the Writing Studio editor (`daena.writing`) out of the trusted main
webview into an isolated plugin webview. General isolation rules are in
[`PLUGIN_PLATFORM.md`](../plugins/PLUGIN_PLATFORM.md); the Language move in
[`plugin-isolation.md`](plugin-isolation.md) is the precedent.

## Decisions

- **The shell is the only writer.** The webview hosts the editor and hands its
  text to the shell. The shell treats it like typing in its own editor today
  (`updateDocumentBody`, autosave, `persistDocumentSnapshot`), so the document
  and inspector field edits still save in one `saveEntry` call with one
  revision. Conflict handling, recovery copies and the dirty indicator stay in
  the shell. The webview gets no `document.write` capability; the shell accepts
  text only for the entity it gave the webview.
- **What moves:** the content pane for manuscripts and reference pages: the
  editor, its header (title, mode toggle), read mode, the conflict banner, the
  AI rewrite/generate flow, the footer and the empty state.
- **What stays in the shell:** the sidebar, collection tabs, the manuscript
  outline, the create flow, quick open, history, and the inspector (fields,
  relationships, assets, Show on map).
- **AI runs through the broker in the webview.** Writing is the first plugin
  webview to call AI. It needs `ai.text.generate`, a read-only prompt-template
  method, and the project's AI-enabled state at bootstrap.
- **The webview stays mounted across entities.** The shell posts the next
  entity instead of remounting, so outline navigation stays fast.
- **Text larger than a message travels in chunks.** Events and broker calls
  are capped at 256 KiB. Drafts up to a safe inline size go in the state event;
  larger ones go through a chunked hand-off built on the existing transfer
  manager.
- **Leaving flushes, not just asks.** Before any navigation, project close or
  quit, the shell asks the webview for its last text and saves it.

## Protocol

Webview to shell, as `daena.writing/state@1` events (owner-bound, like
Language): `ready`, `draft` (entity id, token, text or a transfer id),
`flush-reply`, `conflict-action` (entity id, token, `reload` | `overwrite` |
`recovery`), `open-link`, `open-entity`, `fullscreen` (entity id, `on`),
`request-rename` and `request-archive` (entity id; the shell checks it is the
selection), `shortcut` (Cmd+K, Cmd+N, Alt+arrows).

Shell to webview, as host messages (capped at 64 KiB): `writing.load` (entity
id, token, revision, editable, placeholder, mode), `writing.flush` (request
id), `writing.saved` (revision), `writing.conflict` (message, canOverwrite),
`writing.save-error`, `writing.editable`, `writing.ai-state`, `writing.fullscreen`
(`on`), `writing.title` (entity id, title). The webview reads
document text itself with `document.list`; broker responses are not size-capped,
so only the webview-to-shell direction needs chunking.

**Tokens.** The shell issues an opaque token with every `writing.load` (a new
entry, or a reload of the same entry after a reload from disk or a conflict
reload). The webview echoes it on every draft and conflict action. The shell
accepts one only if its entity id is the current selection and its token is the
one it last posted; saving never changes the token, and `writing.saved` is
informational.

**Flush.** Before it posts a `writing.load` for the entry the webview already
shows, and from `flushAutoSave` (so every navigation, project close and window
close request), the shell posts `writing.flush` and waits up to 1.5 s for the
`flush-reply`. The reply carries the unsent text as a draft, or null. If it
carries text on a same-entry reload, that text wins and the webview keeps its
editor. Drafts are applied one at a time in arrival order, and a flush returns
only after the ones that arrived before the reply are applied.

**Chunked hand-off.** Text above 128 KiB (UTF-8), or whose JSON encoding exceeds
192 KiB, goes through `text.transfer.begin` / PUT chunks / `text.transfer.commit`
(capability `text.transfer`, at most 32 MiB). The draft names the transfer id and
the shell claims the text once with `plugin_take_text_transfer`.

## Status

Everything below is behind the `daena:writing-webview` developer flag, off by
default. To try it, run `localStorage.setItem("daena:writing-webview","1")` in
the main window's dev console and reload. With the flag off, Writing Studio
uses the old editor in the main webview.

| Slice | State | Commit |
|---|---|---|
| 1. Platform plumbing | Done | `a228aad` |
| 2. Editing and saving through the shell | Done, not yet tried in the real app | `bb34fc0` |
| 3a. Save before the window closes or the app quits | Done, verified in the real app | `186d149` |
| 3b. Shell integration | Done, not yet tried in the real app | |
| 4. AI through the broker | Not started | |
| 5. Clean-up and switch-over | Not started | |

### Done

1. **Platform plumbing.** `daena.writing` is `sandboxed` with the
   `writing-workspace` host-surface view, an owner-bound
   `daena.writing/state@1` event forwarded to the shell as `writing-state`, and
   capabilities for shared assets, asset writes, AI and `text.transfer`. Rust
   serves `writing.js`/`writing.css` with the relaxed inline-style CSP the
   editor needs. Built by `npm run build:writing-ui`, drift-checked by
   `check:writing-ui`. The broker editor host (`src/lib/editor/broker-host.ts`)
   and leave tracker (`src/lib/modules/plugin-leave.ts`) are shared with
   Language. Bundled grants follow the manifest automatically, so existing
   projects get the new capabilities on their next open without a prompt.
2. **Editing and saving.** The webview runs the shared `RichTextEditor` and
   sends drafts; the shell checks entity and token and feeds
   `updateDocumentBody`, so `persistDocumentSnapshot` still saves document and
   fields together. `flushAutoSave` asks the webview first. Conflicts, saves and
   save errors are pushed to the webview. Every transfer now refreshes its
   deadline on each chunk.
3. **Close and quit (3a).** Closing is a handshake (`src-tauri/src/app_close.rs`):
   Rust holds the close, the shell acknowledges, saves and confirms or cancels;
   an 8 s fallback closes only a shell that never acknowledged. On macOS the app
   menu is built in `src-tauri/src/app_menu.rs` (Tauri's default, item for
   item) with a custom Quit so Cmd+Q uses the same handshake. This also fixed
   the old editor, which lost the last second of typing on quit.
4. **Shell integration (3b).**
   - **Guards.** Every path that hides or unmounts the webview, or changes its
     entry, saves first and stops if the save fails. The content-column toggle,
     history back and forward, creating an entry and the Houses tree's person
     link had no flush and now have one. `scripts/writing-shell-guards.test.mjs`
     checks the order for each path; its list is the audit.
   - **Notices.** While the webview shows, `error` and `projectDiagnostics`
     render as a strip above its slot, in normal flow, with the existing message
     styles and dismiss button. The top banner and the toast step aside for it.
   - **Rename and archive.** The header's Rename and double-click post
     `request-rename`, which opens the shell's edit dialog. Archive posts
     `request-archive`, which runs the entry's own archive confirmation and then
     `archiveSelected`. Both check the entry is the selection. A rename that
     leaves the editor in place sends `writing.title`, so the header updates
     without a reload.
   - **Keyboard.** `writingShortcutForKey` maps Cmd/Ctrl+K, Cmd/Ctrl+N and
     Alt+ArrowLeft/Right with the shell's own modifier rules, and the webview
     forwards them as `shortcut`. The shell runs them through `runShellShortcut`,
     which the window listeners use too.
   - **Full screen.** The editor's toggle and Escape publish `fullscreen` with
     the entry id and `on`. The shell keeps `writingFullscreen` out of
     `modalOverlayOpen`, so the webview stays visible, and the content column
     fills the window: the sidebar, top bar, workspace heading, collection,
     inspector and notice strip step aside. The webview sizes its editor to fill.
     The shell ends it with `writing.fullscreen` (`on: false`) on a selection
     change, when the pane is left or unmounted, and on Escape in the shell.
   - **Article and Edit stay webview-owned.** The webview takes its mode from the
     first `writing.load` and keeps its own after that, so switching needs no
     round trip. The open question from the old 3b list is settled this way.
   - **Tests.** `writing-bridge` and `writing-plugin-source` cover the new and
     changed messages, their rejection cases and the shortcut table.
     `writing-shell-guards` (in `test:shell`) covers the flush order and the shell
     wiring.

### Next: 4. AI

Rewrite and generate through `ai.request.*` from the webview (Writing is the
first plugin webview to call AI). Map the shell's `retrievalQuery`,
`retrievalDepth` and `includeRetrieval` to `retrievalPolicy`. Add a read-only
broker method for prompt templates and pass the project's AI-enabled state at
bootstrap (`writing.ai-state` exists for updates). Field fill stays in the
shell inspector.

### Then: 5. Clean-up and switch-over

- Turn the webview on by default and remove the flag.
- Remove the old Writing editor path from `+page.svelte`, then drop
  `document.write` from the Writing manifest (the shell's trusted Writing
  context needs it until then) and update the fuzz corpus copy.
- Update the tests that inspect `+page.svelte` for Writing code
  (`scripts/writing-outline.test.mjs`, `async-entity-picker.test.mjs`,
  `ui-ux.test.mjs`).
- Update `ARCHITECTURE.md`, `UI_UX.md` and `PLUGIN_SDK.md`, which still
  describe Writing as host-owned.
- Real-app check, windowed and fullscreen: typing and autosave, switching
  entries right after typing, a document over 128 KiB, a conflict (Reload, Keep
  mine, recovery copy), images, mentions, links, overlays, quit right after
  typing.

### Known limits

- Remounting is avoided, but a name looked up for an entity link is cached per
  editor; renames show after reopening.
- Non-macOS builds of the new menu and close code have not been compiled here.
- Two clippy warnings predate this work (`host.rs` useless conversion,
  `plugin_webview_url` argument count).
- Full screen hides the notice strip, as specified, so an `error` raised while
  it is on waits until full screen ends. Save errors still show in the webview
  header.
- Alt+ArrowLeft and Alt+ArrowRight go back and forward from inside the editor
  too, as they already do from the shell editor, so Option+arrow word movement
  is taken there.
