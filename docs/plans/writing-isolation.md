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
  webview to call AI. It needs `ai.text.generate`; the project's AI-enabled
  state, the provider's name and the editor prompt templates reach it in the
  shell's `writing.ai-state` message, not through a broker method.
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
`writing.save-error`, `writing.editable`, `writing.ai-state` (`enabled`, `providerName`, `templates`),
`writing.fullscreen` (`on`), `writing.title` (entity id, title). The webview reads
document text itself with `document.list`. The host does not cap broker
responses, but the SDK client does (256 KiB by default), so Writing raises its
client's limit to 96 MiB (`WRITING_RPC_MAX_RESPONSE_BYTES`); only the
webview-to-shell direction needs chunking.

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
| 3b. Shell integration | Done, not yet tried in the real app | `4212e99` |
| 4. AI through the broker | Done, not yet tried in the real app | |
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

5. **AI through the broker (4).** The rewrite/generate flow runs in the webview
   when it is shown. The shell's own dialog and handlers stay for the old
   editor (flag off) and go in slice 5. Field fill stays in the shell.
   - **State and templates travel in `writing.ai-state`**, not through a broker
     method. The message is `{enabled, providerName, templates}`; the shell
     posts it after the webview's `ready` (every `ready` counts, so a webview
     that restarted gets it again) and whenever `projectInfo.aiEnabled`, the
     provider's name or `aiPromptTemplates` change. This replaces the planned
     read-only prompt-template method: the shell already holds the merged
     templates, and a push needs no new RPC method, capability or contract
     artifact and updates the webview when settings change. Templates are the
     shell's merged `kind === "editor"` ones with `id`, `label`, `instruction`,
     `requiresSelection`, `enabled` and `kind`. Disabled ones stay in the list,
     because the editor shows its built-in menu when it gets none. The message
     is built by `writingAiStateMessage` and kept under 60 KiB: when the
     templates do not fit, trailing ones are dropped (at most 64 are sent), and
     posting never fails on size. The webview parser rejects the whole message
     on any malformed template, a repeated id or a kind other than `editor`.
   - **Dialog.** `AiRewriteDialog.svelte` is the shell dialog ported (same
     titles and buttons, `AiProposalPreview`, usage line, messages), with the
     errors shown inside it. The rules live in pure modules: `ai-flow.ts`
     (retrieval policy and query, source text, accept guard, event reducer) and
     `ai-run.ts` (the request lifecycle, driven by an injected broker and
     timers). `validateAiProposal` moved to `src/lib/ai/proposal.ts` and the
     shell imports it from there. The editor is read-only while the dialog is
     open. Escape closes it when no request runs.
   - **Start.** The pending draft goes out at once (editor flush, then
     `publishNow`, without waiting for `writing.saved`); the entity, token and
     markdown at that moment are the snapshot. The request is
     `generate_text` with `immediateContext.selection` as in the shell, and a
     `retrievalPolicy` built the way `direct_retrieval_context` builds it:
     `related`, seed = the entity, depth 2, 8 passages, shared fields, no kind
     filter, `query` = name + instruction + plain selection cut to 4000
     characters (`project` mode if there were no entity).
   - **Poll, cancel, release.** `ai.request.poll` every 200 ms, deduped by
     sequence, until a terminal phase. The broker keeps only the last 64 events
     per request, so the run notices a sequence gap and takes the proposal from
     the terminal event's output (or `ai.request.result` when a completed event
     has none). After the end the run calls `ai.request.citations` and ignores
     the answer, so the broker drops its record. Cancel calls
     `ai.request.cancel` and waits for the `cancelled` event. Discard, Escape,
     any new `writing.load` and unmounting the dialog cancel and release a
     running request. Five failed polls in a row end the run.
   - **Accept** only when entity, token and the editor's markdown still equal
     the snapshot (the shell's message otherwise), then `validateAiProposal`,
     then `insertAiTextAtRequest` or `replaceAiTextWithMarkdown` with the shell's
     messages for a lost position. The new text is sent as a draft at once and
     the dialog closes; saving and conflicts stay with the shell.
   - **Broker fixes this needed.** (a) The sandboxed `/__rpc` path built its AI
     context with no settings store, so `ai.request.start` could never find the
     project's provider ("Configure an AI provider for this project first")
     for any isolated plugin. It now takes the managed `SharedSettings`.
     (b) Retrieval parity: the broker's retrieval is lexical only, while the
     shell's `ai_generate_text` also adds semantic (embedding) passages. The
     plugin protocol handler runs on the main thread, so the embedding call
     cannot run in `ai.request.start`. For `generate_text` with a `related` or
     `project` policy that has a query, and a provider with an embedding model,
     the request's own worker now runs a `ContextPreparation` after `started`:
     it embeds the query, merges the semantic passages with
     `build_retrieval_context_with_semantic` and replaces the request's context
     and citations. It holds no project lease or runtime lock across the network
     call, and every failure (no index, embedding error, project switched, or a
     context that no longer fits the input limit) keeps the lexical context the
     request started with. `semantic_retrieval_passages` was split into a
     blocking function and its async wrapper, and no longer holds the AI
     runtime lock while it embeds (`ai.request.poll` takes that lock on the
     main thread). Capability checks: `generate_text` is authorized by
     `ai.text.generate`, and the Writing manifest's `document.read`,
     `relationship.read`, `field.read:self` and `search.query` satisfy the
     retrieval checks (a broker test sends the webview's exact policy).
   - **Tests.** `writing-plugin-source` covers the `writing.ai-state` shape
     (accepted and rejected), the policy and query mapping, source text, the
     accept guard, the event reducer (dedupe, terminal phases, gaps, usage) and
     the run (polling, cancel, dispose, release, failures) on fake timers.
     `writing-bridge` covers `writingAiStateMessage` (filtering, caps, the size
     budget) and the shell wiring. Rust has tests for the context
     preparation, the deferral conditions and the broker with the Writing
     policy.

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
- AI retrieval reads the saved document. The run sends the pending draft at
  once but does not wait for the shell to save it, so the entity's own text in
  the retrieved context can lag the editor by about one autosave interval. The
  selection or cursor context sent with the request is always current.
- Semantic passages are added only to `generate_text` (not
  `generate_structured`), and they arrive after the request starts, so a
  project with an embedding model waits for the embedding call before the first
  token. Without an embedding model, an index or a query the context is
  lexical, as before.
- The webview dialog covers the Writing pane, not the whole window as the
  shell's does. Escape does nothing while a request runs (the shell's window
  handler closed and cancelled even then).
- Fixed during the slice-4 review: "Custom instruction" could never start a run
  in the shell or the webview. Every template but `generate` opened in rewrite
  mode, and templates that do not require a selection dropped it, so the
  dialog had nothing to rewrite. Both now use `aiActionRequest`
  (`src/lib/ai/promptTemplates.ts`): `generate` writes at the cursor, a template
  that requires a selection rewrites it, and any other template rewrites the
  selection when there is one and writes at the cursor when there is none. The
  instruction placeholder follows the mode.
- A request keeps running if the shell hides or unmounts the webview while it
  works, because the page is torn down without a chance to cancel; it ends at
  the provider or at the deadline (5 minutes by default, as in the shell).
- Fixed during the slice-4 review: the plugin RPC client caps responses at
  256 KiB by default, and Writing used the default, so any document whose
  `document.list` answer was over 256 KiB (about 40,000 words) could not be
  opened in the webview. Writing's client now accepts up to 96 MiB. The Language
  webview still uses the default. The slice-5 real-app check with a large
  document covers this.
