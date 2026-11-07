import assert from "node:assert/strict";
import fs from "node:fs";
import { PluginLeaveTracker } from "../src/lib/modules/plugin-leave.ts";
import { LanguageLeaveTracker } from "../src/lib/modules/language-bridge.ts";
import {
  WRITING_DRAFT_INLINE_LIMIT_BYTES,
  WRITING_FLUSH_TIMEOUT_MS,
  createWritingFlushBroker,
  isWritingConflictAction,
  isWritingShortcut,
  WRITING_HOST_MESSAGE_BUDGET_BYTES,
  parseWritingStateEvent,
  writingAiStateMessage,
  writingConflictMessage,
  writingFullscreenMessage,
  writingLoadMessage,
  writingMessageCurrent,
  writingTitleMessage,
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
assert.deepEqual(parseWritingStateEvent({ kind: "open-entity", entityId: "e-1", newTab: true }), {
  kind: "open-entity",
  entityId: "e-1",
  newTab: true,
});
assert.deepEqual(parseWritingStateEvent({ kind: "open-entity", entityId: "e-1", newTab: false }), {
  kind: "open-entity",
  entityId: "e-1",
  newTab: false,
});
assert.deepEqual(parseWritingStateEvent({ kind: "open-entity", entityId: "e-1", newTab: "yes" }), {
  kind: "open-entity",
  entityId: "e-1",
});
assert.deepEqual(parseWritingStateEvent({ kind: "open-entity", entityId: "e-1", newTab: 1 }), {
  kind: "open-entity",
  entityId: "e-1",
});
assert.equal(parseWritingStateEvent({ kind: "open-entity", entityId: "", newTab: true }), null);

// Fullscreen is the webview's own: it names the entry and the state after the change.
assert.equal(parseWritingStateEvent({ kind: "fullscreen" }), null);
assert.equal(parseWritingStateEvent({ kind: "fullscreen", entityId: "e1" }), null, "on is required");
assert.equal(parseWritingStateEvent({ kind: "fullscreen", entityId: "e1", on: "yes" }), null);
assert.equal(parseWritingStateEvent({ kind: "fullscreen", on: true }), null, "entityId is required");
assert.equal(parseWritingStateEvent({ kind: "fullscreen", entityId: "", on: true }), null);
assert.equal(
  parseWritingStateEvent({ kind: "fullscreen", entityId: "e1", enabled: true }),
  null,
  "the old field is gone",
);
assert.deepEqual(parseWritingStateEvent({ kind: "fullscreen", entityId: "e1", on: true }), {
  kind: "fullscreen",
  entityId: "e1",
  on: true,
});
assert.deepEqual(parseWritingStateEvent({ kind: "fullscreen", entityId: "e1", on: false }), {
  kind: "fullscreen",
  entityId: "e1",
  on: false,
});
assert.equal(parseWritingStateEvent({ kind: "shortcut" }), null);
assert.equal(parseWritingStateEvent({ kind: "shortcut", shortcut: "save" }), null);
assert.equal(parseWritingStateEvent({ kind: "shortcut", shortcut: "tab-9" }), null);
assert.equal(parseWritingStateEvent({ kind: "shortcut", shortcut: 1 }), null);
assert.deepEqual(parseWritingStateEvent({ kind: "shortcut", shortcut: "quick-open" }), {
  kind: "shortcut",
  shortcut: "quick-open",
});
assert.deepEqual(parseWritingStateEvent({ kind: "shortcut", shortcut: "tab-close" }), {
  kind: "shortcut",
  shortcut: "tab-close",
});
assert.deepEqual(parseWritingStateEvent({ kind: "shortcut", shortcut: "tab-last" }), {
  kind: "shortcut",
  shortcut: "tab-last",
});

// Drafts: text or a transfer reference, never both, never neither.
const inline = { entityId: "e1", token: "t1", text: "Chapter one" };
assert.deepEqual(parseWritingStateEvent({ kind: "draft", draft: inline }), { kind: "draft", draft: inline });
assert.deepEqual(parseWritingStateEvent({ kind: "draft", draft: { ...inline, text: "" } }), {
  kind: "draft",
  draft: { ...inline, text: "" },
});
const chunked = { entityId: "e1", token: "t1", transfer: "tr-1" };
assert.deepEqual(parseWritingStateEvent({ kind: "draft", draft: chunked }), { kind: "draft", draft: chunked });
assert.equal(parseWritingStateEvent({ kind: "draft" }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: null }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, transfer: "tr-1" } }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { entityId: "e1", token: "t1" } }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, entityId: "" } }), null);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, token: 3 } }), null);
assert.equal(
  parseWritingStateEvent({ kind: "draft", draft: { ...inline, token: "" } }),
  null,
  "a draft needs its token",
);
assert.equal(
  parseWritingStateEvent({ kind: "draft", draft: { entityId: "e1", baseRevision: "r1", text: "Chapter one" } }),
  null,
  "the old base-revision draft shape is gone",
);
assert.equal(parseWritingStateEvent({ kind: "draft", draft: { ...inline, token: "t".repeat(129) } }), null);
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

