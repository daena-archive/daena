import assert from "node:assert/strict";
import fs from "node:fs";
import {
  DEFAULT_WRITING_MODE,
  WRITING_CONFLICT_ACTIONS,
  WRITING_DRAFT_INLINE_LIMIT_BYTES,
  WRITING_STATE_EVENT_TYPE,
  createWritingStatePublisher,
  isWritingConflictAction,
  isWritingMode,
  isWritingShortcut,
  parseWritingDraft,
  parseWritingHostMessage,
  parseWritingMountParams,
  parseWritingStatePayload,
  pickWritingManifest,
  writingShortcutForKey,
} from "../packages/modules/writing/src/plugin-protocol.ts";
import { entityIdsInMarkdown, loadWritingDocument } from "../packages/modules/writing/src/document-source.ts";
import {
  MAX_WRITING_AI_LABEL_LENGTH,
  MAX_WRITING_AI_PROVIDER_NAME_LENGTH,
  MAX_WRITING_AI_TEMPLATES,
} from "../packages/modules/writing/src/plugin-protocol.ts";
import {
  WRITING_AI_PASSAGE_COUNT,
  WRITING_AI_QUERY_LIMIT,
  WRITING_AI_RELATIONSHIP_DEPTH,
  WRITING_AI_STALE_MESSAGE,
  aiAcceptGuard,
  aiCanStart,
  aiSourceText,
  applyAiEvent,
  applyAiEvents,
  buildAiStartRequest,
  initialAiRunState,
  retrievalPolicyFor,
  retrievalQueryFor,
} from "../packages/modules/writing/src/ai-flow.ts";
import {
  WRITING_AI_MAX_POLL_FAILURES,
  WRITING_AI_POLL_MS,
  startAiRun,
} from "../packages/modules/writing/src/ai-run.ts";
import { validateAiProposal } from "../src/lib/ai/proposal.ts";
import { aiActionRequest, instructionFor, mergePromptTemplates } from "../src/lib/ai/promptTemplates.ts";
import {
  WRITING_DRAFT_IDLE_MS,
  WRITING_DRAFT_INLINE_JSON_LIMIT_BYTES,
  WRITING_DRAFT_RETRY_MS,
  createDraftSync,
  draftNeedsTransfer,
} from "../packages/modules/writing/src/draft-sync.ts";
import {
  parseWritingStateEvent,
  writingConflictMessage,
  writingAiStateMessage,
  writingFullscreenMessage,
  writingLoadMessage,
  writingTitleMessage,
} from "../src/lib/modules/writing-bridge.ts";

// Mount parameters: the shell mounts without a selection and posts writing.load afterwards.
{
  const launch = parseWritingMountParams("?project=p&view=writing-workspace&entityId=e-1");
  assert.equal(launch.projectId, "p");
  assert.equal(launch.entityId, "e-1");
  const bare = parseWritingMountParams("?project=p");
  assert.equal(bare.entityId, null, "no selection means no entity");
  assert.equal(parseWritingMountParams("?entityId=").entityId, null, "an empty entityId counts as none");
}

assert.equal(WRITING_STATE_EVENT_TYPE, "daena.writing/state@1");
assert.equal(DEFAULT_WRITING_MODE, "read");
assert.equal(isWritingMode("edit"), true);
assert.equal(isWritingMode("preview"), false);
assert.equal(isWritingShortcut("create"), true);
assert.equal(isWritingShortcut("Cmd+N"), false);

