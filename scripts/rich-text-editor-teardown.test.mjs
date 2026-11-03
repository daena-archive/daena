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

// Entity search: an editor with no searchEntities prop searches through the installed host, as the Language panes do.
const hostSearches = [];
const explicitSearches = [];
const searchPage = (name, limit) => ({
  items: [{ id: "e-2", name, entityType: null, deleted: false, revision: "r2" }],
  total: 1,
  offset: 0,
  limit,
  hasMore: false,
});
const hostWithSearch = (searchEntities) => ({
  readAssetBytesByPath: async () => null,
  getAssetByPath: async () => null,
  openExternal: async () => {},
  openEntity: null,
  assets: null,
  searchEntities,
  resolveEntityNames: null,
});
const mentionMenuText = () => document.querySelector(".entity-mention-menu")?.textContent ?? "";

function mountSearchEditor(props) {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const app = harness.mountEntitySearchParent(container, props);
  hosts.push({
    setShown(next) {
      if (!next) harness.unmount(app);
    },
  });
  harness.flushSync();
  const pm = container.querySelector(".ProseMirror");
  assert.ok(pm?.editor, "RichTextEditor mounts without a host or search prop");
  // Focused, as a user typing is: an unfocused editor is re-synced from its value prop on every update.
  pm.focus();
  return { editor: pm.editor };
}

await scenario("entity search: without a searchEntities prop the installed host searches", async () => {
  hostSearches.length = 0;
  harness.setDefaultEditorHost(
    hostWithSearch(async (query) => {
      hostSearches.push(query);
      return searchPage("Kalen", query.limit);
    }),
  );
  const { editor } = mountSearchEditor({ entityId: "lang-1" });
  editor.commands.insertContent("@Ka");
  await wait(200);
  assert.equal(hostSearches.length, 1, "the host's search runs once for the mention");
  assert.equal(hostSearches[0].text, "Ka");
  assert.deepEqual(hostSearches[0].excludeIds, ["lang-1"], "the editing entity is excluded");
  assert.match(mentionMenuText(), /Kalen/, "the host's result is listed in the mention menu");
});

await scenario("entity search: an explicit searchEntities prop wins over the host's search", async () => {
  hostSearches.length = 0;
  explicitSearches.length = 0;
  harness.setDefaultEditorHost(
    hostWithSearch(async (query) => {
      hostSearches.push(query);
      return searchPage("Host", query.limit);
    }),
  );
  const { editor } = mountSearchEditor({
    explicitSearch: async (query) => {
      explicitSearches.push(query);
      return searchPage("Explicit", query.limit);
    },
  });
  editor.commands.insertContent("@Ex");
  await wait(200);
  assert.equal(explicitSearches.length, 1, "the prop's search runs");
  assert.deepEqual(hostSearches, [], "the host's search is not used while the prop is passed");
  assert.match(mentionMenuText(), /Explicit/);
});

await scenario("entity search: with no host and no prop a mention lists nothing and does not throw", async () => {
  harness.setDefaultEditorHost(null);
  const { editor } = mountSearchEditor({});
  editor.commands.insertContent("@Ka");
  await wait(200);
  assert.match(mentionMenuText(), /No matching entities/, "the empty fallback answers the search");
});

// Entity references: the popover offers Open when the host can open entities, and Cmd/Ctrl+click opens directly.
const openedEntities = [];
const referenceValue = "See [[Ardashir]](e-9) here.";
const hostWithOpen = (openEntity) => ({
  readAssetBytesByPath: async () => null,
  getAssetByPath: async () => null,
  openExternal: async () => {},
  openEntity,
  assets: null,
  searchEntities: null,
  resolveEntityNames: null,
});

function mountReferenceEditor(props = {}) {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const app = harness.mountEntitySearchParent(container, { initialValue: referenceValue, ...props });
  hosts.push({
    setShown(next) {
      if (!next) harness.unmount(app);
    },
  });
  harness.flushSync();
  const pm = container.querySelector(".ProseMirror");
  assert.ok(pm?.editor, "RichTextEditor mounts with an entity reference");
  return { container, editor: pm.editor, anchor: pm.querySelector("a[data-entity-id]") };
}