// Conflict actions name the entity and the load they were chosen under.
assert.equal(isWritingConflictAction("recovery"), true);
assert.equal(isWritingConflictAction("merge"), false);
for (const action of ["reload", "overwrite", "recovery"]) {
  const event = { kind: "conflict-action", entityId: "e1", token: "t1", action };
  assert.deepEqual(parseWritingStateEvent(event), event);
}
for (const bad of [
  { kind: "conflict-action" },
  { kind: "conflict-action", entityId: "e1", token: "t1" },
  { kind: "conflict-action", entityId: "e1", token: "t1", action: "merge" },
  { kind: "conflict-action", entityId: "e1", action: "reload" },
  { kind: "conflict-action", token: "t1", action: "reload" },
  { kind: "conflict-action", entityId: "", token: "t1", action: "reload" },
  { kind: "conflict-action", entityId: "e1", token: "", action: "reload" },
  { kind: "conflict-action", entityId: 1, token: "t1", action: "reload" },
]) {
  assert.equal(parseWritingStateEvent(bad), null, JSON.stringify(bad));
}

// The load message is read-only unless the shell says otherwise, and always carries its token.
assert.deepEqual(writingLoadMessage("e1", "t1", "r1"), {
  type: "writing.load",
  entityId: "e1",
  token: "t1",
  revision: "r1",
  editable: false,
  placeholder: "",
  mode: "read",
});
assert.deepEqual(writingLoadMessage("e1", "t2", "", { editable: true, placeholder: "Write…", mode: "edit" }), {
  type: "writing.load",
  entityId: "e1",
  token: "t2",
  revision: "",
  editable: true,
  placeholder: "Write…",
  mode: "edit",
});

// The shell only turns the webview's full screen off, and renames reach the header without a reload.
assert.deepEqual(writingFullscreenMessage(false), { type: "writing.fullscreen", on: false });
assert.deepEqual(writingFullscreenMessage(true), { type: "writing.fullscreen", on: true });
assert.deepEqual(writingTitleMessage("e1", "Chapter two"), {
  type: "writing.title",
  entityId: "e1",
  title: "Chapter two",
});

// Acceptance: a draft or conflict action counts only for the selected entry and the load the shell last posted.
{
  const load = { entityId: "e1", token: "t1" };
  assert.equal(writingMessageCurrent(inline, "e1", load), true);
  assert.equal(writingMessageCurrent(inline, "e2", load), false, "another entry is selected");
  assert.equal(writingMessageCurrent({ ...inline, token: "t0" }, "e1", load), false, "a draft from before a reload");
  assert.equal(writingMessageCurrent({ ...inline, entityId: "e2" }, "e1", load), false, "a draft for another entry");
  assert.equal(writingMessageCurrent(inline, "e1", null), false, "no load posted yet");
  assert.equal(writingMessageCurrent(inline, null, load), false, "nothing selected");
  assert.equal(writingMessageCurrent(inline, undefined, load), false);
  // The load is for an entry that is no longer selected: its token does not carry over to the new one.
  assert.equal(writingMessageCurrent({ entityId: "e2", token: "t1" }, "e2", load), false);
  // Saving keeps the token, so later drafts for the same load stay current.
  assert.equal(writingMessageCurrent({ ...inline, text: "More" }, "e1", { ...load }), true);
}

