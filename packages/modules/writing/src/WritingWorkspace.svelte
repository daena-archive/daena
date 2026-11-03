<script lang="ts">
import { onMount } from "svelte";
import RichTextEditor from "../../../../src/lib/editor/RichTextEditor.svelte";
import MarkdownArticle from "../../../../src/lib/markdown/MarkdownArticle.svelte";
import { normalizeDocument } from "../../../../src/lib/markdown/normalize.ts";
import { defaultEditorHost } from "../../../../src/lib/editor/host.ts";
import { createDraftSync } from "./draft-sync.ts";
import {
  entityIdsInMarkdown,
  loadWritingDocument,
  type WritingDocument,
  type WritingDocumentRpc,
} from "./document-source.ts";
import type {
  WritingConflictAction,
  WritingHostMessage,
  WritingMode,
  WritingWorkspaceBridge,
} from "./plugin-protocol.ts";

/**
 * The Writing editor. The shell selects the entry and posts `writing.load`; this component stays mounted
 * across loads. It shows the entry with the shared rich-text editor and hands every change to the shell as a
 * draft. It never saves: the shell does, and answers with `writing.saved`, `writing.conflict` or
 * `writing.save-error`.
 */
let {
  rpc,
  bridge,
  initialEntityId = null,
}: { rpc: WritingDocumentRpc; bridge: WritingWorkspaceBridge; initialEntityId?: string | null } = $props();

type MentionedEntity = { id: string; name: string; deleted: boolean };
/** One load of one entry. A reload gets a new view (and so a fresh editor), as does every other entry. */
type View = WritingDocument & { token: string; placeholder: string };

let loading = $state(false);
let loadError = $state("");
let view = $state<View | null>(null);
let mentioned = $state<MentionedEntity[]>([]);
/** True once an entity was requested; before that the pane shows the "select an entry" state. */
let requested = $state(false);
/** The shell's say on editing; also false while an entry loads or a conflict is shown. */
let shellEditable = $state(false);
let mode = $state<WritingMode>("edit");
let modeChosen = false;
let fullscreen = $state(false);
let conflict = $state<{ message: string; canOverwrite: boolean } | null>(null);
let saveError = $state("");
/** Text waiting to be published, then published and waiting for the shell's `writing.saved`. */
let pending = $state(false);
let awaitingSave = $state(false);
let savedNote = $state(false);
let editorRef = $state<{ flushPendingChanges: () => void } | null>(null);
let loadSeq = 0;

const sync = createDraftSync({
  publish: (payload) => bridge.publish(payload),
  transfer: (text) => bridge.transferText(text),
  onError: (cause) => console.warn("A Writing draft could not be sent", cause),
  onPending: (waiting) => {
    pending = waiting;
    if (waiting) {
      awaitingSave = false;
      savedNote = false;
    } else {
      awaitingSave = true;
    }
  },
});

const editable = $derived(shellEditable && !loading && !conflict && view !== null);
const statusLabel = $derived(
  pending ? "Unsaved changes" : awaitingSave ? "Saving…" : savedNote ? "Saved" : "",
);

function startLoad(message: Extract<WritingHostMessage, { type: "writing.load" }>) {
  const seq = ++loadSeq;
  // Whatever the shell had not received is the shell's to discard or have flushed first.
  sync.reset();
  awaitingSave = false;
  savedNote = false;
  conflict = null;
  saveError = "";
  shellEditable = message.editable;
  if (!modeChosen) {
    mode = message.mode;
    modeChosen = true;
  }
  void show(seq, message);
}

async function show(seq: number, message: Extract<WritingHostMessage, { type: "writing.load" }>) {
  requested = true;
  loading = true;
  loadError = "";
  try {
    const document = await loadWritingDocument(rpc, message.entityId, normalizeDocument);
    if (seq !== loadSeq) return;
    view = document ? { ...document, token: message.token, placeholder: message.placeholder } : null;
    if (!document) loadError = "This entry is no longer available.";
    mentioned = [];
    if (document) void resolveMentions(seq, document.markdown);
  } catch (cause) {
    if (seq !== loadSeq) return;
    view = null;
    mentioned = [];
    loadError = cause instanceof Error ? cause.message : String(cause);
  } finally {
    if (seq === loadSeq) loading = false;
  }
}

/** Entity references render as missing unless their names are known, so names are looked up like the editor does. */
async function resolveMentions(seq: number, markdown: string) {
  const ids = entityIdsInMarkdown(markdown);
  const resolve = defaultEditorHost()?.resolveEntityNames;
  if (ids.length === 0 || !resolve) return;
  try {
    const names = await resolve(ids);
    if (seq !== loadSeq) return;
    mentioned = Object.entries(names).map(([id, name]) => ({ id, name, deleted: false }));
  } catch {
    // The references keep the labels the text gives them.
  }
}