function hoverReference(anchor) {
  anchor.dispatchEvent(new window.MouseEvent("mouseover", { bubbles: true }));
  harness.flushSync();
}

await scenario("entity reference: hovering offers Open before Edit, and Open opens the referenced id", async () => {
  openedEntities.length = 0;
  harness.setDefaultEditorHost(hostWithOpen((entityId) => openedEntities.push(entityId)));
  const { container, anchor } = mountReferenceEditor();
  assert.equal(anchor?.dataset.entityId, "e-9", "the reference renders with its entity id");
  hoverReference(anchor);
  const open = container.querySelector(".entity-reference-open");
  const edit = container.querySelector(".entity-reference-edit");
  assert.equal(open?.textContent, "Open", "the popover offers Open");
  assert.equal(edit?.textContent, "Edit reference", "Edit reference stays in the popover");
  assert.equal(open.nextElementSibling, edit, "Open sits before Edit");
  open.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
  assert.deepEqual(openedEntities, ["e-9"], "the host opens the referenced entity once");
  harness.flushSync();
  assert.equal(container.querySelector(".entity-reference-edit"), null, "the popover closes after Open");
});

await scenario("entity reference: without openEntity on the host the popover keeps only Edit", async () => {
  harness.setDefaultEditorHost(hostWithOpen(null));
  const { container, anchor } = mountReferenceEditor();
  hoverReference(anchor);
  assert.equal(container.querySelector(".entity-reference-open"), null, "no Open action without openEntity");
  assert.equal(container.querySelector(".entity-reference-edit")?.textContent, "Edit reference");
});

await scenario("entity reference: Cmd or Ctrl+click opens the entity without showing the popover", async () => {
  openedEntities.length = 0;
  harness.setDefaultEditorHost(hostWithOpen((entityId) => openedEntities.push(entityId)));
  const { container, editor, anchor } = mountReferenceEditor();
  for (const modifiers of [{ metaKey: true }, { ctrlKey: true }]) {
    const click = new window.MouseEvent("click", { bubbles: true, ...modifiers });
    anchor.dispatchEvent(click);
    const handled = editor.view.someProp("handleClick", (handleClick) =>
      handleClick(editor.view, editor.view.posAtDOM(anchor, 0), click),
    );
    assert.equal(handled, true, "the editor handles the modified click");
  }
  assert.deepEqual(openedEntities, ["e-9", "e-9"], "each modified click opens the entity");
  assert.equal(container.querySelector(".entity-reference-edit"), null, "no popover after a modified click");
});

await scenario("entity reference: a plain click shows the popover and does not open the entity", async () => {
  openedEntities.length = 0;
  harness.setDefaultEditorHost(hostWithOpen((entityId) => openedEntities.push(entityId)));
  const { container, editor, anchor } = mountReferenceEditor();
  const click = new window.MouseEvent("click", { bubbles: true });
  anchor.dispatchEvent(click);
  const handled = editor.view.someProp("handleClick", (handleClick) =>
    handleClick(editor.view, editor.view.posAtDOM(anchor, 0), click),
  );
  harness.flushSync();
  assert.equal(handled, true, "the editor handles the click");
  assert.deepEqual(openedEntities, [], "a plain click does not open the entity");
  assert.equal(container.querySelector(".entity-reference-edit")?.textContent, "Edit reference");
  assert.equal(container.querySelector(".entity-reference-open")?.textContent, "Open");
});
// Unlabelled references ([[]](id)) take their name from the host when the entities prop does not list the entity.
const POPOVER_GRACE_WAIT_MS = 300; // longer than the popover's grace period, so a scheduled close has run
const resolvedBatches = [];
const unlabelledValue = "Meet [[]](e-7) here.";
const hostWithNames = (resolveEntityNames) => ({
  readAssetBytesByPath: async () => null,
  getAssetByPath: async () => null,
  openExternal: async () => {},
  openEntity: null,
  assets: null,
  searchEntities: null,
  resolveEntityNames,
});
const referenceLabel = (container, entityId) => container.querySelector(`a[data-entity-id="${entityId}"]`)?.textContent;

