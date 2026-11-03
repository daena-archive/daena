import assert from "node:assert/strict";
import { PluginLeaveTracker } from "../src/lib/modules/plugin-leave.ts";
import { LanguageLeaveTracker } from "../src/lib/modules/language-bridge.ts";
import {
  WRITING_DRAFT_INLINE_LIMIT_BYTES,
  isWritingShortcut,
  parseWritingStateEvent,
  writingLoadMessage,
  writingWebviewHandles,
} from "../src/lib/modules/writing-bridge.ts";
import { openableLinkHref } from "../src/lib/modules/plugin-links.ts";

// The Language alias is the generic tracker, so Language's leave behaviour is unchanged.
assert.equal(LanguageLeaveTracker, PluginLeaveTracker);

// Link opening is shared with Language: only http, https and mailto leave the app.
assert.equal(openableLinkHref("https://example.com/a"), "https://example.com/a");
assert.equal(openableLinkHref("javascript:alert(1)"), null);

// The shell renders manuscripts and reference pages in the webview; other Writing tabs keep the shell editor.
assert.equal(writingWebviewHandles("daena.writing:manuscript"), true);
assert.equal(writingWebviewHandles("daena.writing:reference-page"), true);
assert.equal(writingWebviewHandles("daena.lore:person"), false);
assert.equal(writingWebviewHandles("manuscript"), false);
assert.equal(writingWebviewHandles(null), false);
assert.equal(writingWebviewHandles(undefined), false);

for (const shortcut of ["quick-open", "create", "history-back", "history-forward"]) {
  assert.equal(isWritingShortcut(shortcut), true, shortcut);
}
assert.equal(isWritingShortcut("save"), false);
assert.equal(isWritingShortcut(undefined), false);

// Payload validation: unknown kinds and malformed shapes are ignored.
assert.equal(parseWritingStateEvent(null), null);
assert.equal(parseWritingStateEvent("ready"), null);
assert.equal(parseWritingStateEvent([{ kind: "ready" }]), null);
assert.equal(parseWritingStateEvent({}), null);
assert.equal(parseWritingStateEvent({ kind: "pane", pane: "overview" }), null, "unknown kinds ignored");
assert.equal(parseWritingStateEvent({ kind: "ready", entityId: 7 }), null);
assert.equal(parseWritingStateEvent({ kind: "ready", entityId: "x".repeat(129) }), null);
assert.deepEqual(parseWritingStateEvent({ kind: "ready" }), { kind: "ready", entityId: null });
assert.deepEqual(parseWritingStateEvent({ kind: "ready", entityId: "e1" }), { kind: "ready", entityId: "e1" });

// Links and entity ids.
assert.equal(parseWritingStateEvent({ kind: "open-link" }), null);
assert.equal(parseWritingStateEvent({ kind: "open-link", href: 3 }), null);
assert.equal(parseWritingStateEvent({ kind: "open-link", href: `https://example.com/${"a".repeat(5000)}` }), null);
assert.deepEqual(parseWritingStateEvent({ kind: "open-link", href: "https://example.com" }), {
  kind: "open-link",
  href: "https://example.com",
});
for (const kind of ["open-entity", "request-rename", "request-archive"]) {
  assert.equal(parseWritingStateEvent({ kind }), null, `${kind} needs an entity`);
  assert.equal(parseWritingStateEvent({ kind, entityId: "" }), null, `${kind} drops empty ids`);
  assert.equal(parseWritingStateEvent({ kind, entityId: 42 }), null, `${kind} ids are strings`);
  assert.equal(parseWritingStateEvent({ kind, id: "e-1" }), null, `${kind} field is entityId`);
  assert.equal(parseWritingStateEvent({ kind, entityId: "x".repeat(129) }), null, `${kind} drops overlong ids`);
  assert.deepEqual(parseWritingStateEvent({ kind, entityId: "e-1" }), { kind, entityId: "e-1" });
}

// Fullscreen and shortcuts.
assert.equal(parseWritingStateEvent({ kind: "fullscreen" }), null);
assert.equal(parseWritingStateEvent({ kind: "fullscreen", enabled: "yes" }), null);
assert.deepEqual(parseWritingStateEvent({ kind: "fullscreen", enabled: true }), { kind: "fullscreen", enabled: true });
assert.equal(parseWritingStateEvent({ kind: "shortcut" }), null);
assert.equal(parseWritingStateEvent({ kind: "shortcut", shortcut: "save" }), null);
assert.deepEqual(parseWritingStateEvent({ kind: "shortcut", shortcut: "quick-open" }), {
  kind: "shortcut",
  shortcut: "quick-open",
});

// Drafts: text or a transfer reference, never both, never neither.
const inline = { entityId: "e1", baseRevision: "r1", text: "Chapter one" };
assert.deepEqual(parseWritingStateEvent({ kind: "draft", draft: inline }), { kind: "draft", draft: inline });
assert.deepEqual(parseWritingStateEvent({ kind: "draft", draft: { ...inline, text: "" } }), {
  kind: "draft",
  draft: { ...inline, text: "" },
});
const chunked = { entityId: "e1", baseRevision: "r1", transfer: "t-1" };
assert.deepEqual(parseWritingStateEvent({ kind: "draft", draft: chunked }), { kind: "draft", draft: chunked });
assert.equal(parseWritingStateEvent({ kind: "draft" }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: null }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, transfer: "t-1" } }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { entityId: "e1", baseRevision: "r1" } }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, entityId: "" } }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, baseRevision: 3 } }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...chunked, transfer: "" } }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, text: 12 } }), null);
{
  const atLimit = "a".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES);
  assert.notEqual(parseWritingStateEvent({ kind: "draft", draft: { ...inline, text: atLimit } }), null);
  assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, text: `${atLimit}a` } }), null);
  // The limit counts UTF-8 bytes, so multi-byte text hits it sooner than its length suggests.
  const wide = "é".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES / 2 + 1);
  assert.ok(wide.length < WRITING_DRAFT_INLINE_LIMIT_BYTES);
  assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, text: wide } }), null);
}

// Flush replies carry the draft, or null when nothing is unsaved.
assert.deepEqual(parseWritingStateEvent({ kind: "flush-reply", requestId: "f-1", draft: null }), {
  kind: "flush-reply",
  requestId: "f-1",
  draft: null,
});
assert.deepEqual(parseWritingStateEvent({ kind: "flush-reply", requestId: "f-1", draft: inline }), {
  kind: "flush-reply",
  requestId: "f-1",
  draft: inline,
});
assert.equal(parseWritingStateEvent({ kind: "flush-reply", draft: null }), null, "requestId is required");
assert.equal(parseWritingStateEvent({ kind: "flush-reply", requestId: "", draft: null }), null);
assert.equal(parseWritingStateEvent({ kind: "flush-reply", requestId: "f-1" }), null, "draft must be present");
assert.equal(parseWritingStateEvent({ kind: "flush-reply", requestId: "f-1", draft: { entityId: "e1" } }), null);

// The load message is read-only unless the shell says otherwise.
assert.deepEqual(writingLoadMessage("e1", "r1"), {
  type: "writing.load",
  entityId: "e1",
  revision: "r1",
  editable: false,
  placeholder: "",
  mode: "read",
});
assert.deepEqual(writingLoadMessage("e1", "", { editable: true, placeholder: "Write…", mode: "edit" }), {
  type: "writing.load",
  entityId: "e1",
  revision: "",
  editable: true,
  placeholder: "Write…",
  mode: "edit",
});

console.log("writing bridge checks passed");