/**
 * Editor output for `source`. Changes from an editor that a newer load replaced are dropped; changes made
 * while the editor is read-only for the shell's reasons (a conflict, say) are still the user's text and go on.
 */
function handleChange(source: View, markdown: string) {
  if (source !== view) return;
  // The editor treats `value` as its source of truth when it is not focused, so it follows what it emitted.
  source.markdown = markdown;
  sync.change(source.entityId, source.token, markdown);
}

function publishNow() {
  editorRef?.flushPendingChanges();
  void sync.publishNow();
}

function setMode(next: WritingMode) {
  if (next === mode) return;
  // Leaving the editor sends what it holds without waiting for the pause, like the shell's flush.
  if (mode === "edit") publishNow();
  mode = next;
  if (next === "read" && view) void resolveMentions(loadSeq, view.markdown);
}

function chooseConflict(action: WritingConflictAction) {
  if (!view) return;
  void bridge.publish({ kind: "conflict-action", entityId: view.entityId, token: view.token, action });
}

/** Links leave through the shell; anchors inside the article stay in the page. */
function handleArticleClick(event: MouseEvent) {
  const anchor = (event.target as Element | null)?.closest?.("a[href]");
  if (!anchor) return;
  const href = anchor.getAttribute("href") ?? "";
  if (href.startsWith("#")) return;
  event.preventDefault();
  if (/^(?:https?:|mailto:)/i.test(href)) void bridge.publish({ kind: "open-link", href });
}

function handleHostMessage(message: WritingHostMessage) {
  switch (message.type) {
    case "writing.load":
      startLoad(message);
      return;
    case "writing.flush":
      // Pending editor changes become a draft first; the reply carries it, or null when nothing is unsent.
      editorRef?.flushPendingChanges();
      void sync.flush(message.requestId);
      return;
    case "writing.editable":
      shellEditable = message.editable;
      // The shell is editable only when it holds no conflict, so a banner left over from one is stale.
      if (message.editable) conflict = null;
      return;
    case "writing.saved":
      if (message.entityId !== view?.entityId) return;
      conflict = null;
      saveError = "";
      if (!sync.hasUnsent()) {
        awaitingSave = false;
        savedNote = true;
      }
      return;
    case "writing.conflict":
      if (message.entityId !== view?.entityId) return;
      conflict = { message: message.message, canOverwrite: message.canOverwrite };
      return;
    case "writing.save-error":
      if (message.entityId !== view?.entityId) return;
      saveError = message.message;
      return;
    case "writing.ai-state":
      // The AI flow arrives in a later slice.
      return;
  }
}

onMount(() => {
  if (initialEntityId) {
    // The shell posts writing.load for the selection once the webview reports ready; this only shows progress.
    requested = true;
    loading = true;
  }
  const off = bridge.onHostMessage(handleHostMessage);
  return () => {
    off();
    sync.dispose();
  };
});
</script>

