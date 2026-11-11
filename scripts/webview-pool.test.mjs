import assert from "node:assert/strict";
import {
  WEBVIEW_POOL_LIMIT,
  emptyWebviewPool,
  evicted,
  findWebviewEntry,
  markReady,
  prunePlugins,
  pruneTabs,
  removeWebviews,
  retainSet,
  touch,
  webviewMountKey,
  webviewRefs,
} from "../src/lib/plugins/webview-pool.ts";

function deepFreeze(value) {
  if (value !== null && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const key of Object.keys(value)) deepFreeze(value[key]);
  }
  return value;
}

const W = "daena.writing";
const L = "daena.language";
const ref = (instanceId, pluginId = W) => ({ pluginId, instanceId });
const ids = (refs) => refs.map((item) => `${item.pluginId}:${item.instanceId}`);

assert.equal(WEBVIEW_POOL_LIMIT, 3, "the pool holds three webviews, including the one on screen");

// touch: adds, refreshes, and resets readiness only when the mount key changes.
{
  const pool = deepFreeze(emptyWebviewPool);
  const first = touch(pool, ref("a"), "k1", 10);
  assert.deepEqual(first, [{ pluginId: W, instanceId: "a", mountKey: "k1", ready: false, lastUsedAt: 10 }]);
  assert.deepEqual(pool, [], "the input pool is not changed");

  const ready = markReady(first, ref("a"), true);
  assert.equal(findWebviewEntry(ready, ref("a")).ready, true);
  assert.equal(findWebviewEntry(first, ref("a")).ready, false, "markReady does not change its input");

  const again = touch(ready, ref("a"), "k1", 20);
  assert.equal(findWebviewEntry(again, ref("a")).ready, true, "same key keeps readiness");
  assert.equal(findWebviewEntry(again, ref("a")).lastUsedAt, 20);
  assert.equal(touch(again, ref("a"), "k1", 20), again, "an unchanged touch returns the same pool");

  const changed = touch(again, ref("a"), "k2", 30);
  assert.equal(findWebviewEntry(changed, ref("a")).ready, false, "a new mount key is a new document");
  assert.equal(findWebviewEntry(changed, ref("a")).mountKey, "k2");
}

// Keyed by (instance, plugin): the same tab id under two plugins is two entries.
{
  let pool = deepFreeze(emptyWebviewPool);
  pool = touch(pool, ref("a", W), "w", 1);
  pool = touch(pool, ref("a", L), "l", 2);
  assert.equal(pool.length, 2);
  pool = markReady(pool, ref("a", L), true);
  assert.equal(findWebviewEntry(pool, ref("a", W)).ready, false);
  assert.equal(findWebviewEntry(pool, ref("a", L)).ready, true);
}

// markReady: an unknown instance is ignored, and a no-op returns the same pool.
{
  const pool = deepFreeze(touch(emptyWebviewPool, ref("a"), "k", 1));
  assert.equal(markReady(pool, ref("zzz"), true), pool);
  assert.equal(markReady(pool, ref("a"), false), pool);
}

// retainSet: the shown entry first, then the most recently used others, `limit` in all.
{
  let pool = emptyWebviewPool;
  pool = touch(pool, ref("a"), "k", 10);
  pool = touch(pool, ref("b"), "k", 20);
  pool = touch(pool, ref("c"), "k", 30);
  pool = touch(pool, ref("d"), "k", 40);
  deepFreeze(pool);
  assert.deepEqual(ids(retainSet(pool, ref("d"), 3)), [`${W}:d`, `${W}:c`, `${W}:b`]);
  assert.deepEqual(ids(retainSet(pool, ref("a"), 3)), [`${W}:a`, `${W}:d`, `${W}:c`], "the shown one is never evicted");
  assert.deepEqual(ids(retainSet(pool, ref("a"), 1)), [`${W}:a`]);
  assert.deepEqual(ids(retainSet(pool, ref("a"), 0)), [`${W}:a`], "the shown one stays at limit 0");
  assert.deepEqual(ids(retainSet(pool, ref("a"), 10)), [`${W}:a`, `${W}:d`, `${W}:c`, `${W}:b`]);
  // A shown instance the pool does not know yet is still retained, and counts toward the limit.
  assert.deepEqual(ids(retainSet(pool, ref("new"), 3)), [`${W}:new`, `${W}:d`, `${W}:c`]);
  assert.deepEqual(ids(retainSet(emptyWebviewPool, ref("new"), 3)), [`${W}:new`]);
  // Equal use time: the entry added later is kept.
  const tied = touch(touch(touch(emptyWebviewPool, ref("a"), "k", 5), ref("b"), "k", 5), ref("c"), "k", 5);
  assert.deepEqual(ids(retainSet(tied, ref("c"), 2)), [`${W}:c`, `${W}:b`]);
  // Another plugin's instance with the same id is another instance.
  const mixed = touch(touch(emptyWebviewPool, ref("a", L), "k", 1), ref("a", W), "k", 2);
  assert.deepEqual(ids(retainSet(mixed, ref("a", W), 3)), [`${W}:a`, `${L}:a`]);
}