// Host messages: only the documented shapes are accepted.
assert.deepEqual(parseWritingHostMessage({ type: "writing.load", entityId: "e1", token: "t1", revision: "r1" }), {
  type: "writing.load",
  entityId: "e1",
  token: "t1",
  revision: "r1",
  editable: false,
  placeholder: "",
  mode: "read",
});
assert.deepEqual(
  parseWritingHostMessage({
    type: "writing.load",
    entityId: "e1",
    token: "t2",
    revision: "",
    editable: true,
    placeholder: "Write…",
    mode: "edit",
  }),
  {
    type: "writing.load",
    entityId: "e1",
    token: "t2",
    revision: "",
    editable: true,
    placeholder: "Write…",
    mode: "edit",
  },
  "a new entity has the empty revision",
);
for (const bad of [
  { type: "writing.load", token: "t", revision: "r1" },
  { type: "writing.load", entityId: "", token: "t", revision: "r1" },
  { type: "writing.load", entityId: "x".repeat(129), token: "t", revision: "r1" },
  { type: "writing.load", entityId: "e1", token: "t" },
  { type: "writing.load", entityId: "e1", token: "t", revision: 4 },
  { type: "writing.load", entityId: "e1", token: "t", revision: "r1", editable: "yes" },
  { type: "writing.load", entityId: "e1", token: "t", revision: "r1", placeholder: 3 },
  { type: "writing.load", entityId: "e1", token: "t", revision: "r1", mode: "preview" },
  { type: "writing.load", entityId: "e1", revision: "r1" },
  { type: "writing.load", entityId: "e1", token: "", revision: "r1" },
  { type: "writing.load", entityId: "e1", token: 7, revision: "r1" },
  { type: "writing.load", entityId: "e1", token: "t".repeat(129), revision: "r1" },
]) {
  assert.equal(parseWritingHostMessage(bad), null, JSON.stringify(bad));
}
assert.equal(
  parseWritingHostMessage({
    type: "writing.load",
    entityId: "e1",
    token: "t",
    revision: "r",
    placeholder: "p".repeat(500),
  }).placeholder.length,
  200,
  "placeholders are bounded",
);
assert.deepEqual(parseWritingHostMessage({ type: "writing.flush", requestId: "f-1" }), {
  type: "writing.flush",
  requestId: "f-1",
});
assert.equal(parseWritingHostMessage({ type: "writing.flush", requestId: "" }), null);
assert.equal(parseWritingHostMessage({ type: "writing.flush" }), null);
assert.deepEqual(parseWritingHostMessage({ type: "writing.saved", entityId: "e1", revision: "r2" }), {
  type: "writing.saved",
  entityId: "e1",
  revision: "r2",
});
assert.equal(parseWritingHostMessage({ type: "writing.saved", entityId: "e1" }), null);
assert.deepEqual(parseWritingHostMessage({ type: "writing.conflict", entityId: "e1" }), {
  type: "writing.conflict",
  entityId: "e1",
  message: "",
  canOverwrite: true,
});
assert.deepEqual(
  parseWritingHostMessage({ type: "writing.conflict", entityId: "e1", message: "Changed", canOverwrite: false }),
  { type: "writing.conflict", entityId: "e1", message: "Changed", canOverwrite: false },
);
assert.equal(parseWritingHostMessage({ type: "writing.conflict", entityId: "e1", message: 5 }), null);
assert.equal(parseWritingHostMessage({ type: "writing.conflict", entityId: "e1", canOverwrite: "no" }), null);
assert.equal(parseWritingHostMessage({ type: "writing.conflict" }), null);
assert.deepEqual(parseWritingHostMessage({ type: "writing.save-error", entityId: "e1", message: "Disk full" }), {
  type: "writing.save-error",
  entityId: "e1",
  message: "Disk full",
});
assert.equal(parseWritingHostMessage({ type: "writing.save-error", entityId: "e1" }), null);
assert.equal(parseWritingHostMessage({ type: "writing.save-error", message: "x" }), null);
assert.deepEqual(parseWritingHostMessage({ type: "writing.editable", editable: false }), {
  type: "writing.editable",
  editable: false,
});
assert.equal(parseWritingHostMessage({ type: "writing.editable", editable: 0 }), null);
// AI state: whether AI is on, the provider's name and the editor templates; the whole message is validated.
{
  const template = (overrides = {}) => ({
    id: "rewrite",
    label: "Rewrite selection",
    instruction: "Rewrite this.",
    requiresSelection: true,
    enabled: true,
    kind: "editor",
    ...overrides,
  });
  const state = (overrides = {}) => ({
    type: "writing.ai-state",
    enabled: true,
    providerName: "LM Studio",
    templates: [template()],
    ...overrides,
  });
  assert.deepEqual(parseWritingHostMessage(state()), state());
  assert.deepEqual(parseWritingHostMessage(state({ enabled: false, providerName: "", templates: [] })), {
    type: "writing.ai-state",
    enabled: false,
    providerName: "",
    templates: [],
  });
  // `requiresSelection` and `enabled` may be left out; they default to false and true.
  assert.deepEqual(
    parseWritingHostMessage(state({ templates: [{ id: "custom", label: "Custom", instruction: "", kind: "editor" }] }))
      .templates,
    [{ id: "custom", label: "Custom", instruction: "", requiresSelection: false, enabled: true, kind: "editor" }],
  );
  assert.equal(
    parseWritingHostMessage(state({ providerName: "p".repeat(MAX_WRITING_AI_PROVIDER_NAME_LENGTH + 50) })).providerName
      .length,
    MAX_WRITING_AI_PROVIDER_NAME_LENGTH,
    "the provider name is cut, like a title",
  );
  // Rejections: the old shape, wrong types, and any malformed template.
  assert.equal(parseWritingHostMessage({ type: "writing.ai-state" }), null);
  assert.equal(parseWritingHostMessage({ type: "writing.ai-state", enabled: true }), null, "the old shape is gone");
  assert.equal(parseWritingHostMessage(state({ enabled: "yes" })), null);
  assert.equal(parseWritingHostMessage(state({ enabled: undefined })), null);
  assert.equal(parseWritingHostMessage(state({ providerName: undefined })), null);
  assert.equal(parseWritingHostMessage(state({ providerName: 7 })), null);
  assert.equal(parseWritingHostMessage(state({ templates: undefined })), null);
  assert.equal(parseWritingHostMessage(state({ templates: "rewrite" })), null);
  assert.equal(parseWritingHostMessage(state({ templates: { rewrite: template() } })), null);
  for (const bad of [
    null,
    "rewrite",
    [],
    template({ id: "" }),
    template({ id: 3 }),
    template({ id: "x".repeat(129) }),
    template({ label: "" }),
    template({ label: 3 }),
    template({ label: "l".repeat(MAX_WRITING_AI_LABEL_LENGTH + 1) }),
    template({ instruction: undefined }),
    template({ instruction: 3 }),
    template({ instruction: "i".repeat(60 * 1024 + 1) }),
    template({ requiresSelection: "yes" }),
    template({ enabled: 1 }),
    template({ kind: "git" }),
    template({ kind: "image" }),
    template({ kind: undefined }),
  ]) {
    assert.equal(
      parseWritingHostMessage(state({ templates: [template({ id: "ok" }), bad] })),
      null,
      `one bad template rejects the message: ${JSON.stringify(bad)?.slice(0, 60)}`,
    );
  }
  assert.equal(
    parseWritingHostMessage(state({ templates: [template(), template({ label: "Again" })] })),
    null,
    "a repeated id is ambiguous",
  );
  const many = Array.from({ length: MAX_WRITING_AI_TEMPLATES + 1 }, (_, index) => template({ id: `t${index}` }));
  assert.equal(parseWritingHostMessage(state({ templates: many })), null, "too many templates");
  assert.notEqual(parseWritingHostMessage(state({ templates: many.slice(0, MAX_WRITING_AI_TEMPLATES) })), null);

  // What the shell builds is what the webview accepts: the real merged templates, the git and image ones left out.
  const merged = mergePromptTemplates({
    templates: [
      { id: "rewrite", label: "Rewrite", instruction: 'Rewrite it, "vividly".', enabled: false },
      { id: "mine", label: "Mine", instruction: "Do mine.", kind: "editor", requiresSelection: true },
    ],
  });
  const built = writingAiStateMessage({ enabled: true, providerName: "LM Studio", templates: merged });
  assert.ok(merged.some((item) => item.kind === "git") && merged.some((item) => item.kind === "image"));
  assert.ok(built.templates.every((item) => item.kind === "editor"));
  assert.deepEqual(
    built.templates.map((item) => item.id),
    merged.filter((item) => item.kind === "editor").map((item) => item.id),
  );
  assert.deepEqual(Object.keys(built.templates[0]).sort(), [
    "enabled",
    "id",
    "instruction",
    "kind",
    "label",
    "requiresSelection",
  ]);
  const parsed = parseWritingHostMessage(JSON.parse(JSON.stringify(built)));
  assert.deepEqual(parsed, built, "the builder and the parser agree");
  assert.equal(parsed.templates.find((item) => item.id === "rewrite").enabled, false, "disabled ones stay");
  // The dialog finds instructions in the received templates exactly as the shell does in its own.
  assert.equal(
    instructionFor(parsed.templates, "generate"),
    instructionFor(merged, "generate"),
    "the same instruction for an enabled template",
  );
  assert.equal(instructionFor(parsed.templates, "rewrite"), "", "and none for a disabled one");
  assert.equal(instructionFor(parsed.templates, "mine"), "Do mine.");
  assert.equal(instructionFor(parsed.templates, "git-message"), "", "git templates are not sent");
}
// Full screen is turned off by the shell (`on` is the state after the change) and titles arrive after a rename.
assert.deepEqual(parseWritingHostMessage({ type: "writing.fullscreen", on: false }), {
  type: "writing.fullscreen",
  on: false,
});
assert.equal(parseWritingHostMessage({ type: "writing.fullscreen" }), null);
assert.equal(parseWritingHostMessage({ type: "writing.fullscreen", on: "false" }), null);
assert.equal(parseWritingHostMessage({ type: "writing.fullscreen", enabled: false }), null, "the old field is gone");
assert.deepEqual(parseWritingHostMessage({ type: "writing.title", entityId: "e1", title: "Chapter two" }), {
  type: "writing.title",
  entityId: "e1",
  title: "Chapter two",
});
assert.equal(parseWritingHostMessage({ type: "writing.title", entityId: "e1" }), null, "a title is required");
assert.equal(parseWritingHostMessage({ type: "writing.title", title: "Chapter two" }), null);
assert.equal(parseWritingHostMessage({ type: "writing.title", entityId: "", title: "x" }), null);
assert.equal(parseWritingHostMessage({ type: "writing.title", entityId: "e1", title: 7 }), null);
assert.equal(
  parseWritingHostMessage({ type: "writing.title", entityId: "e1", title: "t".repeat(2000) }).title.length,
  512,
  "titles are capped",
);
assert.equal(parseWritingHostMessage({ type: "language.pane", pane: "overview" }), null);
assert.equal(parseWritingHostMessage("writing.load"), null);
assert.equal(parseWritingHostMessage(null), null);
assert.equal(parseWritingHostMessage([]), null);

// What the shell builds, the webview accepts unchanged.
{
  const load = writingLoadMessage("e1", "t1", "r1", { editable: true, placeholder: "Write…", mode: "edit" });
  assert.deepEqual(parseWritingHostMessage(load), load);
  const conflict = writingConflictMessage("e1", { diagnostics: [] });
  assert.deepEqual(parseWritingHostMessage(conflict), conflict);
  const attention = writingConflictMessage("e1", { diagnostics: ["Needs attention"] });
  assert.deepEqual(parseWritingHostMessage(attention), attention);
  for (const message of [
    { type: "writing.flush", requestId: "f-1" },
    { type: "writing.saved", entityId: "e1", revision: "r2" },
    { type: "writing.save-error", entityId: "e1", message: "Disk full" },
    { type: "writing.editable", editable: true },
  ]) {
    assert.deepEqual(parseWritingHostMessage(message), message);
  }
  for (const message of [
    writingFullscreenMessage(true),
    writingFullscreenMessage(false),
    writingTitleMessage("e1", "A"),
  ]) {
    assert.deepEqual(parseWritingHostMessage(message), message);
  }
}