// The conflict banner message follows the shell's conflict: a source that needs attention cannot be overwritten.
assert.deepEqual(writingConflictMessage("e1", { diagnostics: [] }), {
  type: "writing.conflict",
  entityId: "e1",
  message: "Your unsaved document and details are preserved. Choose how to reconcile them before saving.",
  canOverwrite: true,
});
assert.deepEqual(writingConflictMessage("e1", { diagnostics: ["Portable files changed externally."] }), {
  type: "writing.conflict",
  entityId: "e1",
  message: "Portable files changed externally.",
  canOverwrite: false,
});

// Flush requests: replies are matched by request id, and a silent webview cannot hold the shell up.
{
  const fakeTimers = () => {
    const timers = new Map();
    let next = 1;
    return {
      setTimer: (callback, ms) => {
        timers.set(next, { callback, ms });
        return next++;
      },
      clearTimer: (handle) => timers.delete(handle),
      active: () => timers.size,
      delays: () => [...timers.values()].map((timer) => timer.ms),
      fireAll() {
        for (const [handle, timer] of [...timers]) {
          timers.delete(handle);
          timer.callback();
        }
      },
    };
  };
  const make = (post) => {
    const clock = fakeTimers();
    const posted = [];
    const timedOut = [];
    let id = 0;
    const broker = createWritingFlushBroker({
      post: async (message) => {
        posted.push(message);
        return post ? post(message) : true;
      },
      newRequestId: () => `f-${++id}`,
      onTimeout: (requestId) => timedOut.push(requestId),
      setTimer: clock.setTimer,
      clearTimer: clock.clearTimer,
    });
    return { broker, clock, posted, timedOut };
  };
  const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

  assert.equal(WRITING_FLUSH_TIMEOUT_MS, 1500);
  // A matching reply resolves with its draft and cancels the timeout.
  {
    const { broker, clock, posted } = make();
    const pending = broker.request();
    await settle();
    assert.deepEqual(posted, [{ type: "writing.flush", requestId: "f-1" }]);
    assert.deepEqual(clock.delays(), [1500]);
    assert.equal(broker.reply("f-1", inline), true);
    assert.deepEqual(await pending, inline);
    assert.equal(clock.active(), 0);
    assert.equal(broker.reply("f-1", inline), false, "a request is answered once");
  }
  // Nothing unsent resolves with null.
  {
    const { broker } = make();
    const pending = broker.request();
    await settle();
    assert.equal(broker.reply("f-1", null), true);
    assert.equal(await pending, null);
  }
  // Replies for other requests do not settle this one; concurrent requests are matched independently.
  {
    const { broker } = make();
    const first = broker.request();
    const second = broker.request();
    await settle();
    assert.equal(broker.reply("f-unknown", inline), false);
    assert.equal(broker.reply("f-2", chunked), true);
    assert.deepEqual(await second, chunked);
    assert.equal(broker.reply("f-1", null), true);
    assert.equal(await first, null);
  }
  // A silent webview times out, is logged, and a late reply no longer matches (the shell treats it as a draft).
  {
    const { broker, clock, timedOut } = make();
    const pending = broker.request();
    await settle();
    clock.fireAll();
    assert.equal(await pending, null);
    assert.deepEqual(timedOut, ["f-1"]);
    assert.equal(broker.reply("f-1", inline), false, "a reply after the timeout is not a flush answer");
  }
  // A webview that cannot be reached does not wait for the timeout.
  {
    const { broker, clock, timedOut } = make(async () => false);
    assert.equal(await broker.request(), null);
    assert.equal(clock.active(), 0);
    assert.deepEqual(timedOut, []);
  }
  // Releasing (the webview unmounted) settles everything that waits.
  {
    const { broker, clock } = make();
    const a = broker.request();
    const b = broker.request();
    await settle();
    broker.releaseAll();
    assert.deepEqual(await Promise.all([a, b]), [null, null]);
    assert.equal(clock.active(), 0);
  }
}

