import assert from "node:assert/strict";
import { rm } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { Window } from "happy-dom";
import { buildRichTextEditorHarness } from "./rich-text-editor-teardown/build-harness.mjs";

// Teardown contract for RichTextEditor:
// - destroying a focused or unfocused editor must not throw and must not call onChange;
// - a parent that needs the pending edit calls the exported flushPendingChanges() first;
// - a normal blur from a DOM event still calls onChange synchronously.
// The runes and legacy parents are bundled with the real editor and the client Svelte runtime.

const root = resolve(import.meta.dirname, "..");
const outDir = resolve(root, "node_modules/.cache/rich-text-editor-teardown");

const window = new Window({ url: "http://localhost/" });
const { document } = window;
Object.assign(globalThis, { window, document });
for (const key of Object.getOwnPropertyNames(window)) {
  if (key in globalThis) continue;
  Object.defineProperty(globalThis, key, { configurable: true, get: () => window[key] });
}
// Node's experimental web storage globals warn on access; use the window's storage instead.
for (const key of ["localStorage", "sessionStorage"]) {
  Object.defineProperty(globalThis, key, { configurable: true, value: window[key] });
}

// The reported stack shows a blur dispatched synchronously from node.remove(), which Svelte calls
// while it removes a block's DOM. happy-dom, Chromium and WebKit do not dispatch that blur for a
// plain focus-then-remove, so the test models it: the blur fires while the node is still attached.
const originalRemove = window.Element.prototype.remove;
window.Element.prototype.remove = function removeWithBlur() {
  const active = document.activeElement;
  if (active && this.contains(active)) active.dispatchEvent(new window.Event("blur"));
  return originalRemove.call(this);
};

const reportedErrors = [];
window.addEventListener("error", (event) => reportedErrors.push(event.error ?? event.message));
process.on("unhandledRejection", (reason) => reportedErrors.push(reason));

const harness = await import(pathToFileURL(await buildRichTextEditorHarness(outDir)).href);

const DEBOUNCE_WAIT_MS = 250;
const wait = (ms) => new Promise((resolveWait) => window.setTimeout(resolveWait, ms));

// onChange calls seen by the parent, and what insideReactiveUpdate() returned in each blur listener.
let records = [];
let blurContexts = [];
const hosts = [];

function mountEditor(mountParent) {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const handle = mountParent(container, (entityId, value) => records.push({ entityId, value }));
  hosts.push(handle);
  harness.flushSync();
  const pm = container.querySelector(".ProseMirror");
  assert.ok(pm?.editor, "RichTextEditor mounts a TipTap editor");
  pm.addEventListener("blur", () => blurContexts.push(harness.insideReactiveUpdate()));
  return { pm, editor: pm.editor, handle };
}

const failures = [];
async function scenario(name, run) {
  const errorsBefore = reportedErrors.length;
  records = [];
  blurContexts = [];
  const problems = [];
  try {
    await run();
  } catch (error) {
    problems.push(error.message);
  } finally {
    for (const handle of hosts.splice(0)) harness.flushSync(() => handle.setShown(false));
    await wait(DEBOUNCE_WAIT_MS);
  }
  const errors = reportedErrors.slice(errorsBefore).map((error) => String(error?.message ?? error));
  if (errors.length) problems.push(`reported errors: ${errors.join("; ")}`);
  if (problems.length) {
    failures.push(name);
    console.error(`not ok - ${name}\n  ${problems.join("\n  ")}`);
  } else {
    console.log(`ok - ${name}`);
  }
}

const contexts = {};
for (const [parentKind, mountParent] of [
  ["runes parent", harness.mountRunesParent],
  ["legacy parent", harness.mountLegacyParent],
]) {
  await scenario(`${parentKind}: destroying a focused editor does not throw or call onChange`, async () => {
    const { pm, editor, handle } = mountEditor(mountParent);
    pm.focus();
    editor.commands.insertContent("Pending text");
    harness.flushSync(() => handle.setShown(false));
    contexts.removal = blurContexts[0];
    assert.deepEqual(records, [], "onChange is not called during teardown");
    await wait(DEBOUNCE_WAIT_MS);
    assert.deepEqual(records, [], "onChange is not called after teardown");
    assert.equal(blurContexts.length, 1, "the removal dispatches one blur");
    assert.equal(contexts.removal, true, "the removal blur runs inside a reactive update");
  });

  await scenario(`${parentKind}: destroying an unfocused editor does not call onChange`, async () => {
    const { editor, handle } = mountEditor(mountParent);
    editor.commands.insertContent("Unfocused text");
    harness.flushSync(() => handle.setShown(false));
    await wait(DEBOUNCE_WAIT_MS);
    assert.deepEqual(records, []);
  });

  await scenario(`${parentKind}: a parent flush before unmount delivers the pending edit once`, async () => {
    const { editor, handle } = mountEditor(mountParent);
    editor.commands.insertContent("Flushed text");
    assert.deepEqual(records, [], "the edit is still inside the debounce window");
    handle.flushEditor();
    assert.equal(records.length, 1, "flushPendingChanges delivers synchronously");
    harness.flushSync(() => handle.setShown(false));
    await wait(DEBOUNCE_WAIT_MS);
    assert.equal(records.length, 1, "unmounting afterwards does not deliver it again");
    assert.equal(records[0].entityId, "A");
    assert.match(records[0].value, /Flushed text/);
  });

  await scenario(`${parentKind}: a normal blur delivers the pending edit synchronously once`, async () => {
    const { pm, editor, handle } = mountEditor(mountParent);
    pm.focus();
    editor.commands.insertContent("Blur text");
    pm.blur();
    contexts.plainEvent = blurContexts[0];
    assert.equal(records.length, 1, "blur delivers the edit before returning");
    assert.match(records[0].value, /Blur text/);
    assert.equal(contexts.plainEvent, false, "a plain DOM event handler is outside any reactive update");
    harness.flushSync(() => handle.setShown(false));
    await wait(DEBOUNCE_WAIT_MS);
    assert.equal(records.length, 1, "destroying afterwards does not deliver it again");
  });

  await scenario(`${parentKind}: switching the entity key with a pending edit does not write it to B`, async () => {
    const { pm, editor, handle } = mountEditor(mountParent);
    pm.focus();
    editor.commands.insertContent("A text");
    harness.flushSync(() => handle.setEntity("B"));
    await wait(DEBOUNCE_WAIT_MS);
    assert.deepEqual(records, [], "A's pending text is not delivered as B's (or anyone's) edit");
  });

  await scenario(
    `${parentKind}: flushing before an entity switch records A's text once and nothing for B`,
    async () => {
      const { pm, editor, handle } = mountEditor(mountParent);
      pm.focus();
      editor.commands.insertContent("A text");
      handle.flushEditor();
      harness.flushSync(() => handle.setEntity("B"));
      await wait(DEBOUNCE_WAIT_MS);
      assert.equal(records.length, 1, "exactly one delivery");
      assert.equal(records[0].entityId, "A");
      assert.match(records[0].value, /A text/);
    },
  );
}

console.log(
  `insideReactiveUpdate(): removal blur -> ${contexts.removal}, plain event handler blur -> ${contexts.plainEvent}`,
);

await rm(outDir, { recursive: true, force: true });
if (failures.length) {
  console.error(`${failures.length} scenario(s) failed`);
  process.exit(1);
}