// Shortcut mapping: the same keys and modifier rules as the shell's window listeners, and nothing else.
{
  const press = (key, modifiers = {}) => ({
    key,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    repeat: false,
    ...modifiers,
  });
  const cmd = { metaKey: true };
  const ctrl = { ctrlKey: true };
  assert.equal(writingShortcutForKey(press("k", cmd)), "quick-open");
  assert.equal(writingShortcutForKey(press("K", ctrl)), "quick-open", "the key is matched without case");
  assert.equal(writingShortcutForKey(press("n", cmd)), "create");
  assert.equal(writingShortcutForKey(press("N", ctrl)), "create");
  assert.equal(writingShortcutForKey(press("ArrowLeft", { altKey: true })), "history-back");
  assert.equal(writingShortcutForKey(press("ArrowRight", { altKey: true })), "history-forward");
  assert.equal(
    writingShortcutForKey(press("ArrowLeft", { altKey: true, editable: true })),
    null,
    "Alt+arrows in a text field move by word instead of going back",
  );
  assert.equal(writingShortcutForKey(press("k", { ...cmd, editable: true })), "quick-open", "Cmd+K works while typing");
  // Held keys do not repeat quick open or create, but history is not gated on repeat (as in the shell).
  assert.equal(writingShortcutForKey(press("k", { ...cmd, repeat: true })), null);
  assert.equal(writingShortcutForKey(press("n", { ...ctrl, repeat: true })), null);
  assert.equal(writingShortcutForKey(press("ArrowLeft", { altKey: true, repeat: true })), "history-back");
  // Shift and Alt change the meaning of K and N, so they do not trigger the shortcut.
  assert.equal(writingShortcutForKey(press("k", { ...cmd, shiftKey: true })), null);
  assert.equal(writingShortcutForKey(press("k", { ...cmd, altKey: true })), null);
  assert.equal(writingShortcutForKey(press("n", { ...ctrl, altKey: true })), null);
  // History needs Alt alone: Cmd or Ctrl or Shift with Alt means something else.
  assert.equal(writingShortcutForKey(press("ArrowLeft", { altKey: true, metaKey: true })), null);
  assert.equal(writingShortcutForKey(press("ArrowLeft", { altKey: true, ctrlKey: true })), null);
  assert.equal(writingShortcutForKey(press("ArrowLeft", { altKey: true, shiftKey: true })), null);
  assert.equal(writingShortcutForKey(press("ArrowUp", { altKey: true })), null);
  // Without a modifier, or with the wrong key, nothing is forwarded.
  assert.equal(writingShortcutForKey(press("k")), null, "K alone is typing");
  assert.equal(writingShortcutForKey(press("n")), null, "N alone is typing");
  assert.equal(writingShortcutForKey(press("ArrowLeft")), null);
  assert.equal(writingShortcutForKey(press("s", cmd)), null, "save is the editor's own");
  assert.equal(writingShortcutForKey(press("f", cmd)), null, "search is the editor's own");
  assert.equal(writingShortcutForKey(press("Escape")), null);
}

// State payloads: the webview parser and the shell bridge accept and reject the same things.
{
  const inline = { entityId: "e1", token: "t1", text: "Hello" };
  const accepted = [
    { kind: "ready", entityId: null },
    { kind: "ready", entityId: "e1" },
    { kind: "draft", draft: inline },
    { kind: "draft", draft: { entityId: "e1", token: "t1", transfer: "tr-1" } },
    { kind: "flush-reply", requestId: "f-1", draft: null },
    { kind: "flush-reply", requestId: "f-1", draft: inline },
    { kind: "conflict-action", entityId: "e1", token: "t1", action: "reload" },
    { kind: "conflict-action", entityId: "e1", token: "t1", action: "overwrite" },
    { kind: "conflict-action", entityId: "e1", token: "t1", action: "recovery" },
    { kind: "open-link", href: "https://example.com" },
    { kind: "open-entity", entityId: "e2" },
    { kind: "open-entity", entityId: "e2", newTab: true },
    { kind: "open-entity", entityId: "e2", newTab: false },
    { kind: "shortcut", shortcut: "tab-close" },
    { kind: "fullscreen", entityId: "e1", on: false },
    { kind: "fullscreen", entityId: "e1", on: true },
    { kind: "request-rename", entityId: "e1" },
    { kind: "request-archive", entityId: "e1" },
    { kind: "shortcut", shortcut: "history-back" },
  ];
  for (const payload of accepted) {
    assert.deepEqual(parseWritingStatePayload(payload), payload, `webview accepts ${payload.kind}`);
    assert.deepEqual(parseWritingStateEvent(payload), payload, `shell accepts ${payload.kind}`);
  }
  const rejected = [
    null,
    "ready",
    [],
    {},
    { kind: "unknown" },
    { kind: "ready", entityId: 1 },
    { kind: "draft", draft: { ...inline, text: undefined } },
    { kind: "draft", draft: { ...inline, transfer: "tr-1" } },
    { kind: "draft", draft: { entityId: "e1", text: "Hello" } },
    { kind: "draft", draft: { ...inline, token: "" } },
    { kind: "draft", draft: { ...inline, token: 9 } },
    { kind: "draft", draft: { entityId: "e1", baseRevision: "r1", text: "Hello" } },
    { kind: "draft", draft: { ...inline, text: "a".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES + 1) } },
    { kind: "conflict-action", entityId: "e1", token: "t1" },
    { kind: "conflict-action", entityId: "e1", token: "t1", action: "merge" },
    { kind: "conflict-action", entityId: "e1", action: "reload" },
    { kind: "conflict-action", token: "t1", action: "reload" },
    { kind: "conflict-action", entityId: "", token: "t1", action: "reload" },
    { kind: "flush-reply", draft: null },
    { kind: "flush-reply", requestId: "f-1" },
    { kind: "open-link", href: 1 },
    { kind: "open-entity", entityId: "" },
    { kind: "fullscreen", entityId: "e1", on: "true" },
    { kind: "fullscreen", on: true },
    { kind: "fullscreen", entityId: "", on: true },
    { kind: "fullscreen", entityId: "e1", enabled: true },
    { kind: "request-rename" },
    { kind: "shortcut", shortcut: "save" },
    { kind: "shortcut", shortcut: "tab-9" },
  ];
  for (const payload of rejected) {
    assert.equal(parseWritingStatePayload(payload), null, `webview rejects ${JSON.stringify(payload)}`);
    assert.equal(parseWritingStateEvent(payload), null, `shell rejects ${JSON.stringify(payload)}`);
  }
  assert.equal(parseWritingDraft({ ...inline, text: "é".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES) }), null);
  const stripped = { kind: "open-entity", entityId: "e2", newTab: "yes" };
  assert.deepEqual(parseWritingStatePayload(stripped), { kind: "open-entity", entityId: "e2" });
  assert.deepEqual(parseWritingStateEvent(stripped), { kind: "open-entity", entityId: "e2" });
}

assert.deepEqual([...WRITING_CONFLICT_ACTIONS], ["reload", "overwrite", "recovery"]);
assert.equal(isWritingConflictAction("overwrite"), true);
assert.equal(isWritingConflictAction("merge"), false);

// Publishing: one event type, payload passed through unchanged.
{
  const calls = [];
  const publish = createWritingStatePublisher({
    async call(method, payload) {
      calls.push({ method, payload });
      return null;
    },
  });
  assert.equal(await publish({ kind: "ready", entityId: null }), true);
  await publish({ kind: "open-entity", entityId: "e-1" });
  assert.deepEqual(calls, [
    { method: "event.publish", payload: { type: "daena.writing/state@1", payload: { kind: "ready", entityId: null } } },
    {
      method: "event.publish",
      payload: { type: WRITING_STATE_EVENT_TYPE, payload: { kind: "open-entity", entityId: "e-1" } },
    },
  ]);
}

// A failed publish is reported, never thrown into the caller.
{
  const errors = [];
  const publish = createWritingStatePublisher(
    {
      async call() {
        throw new Error("broker offline");
      },
    },
    (cause) => errors.push(cause),
  );
  assert.equal(await publish({ kind: "open-link", href: "https://example.com" }), false);
  assert.equal(errors.length, 1);
  assert.equal(errors[0].message, "broker offline");
}