<section class="writing-workspace">
  <div class="writing-panel">
    {#if view}
      {@const source = view}
      <header class="writing-header">
        <h2>{source.title}</h2>
        <div class="writing-controls">
          <div class="writing-mode-toggle" aria-label="Document mode">
            <button type="button" class:active={mode === "read"} aria-pressed={mode === "read"} onclick={() => setMode("read")}
              >Article</button
            ><button type="button" class:active={mode === "edit"} aria-pressed={mode === "edit"} onclick={() => setMode("edit")}
              >Edit</button>
          </div>
          <span class="writing-status" role="status" aria-live="polite">{statusLabel}</span>
        </div>
      </header>
      {#if conflict}
        <div class="writing-conflict" role="alert">
          <div class="writing-conflict-copy">
            <strong>{conflict.canOverwrite ? "This entry changed elsewhere" : "Canonical source needs attention"}</strong>
            <p>{conflict.message}</p>
          </div>
          <div class="writing-conflict-actions">
            <button type="button" onclick={() => chooseConflict("reload")}>Reload disk</button>
            <button type="button" disabled={!conflict.canOverwrite} onclick={() => chooseConflict("overwrite")}
              >Keep mine (overwrite)</button
            >
            <button type="button" onclick={() => chooseConflict("recovery")}>Save recovery copy</button>
          </div>
        </div>
      {/if}
      {#if saveError}
        <p class="writing-save-error" role="alert">{saveError}</p>
      {/if}
      {#if mode === "read"}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="writing-scroll" onclick={handleArticleClick}>
          <MarkdownArticle
            markdown={source.markdown}
            entities={mentioned}
            onOpenEntity={(entityId) => void bridge.publish({ kind: "open-entity", entityId })} />
        </div>
      {:else}
        <div class="writing-editor">
          {#key source}
            <RichTextEditor
              bind:this={editorRef}
              value={source.markdown}
              entityId={source.entityId}
              defaultNamespace="writing"
              {editable}
              {fullscreen}
              placeholder={source.placeholder}
              onChange={(markdown) => handleChange(source, markdown)}
              onFullscreenChange={(next) => (fullscreen = next)}
              onSaveRequest={publishNow} />
          {/key}
        </div>
      {/if}
    {:else if loadError}
      <p class="writing-state writing-error" role="alert">{loadError}</p>
    {:else if loading}
      <p class="writing-state" role="status" aria-live="polite">Loading entry…</p>
    {:else}
      <div class="writing-state" role="status">
        <h3>{requested ? "Nothing to show." : "Your draft is waiting."}</h3>
        <p>Select an entry from the collection, or create something new to begin writing.</p>
      </div>
    {/if}
  </div>
</section>

<style>
.writing-workspace {
  display: block;
  height: 100%;
  min-height: 0;
  color: var(--ink);
}
.writing-panel {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: hidden;
  /* The shell runs this webview flush to the window's bottom edge, so the panel has no bottom edge. */
  border: 1px solid var(--line);
  border-bottom: 0;
  border-radius: 12px 12px 0 0;
  background: var(--surface);
  padding: 24px 25px 0;
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.05));
  box-sizing: border-box;
}
.writing-header {
  display: flex;
  flex: 0 0 auto;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding-bottom: 14px;
}
.writing-header h2 {
  min-width: 0;
  margin: 0;
  color: var(--ink);
  font-family: var(--font-display, Georgia, serif);
  font-size: 28px;
  line-height: 1.2;
  overflow-wrap: anywhere;
}
.writing-controls {
  display: flex;
  flex: 0 0 auto;
  align-items: flex-end;
  flex-direction: column;
  gap: 7px;
}
.writing-mode-toggle {
  display: flex;
  gap: 2px;
  padding: 2px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
}
.writing-mode-toggle button {
  padding: 5px 9px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
  font-size: 10px;
  font-weight: 700;
}
.writing-mode-toggle button.active {
  background: var(--surface);
  box-shadow: var(--shadow-sm);
  color: var(--ink);
}
.writing-status {
  min-height: 14px;
  color: var(--ink-faint);
  font-size: 11px;
}
.writing-conflict {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px 16px;
  margin-bottom: 12px;
  padding: 10px 12px;
  border: 1px solid var(--theme-warning-border, #d9a46a);
  border-left-width: 3px;
  border-radius: 8px;
  background: var(--theme-warning-bg, #fff8ee);
  font-size: 12px;
  line-height: 1.45;
}
.writing-conflict-copy strong {
  color: var(--ink);
}
.writing-conflict-copy p {
  margin: 2px 0 0;
  color: var(--ink-soft);
}
.writing-conflict-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.writing-conflict-actions button {
  padding: 7px 10px;
  border: 1px solid var(--theme-warning-border, #ded8cd);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
  font-size: 12px;
}
.writing-conflict-actions button:hover:not(:disabled) {
  background: var(--surface-muted);
  color: var(--ink);
}
.writing-conflict-actions button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}
.writing-save-error {
  flex: 0 0 auto;
  margin: 0 0 12px;
  color: var(--danger);
  font-size: 12px;
}
/* The editor fills the panel and scrolls inside itself; a scrolling document would draw the macOS scroll-edge effect. */
.writing-editor {
  display: flex;
  min-height: 0;
  flex: 1 1 auto;
  flex-direction: column;
}
.writing-scroll {
  min-height: 0;
  flex: 1 1 auto;
  overflow: auto;
  padding-bottom: 24px;
}
.writing-state {
  margin: auto;
  max-width: 420px;
  color: var(--ink-soft);
  text-align: center;
}
.writing-state h3 {
  margin: 0 0 6px;
  color: var(--ink);
  font-family: var(--font-display, Georgia, serif);
}
.writing-state p {
  margin: 0;
}
.writing-error {
  color: var(--danger);
}
@media (max-width: 760px) {
  .writing-panel {
    padding: 18px 14px 0;
  }
}
</style>
