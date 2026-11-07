import assert from "node:assert/strict";
import {
  LANGUAGE_LEAVE_PENDING_TIMEOUT_MS,
  LanguageLeaveTracker,
  isLanguagePane,
  openableLinkHref,
  parseLanguageStateEvent,
} from "../src/lib/modules/language-bridge.ts";

// Fake timers so the 1500 ms rule is exercised without waiting.
function fakeTimers() {
  let now = 0;
  let nextHandle = 1;
  const timers = new Map();
  return {
    timers: {
      set(callback, ms) {
        const handle = nextHandle++;
        timers.set(handle, { at: now + ms, callback });
        return handle;
      },
      clear(handle) {
        timers.delete(handle);
      },
    },
    advance(ms) {
      now += ms;
      for (const [handle, timer] of [...timers.entries()]) {
        if (timer.at <= now && timers.has(handle)) {
          timers.delete(handle);
          timer.callback();
        }
      }
    },
    count: () => timers.size,
  };
}

// Records a promise result. Resolution callbacks run as microtasks, so tests call flush() first.
function watch(promise) {
  const record = { state: "pending", value: undefined };
  promise.then((value) => {
    record.state = "resolved";
    record.value = value;
  });
  return record;
}

async function flush() {
  await new Promise((resolve) => setImmediate(resolve));
}

// Link opening: only http, https and mailto leave the app.
assert.equal(openableLinkHref("https://example.com/a?b=1"), "https://example.com/a?b=1");
assert.equal(openableLinkHref("http://example.com"), "http://example.com/");
assert.equal(openableLinkHref("HTTPS://Example.com"), "https://example.com/");
assert.equal(openableLinkHref("mailto:writer@example.com"), "mailto:writer@example.com");
assert.equal(openableLinkHref("  https://example.com  "), "https://example.com/");
for (const blocked of [
  "javascript:alert(1)",
  "file:///etc/passwd",
  "daena://project/open",
  "plugin://daena.language/dist/ui/index.html",
  "ftp://example.com/file",
  "data:text/html,<script>1</script>",
  "/relative/path",
  "example.com",
  "",
  "   ",
]) {
  assert.equal(openableLinkHref(blocked), null, `scheme must be blocked: ${blocked}`);
}
assert.equal(openableLinkHref(undefined), null);
assert.equal(openableLinkHref(42), null);
assert.equal(openableLinkHref(`https://example.com/${"a".repeat(5000)}`), null, "oversized hrefs are refused");

// Pane names come from the protocol list only.
for (const pane of ["overview", "lexicon", "sounds", "writing", "grammar", "forms", "samples"]) {
  assert.equal(isLanguagePane(pane), true, pane);
}
assert.equal(isLanguagePane("settings"), false);
assert.equal(isLanguagePane(undefined), false);

// Payload validation: unknown kinds and malformed shapes are ignored.
assert.equal(parseLanguageStateEvent(null), null);
assert.equal(parseLanguageStateEvent("ready"), null);
assert.equal(parseLanguageStateEvent([{ kind: "ready" }]), null);
assert.equal(parseLanguageStateEvent({ kind: "unknown-kind", pane: "overview" }), null, "unknown kinds ignored");
assert.equal(parseLanguageStateEvent({}), null);
assert.equal(parseLanguageStateEvent({ kind: "pane", pane: "settings" }), null);
assert.equal(parseLanguageStateEvent({ kind: "leave-pending" }), null);
assert.equal(parseLanguageStateEvent({ kind: "leave-reply", requestId: "r1" }), null, "allowed must be boolean");
assert.equal(parseLanguageStateEvent({ kind: "leave-reply", requestId: "r1", allowed: "yes" }), null);
assert.equal(parseLanguageStateEvent({ kind: "entity-changed", entityId: "e1" }), null, "deleted must be boolean");
assert.equal(parseLanguageStateEvent({ kind: "entity-changed", entityId: "", deleted: false }), null);
assert.equal(parseLanguageStateEvent({ kind: "open-link" }), null);
assert.equal(parseLanguageStateEvent({ kind: "ready", pane: "overview", entityId: 7 }), null);

