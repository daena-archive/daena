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
`recovery`), `open-link`, `open-entity`, `fullscreen`, `request-rename`,
`request-archive`, `shortcut` (Cmd+K, Cmd+N, Alt+arrows).

Shell to webview, as host messages (capped at 64 KiB): `writing.load` (entity
id, token, revision, editable, placeholder, mode), `writing.flush` (request
id), `writing.saved` (revision), `writing.conflict` (message, canOverwrite),
`writing.save-error`, `writing.editable`, `writing.ai-state`. The webview reads
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

## Slices

1. **Platform plumbing.** Manifest becomes `sandboxed` with the
   `writing-workspace` host-surface view, `event.publish:daena.writing/state@1`,
   `asset.read:shared`, `asset.write:self`, `ai.text.generate`; drop
   `document.write`. Rust asset routes, owner-bound event forwarding, relaxed
   inline-style CSP for the editor, build script and drift check, fuzz corpus
   and manifest tests. Move the reusable Language pieces (broker editor host,
   leave tracker) into shared modules. The webview mounts in the Writing
   content pane and renders the selected document read-only.
2. **Editing and saving.** Done. Editor in the webview; `draft` events feed the
   shell's existing autosave; chunked hand-off for large text; flush on leave
   and close; conflict, saved and save-error states pushed back to the webview.
   Window close and quit are not covered yet: the Rust close handler refuses
   project commands (`begin_project_command`) as soon as the close request
   arrives, before the main webview can flush anything.
3. **Shell integration.** Keep the webview alive across entities; guard every
   navigation path; rename, archive and recovery through shell messages;
   keyboard forwarding; fullscreen owned by the webview.
4. **AI.** Rewrite and generate through `ai.request.*`, retrieval mapped to
   `retrievalPolicy`, prompt templates through a new read-only broker method,
   AI-enabled state at bootstrap.
5. **Clean-up.** Remove the old editor path from the main webview, update the
   tests that inspect `+page.svelte`, update `ARCHITECTURE.md`, `UI_UX.md`,
   `PLUGIN_SDK.md`, and run the real-app check.