// evicted: what the host closes when only `retain` survives.
{
  let pool = emptyWebviewPool;
  for (const [i, id] of ["a", "b", "c", "d"].entries()) pool = touch(pool, ref(id), "k", i);
  deepFreeze(pool);
  const retain = retainSet(pool, ref("d"), 3);
  assert.deepEqual(ids(evicted(pool, retain)), [`${W}:a`], "the least recently used hidden entry goes");
  assert.deepEqual(evicted(pool, webviewRefs(pool)), []);
  assert.deepEqual(ids(evicted(pool, [])), ids(webviewRefs(pool)));
  const dropped = removeWebviews(pool, evicted(pool, retain));
  assert.deepEqual(ids(webviewRefs(dropped)), [`${W}:b`, `${W}:c`, `${W}:d`]);
  assert.equal(removeWebviews(pool, []), pool);
  assert.equal(removeWebviews(pool, [ref("zzz")]), pool);
}

// pruneTabs: an entry lives only while its tab does.
{
  let pool = emptyWebviewPool;
  pool = touch(pool, ref("a"), "k", 1);
  pool = touch(pool, ref("b"), "k", 2);
  pool = touch(pool, ref("b", L), "k", 3);
  deepFreeze(pool);
  const pruned = pruneTabs(pool, new Set(["a"]));
  assert.deepEqual(ids(webviewRefs(pruned)), [`${W}:a`], "both plugins' instances of a closed tab go");
  assert.equal(pruneTabs(pool, new Set(["a", "b", "extra"])), pool, "nothing to prune returns the same pool");
  assert.deepEqual(pruneTabs(pool, new Set()), []);
  // The caller unmounts what pruning dropped.
  assert.deepEqual(ids(evicted(pool, webviewRefs(pruned))), [`${W}:b`, `${L}:b`]);
}

// A tab closed and reopened under the same id gets no stale entry back.
{
  let pool = touch(emptyWebviewPool, ref("a"), "k", 1);
  pool = markReady(pool, ref("a"), true);
  pool = pruneTabs(pool, new Set(["other"]));
  pool = touch(pool, ref("a"), "k", 9);
  assert.equal(findWebviewEntry(pool, ref("a")).ready, false, "a reopened tab starts not ready");
}

// prunePlugins: a disabled plugin leaves nothing behind.
{
  let pool = emptyWebviewPool;
  pool = touch(pool, ref("a", W), "k", 1);
  pool = touch(pool, ref("a", L), "k", 2);
  deepFreeze(pool);
  assert.deepEqual(ids(webviewRefs(prunePlugins(pool, new Set([L])))), [`${L}:a`]);
  assert.deepEqual(prunePlugins(pool, new Set()), []);
  assert.equal(prunePlugins(pool, new Set([W, L])), pool);
}

// webviewMountKey: unambiguous, and sensitive to every param.
{
  const base = webviewMountKey(W, { viewId: "editor", entityId: "e1", moduleState: "{}" });
  assert.equal(base, webviewMountKey(W, { viewId: "editor", entityId: "e1", moduleState: "{}" }));
  assert.notEqual(base, webviewMountKey(W, { viewId: "editor", entityId: "e2", moduleState: "{}" }));
  assert.notEqual(base, webviewMountKey(L, { viewId: "editor", entityId: "e1", moduleState: "{}" }));
  assert.notEqual(base, webviewMountKey(W, { viewId: "editor", entityId: "e1" }));
  assert.notEqual(webviewMountKey(W, { viewId: "" }), webviewMountKey(W), "undefined is not an empty string");
  assert.notEqual(
    webviewMountKey(W, { viewId: "a", mapEntityId: "b" }),
    webviewMountKey(W, { viewId: "ab" }),
    "values do not run together",
  );
}

console.log("webview pool: ok");