assert.deepEqual(parseLanguageStateEvent({ kind: "ready", pane: "lexicon", entityId: null }), {
  kind: "ready",
  pane: "lexicon",
  entityId: null,
});
assert.deepEqual(parseLanguageStateEvent({ kind: "ready" }), { kind: "ready", pane: null, entityId: null });
assert.deepEqual(parseLanguageStateEvent({ kind: "ready", pane: "bogus", entityId: "e1" }), {
  kind: "ready",
  pane: null,
  entityId: "e1",
});
assert.deepEqual(parseLanguageStateEvent({ kind: "pane", pane: "grammar" }), { kind: "pane", pane: "grammar" });
assert.deepEqual(parseLanguageStateEvent({ kind: "leave-pending", requestId: "r1" }), {
  kind: "leave-pending",
  requestId: "r1",
});
assert.deepEqual(parseLanguageStateEvent({ kind: "leave-reply", requestId: "r1", allowed: false }), {
  kind: "leave-reply",
  requestId: "r1",
  allowed: false,
});
assert.deepEqual(parseLanguageStateEvent({ kind: "entity-changed", entityId: "e1", deleted: true }), {
  kind: "entity-changed",
  entityId: "e1",
  deleted: true,
});
assert.deepEqual(parseLanguageStateEvent({ kind: "open-link", href: "https://example.com" }), {
  kind: "open-link",
  href: "https://example.com",
});

// open-entity: the id must be a non-empty string no longer than the shared id limit (128).
assert.equal(parseLanguageStateEvent({ kind: "open-entity" }), null);
assert.equal(parseLanguageStateEvent({ kind: "open-entity", entityId: "" }), null, "empty ids are dropped");
assert.equal(parseLanguageStateEvent({ kind: "open-entity", entityId: 42 }), null, "ids must be strings");
assert.equal(parseLanguageStateEvent({ kind: "open-entity", id: "e-1" }), null, "the field is entityId");
assert.equal(
  parseLanguageStateEvent({ kind: "open-entity", entityId: "x".repeat(129) }),
  null,
  "overlong ids are dropped",
);
assert.deepEqual(parseLanguageStateEvent({ kind: "open-entity", entityId: "e-1" }), {
  kind: "open-entity",
  entityId: "e-1",
});
assert.deepEqual(parseLanguageStateEvent({ kind: "open-entity", entityId: "x".repeat(128) }), {
  kind: "open-entity",
  entityId: "x".repeat(128),
});
assert.deepEqual(parseLanguageStateEvent({ kind: "open-entity", entityId: "e-1", newTab: true }), {
  kind: "open-entity",
  entityId: "e-1",
  newTab: true,
});
assert.deepEqual(parseLanguageStateEvent({ kind: "open-entity", entityId: "e-1", newTab: false }), {
  kind: "open-entity",
  entityId: "e-1",
  newTab: false,
});
assert.deepEqual(parseLanguageStateEvent({ kind: "open-entity", entityId: "e-1", newTab: "yes" }), {
  kind: "open-entity",
  entityId: "e-1",
});
assert.deepEqual(parseLanguageStateEvent({ kind: "open-entity", entityId: "e-1", newTab: null }), {
  kind: "open-entity",
  entityId: "e-1",
});
assert.equal(parseLanguageStateEvent({ kind: "open-entity", entityId: "", newTab: true }), null);
assert.equal(parseLanguageStateEvent({ kind: "shortcut" }), null);
assert.equal(parseLanguageStateEvent({ kind: "shortcut", shortcut: "quick-open" }), null);
assert.equal(parseLanguageStateEvent({ kind: "shortcut", shortcut: "tab-9" }), null);
assert.equal(parseLanguageStateEvent({ kind: "shortcut", shortcut: 1 }), null);
assert.deepEqual(parseLanguageStateEvent({ kind: "shortcut", shortcut: "tab-next" }), {
  kind: "shortcut",
  shortcut: "tab-next",
});
assert.deepEqual(parseLanguageStateEvent({ kind: "shortcut", shortcut: "tab-last" }), {
  kind: "shortcut",
  shortcut: "tab-last",
});