// AI state for the webview: the editor templates, the provider's name, and always a message that fits.
{
  const template = (id, extra = {}) => ({
    id,
    label: `Label ${id}`,
    instruction: `Instruction ${id}`,
    kind: "editor",
    ...extra,
  });
  const bytes = (message) => new TextEncoder().encode(JSON.stringify(message)).byteLength;
  const message = writingAiStateMessage({
    enabled: true,
    providerName: "LM Studio",
    templates: [
      template("rewrite", { requiresSelection: true, bundled: true }),
      template("generate", { enabled: false }),
      template("git-message", { kind: "git" }),
      template("image-entity", { kind: "image" }),
      template("custom"),
    ],
  });
  assert.deepEqual(message, {
    type: "writing.ai-state",
    enabled: true,
    providerName: "LM Studio",
    templates: [
      {
        id: "rewrite",
        label: "Label rewrite",
        instruction: "Instruction rewrite",
        requiresSelection: true,
        enabled: true,
        kind: "editor",
      },
      {
        id: "generate",
        label: "Label generate",
        instruction: "Instruction generate",
        requiresSelection: false,
        enabled: false,
        kind: "editor",
      },
      {
        id: "custom",
        label: "Label custom",
        instruction: "Instruction custom",
        requiresSelection: false,
        enabled: true,
        kind: "editor",
      },
    ],
  });
  assert.equal(WRITING_HOST_MESSAGE_BUDGET_BYTES < 64 * 1024, true, "keeps a margin under the host's 64 KiB cap");
  assert.deepEqual(writingAiStateMessage({ enabled: false, providerName: "", templates: [] }), {
    type: "writing.ai-state",
    enabled: false,
    providerName: "",
    templates: [],
  });
  assert.equal(
    writingAiStateMessage({ enabled: true, providerName: "p".repeat(500), templates: [] }).providerName.length,
    200,
  );
  assert.equal(
    writingAiStateMessage({ enabled: true, providerName: "", templates: [template("a", { label: "l".repeat(500) })] })
      .templates[0].label.length,
    200,
  );
  // Unusable templates are left out rather than failing the post.
  assert.deepEqual(
    writingAiStateMessage({
      enabled: true,
      providerName: "",
      templates: [
        template("", {}),
        template("blank", { label: "  " }),
        template("a"),
        template("a"),
        template("x".repeat(129)),
      ],
    }).templates.map((item) => item.id),
    ["a"],
  );
  // At most 64 templates, even when they would fit.
  assert.equal(
    writingAiStateMessage({
      enabled: true,
      providerName: "",
      templates: Array.from({ length: 100 }, (_, index) => template(`t${index}`)),
    }).templates.length,
    64,
  );
  // Over the budget, trailing templates are dropped until it fits; the leading ones are kept whole.
  {
    const big = "x".repeat(20 * 1024);
    const templates = [
      template("first"),
      ...Array.from({ length: 6 }, (_, index) => template(`big${index}`, { instruction: big })),
    ];
    const trimmed = writingAiStateMessage({ enabled: true, providerName: "P", templates });
    assert.ok(bytes(trimmed) <= WRITING_HOST_MESSAGE_BUDGET_BYTES, `${bytes(trimmed)} bytes fit the budget`);
    assert.ok(trimmed.templates.length < templates.length, "some were dropped");
    assert.deepEqual(
      trimmed.templates.map((item) => item.id),
      templates.slice(0, trimmed.templates.length).map((item) => item.id),
      "from the end",
    );
    assert.equal(trimmed.templates[0].id, "first");
    assert.equal(trimmed.templates[1].instruction, big, "kept templates are not altered");
  }
  // Text that JSON escapes (quotes, newlines, non-ASCII) counts at its encoded size.
  {
    const escaped = '"\n\u00e9'.repeat(8000);
    const trimmed = writingAiStateMessage({
      enabled: true,
      providerName: "",
      templates: [template("a", { instruction: escaped }), template("b", { instruction: escaped })],
    });
    assert.ok(bytes(trimmed) <= WRITING_HOST_MESSAGE_BUDGET_BYTES);
    assert.equal(trimmed.templates.length, 1);
  }
  // A single template that cannot fit leaves an empty list rather than an oversized message.
  {
    const trimmed = writingAiStateMessage({
      enabled: true,
      providerName: "",
      templates: [template("huge", { instruction: "x".repeat(200 * 1024) })],
    });
    assert.deepEqual(trimmed.templates, []);
    assert.ok(bytes(trimmed) <= WRITING_HOST_MESSAGE_BUDGET_BYTES);
  }
  // A smaller budget is honoured.
  assert.equal(
    writingAiStateMessage({ enabled: true, providerName: "", templates: [template("a"), template("b")] }, 200).templates
      .length,
    1,
  );
}