await scenario("editor HTML export creates no image in the live document", async () => {
  harness.setDefaultEditorHost(hostWithNames(async () => ({})));
  const { editor } = mountReferenceEditor({ initialValue: "Before ![map](assets/images/map.png) after.\n" });
  const created = [];
  const createElement = document.createElement;
  document.createElement = function (tagName, options) {
    created.push(String(tagName).toLowerCase());
    return createElement.call(this, tagName, options);
  };
  let exported;
  let viaGetHtml;
  try {
    exported = harness.editorHtml(editor);
    assert.ok(!created.includes("img"), "no <img> is created in the live document, so nothing is fetched");
    viaGetHtml = editor.getHTML();
    assert.ok(created.includes("img"), "getHTML() does create one there, which is what fetched assets/... paths");
  } finally {
    document.createElement = createElement;
  }
  assert.equal(exported, viaGetHtml, "the export matches getHTML");
  assert.ok(exported.includes('src="assets/images/map.png"'), "the asset path is kept for saving");
});

await scenario("unlabelled entity reference: with no entities prop the host supplies the name", async () => {
  resolvedBatches.length = 0;
  harness.setDefaultEditorHost(
    hostWithNames(async (ids) => {
      resolvedBatches.push(ids);
      return { "e-7": "Ardashir" };
    }),
  );
  const { container, editor } = mountReferenceEditor({ initialValue: unlabelledValue });
  assert.ok(container.querySelector('a[data-entity-id="e-7"]'), "the reference is kept while its name is pending");
  await wait(20);
  assert.equal(referenceLabel(container, "e-7"), "Ardashir", "the host's name is the label");
  assert.deepEqual(resolvedBatches, [["e-7"]], "the host is asked once for the id");
  assert.equal(
    harness.markdownFromEditorHtml(editor.getHTML()),
    "Meet [[]](e-7) here.\n",
    "the reference saves back unlabelled",
  );
  editor.commands.insertContent("x");
  await wait(20);
  assert.deepEqual(resolvedBatches, [["e-7"]], "later edits do not ask the host again");
});

await scenario("unlabelled entity reference: a name in the entities prop wins and the host is not asked", async () => {
  resolvedBatches.length = 0;
  harness.setDefaultEditorHost(
    hostWithNames(async (ids) => {
      resolvedBatches.push(ids);
      return { "e-7": "Host name" };
    }),
  );
  const { container } = mountReferenceEditor({
    initialValue: unlabelledValue,
    entities: [{ id: "e-7", name: "Listed name", entity_type: null, deleted: false, revision: "r7" }],
  });
  await wait(20);
  assert.equal(referenceLabel(container, "e-7"), "Listed name", "the entities prop names the reference");
  assert.deepEqual(resolvedBatches, [], "ids the entities prop lists are not asked of the host");
});

await scenario(
  "unlabelled entity reference: a failed host lookup keeps the reference with its id as the label",
  async () => {
    harness.setDefaultEditorHost(
      hostWithNames(async () => {
        throw new Error("broker unavailable");
      }),
    );
    const { container, editor } = mountReferenceEditor({ initialValue: unlabelledValue });
    await wait(20);
    assert.equal(referenceLabel(container, "e-7"), "e-7", "the id stands in until a name is known");
    assert.equal(
      harness.markdownFromEditorHtml(editor.getHTML()),
      "Meet [[]](e-7) here.\n",
      "the reference is not dropped from the document",
    );
  },
);

// The reference popover closes on scroll and resize, when the pointer leaves the link and the popover, and when
// focus leaves the editor and the popover. Keyboard focus inside the popover keeps it open.
const popoverOf = (container) => container.querySelector(".entity-reference-popover");
function leaveReference(anchor, relatedTarget = null) {
  anchor.dispatchEvent(new window.MouseEvent("mouseout", { bubbles: true, relatedTarget }));
  harness.flushSync();
}

await scenario("entity reference popover: a scroll inside the page closes it", async () => {
  harness.setDefaultEditorHost(hostWithOpen(() => {}));
  const { container, anchor } = mountReferenceEditor();
  hoverReference(anchor);
  assert.ok(popoverOf(container), "hovering shows the popover");
  container.querySelector(".editor-content").dispatchEvent(new window.Event("scroll"));
  harness.flushSync();
  assert.ok(!popoverOf(container), "a scroll of an ancestor closes it");
});