assert.equal(LANGUAGE_LEAVE_PENDING_TIMEOUT_MS, 1500);

// Can-leave: no leave-pending within the timeout means the webview is gone, so allow.
{
  const clock = fakeTimers();
  const tracker = new LanguageLeaveTracker({ timers: clock.timers });
  const request = watch(tracker.begin("r-timeout"));
  clock.advance(1499);
  await flush();
  assert.equal(request.state, "pending", "still waiting one millisecond before the timeout");
  clock.advance(1);
  await flush();
  assert.equal(request.state, "resolved");
  assert.equal(request.value, true, "pending timeout resolves allowed");
  assert.equal(tracker.outstanding, 0);
}

// Can-leave: pending then reply false keeps the shell from leaving.
{
  const clock = fakeTimers();
  const tracker = new LanguageLeaveTracker({ timers: clock.timers });
  const request = watch(tracker.begin("r-refuse"));
  tracker.pending("r-refuse");
  clock.advance(60_000);
  await flush();
  assert.equal(request.state, "pending", "after leave-pending the wait has no timeout");
  tracker.reply("r-refuse", false);
  await flush();
  assert.equal(request.state, "resolved");
  assert.equal(request.value, false, "pending then reply false resolves false");
  assert.equal(clock.count(), 0, "no timers left behind");
}

// Can-leave: pending then reply true.
{
  const clock = fakeTimers();
  const tracker = new LanguageLeaveTracker({ timers: clock.timers });
  const request = watch(tracker.begin("r-allow"));
  tracker.pending("r-allow");
  tracker.reply("r-allow", true);
  await flush();
  assert.equal(request.value, true);
}

// Can-leave: ready while pending releases the request as allowed.
{
  const clock = fakeTimers();
  const tracker = new LanguageLeaveTracker({ timers: clock.timers });
  const request = watch(tracker.begin("r-ready"));
  tracker.pending("r-ready");
  tracker.releaseAll();
  await flush();
  assert.equal(request.state, "resolved");
  assert.equal(request.value, true, "ready while pending resolves true");
  tracker.reply("r-ready", false);
  await flush();
  assert.equal(request.value, true, "a late reply after release changes nothing");
}

// Can-leave: ready before any leave-pending also releases the request.
{
  const clock = fakeTimers();
  const tracker = new LanguageLeaveTracker({ timers: clock.timers });
  const request = watch(tracker.begin("r-early"));
  tracker.releaseAll();
  await flush();
  assert.equal(request.value, true);
  assert.equal(clock.count(), 0);
}

// Unknown replies are ignored, and the timeout does not fire after leave-pending.
{
  const clock = fakeTimers();
  const tracker = new LanguageLeaveTracker({ timers: clock.timers });
  tracker.reply("never-sent", false);
  tracker.pending("never-sent");
  assert.equal(tracker.outstanding, 0);
  const request = watch(tracker.begin("r-late"));
  tracker.pending("r-late");
  clock.advance(5000);
  await flush();
  assert.equal(request.state, "pending", "pending cancels the 1500 ms timeout");
  tracker.reply("r-late", true);
  await flush();
  assert.equal(request.value, true);
}

// Concurrent requests resolve independently by requestId.
{
  const clock = fakeTimers();
  const tracker = new LanguageLeaveTracker({ timers: clock.timers });
  const first = watch(tracker.begin("r-a"));
  const second = watch(tracker.begin("r-b"));
  tracker.pending("r-a");
  tracker.reply("r-a", false);
  await flush();
  assert.equal(first.value, false);
  assert.equal(second.state, "pending");
  clock.advance(LANGUAGE_LEAVE_PENDING_TIMEOUT_MS);
  await flush();
  assert.equal(second.value, true, "the other request still times out on its own clock");
}

console.log("language bridge checks passed");