// Payloads go out in call order even when an earlier one is slow, and one failure does not stall the next.
{
  const order = [];
  const release = [];
  const publish = createWritingStatePublisher(
    {
      async call(_method, { payload }) {
        order.push(`start:${payload.kind}`);
        if (payload.kind === "draft") await new Promise((resolve) => release.push(resolve));
        if (payload.kind === "open-link") throw new Error("offline");
        order.push(`end:${payload.kind}`);
      },
    },
    () => undefined,
  );
  const draft = publish({ kind: "draft", draft: { entityId: "e", token: "t", text: "x" } });
  const reply = publish({ kind: "flush-reply", requestId: "f", draft: null });
  const failed = publish({ kind: "open-link", href: "https://example.com" });
  const after = publish({ kind: "shortcut", shortcut: "create" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(order, ["start:draft"], "the flush reply waits for the draft before it");
  release[0]();
  assert.deepEqual(await Promise.all([draft, reply, failed, after]), [true, true, false, true]);
  assert.deepEqual(order, [
    "start:draft",
    "end:draft",
    "start:flush-reply",
    "end:flush-reply",
    "start:open-link",
    "start:shortcut",
    "end:shortcut",
  ]);
}

// Draft sync: coalescing, inline versus chunked, the unsent flag and flush replies, on fake timers.
{
  const fakeTimers = () => {
    const timers = new Map();
    let next = 1;
    return {
      setTimer(callback, ms) {
        timers.set(next, { callback, ms });
        return next++;
      },
      clearTimer(handle) {
        timers.delete(handle);
      },
      count: () => timers.size,
      pendingDelays: () => [...timers.values()].map((timer) => timer.ms),
      async fire() {
        const due = [...timers.entries()];
        timers.clear();
        for (const [, timer] of due) timer.callback();
        await new Promise((resolve) => setTimeout(resolve, 0));
      },
    };
  };
  const make = (overrides = {}) => {
    const clock = fakeTimers();
    const sent = [];
    const transfers = [];
    const pending = [];
    const sync = createDraftSync({
      publish: async (payload) => {
        sent.push(payload);
        return overrides.publishResult ? overrides.publishResult(payload) : true;
      },
      transfer: async (text) => {
        if (overrides.transfer) return overrides.transfer(text);
        transfers.push(text);
        return `transfer-${transfers.length}`;
      },
      onError: (cause) => (overrides.errors ?? []).push(cause),
      onPending: (waiting) => pending.push(waiting),
      setTimer: clock.setTimer,
      clearTimer: clock.clearTimer,
    });
    return { sync, clock, sent, transfers, pending };
  };

  assert.equal(WRITING_DRAFT_IDLE_MS, 500);
  // Changes coalesce: the idle timer restarts and only the last text is published, tagged with its load.
  {
    const { sync, clock, sent, pending } = make();
    sync.change("e1", "t1", "H");
    sync.change("e1", "t1", "He");
    sync.change("e1", "t1", "Hel");
    assert.equal(clock.count(), 1, "one idle timer, restarted by each change");
    assert.deepEqual(clock.pendingDelays(), [WRITING_DRAFT_IDLE_MS]);
    assert.equal(sync.hasUnsent(), true);
    assert.deepEqual(sent, [], "nothing is published while the user is typing");
    await clock.fire();
    assert.deepEqual(sent, [{ kind: "draft", draft: { entityId: "e1", token: "t1", text: "Hel" } }]);
    assert.equal(sync.hasUnsent(), false);
    assert.deepEqual(pending, [true, false]);
    await clock.fire();
    assert.equal(sent.length, 1, "a published draft is not sent twice");
  }
  // The text keeps the load it was edited under even when a newer load starts before the timer fires.
  {
    const { sync, clock, sent } = make();
    sync.change("e1", "t1", "Old entry");
    sync.reset();
    assert.equal(sync.hasUnsent(), false, "a new load drops what the shell chose to replace");
    await clock.fire();
    assert.deepEqual(sent, []);
    sync.change("e2", "t2", "New entry");
    await clock.fire();
    assert.deepEqual(sent, [{ kind: "draft", draft: { entityId: "e2", token: "t2", text: "New entry" } }]);
  }
  // Large text goes through a transfer and the draft names it.
  {
    const { sync, clock, sent, transfers } = make();
    const big = "a".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES + 1);
    sync.change("e1", "t1", big);
    await clock.fire();
    assert.deepEqual(transfers, [big]);
    assert.deepEqual(sent, [{ kind: "draft", draft: { entityId: "e1", token: "t1", transfer: "transfer-1" } }]);
    const atLimit = "a".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES);
    assert.equal(draftNeedsTransfer(atLimit), false);
    assert.equal(draftNeedsTransfer(`${atLimit}a`), true);
    assert.equal(
      draftNeedsTransfer("é".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES / 2 + 1)),
      true,
      "bytes, not characters",
    );
    // Escaping can push small text over the event cap: every quote becomes two bytes.
    const quotes = '"'.repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES);
    assert.equal(draftNeedsTransfer(quotes), true);
    assert.ok(WRITING_DRAFT_INLINE_JSON_LIMIT_BYTES < 256 * 1024);
  }
  // Flush: the unsent text rides in the reply and the idle timer is cancelled.
  {
    const { sync, clock, sent } = make();
    sync.change("e1", "t1", "Unsent");
    await sync.flush("f-1");
    assert.deepEqual(sent, [
      { kind: "flush-reply", requestId: "f-1", draft: { entityId: "e1", token: "t1", text: "Unsent" } },
    ]);
    assert.equal(clock.count(), 0, "the idle timer is cancelled");
    assert.equal(sync.hasUnsent(), false);
    // Nothing unsent since the last draft: the reply says so.
    await sync.flush("f-2");
    assert.deepEqual(sent[1], { kind: "flush-reply", requestId: "f-2", draft: null });
    // A draft published by the idle timer counts as sent.
    sync.change("e1", "t1", "Later");
    await clock.fire();
    await sync.flush("f-3");
    assert.deepEqual(sent[3], { kind: "flush-reply", requestId: "f-3", draft: null });
  }
  // A flush asked while a chunked draft is still being handed over waits for it, so the reply comes second.
  {
    let finish;
    const { sync, clock, sent } = make({
      transfer: () => new Promise((resolve) => (finish = resolve)),
    });
    sync.change("e1", "t1", "b".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES + 1));
    await clock.fire();
    const flushed = sync.flush("f-1");
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.deepEqual(sent, [], "neither the draft nor the reply is published yet");
    finish("transfer-late");
    await flushed;
    assert.deepEqual(sent, [
      { kind: "draft", draft: { entityId: "e1", token: "t1", transfer: "transfer-late" } },
      { kind: "flush-reply", requestId: "f-1", draft: null },
    ]);
  }
  // A publish that does not arrive leaves the text unsent and retries later; newer text supersedes it.
  {
    let ok = false;
    const { sync, clock, sent } = make({ publishResult: () => ok });
    sync.change("e1", "t1", "Retry me");
    await clock.fire();
    assert.equal(sync.hasUnsent(), true, "a draft that did not arrive is still unsent");
    assert.deepEqual(clock.pendingDelays(), [WRITING_DRAFT_RETRY_MS]);
    ok = true;
    await clock.fire();
    assert.equal(sent.length, 2);
    assert.equal(sync.hasUnsent(), false);

    ok = false;
    sync.change("e1", "t1", "First");
    await clock.fire();
    sync.change("e1", "t1", "Second");
    ok = true;
    await clock.fire();
    assert.deepEqual(sent.at(-1), { kind: "draft", draft: { entityId: "e1", token: "t1", text: "Second" } });
  }
  // A failed hand-off is not answered with "nothing unsent": the shell times out instead of losing text.
  {
    const errors = [];
    const { sync, sent } = make({
      errors,
      transfer: async () => {
        throw new Error("transfer refused");
      },
    });
    sync.change("e1", "t1", "c".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES + 1));
    await sync.flush("f-1");
    assert.deepEqual(sent, [], "no reply is sent");
    assert.equal(sync.hasUnsent(), true);
    assert.equal(errors[0].message, "transfer refused");
  }
  // Text typed before any load is still tagged by the caller; reset during a failed send drops the retry.
  {
    let ok = false;
    const { sync, clock } = make({ publishResult: () => ok });
    sync.change("e1", "t1", "Gone after reload");
    await clock.fire();
    sync.reset();
    await clock.fire();
    assert.equal(sync.hasUnsent(), false);
  }
}

// Manifest: the qualified effective manifest wins over the bundled one.
{
  const bundled = { id: "daena.writing", schemas: [{ entityTypes: [{ id: "manuscript" }] }] };
  const effective = { id: "daena.writing", schemas: [{ entityTypes: [{ id: "daena.writing:manuscript" }] }] };
  assert.equal(pickWritingManifest([{ id: "daena.language" }, effective], bundled), effective);
  assert.equal(pickWritingManifest([{ id: "daena.language" }], bundled), bundled);
}

// Entity references in Markdown are found by their daena://entity links.
assert.deepEqual(
  entityIdsInMarkdown(
    "See [Aven](daena://entity/id-1) and [again](daena://entity/id-1), [Vale](daena://entity/a%20b).",
  ),
  ["id-1", "a b"],
);
assert.deepEqual(entityIdsInMarkdown("No links, only [web](https://example.com)."), []);
assert.deepEqual(entityIdsInMarkdown("[bad](daena://entity/%E0%A4%A)"), [], "a malformed escape is skipped");