await scenario("entity reference popover: a window scroll or resize closes it", async () => {
  harness.setDefaultEditorHost(hostWithOpen(() => {}));
  const { container, anchor } = mountReferenceEditor();
  hoverReference(anchor);
  window.dispatchEvent(new window.Event("scroll"));
  harness.flushSync();
  assert.ok(!popoverOf(container), "a window scroll closes it");
  hoverReference(anchor);
  window.dispatchEvent(new window.Event("resize"));
  harness.flushSync();
  assert.ok(!popoverOf(container), "a window resize closes it");
});

await scenario(
  "entity reference popover: it closes after the pointer leaves the link, once the grace period ends",
  async () => {
    harness.setDefaultEditorHost(hostWithOpen(() => {}));
    const { container, anchor } = mountReferenceEditor();
    hoverReference(anchor);
    leaveReference(anchor);
    await wait(50);
    harness.flushSync();
    assert.ok(popoverOf(container), "the grace period keeps it open at first");
    await wait(POPOVER_GRACE_WAIT_MS);
    harness.flushSync();
    assert.ok(!popoverOf(container), "it closes after the grace period");
  },
);

await scenario(
  "entity reference popover: the pointer can cross into the popover and closes when it leaves",
  async () => {
    harness.setDefaultEditorHost(hostWithOpen(() => {}));
    const { container, anchor } = mountReferenceEditor();
    hoverReference(anchor);
    const popover = popoverOf(container);
    leaveReference(anchor, popover);
    await wait(50);
    popover.dispatchEvent(new window.MouseEvent("mouseenter"));
    await wait(POPOVER_GRACE_WAIT_MS);
    harness.flushSync();
    assert.ok(popoverOf(container) === popover, "it stays open while the pointer is on it");
    popover.dispatchEvent(new window.MouseEvent("mouseleave"));
    await wait(POPOVER_GRACE_WAIT_MS);
    harness.flushSync();
    assert.ok(!popoverOf(container), "it closes once the pointer leaves it");
  },
);

await scenario("entity reference popover: returning to the link inside the grace period keeps it open", async () => {
  harness.setDefaultEditorHost(hostWithOpen(() => {}));
  const { container, anchor } = mountReferenceEditor();
  hoverReference(anchor);
  leaveReference(anchor);
  hoverReference(anchor);
  await wait(POPOVER_GRACE_WAIT_MS);
  harness.flushSync();
  assert.ok(popoverOf(container), "the pending close is cancelled by the return");
});

await scenario(
  "entity reference popover: Tab from the editor into its buttons keeps it open, focus leaving closes it",
  async () => {
    harness.setDefaultEditorHost(hostWithOpen(() => {}));
    const { container, editor, anchor } = mountReferenceEditor();
    const pm = container.querySelector(".ProseMirror");
    pm.focus();
    editor.commands.setTextSelection(editor.view.posAtDOM(anchor, 0) + 1);
    harness.flushSync();
    const open = container.querySelector(".entity-reference-open");
    assert.ok(open, "a focused caret on the reference shows the popover");
    open.focus();
    harness.flushSync();
    assert.ok(popoverOf(container), "moving focus into the popover keeps it open");
    open.blur();
    harness.flushSync();
    assert.ok(!popoverOf(container), "focus leaving the popover closes it");
  },
);

await scenario(
  "entity reference popover: the editor losing focus closes it and the blur does not reopen it",
  async () => {
    harness.setDefaultEditorHost(hostWithOpen(() => {}));
    const { container, editor, anchor } = mountReferenceEditor();
    const pm = container.querySelector(".ProseMirror");
    pm.focus();
    editor.commands.setTextSelection(editor.view.posAtDOM(anchor, 0) + 1);
    harness.flushSync();
    assert.ok(popoverOf(container), "a focused caret on the reference shows the popover");
    pm.blur();
    harness.flushSync();
    assert.ok(!popoverOf(container), "the editor blurring closes the popover");
  },
);

harness.setDefaultEditorHost(null);

console.log(
  `insideReactiveUpdate(): removal blur -> ${contexts.removal}, plain event handler blur -> ${contexts.plainEvent}`,
);

await rm(outDir, { recursive: true, force: true });
if (failures.length) {
  console.error(`${failures.length} scenario(s) failed`);
  process.exit(1);
}