// Route wiring: the shell stays the only writer and every leave path goes through the one flush.
{
  const page = fs.readFileSync("src/routes/+page.svelte", "utf8");
  const between = (from, to) => {
    const start = page.indexOf(from);
    assert.ok(start >= 0, `${from} exists`);
    const end = page.indexOf(to, start + from.length);
    assert.ok(end > start, `${to} follows ${from}`);
    return page.slice(start, end);
  };
  const flush = between("async function flushAutoSave()", "async function loadSelectedState");
  assert.match(
    flush,
    /await flushWritingWebview\(\);\s*editorRef\?\.flushPendingChanges\(\)/,
    "the webview is asked first",
  );
  const apply = between("async function applyWritingDraft", "function enqueueWritingDraft");
  assert.match(apply, /updateDocumentBody\(text\)/, "a draft is applied like typing in the shell editor");
  assert.match(apply, /project\.takePluginTextTransfer\(WRITING_PLUGIN_ID, draft\.transfer\)/);
  assert.match(apply, /writingMessageCurrent\(draft, selected\?\.id, writingLoad\)/, "entity and token are checked");
  const writingRegion = between("/** Whether the webview may edit", "function pluginViewLabel");
  assert.doesNotMatch(writingRegion, /project\.save\w*\(|persistDocumentSnapshot\(/, "no second save path");
  const conflict = between("function handleWritingConflictAction", "/**\n * Writing state from the webview");
  assert.match(conflict, /writingMessageCurrent\(event, selected\?\.id, writingLoad\)/);
  assert.match(conflict, /reloadConflict\(\)/);
  assert.match(conflict, /overwriteConflict\(\)/);
  assert.match(conflict, /saveConflictRecoveryCopy\(\)/);
  const save = between("async function persistDocumentSnapshot()", "async function saveDocument()");
  assert.match(save, /notifyWritingSaved\(entityId\)/, "a successful save is reported to the webview");
  const load = between("async function postWritingLoad", "$effect(() => {");
  assert.match(
    load,
    /writingLoad\?\.entityId === entityId && \(await flushWritingWebview\(\)\)/,
    "a reload of the same entry flushes the webview first",
  );
  // AI: the webview runs the flow, so the shell tells it the AI state instead of opening its own dialog.
  const aiStateEffect = between(
    "// The webview runs the AI rewrite flow itself",
    "$effect(() => {\n  // Editability follows",
  );
  assert.match(aiStateEffect, /void writingReadyCount;/, "a restarted webview is told again");
  assert.match(aiStateEffect, /if \(!writingWebview\.mounted \|\| !writingWebview\.ready\) return;/);
  assert.match(aiStateEffect, /enabled: projectInfo\?\.aiEnabled \?\? false/);
  assert.match(aiStateEffect, /providerName: aiProvider\.name/);
  assert.match(aiStateEffect, /templates: aiPromptTemplates/);
  assert.match(aiStateEffect, /postWritingMessage\(message\)/);
  assert.match(page, /writingReadyCount \+= 1;/, "every ready report counts");
  // The webview's dialog is its own: nothing in the overlay logic reads it, and the shell's dialog stays for the old editor.
  const overlay = between("const modalOverlayOpen = $derived(", "const anchoredPopoverOpen");
  assert.doesNotMatch(overlay, /writingAi|aiDialog|ai-state/);
  assert.match(page, /\{#if aiRewriteOpen\}/, "the shell's own dialog is still there for the old editor");
  // Closing the window flushes through the same path.
  const closeRequested = between(".onCloseRequested(async (event) => {", ".then((cleanup) => {");
  assert.match(closeRequested, /await flushAutoSave\(\)/);
}

console.log("writing bridge checks passed");