// Document source: entity and first document through the broker, normalised to Markdown.
{
  const calls = [];
  const rpc = {
    async call(method, payload) {
      calls.push({ method, payload });
      if (method === "entity.get") {
        if (payload.id === "gone") return null;
        if (payload.id === "trashed") return { id: "trashed", name: "Old", deleted: true };
        return { id: payload.id, name: "Chapter one", entity_type: "daena.writing:manuscript", revision: "er-1" };
      }
      if (method === "document.list") {
        if (payload.entityId === "empty") return [];
        if (payload.entityId === "rich")
          return [{ id: "d", entity_id: "rich", format: "rich-text", body: "<p>Hi</p>", revision: "dr-2" }];
        return [{ id: "d", entity_id: payload.entityId, format: "markdown", body: "# Title", revision: "dr-1" }];
      }
      throw new Error(`unexpected ${method}`);
    },
  };
  const normalize = (body, format) => (format === "rich-text" ? `md:${body}` : body);

  assert.deepEqual(await loadWritingDocument(rpc, "e1", normalize), {
    entityId: "e1",
    title: "Chapter one",
    markdown: "# Title",
    revision: "dr-1",
  });
  assert.deepEqual(
    calls.map((call) => call.method).sort(),
    ["document.list", "entity.get"],
    "only the read methods Writing is granted are used",
  );
  assert.deepEqual((await loadWritingDocument(rpc, "rich", normalize)).markdown, "md:<p>Hi</p>");
  assert.deepEqual(await loadWritingDocument(rpc, "empty", normalize), {
    entityId: "empty",
    title: "Chapter one",
    markdown: "",
    revision: "",
  });
  assert.equal(await loadWritingDocument(rpc, "gone", normalize), null);
  assert.equal(await loadWritingDocument(rpc, "trashed", normalize), null);
  await assert.rejects(
    loadWritingDocument(
      {
        async call() {
          throw new Error("broker offline");
        },
      },
      "e1",
      normalize,
    ),
    /broker offline/,
  );
}

