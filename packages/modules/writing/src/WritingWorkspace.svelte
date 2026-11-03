<script lang="ts">
import { onMount } from "svelte";
import MarkdownArticle from "../../../../src/lib/markdown/MarkdownArticle.svelte";
import { normalizeDocument } from "../../../../src/lib/markdown/normalize.ts";
import { defaultEditorHost } from "../../../../src/lib/editor/host.ts";
import {
  entityIdsInMarkdown,
  loadWritingDocument,
  type WritingDocument,
  type WritingDocumentRpc,
} from "./document-source.ts";
import type { WritingWorkspaceBridge } from "./plugin-protocol.ts";

/**
 * Slice 1 of the Writing isolation: the webview renders the entry the shell selected, read-only.
 * `bridge` connects it to the shell; the shell posts `writing.load` for every selection change, and this
 * component stays mounted across them.
 */
let {
  rpc,
  bridge,
  initialEntityId = null,
}: { rpc: WritingDocumentRpc; bridge: WritingWorkspaceBridge; initialEntityId?: string | null } = $props();

type MentionedEntity = { id: string; name: string; deleted: boolean };

let loading = $state(false);
let loadError = $state("");
let shown = $state<WritingDocument | null>(null);
let mentioned = $state<MentionedEntity[]>([]);
/** True once an entity was requested; before that the pane shows the "select an entry" state. */
let requested = $state(false);
let loadToken = 0;

async function show(entityId: string) {
  const token = ++loadToken;
  requested = true;
  loading = true;
  loadError = "";
  try {
    const document = await loadWritingDocument(rpc, entityId, normalizeDocument);
    if (token !== loadToken) return;
    shown = document;
    if (!document) loadError = "This entry is no longer available.";
    mentioned = [];
    if (document) void resolveMentions(token, document.markdown);
  } catch (cause) {
    if (token !== loadToken) return;
    shown = null;
    mentioned = [];
    loadError = cause instanceof Error ? cause.message : String(cause);
  } finally {
    if (token === loadToken) loading = false;
  }
}

/** Entity references render as missing unless their names are known, so names are looked up like the editor does. */
async function resolveMentions(token: number, markdown: string) {
  const ids = entityIdsInMarkdown(markdown);
  const resolve = defaultEditorHost()?.resolveEntityNames;
  if (ids.length === 0 || !resolve) return;
  try {
    const names = await resolve(ids);
    if (token !== loadToken) return;
    mentioned = Object.entries(names).map(([id, name]) => ({ id, name, deleted: false }));
  } catch {
    // The references keep the labels the text gives them.
  }
}

/** Links leave through the shell; anchors inside the article stay in the page. */
function handleArticleClick(event: MouseEvent) {
  const anchor = (event.target as Element | null)?.closest?.("a[href]");
  if (!anchor) return;
  const href = anchor.getAttribute("href") ?? "";
  if (href.startsWith("#")) return;
  event.preventDefault();
  if (/^(?:https?:|mailto:)/i.test(href)) bridge.publish({ kind: "open-link", href });
}

onMount(() => {
  if (initialEntityId) void show(initialEntityId);
  return bridge.onHostMessage((message) => {
    if (message.type === "writing.load") void show(message.entityId);
  });
});
</script>

<section class="writing-workspace">
  <div class="writing-panel">
    {#if shown}
      <header class="writing-header">
        <h2>{shown.title}</h2>
      </header>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="writing-scroll" onclick={handleArticleClick}>
        <MarkdownArticle
          markdown={shown.markdown}
          entities={mentioned}
          onOpenEntity={(entityId) => bridge.publish({ kind: "open-entity", entityId })} />
      </div>
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
  flex: 0 0 auto;
  padding-bottom: 14px;
}
.writing-header h2 {
  margin: 0;
  color: var(--ink);
  font-family: var(--font-display, Georgia, serif);
  font-size: 28px;
  line-height: 1.2;
  overflow-wrap: anywhere;
}
/* The article scrolls inside the panel; a scrolling document would draw the macOS scroll-edge effect. */
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
