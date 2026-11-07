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
  WRITING_DRAFT_IDLE_MS,
  WRITING_DRAFT_INLINE_JSON_LIMIT_BYTES,
  WRITING_DRAFT_RETRY_MS,
  createDraftSync,
  draftNeedsTransfer,
} from "../packages/modules/writing/src/draft-sync.ts";
import {
  parseWritingStateEvent,
  writingConflictMessage,
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
assert.deepEqual(parseWritingHostMessage({ type: "writing.ai-state", enabled: true }), {
  type: "writing.ai-state",
  enabled: true,
});
assert.equal(parseWritingHostMessage({ type: "writing.ai-state" }), null);
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
  ];
  for (const payload of rejected) {
    assert.equal(parseWritingStatePayload(payload), null, `webview rejects ${JSON.stringify(payload)}`);
    assert.equal(parseWritingStateEvent(payload), null, `shell rejects ${JSON.stringify(payload)}`);
  }
  assert.equal(parseWritingDraft({ ...inline, text: "é".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES) }), null);
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
  assert.match(main, /createBrowserPluginRpcTransport\(\)/);
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
  assert.match(workspace, /editable: isEditableKeyTarget\(event\.target\)/, "history keys know when the caret is in a field");
  assert.match(workspace, /kind: "shortcut", shortcut/);
  // A rename updates the title in place: replacing `view` would remount the editor, which is keyed on it.
  assert.match(workspace, /view\.title = message\.title/);
  assert.match(workspace, /\.writing-editor :global\(\.editor-shell\)/, "the shared editor fills the panel");
  assert.doesNotMatch(workspace, /document\.save|documents\.save|entity\.update/, "the webview never saves");
  assert.doesNotMatch(main, /document\.save|documents\.save|entity\.update|field\.set/, "no write calls in the entry");
  assert.match(main, /transferText: \(text\) => transferText\(client, text\)/, "large drafts use the chunked hand-off");
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
  assert.doesNotMatch(
    bundle,
    /__TAURI_INTERNALS__|@tauri-apps|trusted_module_rpc/,
    "no Tauri or trusted RPC in the bundle",
  );
}

console.log("writing plugin source tests passed");