// Entry wiring: the webview talks to the broker only, installs the shared broker editor host and reports ready.
{
  const main = fs.readFileSync("packages/modules/writing/src/plugin-main.ts", "utf8");
  assert.match(main, /createBrowserPluginRpcTransport\(\{/, "the entry uses the browser transport");
  assert.match(
    main,
    /client\.call<ModuleManifest\[\]>\("schema\.list"/,
    "the effective manifest comes from schema.list",
  );
  assert.match(main, /pickWritingManifest\(/);
  assert.match(
    main,
    /setDefaultEditorHost\(\s*createBrokerEditorHost\(/,
    "assets and entity links go through the broker host",
  );
  assert.match(main, /src\/lib\/editor\/broker-host\.ts/, "the broker host is the shared one");
  assert.match(main, /publish\(\{ kind: "ready"/);
  assert.match(main, /launchEntityId: \(\) => shownEntityId/, "the host follows the entity being shown");
  assert.doesNotMatch(main, /__TAURI|@tauri-apps|trusted_module_rpc/, "no Tauri access from the webview");

  const language = fs.readFileSync("packages/modules/language/src/plugin-main.ts", "utf8");
  assert.match(language, /src\/lib\/editor\/broker-host\.ts/, "Language uses the same shared broker host");

  const html = fs.readFileSync("src-tauri/plugin-assets/writing/index.html", "utf8");
  assert.match(html, /id="writing-root"/);
  assert.match(html, /writing\.js/);
  assert.match(html, /writing\.css/);
  assert.doesNotMatch(html, /plugin\.js/, "the shared plugin.js stub is gone");

  const css = fs.readFileSync("packages/modules/writing/src/plugin.css", "utf8");
  assert.match(
    css,
    /html,\s*body\s*{[^}]*height: 100%;[^}]*overflow: hidden;/,
    "the webview does not scroll its document",
  );

  const workspace = fs.readFileSync("packages/modules/writing/src/WritingWorkspace.svelte", "utf8");
  assert.match(workspace, /border-bottom: 0;/, "the panel runs flush to the window's bottom edge");
  assert.match(workspace, /case "writing\.load"/);
  assert.match(workspace, /<RichTextEditor/, "the webview edits with the shared rich-text editor");
  assert.match(workspace, /case "writing\.flush"/);
  assert.match(workspace, /chooseConflict\("reload"\)/);
  assert.match(workspace, /chooseConflict\("overwrite"\)/);
  assert.match(workspace, /chooseConflict\("recovery"\)/);
  // Full screen: the editor's toggle and Escape both report through setFullscreen, and the shell's off switch sets it.
  assert.match(workspace, /onFullscreenChange=\{setFullscreen\}/);
  assert.match(workspace, /kind: "fullscreen", entityId: view\.entityId, on: next/);
  assert.match(workspace, /case "writing\.fullscreen":[\s\S]*?fullscreen = message\.on;/);
  assert.doesNotMatch(workspace, /onFullscreenChange=\{\(next\) => \(fullscreen = next\)\}/, "toggles are published");
  // Rename and archive ask the shell; the shell checks the selection.
  assert.match(workspace, /kind: "request-rename", entityId: view\.entityId/);
  assert.match(workspace, /kind: "request-archive", entityId: view\.entityId/);
  // Forwarded shortcuts: the webview's own key listener maps keys with the shared rules, and prevents the default.
  assert.match(workspace, /window\.addEventListener\("keydown", handleShortcutKey\)/);
  assert.match(workspace, /writingShortcutForKey\(\{/);
  assert.match(
    workspace,
    /editable: isEditableKeyTarget\(event\.target\)/,
    "history keys know when the caret is in a field",
  );
  assert.match(workspace, /kind: "shortcut", shortcut/);
  // A rename updates the title in place: replacing `view` would remount the editor, which is keyed on it.
  assert.match(workspace, /view\.title = message\.title/);
  assert.match(workspace, /\.writing-editor :global\(\.editor-shell\)/, "the shared editor fills the panel");
  assert.doesNotMatch(workspace, /document\.save|documents\.save|entity\.update/, "the webview never saves");
  assert.doesNotMatch(main, /document\.save|documents\.save|entity\.update|field\.set/, "no write calls in the entry");
  assert.match(main, /transferText: \(text\) => transferText\(client, text\)/, "large drafts use the chunked hand-off");
  assert.match(
    main,
    /createBrowserPluginRpcTransport\(\{ maxResponseBytes: WRITING_RPC_MAX_RESPONSE_BYTES \}\)/,
    "long documents load: the response limit is raised past the SDK's 256 KiB default",
  );

  // AI: the editor's menu opens the webview's own dialog, which runs through the broker.
  assert.match(workspace, /aiEnabled=\{aiState\.enabled\}/);
  assert.match(workspace, /aiTemplates=\{aiState\.templates\}/);
  assert.match(workspace, /onAiRequest=\{openAiAction\}/);
  assert.match(workspace, /case "writing\.ai-state":[\s\S]*?aiState = \{ enabled: message\.enabled/);
  assert.match(
    workspace,
    /shellEditable && !loading && !conflict && view !== null && aiDialog === null/,
    "the editor is read-only while the dialog is open",
  );
  const startLoadSource = workspace.slice(
    workspace.indexOf("function startLoad"),
    workspace.indexOf("async function show("),
  );
  assert.match(startLoadSource, /closeAiDialog\(\);/, "a new load closes the dialog and so stops its request");
  const prepare = workspace.slice(
    workspace.indexOf("async function prepareAiRun"),
    workspace.indexOf("async function acceptAiProposal"),
  );
  assert.match(prepare, /editorRef\?\.flushPendingChanges\(\);\s*await sync\.publishNow\(\);/, "drafts go out at once");
  assert.doesNotMatch(prepare, /writing\.saved|awaitingSave/, "and the run does not wait for the save");
  const acceptSource = workspace.slice(
    workspace.indexOf("async function acceptAiProposal"),
    workspace.indexOf("function chooseConflict"),
  );
  assert.ok(
    acceptSource.indexOf("aiAcceptGuard(") < acceptSource.indexOf("validateAiProposal(") &&
      acceptSource.indexOf("validateAiProposal(") < acceptSource.indexOf("insertAiTextAtRequest("),
    "guard, then validation, then the editor",
  );
  assert.match(acceptSource, /replaceAiTextWithMarkdown\(proposal\)/);
  assert.match(
    acceptSource,
    /handleChange\(view, next\);[\s\S]*await sync\.publishNow\(\);/,
    "the new text goes out at once",
  );

  const dialog = fs.readFileSync("packages/modules/writing/src/AiRewriteDialog.svelte", "utf8");
  for (const label of ["Generate rewrite", "Generate text", "Accept proposal", "Retry", "Discard", "Cancel"]) {
    assert.ok(dialog.includes(label), `the dialog keeps the shell's "${label}" label`);
  }
  for (const title of ["Cancelling request…", "Rewriting selection…", "Review generated text", "Review rewrite"]) {
    assert.ok(dialog.includes(title), `the dialog keeps the shell's "${title}" title`);
  }
  assert.match(dialog, /AiProposalPreview/, "the shared proposal preview");
  assert.match(dialog, /event\.key !== "Escape"[\s\S]*?if \(!busy\) close\(\);/, "Escape closes unless a request runs");
  assert.match(dialog, /onDestroy\(\(\) => active\?\.dispose\(\)\)/, "unmounting stops the request");
  assert.match(dialog, /startAiRun\(/);
  assert.doesNotMatch(dialog + workspace, /@tauri-apps|__TAURI|\blisten\(|ai-stream/, "no shell events in the webview");
  for (const file of ["ai-flow.ts", "ai-run.ts"]) {
    const source = fs.readFileSync(`packages/modules/writing/src/${file}`, "utf8");
    assert.doesNotMatch(source, /@tauri-apps|\$lib\/project\/client|invoke\(/, `${file} is free of shell modules`);
  }
}

// AI menu choices: shared by the shell and the webview, so Custom instruction can run with or without a selection.
{
  const templates = mergePromptTemplates();
  const rewrite = (markdown, plain) => ({
    mode: "rewrite",
    selectionMarkdown: markdown,
    selectionPlain: plain,
    generationContext: "",
  });
  const generate = (context) => ({
    mode: "generate",
    selectionMarkdown: "",
    selectionPlain: "",
    generationContext: context,
  });
  assert.deepEqual(aiActionRequest(templates, "rewrite", "**A**", "A", "**A**"), rewrite("**A**", "A"));
  assert.equal(aiActionRequest(templates, "rewrite", " ", "", "ctx"), null, "a selection template needs a selection");
  assert.deepEqual(aiActionRequest(templates, "generate", "", "", "before\n[CURSOR]"), generate("before\n[CURSOR]"));
  assert.deepEqual(
    aiActionRequest(templates, "generate", "**A**", "A", "ctx"),
    generate("ctx"),
    "generate always writes at the cursor",
  );
  assert.deepEqual(
    aiActionRequest(templates, "custom", "**A**", "A", "ctx"),
    rewrite("**A**", "A"),
    "Custom instruction rewrites the selection when there is one",
  );
  assert.deepEqual(
    aiActionRequest(templates, "custom", "", "", "ctx"),
    generate("ctx"),
    "and writes at the cursor when there is none, so it can always start",
  );
  assert.equal(aiCanStart("generate", "", "Write a haiku."), true);
  assert.deepEqual(aiActionRequest([{ id: "mine" }], "mine", "", "", "ctx"), generate("ctx"), "a user's own template");
  assert.deepEqual(aiActionRequest([], "unknown", "x", "x", "ctx"), rewrite("x", "x"));
  for (const source of [
    fs.readFileSync("src/routes/+page.svelte", "utf8"),
    fs.readFileSync("packages/modules/writing/src/WritingWorkspace.svelte", "utf8"),
  ]) {
    const open = source.slice(source.indexOf("function openAiAction"), source.indexOf("function openAiAction") + 600);
    assert.match(
      open,
      /aiActionRequest\((aiPromptTemplates|aiState\.templates), action, markdown, plainText, context\)/,
    );
    assert.doesNotMatch(open, /action === "generate"/, "the mode comes from the shared rule");
  }
}

// AI flow: what the webview asks the broker, mapped from the shell's own request.
{
  assert.equal(WRITING_AI_RELATIONSHIP_DEPTH, 2);
  assert.equal(WRITING_AI_PASSAGE_COUNT, 8);
  assert.equal(WRITING_AI_QUERY_LIMIT, 4000);

  // The query is the shell's: entity name, instruction and plain selection, space-joined, empty parts dropped, cut.
  assert.equal(
    retrievalQueryFor({ entityName: "Chapter one", instruction: "Make it vivid.", selectionPlain: "The sea." }),
    "Chapter one Make it vivid. The sea.",
  );
  assert.equal(
    retrievalQueryFor({ entityName: "", instruction: "Write on.", selectionPlain: "" }),
    "Write on.",
    "a generate request has no selection",
  );
  assert.equal(retrievalQueryFor({ entityName: undefined, instruction: "", selectionPlain: "" }), "");
  assert.equal(retrievalQueryFor({ entityName: "N", instruction: "i".repeat(5000), selectionPlain: "s" }).length, 4000);

  // The policy is `direct_retrieval_context`'s: related to the entity, depth 2, eight passages, shared fields.
  assert.deepEqual(retrievalPolicyFor("e1", "the query"), {
    mode: "related",
    query: "the query",
    seedIds: ["e1"],
    allowedSourceKinds: [],
    relationshipDepth: 2,
    passageCount: 8,
    includeSharedFields: true,
  });
  assert.deepEqual(retrievalPolicyFor("e1", "   "), {
    mode: "related",
    seedIds: ["e1"],
    allowedSourceKinds: [],
    relationshipDepth: 2,
    passageCount: 8,
    includeSharedFields: true,
  });
  assert.deepEqual(retrievalPolicyFor(null, "only a query"), {
    mode: "project",
    query: "only a query",
    seedIds: [],
    allowedSourceKinds: [],
    relationshipDepth: 0,
    passageCount: 8,
    includeSharedFields: true,
  });
  assert.equal(retrievalPolicyFor(null, ""), null, "no entity and no query means no retrieval");
  assert.equal(retrievalPolicyFor("  ", " "), null);

  // Source text: rewrite sends the selection as it is; generate prefers the cursor context, then the document.
  const parts = {
    selectionMarkdown: "**Selected**",
    generationContext: "before\n[CURSOR]\nafter",
    documentMarkdown: "Doc",
  };
  assert.equal(aiSourceText("rewrite", parts), "**Selected**");
  assert.equal(aiSourceText("generate", parts), "before\n[CURSOR]\nafter");
  assert.equal(aiSourceText("generate", { ...parts, generationContext: "  " }), "Doc");
  assert.equal(aiSourceText("generate", { ...parts, generationContext: "", documentMarkdown: " \n" }), "[CURSOR]");

  assert.equal(aiCanStart("rewrite", "text", "Do it"), true);
  assert.equal(aiCanStart("rewrite", "  ", "Do it"), false, "a rewrite needs a selection");
  assert.equal(aiCanStart("generate", "", "Do it"), true);
  assert.equal(aiCanStart("generate", "", "  "), false, "every request needs an instruction");

  const rewrite = buildAiStartRequest({
    mode: "rewrite",
    instruction: "Make it vivid.",
    selectionMarkdown: "**The** sea.",
    selectionPlain: "The sea.",
    generationContext: "",
    documentMarkdown: "Whole document",
    entityId: "e1",
    entityName: "Chapter one",
    taskId: "task-1",
  });
  assert.deepEqual(rewrite, {
    operation: "generate_text",
    taskId: "task-1",
    userInstruction: "Make it vivid.",
    immediateContext: { selection: "**The** sea." },
    retrievalPolicy: {
      mode: "related",
      query: "Chapter one Make it vivid. The sea.",
      seedIds: ["e1"],
      allowedSourceKinds: [],
      relationshipDepth: 2,
      passageCount: 8,
      includeSharedFields: true,
    },
  });
  const generate = buildAiStartRequest({
    mode: "generate",
    instruction: "Write on.",
    selectionMarkdown: "",
    selectionPlain: "",
    generationContext: "",
    documentMarkdown: "",
    entityId: "e1",
    entityName: "Chapter one",
    taskId: "task-2",
  });
  assert.equal(generate.immediateContext.selection, "[CURSOR]");
  assert.equal(generate.retrievalPolicy.query, "Chapter one Write on.");
  // Only the keys the broker's payload schema allows.
  assert.deepEqual(Object.keys(rewrite).sort(), [
    "immediateContext",
    "operation",
    "retrievalPolicy",
    "taskId",
    "userInstruction",
  ]);

  // Accept guard: the entry, its load and its text must all be what the run started against.
  const source = { entityId: "e1", token: "t1", markdown: "Text" };
  assert.equal(aiAcceptGuard({ source, current: { ...source } }), null);
  assert.equal(aiAcceptGuard({ source, current: { ...source, entityId: "e2" } }), WRITING_AI_STALE_MESSAGE);
  assert.equal(aiAcceptGuard({ source, current: { ...source, token: "t2" } }), WRITING_AI_STALE_MESSAGE);
  assert.equal(aiAcceptGuard({ source, current: { ...source, markdown: "Text!" } }), WRITING_AI_STALE_MESSAGE);
  assert.equal(aiAcceptGuard({ source, current: null }), WRITING_AI_STALE_MESSAGE);
  assert.equal(
    WRITING_AI_STALE_MESSAGE,
    "The entity or document changed while the rewrite was being prepared. Discard it and try again.",
    "the shell's own wording",
  );

  // Proposal validation is the shell's, now shared.
  assert.equal(validateAiProposal("A plain sentence with *emphasis*."), null);
  assert.equal(validateAiProposal("  "), "The AI provider returned an empty proposal.");
  assert.match(validateAiProposal("# Heading\ntext"), /block-level Markdown/);
  assert.match(validateAiProposal("- item"), /block-level Markdown/);
  assert.match(validateAiProposal("text <b>bold</b>"), /HTML markup/);
  const page = fs.readFileSync("src/routes/+page.svelte", "utf8");
  assert.match(page, /import \{ validateAiProposal \} from "\$lib\/ai\/proposal"/, "the shell uses the shared rule");
  assert.doesNotMatch(page, /function validateAiProposal/, "and no longer keeps its own copy");

  // Events: a poll repeats the buffer, so applied sequences are skipped; the stream builds the proposal.
  const event = (sequence, phase, extra = {}) => ({
    sequence,
    requestId: "r1",
    phase,
    delta: null,
    output: null,
    error: null,
    ...extra,
  });
  {
    const buffer = [event(0, "started"), event(1, "delta", { delta: "Hel" }), event(2, "delta", { delta: "lo" })];
    let state = applyAiEvents(initialAiRunState(), buffer);
    assert.equal(state.streamText, "Hello");
    assert.equal(state.phase, "running");
    assert.equal(state.lastSequence, 2);
    assert.equal(state.progressMessage, "Writing proposal…");
    // The same buffer again, now with one more event: nothing is applied twice.
    state = applyAiEvents(state, [...buffer, event(3, "delta", { delta: "!" })]);
    assert.equal(state.streamText, "Hello!");
    state = applyAiEvents(state, [event(4, "usage", { output: '{"inputTokens":3,"outputTokens":2,"totalTokens":5}' })]);
    assert.deepEqual(state.usage, { inputTokens: 3, outputTokens: 2, totalTokens: 5 });
    state = applyAiEvents(state, [event(5, "completed", { output: "Hello!" })]);
    assert.equal(state.phase, "completed");
    assert.equal(state.proposal, "Hello!");
    assert.equal(state.error, "");
    assert.equal(state.progressMessage, "");
    // Nothing after a terminal phase changes it.
    const later = applyAiEvents(state, [event(6, "delta", { delta: "late" }), event(7, "failed", { error: "x" })]);
    assert.deepEqual(later, state);
    assert.equal(state.gap, false);
  }
  // Unreadable usage is no usage; a usage line without output changes nothing.
  assert.equal(applyAiEvent(initialAiRunState(), event(0, "usage", { output: "not json" })).usage, null);
  assert.equal(applyAiEvent(initialAiRunState(), event(0, "usage")).usage, null);
  // The terminal output wins when the stream is shorter, and the stream wins when it is longer (as in the shell).
  assert.equal(
    applyAiEvents(initialAiRunState(), [event(0, "delta", { delta: "ab" }), event(1, "completed", { output: "abcd" })])
      .proposal,
    "abcd",
  );
  // Terminal phases and their messages.
  {
    const cancelled = applyAiEvents(initialAiRunState(), [
      event(0, "delta", { delta: "part" }),
      event(1, "cancelled", { output: "partial" }),
    ]);
    assert.equal(cancelled.phase, "cancelled");
    assert.equal(cancelled.proposal, "partial");
    assert.equal(cancelled.error, "");
    const late = applyAiEvents(initialAiRunState(), [
      event(0, "delta", { delta: "part" }),
      event(1, "deadline_exceeded", { output: "partial" }),
    ]);
    assert.equal(late.phase, "deadline_exceeded");
    assert.equal(late.error, "AI generation reached its time limit. The partial proposal is preserved below.");
    assert.equal(
      applyAiEvent(initialAiRunState(), event(0, "deadline_exceeded")).error,
      "AI generation reached its time limit before producing a proposal.",
    );
    const failed = applyAiEvent(initialAiRunState(), event(0, "failed", { error: "Provider unavailable" }));
    assert.equal(failed.phase, "failed");
    assert.equal(failed.error, "Provider unavailable");
    assert.equal(
      applyAiEvent(initialAiRunState(), event(0, "failed")).error,
      "The AI provider could not generate a proposal",
    );
  }
  // A bounded buffer can drop deltas. The run notices the gap, and the completed output is the proposal.
  {
    const dropped = applyAiEvents(initialAiRunState(), [
      event(40, "delta", { delta: "tail " }),
      event(41, "completed", { output: "the whole text" }),
    ]);
    assert.equal(dropped.gap, true);
    assert.equal(dropped.proposal, "the whole text", "not the gappy stream");
    const noOutput = applyAiEvents(initialAiRunState(), [
      event(0, "delta", { delta: "head " }),
      event(9, "delta", { delta: "tail" }),
      event(10, "failed", { error: "boom" }),
    ]);
    assert.equal(noOutput.gap, true);
    assert.equal(noOutput.proposal, "", "a failed run with holes in its stream keeps no proposal");
    const intact = applyAiEvents(initialAiRunState(), [
      event(0, "delta", { delta: "head " }),
      event(1, "delta", { delta: "tail" }),
      event(2, "failed", { error: "boom" }),
    ]);
    assert.equal(intact.proposal, "head tail", "without holes the partial text stays, as in the shell");
  }
}

// AI run: start, poll on a timer, dedupe, stop on a terminal phase, cancel, dispose, and release the request.
{
  const fakeTimers = () => {
    const timers = new Map();
    let next = 1;
    return {
      setTimer(callback, ms) {
        timers.set(next, { callback, ms });
        return next++;
      },
      clearTimer(handle) {
        timers.delete(handle);
      },
      count: () => timers.size,
      delays: () => [...timers.values()].map((timer) => timer.ms),
      async fire() {
        const due = [...timers.entries()];
        timers.clear();
        for (const [, timer] of due) timer.callback();
        await settle();
      },
    };
  };
  const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
  const event = (sequence, phase, extra = {}) => ({
    sequence,
    requestId: "req-1",
    phase,
    delta: null,
    output: null,
    error: null,
    ...extra,
  });
  const request = {
    operation: "generate_text",
    taskId: "task",
    userInstruction: "Rewrite",
    immediateContext: { selection: "Text" },
  };
  /** A fake broker whose `poll` answers come from `script`, one per call (the last repeats). */
  const make = (script, overrides = {}) => {
    const clock = fakeTimers();
    const calls = [];
    const views = [];
    let polls = 0;
    const rpc = {
      async call(method, payload) {
        calls.push({ method, payload });
        if (overrides[method]) return overrides[method](payload, calls.filter((call) => call.method === method).length);
        if (method === "ai.request.start") return { requestId: "req-1" };
        if (method === "ai.request.poll") {
          const answer = script[Math.min(polls, script.length - 1)];
          polls += 1;
          if (answer instanceof Error) throw answer;
          return answer;
        }
        if (method === "ai.request.cancel") return null;
        if (method === "ai.request.citations") return [];
        if (method === "ai.request.result") return { output: "from result" };
        throw new Error(`unexpected ${method}`);
      },
    };
    const run = startAiRun(
      { rpc, onUpdate: (view) => views.push(view), setTimer: clock.setTimer, clearTimer: clock.clearTimer },
      request,
    );
    const methods = () => calls.map((call) => call.method);
    return { run, clock, calls, views, methods };
  };

  assert.equal(WRITING_AI_POLL_MS, 200);
  // The happy path: start, an immediate poll, polling every 200 ms, no repeats, release after the terminal event.
  {
    const first = [event(0, "started"), event(1, "delta", { delta: "He" })];
    const second = [...first, event(2, "delta", { delta: "llo" })];
    const third = [...second, event(3, "completed", { output: "Hello" })];
    const { clock, calls, views, methods } = make([first, second, third]);
    await settle();
    assert.deepEqual(calls[0], { method: "ai.request.start", payload: request });
    assert.deepEqual(methods(), ["ai.request.start", "ai.request.poll"], "the first poll follows the start");
    assert.deepEqual(clock.delays(), [200]);
    assert.equal(views.at(-1).streamText, "He");
    assert.equal(views.at(-1).phase, "running");
    await clock.fire();
    assert.equal(views.at(-1).streamText, "Hello", "events already applied are not applied again");
    assert.equal(clock.count(), 1);
    await clock.fire();
    const last = views.at(-1);
    assert.equal(last.phase, "completed");
    assert.equal(last.proposal, "Hello");
    assert.equal(clock.count(), 0, "polling stops at the terminal phase");
    assert.deepEqual(calls.at(-1), { method: "ai.request.citations", payload: { requestId: "req-1" } }, "released");
    assert.equal(methods().filter((method) => method === "ai.request.poll").length, 3);
    assert.equal(methods().includes("ai.request.result"), false, "the event carried the output");
    await clock.fire();
    assert.equal(methods().filter((method) => method === "ai.request.poll").length, 3, "no polling after the end");
  }
  // A completed event without output falls back to the broker's result.
  {
    const { views, methods, clock } = make([[event(0, "completed")]]);
    await settle();
    assert.equal(views.at(-1).phase, "completed");
    assert.equal(views.at(-1).proposal, "from result");
    assert.deepEqual(methods().slice(-2), ["ai.request.result", "ai.request.citations"]);
    assert.equal(clock.count(), 0);
  }
  // A failed start ends the run with the broker's message and nothing to release.
  {
    const { views, methods, clock } = make([[]], {
      "ai.request.start": () => {
        throw new Error("Configure an AI provider for this project first");
      },
    });
    await settle();
    assert.equal(views.at(-1).phase, "failed");
    assert.equal(views.at(-1).error, "Configure an AI provider for this project first");
    assert.deepEqual(methods(), ["ai.request.start"]);
    assert.equal(clock.count(), 0);
  }
  // Cancel: the broker is asked, the run keeps polling until the cancelled event, and shows "cancelling" meanwhile.
  {
    const running = [event(0, "started"), event(1, "delta", { delta: "Half" })];
    const { run, clock, views, methods } = make([running, [...running, event(2, "cancelled", { output: "Half" })]]);
    await settle();
    await run.cancel();
    assert.equal(methods().includes("ai.request.cancel"), true);
    assert.equal(views.at(-1).cancelPending, true);
    assert.equal(views.at(-1).phase, "running");
    await clock.fire();
    assert.equal(views.at(-1).phase, "cancelled");
    assert.equal(views.at(-1).proposal, "Half");
    assert.equal(views.at(-1).cancelPending, false);
    assert.equal(clock.count(), 0);
    assert.equal(methods().at(-1), "ai.request.citations");
  }
  // Cancel before the broker named the request waits for the name, then cancels.
  {
    let release;
    const gate = new Promise((resolve) => (release = resolve));
    const { run, methods } = make([[event(0, "started")]], {
      "ai.request.start": async () => {
        await gate;
        return { requestId: "req-1" };
      },
    });
    await run.cancel();
    assert.equal(methods().includes("ai.request.cancel"), false, "no id to cancel yet");
    release();
    await settle();
    assert.ok(methods().indexOf("ai.request.cancel") > methods().indexOf("ai.request.start"));
  }
  // A cancel that fails clears the pending flag and says why; a run that ends on its own drops that message.
  {
    const { run, views, clock } = make(
      [[event(0, "started")], [event(0, "started"), event(1, "completed", { output: "Done" })]],
      {
        "ai.request.cancel": () => {
          throw new Error("AI request does not exist");
        },
      },
    );
    await settle();
    await run.cancel();
    assert.equal(views.at(-1).cancelPending, false);
    assert.equal(views.at(-1).error, "AI request does not exist");
    await clock.fire();
    assert.equal(views.at(-1).phase, "completed");
    assert.equal(views.at(-1).error, "", "the request had finished on its own");
  }
  // Dispose (the dialog closed, or another entry loaded): polling stops, a running request is cancelled and
  // released, and the run goes quiet.
  {
    const { run, clock, views, methods } = make([[event(0, "started")]]);
    await settle();
    const seen = views.length;
    run.dispose();
    await settle();
    assert.equal(clock.count(), 0, "the poll timer is cleared");
    assert.deepEqual(methods().slice(-2), ["ai.request.cancel", "ai.request.citations"]);
    await clock.fire();
    assert.equal(views.length, seen, "no updates after dispose");
    run.dispose();
    assert.equal(methods().filter((method) => method === "ai.request.cancel").length, 1, "dispose is idempotent");
  }
  // Disposed while the start call is in flight: the request that comes back is stopped.
  {
    let release;
    const gate = new Promise((resolve) => (release = resolve));
    const { run, methods, clock } = make([[event(0, "started")]], {
      "ai.request.start": async () => {
        await gate;
        return { requestId: "req-1" };
      },
    });
    run.dispose();
    release();
    await settle();
    assert.deepEqual(methods(), ["ai.request.start", "ai.request.cancel", "ai.request.citations"]);
    assert.equal(clock.count(), 0);
  }
  // Disposing a finished run does not cancel anything again.
  {
    const { run, methods } = make([[event(0, "completed", { output: "x" })]]);
    await settle();
    run.dispose();
    await settle();
    assert.equal(methods().includes("ai.request.cancel"), false);
  }
  // Poll errors: transient ones are retried; a request that stays unreachable ends the run.
  {
    const script = [new Error("blip"), [event(0, "started"), event(1, "completed", { output: "Ok" })]];
    const { clock, views } = make(script);
    await settle();
    assert.equal(views.at(-1).phase, "running", "one failed poll is not the end");
    await clock.fire();
    assert.equal(views.at(-1).phase, "completed");
  }
  {
    const { clock, views, methods } = make([new Error("request is gone")]);
    await settle();
    for (let attempt = 1; attempt < WRITING_AI_MAX_POLL_FAILURES; attempt += 1) await clock.fire();
    assert.equal(views.at(-1).phase, "failed");
    assert.equal(views.at(-1).error, "request is gone");
    assert.equal(clock.count(), 0);
    assert.equal(methods().filter((method) => method === "ai.request.poll").length, WRITING_AI_MAX_POLL_FAILURES);
    assert.deepEqual(methods().slice(-2), ["ai.request.cancel", "ai.request.citations"], "and the request is stopped");
  }
}

// Bundle markers: the committed bundle is the broker-only build of this entry.
{
  const bundle = fs.readFileSync("src-tauri/plugin-assets/writing/writing.js", "utf8");
  assert.ok(bundle.length > 512, "the writing bundle is built");
  assert.match(bundle, /(["'`])\/__rpc\1/, "the bundle talks to the broker endpoint");
  assert.match(bundle, /daena\.writing\/state@1/, "the bundle publishes the Writing state event");
  assert.match(bundle, /writing-root/, "the bundle mounts the Writing root");
  assert.match(bundle, /asset\.shared\.read\.begin/, "the bundle reads images through the broker host");
  assert.match(bundle, /text\.transfer\.begin/, "the bundle hands large drafts over through text.transfer");
  assert.match(bundle, /Document editor/, "the bundle carries the shared rich-text editor");
  assert.match(bundle, /ai\.request\.start/, "the bundle starts AI requests through the broker");
  assert.match(bundle, /ai\.request\.poll/, "and polls them");
  assert.match(bundle, /Generate rewrite/, "the bundle carries the AI dialog");
  assert.doesNotMatch(
    bundle,
    /__TAURI_INTERNALS__|@tauri-apps|trusted_module_rpc/,
    "no Tauri or trusted RPC in the bundle",
  );
}

console.log("writing plugin source tests passed");
