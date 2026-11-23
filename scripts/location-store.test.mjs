import assert from "node:assert/strict";
import fs from "node:fs";
import module from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { compileModule } from "svelte/compiler";

// Unit tests for the location store (src/lib/location/location-store.svelte.ts): dirty marking, the save queue,
// conflict state, the stale-result guards and the stale-load token. The store is a Svelte runes module, so this
// test compiles it with `compileModule` while Node loads it: a load hook strips the TypeScript types and runs the
// Svelte compiler on every `*.svelte.ts` file, and a resolve hook maps `$lib/...` and extension-less imports.
// The store is built with a fake LocationContext, so nothing here touches Tauri.

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function probe(base) {
  for (const candidate of [base, `${base}.ts`, `${base}.js`, path.join(base, "index.ts")]) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) return candidate;
  }
  return null;
}

module.registerHooks({
  resolve(specifier, context, nextResolve) {
    let base = null;
    if (specifier.startsWith("$lib/")) base = path.join(root, "src/lib", specifier.slice("$lib/".length));
    else if ((specifier.startsWith("./") || specifier.startsWith("../")) && context.parentURL?.startsWith("file:")) {
      base = path.resolve(path.dirname(fileURLToPath(context.parentURL)), specifier);
    }
    if (base) {
      const found = probe(base);
      if (found) return { url: pathToFileURL(found).href, shortCircuit: true };
    }
    // `svelte` itself resolves to the server build under plain Node, whose `tick` and `flushSync` do nothing. The
    // store's compiled code uses the client runtime, so load the client entry (the `browser` condition) to share it.
    if (specifier === "svelte") {
      return nextResolve(specifier, { ...context, conditions: [...context.conditions, "browser"] });
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url.startsWith("file:") && url.endsWith(".svelte.ts") && !url.includes("/node_modules/")) {
      const source = fs.readFileSync(fileURLToPath(url), "utf8");
      const js = module.stripTypeScriptTypes(source, { mode: "strip" });
      const { js: compiled } = compileModule(js, { filename: fileURLToPath(url), generate: "client" });
      return { format: "module", source: compiled.code, shortCircuit: true };
    }
    return nextLoad(url, context);
  },
});

// A window whose timers the test fires by hand, so autosave and date-field timers are deterministic.
const timers = new Map();
let nextTimer = 1;
// The store reads and writes the collection query and the pane layout through `localStorage`.
const storage = new Map();
globalThis.localStorage = {
  getItem: (key) => (storage.has(key) ? storage.get(key) : null),
  setItem: (key, value) => void storage.set(key, String(value)),
  removeItem: (key) => void storage.delete(key),
};
globalThis.window = {
  innerWidth: 1400,
  setTimeout(fn, ms) {
    const id = nextTimer++;
    timers.set(id, { fn, ms });
    return id;
  },
  clearTimeout(id) {
    timers.delete(id);
  },
};
const fireTimers = () => {
  const due = [...timers.entries()];
  timers.clear();
  for (const [, timer] of due) timer.fn();
};

const { LocationStore, isRevisionConflict, emptyEntityPage, specializedSurfaceKeyForLocation } =
  await import("../src/lib/location/location-store.svelte.ts");
const { ReactiveContextState } = await import("./location-store-reactive.svelte.ts");
const { flushSync, tick: svelteTick } = await import("svelte");
const {
  collectionPaneDefault,
  collectionPaneMax,
  collectionPaneMin,
  inspectorPaneDefault,
  inspectorPaneMax,
  inspectorPaneMin,
  workbenchLayoutStorageKey,
} = await import("../src/lib/shell/workbench-persistence.ts");
/** A snapshot free of Svelte proxies, for deep comparisons. */
const plain = (value) => JSON.parse(JSON.stringify(value));

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
const tick = () => new Promise((resolve) => setImmediate(resolve));

const entity = (id, extra = {}) => ({ id, name: `Entity ${id}`, entity_type: "daena.lore:character", ...extra });
const DEFINITIONS = [
  { key: "title", label: "Title", type: "text" },
  { key: "born", label: "Born", type: "date" },
];

/**
 * A store with a fake context. `project` records every call; individual methods can be replaced per test.
 * `calls` collects the context callbacks the store made, in order.
 */
function createFixture(overrides = {}) {
  storage.clear();
  const calls = [];
  const record =
    (name) =>
    (...args) => {
      calls.push([name, ...args]);
    };
  // The state the store's effects react to is reactive, so a test can change it like the page does.
  const reactive = new ReactiveContextState();
  const state = {
    entities: [entity("a"), entity("b")],
    projectDiagnostics: [],
    get ready() {
      return reactive.ready;
    },
    set ready(value) {
      reactive.ready = value;
    },
    get projectInfo() {
      return reactive.projectInfo;
    },
    set projectInfo(value) {
      reactive.projectInfo = value;
    },
    get moduleCount() {
      return reactive.moduleCount;
    },
    set moduleCount(value) {
      reactive.moduleCount = value;
    },
    entityTypes: ["daena.lore:character"],
    adminPlugins: null,
    // What the page's helpers answer: collection tabs, enabled sections, the section that owns an entity type.
    collectionTabs: { writing: [], timeline: [] },
    enabledSections: ["lore", "houses", "timeline", "writing", "language", "maps"],
    sectionByType: {},
    loreTypes: ["daena.lore:character"],
    manifests: {},
    collectionEntities: [],
    pluginItems: {},
    storedDocuments: { a: "body a", b: "body b" },
    storedRevision: { a: "r1", b: "r1" },
  };
  const concurrency = { current: 0, max: 0, saves: [] };
  const project = {
    listDocuments: async (id) => [
      { entity_id: id, body: state.storedDocuments[id] ?? "", format: "markdown", revision: state.storedRevision[id] },
    ],
    listFields: async () => [],
    listMapLocations: async () => [],
    getEntity: async (id) => state.entities.find((candidate) => candidate.id === id) ?? null,
    saveEntry: async (payload) => {
      concurrency.saves.push(payload);
      state.storedDocuments[payload.document.entity_id] = payload.document.body;
    },
    saveRecoveryCopy: async (id, body) => `/recovery/${id}.md`,
    postPluginWebviewMessage: async () => true,
    takePluginTextTransfer: async () => "",
    aiCancelText: async () => {},
    aiGenerateText: async () => "request",
    aiPollText: async () => [],
    ...overrides.project,
  };
  const ctx = {
    project,
    get entities() {
      return state.entities;
    },
    get projectDiagnostics() {
      return state.projectDiagnostics;
    },
    calendarDefinitions: {},
    get ready() {
      return state.ready;
    },
    get projectInfo() {
      return state.projectInfo;
    },
    get adminPlugins() {
      return state.adminPlugins;
    },
    get moduleCount() {
      return state.moduleCount;
    },
    workspaceSectionOrder: ["lore", "houses", "timeline", "writing", "language", "maps"],
    sectionEntityTypes: () => state.entityTypes,
    activeTabEntityTypes: () => undefined,
    confirmDialog: async () => true,
    setError: record("setError"),
    resetSelectionShellState: record("resetSelectionShellState"),
    recordShellDeparture: record("recordShellDeparture"),
    setShellNavigationRestoring: record("setShellNavigationRestoring"),
    closeNativePluginWebviews: async () => {
      calls.push(["closeNativePluginWebviews"]);
    },
    dismissSettings: async () => {
      calls.push(["dismissSettings"]);
      store.showSettings = false;
      return true;
    },
    openSettings: async (section) => {
      calls.push(["openSettings", section]);
      store.showSettings = true;
      store.settingsSurface = "application";
      store.settingsSection = section;
    },
    openProjectCenter: async (section) => {
      calls.push(["openProjectCenter", section]);
      store.showSettings = true;
      store.settingsSurface = "project";
      store.projectSection = section;
    },
    clearGlobalQuery: record("clearGlobalQuery"),
    refreshAdmin: async () => {
      calls.push(["refreshAdmin"]);
    },
    pluginNavigationItemByKey: (key) => state.pluginItems[key] ?? null,
    appendEntityToCache: (added) => {
      calls.push(["appendEntityToCache", added.id]);
      state.entities.push(added);
    },
    manifestForWorkspaceSection: (target) => state.manifests[target] ?? null,
    collectionTabsFor: (target) => state.collectionTabs[target] ?? [],
    enabledWorkspaceSections: () => state.enabledSections,
    allEntityTypesForSection: () => new Set(state.loreTypes),
    sectionForEntityType: (entityType) => state.sectionByType[entityType] ?? null,
    collectionResult: () => ({ entities: state.collectionEntities }),
    housesModuleContext: () => ({}),
    mapsNavigationItem: () => null,
    emptyInspectorDefinitions: () => [],
    selectedRelationshipIds: () => [],
    listenAiStream: async () => () => {},
    nativeVectorSession: () => null,
    contextFor: () => ({
      module: { capabilities: [] },
      relationships: { list: async () => [] },
      assets: { list: async () => [] },
    }),
    contextOwningRelationshipType: () => {
      throw new Error("not used");
    },
    activeManifest: () => ({ schemas: [] }),
    activeModuleId: () => "daena.lore",
    definitions: () => DEFINITIONS,
    chronologyDateDefinitions: () => DEFINITIONS.filter((definition) => definition.type === "date"),
    namespaceForField: () => "daena.lore",
    sectionEnabled: () => true,
    mapsEnabled: () => false,
    upsertEntityInCache: record("upsertEntityInCache"),
    refreshAfterEntityMutation: async () => {},
    loadEraContexts: async () => [],
    notifyProfileTimelineChanged: record("notifyProfileTimelineChanged"),
    loadProjectStatus: async () => {},
    shownWebviewInstance: "shown",
    writingWebviewShown: false,
    aiProvider: { name: "" },
    aiPromptTemplates: [],
    markShownWebviewReady: record("markShownWebviewReady"),
    openExternal: record("openExternal"),
    openEntityFromWebview: record("openEntityFromWebview"),
    runShellShortcut: record("runShellShortcut"),
    openEntityEditDialog: record("openEntityEditDialog"),
    confirmWritingArchive: record("confirmWritingArchive"),
    ...overrides.ctx,
  };
  const store = new LocationStore(ctx);
  // The leave bridges used to be context callbacks. They are store methods now; record them the same way, in front
  // of the real method, so the leave-sequence checks still see the order.
  for (const name of [
    "flushWritingWebview",
    "canLeaveLanguageSection",
    "requestLanguagePane",
    "notifyWritingSaved",
    "resolveDirtyMapSession",
    "closeAiRewrite",
    "closeAiFieldFill",
  ]) {
    const original = store[name].bind(store);
    store[name] = (...args) => {
      calls.push([name, ...args]);
      return original(...args);
    };
  }
  // The section and the Writing tab belong to the store now; tests set them through the state like the page did.
  for (const name of ["section", "writingView"]) {
    Object.defineProperty(state, name, {
      get: () => store[name],
      set: (value) => {
        store[name] = value;
      },
    });
  }
  // The page is mounted before anything else runs, so the first run of the store's effects is already behind it.
  flushSync();
  return { store, ctx, state, calls, project, concurrency };
}

/** Selects "a" and waits for the load, leaving the store clean with no timers armed. */
async function openEntry(fixture, id = "a") {
  await fixture.store.selectEntity(fixture.state.entities.find((candidate) => candidate.id === id));
  assert.equal(fixture.store.selectedLoading, false);
  timers.clear();
}

// ---- Class shape: what is reactive and what is a plain field --------------------------------------------------
{
  const { store } = createFixture();
  // The same names the page had as plain `let`s must stay plain: assigning them must not need a reactive scope, and
  // the store must not wrap them. A Set stays a plain Set (a $state Set would be a proxy).
  assert.ok(store.dirtyFieldKeys instanceof Set);
  assert.equal(store.selectedLoadToken, 0);
  assert.equal(store.documentRevision, 0);
  assert.equal(store.saveInFlight, null);
  assert.equal(store.saveQueued, false);
  assert.equal(store.autoSaveTimer, null);
  assert.equal(store.documentMode, "edit");
  assert.equal(store.hasUnsavedChanges, false);
  assert.equal(typeof store.destroy, "function");
  store.destroy();
}

// ---- Dirty marking --------------------------------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store } = fixture;
  await openEntry(fixture);
  assert.equal(store.selected.id, "a");
  assert.equal(store.documentBody, "body a");
  assert.equal(store.loadedDocumentRevision, "r1");
  assert.equal(store.hasUnsavedChanges, false);

  store.savedAt = "10:00";
  store.autoSaveFailureCount = 3;
  store.markEntryDirty({ fieldKey: "title" });
  assert.equal(store.documentRevision, 1, "each edit bumps the local edit generation");
  assert.equal(store.hasUnsavedChanges, true);
  assert.equal(store.savedAt, "", "an edit clears the saved stamp");
  assert.equal(store.autoSaveFailureCount, 0, "an edit resets the retry backoff");
  assert.deepEqual([...store.dirtyFieldKeys], ["title"]);
  assert.notEqual(store.autoSaveTimer, null, "an edit arms the autosave timer");
  assert.equal(timers.size, 1);
  assert.equal([...timers.values()][0].ms, 900);

  // A document edit marks the document dirty without adding a field key, and re-arms the one timer.
  store.markEntryDirty({ document: true });
  assert.equal(store.documentRevision, 2);
  assert.deepEqual([...store.dirtyFieldKeys], ["title"]);
  assert.equal(timers.size, 1, "re-arming replaces the timer, there is only ever one");

  store.clearDirtyTracking();
  assert.equal(store.dirtyFieldKeys.size, 0);

  // updateDocumentBody ignores edits while a load is running or the project has diagnostics.
  store.cancelAutoSave();
  store.hasUnsavedChanges = false;
  store.documentRevision = 0;
  store.selectedLoading = true;
  store.updateDocumentBody("typed during load");
  assert.equal(store.documentBody, "body a");
  assert.equal(store.hasUnsavedChanges, false);
  store.selectedLoading = false;
  fixture.state.projectDiagnostics = ["portable files changed"];
  store.updateDocumentBody("typed with diagnostics");
  assert.equal(store.documentBody, "body a");
  fixture.state.projectDiagnostics = [];
  store.updateDocumentBody("typed");
  assert.equal(store.documentBody, "typed");
  assert.equal(store.hasUnsavedChanges, true);
  assert.equal(store.documentRevision, 1);

  // Autosave is not armed without a selection, with a conflict, or with diagnostics.
  store.cancelAutoSave();
  store.documentConflict = { paths: [], diagnostics: [] };
  store.scheduleAutoSave();
  assert.equal(store.autoSaveTimer, null, "no autosave while a conflict is open");
  store.documentConflict = null;
  fixture.state.projectDiagnostics = ["x"];
  store.scheduleAutoSave();
  assert.equal(store.autoSaveTimer, null, "no autosave while the project has diagnostics");
  fixture.state.projectDiagnostics = [];
  store.cancelAutoSave();
  store.destroy();
}

// ---- Date field timers: debounced dirty marking, flushed on demand --------------------------------------------
{
  const fixture = createFixture();
  const { store } = fixture;
  await openEntry(fixture);
  store.updateInspectorDateField("born", "1200-01-01");
  assert.equal(store.fields.born, "1200-01-01");
  assert.equal(store.hasUnsavedChanges, false, "the dirty mark is debounced");
  assert.equal(Object.keys(store.dateFieldDirtyTimers).length, 1);
  store.flushDateFieldDirty();
  assert.equal(store.hasUnsavedChanges, true, "flushing applies the pending date edit now");
  assert.deepEqual([...store.dirtyFieldKeys], ["born"]);
  assert.equal(Object.keys(store.dateFieldDirtyTimers).length, 0);
  store.cancelAutoSave();
  store.destroy();
}

// ---- selectEntity: same-id early return -----------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store, calls } = fixture;
  await openEntry(fixture);
  const listCalls = [];
  const original = fixture.project.listDocuments;
  fixture.project.listDocuments = async (id) => {
    listCalls.push(id);
    return original(id);
  };
  calls.length = 0;
  store.markEntryDirty({ document: true });
  store.cancelAutoSave();
  await store.selectEntity(fixture.state.entities[0]);
  assert.deepEqual(calls, [], "selecting the open entry again touches nothing, not even the flush");
  assert.equal(store.hasUnsavedChanges, true, "and does not drop the pending edit");
  assert.deepEqual(listCalls, []);
  assert.equal(fixture.concurrency.saves.length, 0);
  store.clearSelection();
  store.destroy();
}

// ---- selectEntity: flushes and saves first, then switches -----------------------------------------------------
{
  const fixture = createFixture();
  const { store, calls } = fixture;
  await openEntry(fixture);
  store.updateDocumentBody("edited a");
  store.cancelAutoSave();
  calls.length = 0;
  await store.selectEntity(fixture.state.entities[1]);
  assert.equal(fixture.concurrency.saves.length, 1, "the open entry is saved before the switch");
  assert.equal(fixture.concurrency.saves[0].document.entity_id, "a");
  assert.equal(fixture.concurrency.saves[0].document.body, "edited a");
  assert.equal(store.selected.id, "b");
  assert.equal(store.documentBody, "body b");
  assert.equal(store.hasUnsavedChanges, false);
  assert.equal(store.documentRevision, 0, "a new selection restarts the edit generation");
  assert.deepEqual(
    calls
      .map(([name]) => name)
      .filter((name) => ["flushWritingWebview", "recordShellDeparture", "closeAiRewrite"].includes(name)),
    ["flushWritingWebview", "recordShellDeparture", "closeAiRewrite"],
    "the Writing flush comes first, then history, then the AI dialogs close during the load",
  );
  assert.deepEqual(
    calls.find(([name]) => name === "setError"),
    ["setError", ""],
    "selecting clears the shell error",
  );
  store.destroy();
}

// ---- selectEntity: a failed save blocks the switch ------------------------------------------------------------
{
  const fixture = createFixture({
    project: {
      saveEntry: async () => {
        throw new Error("disk full");
      },
    },
  });
  const { store } = fixture;
  await openEntry(fixture);
  store.updateDocumentBody("edited a");
  store.cancelAutoSave();
  await store.selectEntity(fixture.state.entities[1]);
  assert.equal(store.selected.id, "a", "the selection stays when the save fails");
  assert.equal(store.saveError, "disk full");
  assert.equal(store.hasUnsavedChanges, true);
  assert.equal(store.autoSaveFailureCount, 1);
  assert.equal([...timers.values()][0].ms, 2000, "a failed save retries after the first backoff step");
  timers.clear();
  store.destroy();
}

// ---- Stale loads: selectedLoadToken ---------------------------------------------------------------------------
{
  const slowA = deferred();
  const fixture = createFixture();
  const { store, project } = fixture;
  const original = project.listDocuments;
  project.listDocuments = (id) => (id === "a" ? slowA.promise : original(id));
  const loadA = store.selectEntity(fixture.state.entities[0]);
  await tick();
  assert.equal(store.selected.id, "a");
  assert.equal(store.selectedLoading, true);
  const tokenA = store.selectedLoadToken;
  // Selecting B runs flushAutoSave first (nothing dirty), then B's load, which supersedes A's.
  await store.selectEntity(fixture.state.entities[1]);
  assert.equal(store.selected.id, "b");
  assert.ok(store.selectedLoadToken > tokenA);
  assert.equal(store.documentBody, "body b");
  assert.equal(store.selectedLoading, false);
  slowA.resolve([{ entity_id: "a", body: "STALE BODY A", format: "markdown", revision: "rA" }]);
  await loadA;
  assert.equal(store.documentBody, "body b", "a stale load does not overwrite the current document");
  assert.equal(store.loadedDocumentRevision, "r1", "or the revision");
  assert.equal(store.selectedLoading, false, "or the loading flag");
  store.destroy();
}

// ---- Stale load failure: only the current load reports its error ----------------------------------------------
{
  const failingA = deferred();
  const fixture = createFixture();
  const { store, project, calls } = fixture;
  const original = project.listDocuments;
  project.listDocuments = (id) => (id === "a" ? failingA.promise : original(id));
  const loadA = store.selectEntity(fixture.state.entities[0]);
  await tick();
  await store.selectEntity(fixture.state.entities[1]);
  failingA.reject(new Error("old load failed"));
  await loadA;
  assert.equal(store.selectedLoadError, "", "a stale failure does not mark the new entry as failed");
  assert.equal(store.selected.id, "b");
  // selectEntity reports a failure of the current load through the shell error line.
  const failing = createFixture({
    project: {
      listDocuments: async () => {
        throw new Error("load failed");
      },
    },
  });
  await failing.store.selectEntity(failing.state.entities[0]);
  assert.equal(failing.store.selectedLoadError, "load failed");
  assert.deepEqual(failing.calls.filter(([name]) => name === "setError").at(-1), ["setError", "load failed"]);
  assert.equal(failing.store.selectedLoading, false);
  void calls;
  store.destroy();
  failing.store.destroy();
}

// ---- Save queue: one in flight, one queued --------------------------------------------------------------------
{
  const gates = [deferred(), deferred()];
  let call = 0;
  const fixture = createFixture();
  const { store, project, concurrency } = fixture;
  const record = project.saveEntry;
  project.saveEntry = async (payload, options) => {
    const gate = gates[call++];
    concurrency.current += 1;
    concurrency.max = Math.max(concurrency.max, concurrency.current);
    try {
      await gate.promise;
      await record(payload, options);
    } finally {
      concurrency.current -= 1;
    }
  };
  await openEntry(fixture);

  store.updateDocumentBody("first");
  store.cancelAutoSave();
  const first = store.saveDocument();
  await tick();
  assert.equal(store.isSaving, true);
  assert.notEqual(store.saveInFlight, null, "the first save is in flight");
  assert.equal(call, 1);

  // An edit lands while the save runs, and another save is requested: it queues behind the first.
  store.updateDocumentBody("second");
  store.cancelAutoSave();
  const second = store.saveDocument();
  await tick();
  assert.equal(store.saveQueued, true, "the second request is queued, not started");
  assert.equal(call, 1, "only one saveEntry call exists while the first is in flight");
  // A third request while one is queued still starts nothing new.
  const third = store.saveDocument();
  await tick();
  assert.equal(call, 1);

  gates[0].resolve();
  // The first call does not return until the queued save has run: saveDocument chains it itself.
  await tick();
  await tick();
  assert.equal(call, 2, "the queued save runs after the first completes");
  assert.equal(concurrency.max, 1, "two saves never overlap");
  assert.equal(store.hasUnsavedChanges, true, "the first save does not clear a newer edit");
  gates[1].resolve();
  assert.equal(await first, true);
  assert.equal(await second, true);
  assert.equal(await third, true);
  assert.equal(call, 2, "the third request found nothing left to save");
  assert.equal(store.hasUnsavedChanges, false);
  assert.equal(store.saveInFlight, null);
  assert.equal(store.saveQueued, false);
  assert.equal(store.isSaving, false);
  assert.deepEqual(
    concurrency.saves.map((payload) => payload.document.body),
    ["first", "second"],
    "each save sends the document as it was when the save started",
  );
  assert.equal(
    fixture.calls.filter(([name]) => name === "notifyWritingSaved").length,
    1,
    "only the clean save is reported to Writing",
  );
  store.destroy();
}

// ---- saveDocument with nothing to save, and flushAutoSave ordering ---------------------------------------------
{
  const fixture = createFixture();
  const { store, calls, concurrency } = fixture;
  await openEntry(fixture);
  calls.length = 0;
  assert.equal(await store.saveDocument(), true);
  assert.equal(concurrency.saves.length, 0, "a clean document is not saved");
  assert.equal(await store.flushAutoSave(), true);
  assert.deepEqual(calls, [["flushWritingWebview"]], "flushAutoSave asks the Writing webview first");

  // The editor is flushed (it may mark the document dirty), then pending date edits, then the timer is cancelled.
  let editorFlushed = 0;
  store.editorRef = {
    insertAiTextAtRequest: () => null,
    replaceAiTextWithMarkdown: () => null,
    flushPendingChanges: () => {
      editorFlushed += 1;
      store.updateDocumentBody("flushed from the editor");
    },
  };
  assert.equal(await store.flushAutoSave(), true);
  assert.equal(editorFlushed, 1);
  assert.equal(concurrency.saves.length, 1, "text the editor held back is saved by the flush");
  assert.equal(concurrency.saves[0].document.body, "flushed from the editor");
  assert.equal(store.autoSaveTimer, null, "the flush leaves no autosave timer behind");
  assert.equal(store.hasUnsavedChanges, false);
  store.editorRef = null;
  store.destroy();
}

// ---- Stale-result guards in persistDocumentSnapshot -------------------------------------------------------------
{
  // (1) The selection changes while a save runs: nothing of the old entry's result leaks into the new state.
  const gate = deferred();
  const fixture = createFixture();
  const { store, project, calls } = fixture;
  const record = project.saveEntry;
  project.saveEntry = async (payload, options) => {
    await gate.promise;
    await record(payload, options);
  };
  await openEntry(fixture);
  store.updateDocumentBody("edit of a");
  store.cancelAutoSave();
  const saving = store.saveDocument();
  await tick();
  store.clearSelection();
  store.selected = fixture.state.entities[1];
  store.loadedDocumentRevision = "rB";
  calls.length = 0;
  gate.resolve();
  assert.equal(await saving, true, "the save itself still succeeded");
  assert.equal(store.loadedDocumentRevision, "rB", "the revision of the other entry is not overwritten");
  assert.equal(store.savedAt, "", "no saved stamp for an entry that is no longer open");
  assert.equal(
    calls.filter(([name]) => name === "notifyWritingSaved").length,
    0,
    "Writing is not told the new selection was saved",
  );
  store.destroy();
}
{
  // (2) A newer local edit lands during the save: the document stays dirty and the saved stamp is not set.
  const gate = deferred();
  const fixture = createFixture();
  const { store, project, calls } = fixture;
  const record = project.saveEntry;
  project.saveEntry = async (payload, options) => {
    await gate.promise;
    await record(payload, options);
  };
  await openEntry(fixture);
  store.updateDocumentBody("v1");
  store.cancelAutoSave();
  const saving = store.saveDocument();
  await tick();
  const generationAtSave = store.documentRevision;
  store.updateDocumentBody("v2");
  store.cancelAutoSave();
  assert.ok(store.documentRevision > generationAtSave);
  calls.length = 0;
  gate.resolve();
  // The queued path is not taken (no second saveDocument call yet), so the first result settles alone.
  assert.equal(await saving, true);
  assert.equal(store.hasUnsavedChanges, true, "a newer edit keeps the document dirty");
  assert.equal(store.savedAt, "");
  assert.equal(calls.filter(([name]) => name === "notifyWritingSaved").length, 0);
  assert.equal(store.dirtyFieldKeys.size, 0, "the dirty keys are only cleared by a save that covers every edit");
  store.destroy();
}

// ---- Conflicts --------------------------------------------------------------------------------------------------
{
  assert.equal(isRevisionConflict(new Error("Revision conflict on document")), true);
  assert.equal(isRevisionConflict(new Error("disk full")), false);

  let conflict = true;
  const fixture = createFixture();
  const { store, project, state, concurrency } = fixture;
  const record = project.saveEntry;
  project.saveEntry = async (payload, options) => {
    if (conflict) throw new Error("revision conflict: the stored revision changed");
    await record(payload, options);
  };
  await openEntry(fixture);
  state.storedDocuments.a = "body changed on disk";
  state.storedRevision.a = "r2";

  store.updateDocumentBody("my draft");
  store.cancelAutoSave();
  assert.equal(await store.saveDocument(), false);
  assert.deepEqual(store.documentConflict, { paths: [], diagnostics: [] });
  assert.equal(store.conflictDiskBody, "body changed on disk", "the disk body is kept for the conflict view");
  assert.equal(store.saveError, "", "a conflict is not a save error");
  assert.equal(store.savedAt, "");
  assert.equal(store.hasUnsavedChanges, true, "the draft is still unsaved");
  assert.equal(store.autoSaveTimer, null, "no retry is scheduled for a conflict");
  assert.equal(store.autoSaveFailureCount, 0);

  // While the conflict is open, saving and autosave do nothing and the editor is not saved over the disk.
  assert.equal(await store.saveDocument(), false);
  store.markEntryDirty({ document: true });
  assert.equal(store.autoSaveTimer, null);

  // Overwrite: take the disk revision, clear the conflict, save the draft on top.
  conflict = false;
  await store.overwriteConflict();
  assert.equal(store.documentConflict, null);
  assert.equal(store.conflictDiskBody, "");
  assert.equal(store.loadedDocumentRevision, "r2", "the draft is saved against the revision now on disk");
  assert.equal(concurrency.saves.at(-1).document.body, "my draft");
  assert.equal(store.hasUnsavedChanges, false);
  store.destroy();
}
{
  // Reload discards the draft and takes the disk version.
  let conflict = true;
  const fixture = createFixture();
  const { store, project, state } = fixture;
  const record = project.saveEntry;
  project.saveEntry = async (payload, options) => {
    if (conflict) throw new Error("Revision conflict");
    await record(payload, options);
  };
  await openEntry(fixture);
  state.storedDocuments.a = "disk version";
  state.storedRevision.a = "r9";
  store.updateDocumentBody("draft");
  store.cancelAutoSave();
  await store.saveDocument();
  assert.notEqual(store.documentConflict, null);
  await store.reloadConflict();
  assert.equal(store.documentConflict, null);
  assert.equal(store.documentBody, "disk version");
  assert.equal(store.loadedDocumentRevision, "r9");
  assert.equal(store.hasUnsavedChanges, false);
  assert.equal(store.documentRevision, 0);
  assert.equal(store.dirtyFieldKeys.size, 0);
  assert.equal(store.conflictDiskBody, "");
  conflict = false;
  store.destroy();
}
{
  // An overwrite that cannot be written back leaves a visible conflict rather than a silent failure.
  const fixture = createFixture({
    project: {
      saveEntry: async () => {
        throw new Error("revision conflict");
      },
    },
  });
  const { store } = fixture;
  await openEntry(fixture);
  store.updateDocumentBody("draft");
  store.cancelAutoSave();
  await store.saveDocument();
  assert.notEqual(store.documentConflict, null);
  await store.overwriteConflict();
  assert.notEqual(store.documentConflict, null, "the second conflict is shown again");
  store.destroy();
}
{
  // The recovery copy saves the draft and reports where through the shell error line.
  const fixture = createFixture();
  await openEntry(fixture);
  fixture.store.documentBody = "my draft";
  await fixture.store.saveConflictRecoveryCopy();
  assert.deepEqual(fixture.calls.filter(([name]) => name === "setError").at(-1), [
    "setError",
    "Draft saved as /recovery/a.md",
  ]);
  fixture.store.destroy();
}

// ---- clearSelection ---------------------------------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store, calls } = fixture;
  await openEntry(fixture);
  store.updateDocumentBody("draft");
  store.documentConflict = { paths: [], diagnostics: [] };
  store.editorFullscreen = true;
  calls.length = 0;
  store.clearSelection();
  assert.equal(store.selected, null);
  assert.equal(store.documentBody, "");
  assert.equal(store.hasUnsavedChanges, false);
  assert.equal(store.documentConflict, null);
  assert.equal(store.editorFullscreen, false);
  assert.equal(store.autoSaveTimer, null, "clearing cancels the autosave timer");
  assert.deepEqual(calls, [["resetSelectionShellState"]], "shell-global state follows through the context");
  store.destroy();
}

// ---- Methods the page passes as bare callbacks keep `this` --------------------------------------------------------
{
  const fixture = createFixture();
  await openEntry(fixture);
  // The page binds these in templates (`onChange={loc.updateDocumentBody}`, `beforeWrite={loc.flushAutoSave}`, ...).
  const { updateDocumentBody, flushAutoSave, setEditorFullscreen, reloadConflict, overwriteConflict } = fixture.store;
  const { saveConflictRecoveryCopy, attachAsset, handleAssetSave, handleAssetDelete, handleAssetReplace } =
    fixture.store;
  updateDocumentBody("typed through a bare callback");
  assert.equal(fixture.store.documentBody, "typed through a bare callback");
  setEditorFullscreen(true);
  assert.equal(fixture.store.editorFullscreen, true);
  assert.equal(await flushAutoSave(), true);
  assert.equal(fixture.concurrency.saves.length, 1);
  for (const callback of [reloadConflict, overwriteConflict, saveConflictRecoveryCopy, attachAsset, handleAssetSave]) {
    assert.equal(typeof callback, "function");
  }
  // With no open dialog the asset handlers return before they touch the project.
  await handleAssetSave({});
  await handleAssetDelete();
  await handleAssetReplace();
  fixture.store.destroy();
}

// ---- Collection location: capture and apply ---------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store } = fixture;
  store.collectionQuery = {
    section: "lore",
    textSearch: "ann",
    sortField: "created_at",
    sortDir: "desc",
    pageSize: 25,
    page: 2,
    excludedTypes: ["z", "a"],
    viewMode: "flat",
  };
  store.expandedGroups = new Set(["g2", "g1"]);
  store.collectionScrollBySection.lore = 120;
  const captured = store.currentWorkspaceCollectionLocation();
  assert.deepEqual(plain(captured), {
    query: {
      textSearch: "ann",
      sortField: "created_at",
      sortDir: "desc",
      pageSize: 25,
      page: 2,
      excludedTypes: ["a", "z"],
      viewMode: "flat",
    },
    expandedGroups: ["g1", "g2"],
    scrollTop: 120,
  });
  assert.deepEqual(store.collectionQuery.excludedTypes, ["z", "a"], "capturing sorts a copy, not the live list");

  // Scroll precedence: a queued restore for this section, then the live list, then the remembered number.
  store.collectionListElement = { scrollTop: 45 };
  assert.equal(store.currentWorkspaceCollectionLocation().scrollTop, 45);
  store.queueCollectionScroll("lore", 300);
  assert.equal(store.currentWorkspaceCollectionLocation().scrollTop, 300);
  store.pendingCollectionScroll = { section: "timeline", scrollTop: 999 };
  assert.equal(store.currentWorkspaceCollectionLocation().scrollTop, 45, "another section's queued restore is ignored");

  // Applying it to a fresh store restores the query, the open groups and queues the scroll.
  const other = createFixture();
  const applying = other.store.applyWorkspaceCollectionLocation(captured);
  assert.equal(other.store.collectionQueryRestoring, true, "the restoring flag is up while the query is applied");
  await applying;
  assert.equal(other.store.collectionQueryRestoring, false);
  assert.deepEqual(plain(other.store.collectionQuery), { section: "lore", ...captured.query });
  assert.deepEqual([...other.store.expandedGroups], ["g1", "g2"]);
  assert.deepEqual(plain(other.store.pendingCollectionScroll), { section: "lore", scrollTop: 120 });
  assert.equal(other.store.collectionScrollBySection.lore, 120);
  assert.deepEqual(
    plain(other.store.currentWorkspaceCollectionLocation()),
    plain(captured),
    "capture then apply round-trips",
  );
  assert.notEqual(
    other.store.collectionQuery.excludedTypes,
    captured.query.excludedTypes,
    "the applied list is a copy",
  );
  store.destroy();
  other.store.destroy();
}

// ---- Collection scroll: remember and queue ------------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store } = fixture;
  const { rememberCollectionScroll } = store; // The page hands this to CollectionPane as a bare callback.
  rememberCollectionScroll();
  assert.deepEqual(plain(store.collectionScrollBySection), {}, "no list element, nothing to remember");
  store.collectionListElement = { scrollTop: 77 };
  rememberCollectionScroll();
  assert.equal(store.collectionScrollBySection.lore, 77);
  store.queueCollectionScroll("timeline", 12);
  assert.equal(store.collectionScrollBySection.timeline, 12, "queueing also remembers the number for that section");
  assert.deepEqual(plain(store.pendingCollectionScroll), { section: "timeline", scrollTop: 12 });
  assert.equal(store.collectionScrollBySection.lore, 77, "other sections are untouched");

  // A refresh keeps the user's place: it queues the live scroll position of this section.
  store.collectionListElement = { scrollTop: 33 };
  const epoch = store.collectionRefreshEpoch;
  store.bumpCollectionRefresh();
  assert.equal(store.collectionRefreshEpoch, epoch + 1);
  assert.deepEqual(plain(store.pendingCollectionScroll), { section: "lore", scrollTop: 33 });
  store.collectionListElement = null;
  store.bumpCollectionRefresh();
  assert.equal(
    store.collectionRefreshEpoch,
    epoch + 2,
    "without a list there is nothing to queue, the epoch still bumps",
  );
  store.destroy();
}

// ---- Specialized surface scroll -----------------------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store, state } = fixture;
  const { rememberSpecializedSurfaceScroll } = store; // Bare callback on SpecializedSurface.
  assert.equal(store.currentSpecializedSurfaceScrollTop(), 0, "no surface open, no scroll");
  rememberSpecializedSurfaceScroll(5);
  assert.deepEqual(plain(store.specializedSurfaceScrollByKey), {});

  store.hostView = { plugin: { id: "p" }, view: { id: "v" } };
  assert.equal(store.specializedSurfaceKey(), "plugin:p:v");
  rememberSpecializedSurfaceScroll(40);
  assert.equal(store.specializedSurfaceScrollByKey["plugin:p:v"], 40);
  assert.equal(store.currentSpecializedSurfaceScrollTop(), 40);
  store.specializedSurfaceElement = { scrollTop: 12 };
  assert.equal(store.currentSpecializedSurfaceScrollTop(), 12, "the live element beats the remembered number");
  store.pendingSpecializedSurfaceScroll = { key: "plugin:p:v", scrollTop: 90 };
  assert.equal(store.currentSpecializedSurfaceScrollTop(), 90, "a queued restore beats the live element");
  rememberSpecializedSurfaceScroll(41);
  assert.equal(store.pendingSpecializedSurfaceScroll, null, "the user scrolling cancels the queued restore");
  store.pendingSpecializedSurfaceScroll = { key: "other", scrollTop: 1 };
  rememberSpecializedSurfaceScroll(42);
  assert.notEqual(store.pendingSpecializedSurfaceScroll, null, "a queued restore for another surface is kept");
  store.pendingSpecializedSurfaceScroll = null;

  // Restoring a location queues the number, then scrolls the element once the surface is on screen.
  window.requestAnimationFrame = (callback) => callback();
  // A plain object stands in for the DOM node; $state wraps it in a proxy, so read it back through the store.
  store.specializedSurfaceElement = { scrollTop: 0 };
  store.hostView = null;
  store.section = "houses";
  store.housesView = "tree";
  assert.equal(store.specializedSurfaceKey(), "workspace:houses:tree");
  await store.restoreSpecializedSurfaceScroll({
    kind: "workspace",
    section: "houses",
    view: "tree",
    surfaceScrollTop: 60,
  });
  assert.equal(store.specializedSurfaceElement.scrollTop, 60);
  assert.equal(store.specializedSurfaceScrollByKey["workspace:houses:tree"], 60);
  assert.equal(store.pendingSpecializedSurfaceScroll, null);

  // The surface changed before the frame: the element is left alone and the restore stays queued.
  store.specializedSurfaceElement.scrollTop = 0;
  store.section = "lore";
  store.loreWikiOpen = true;
  assert.equal(store.specializedSurfaceKey(), "workspace:lore:wiki");
  await store.restoreSpecializedSurfaceScroll({
    kind: "workspace",
    section: "houses",
    view: "tree",
    surfaceScrollTop: 70,
  });
  assert.equal(store.specializedSurfaceElement.scrollTop, 0);
  assert.deepEqual(plain(store.pendingSpecializedSurfaceScroll), { key: "workspace:houses:tree", scrollTop: 70 });
  store.pendingSpecializedSurfaceScroll = null;

  // Locations without a full surface restore nothing.
  for (const location of [
    { kind: "home" },
    { kind: "settings", section: "general" },
    { kind: "project", section: "overview" },
    { kind: "workspace", section: "lore", view: "default", surfaceScrollTop: 5 },
  ]) {
    await store.restoreSpecializedSurfaceScroll(location);
    assert.equal(store.pendingSpecializedSurfaceScroll, null);
  }
  assert.equal(specializedSurfaceKeyForLocation({ kind: "plugin", key: "p:v", surfaceScrollTop: 0 }), "plugin:p:v");
  assert.equal(
    specializedSurfaceKeyForLocation({ kind: "workspace", section: "lore", view: "wiki", surfaceScrollTop: 0 }),
    "workspace:lore:wiki",
  );
  delete window.requestAnimationFrame;
  store.destroy();
}

// ---- Panes: resize, reset, toggle, dimensions ---------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store, ctx, calls } = fixture;
  const stored = () => JSON.parse(storage.get(workbenchLayoutStorageKey("lore")));
  assert.deepEqual(plain(store.workbenchPaneVisibility), { collection: true, content: true, inspector: true });
  assert.deepEqual(plain(store.workbenchPaneWidths), {
    collection: collectionPaneDefault,
    inspector: inspectorPaneDefault,
  });

  store.resizeWorkbenchPane("collection", 30);
  assert.equal(store.workbenchPaneWidths.collection, collectionPaneDefault + 30);
  assert.equal(store.workbenchPaneWidths.inspector, inspectorPaneDefault);
  assert.equal(store.restoredWorkspacePaneDimensions.collectionWidth, collectionPaneDefault + 30);
  assert.equal(store.restoredWorkspacePaneDimensions.contentWidth, 640, "the content width falls back to 640");
  assert.equal(store.restoredWorkspacePaneDimensions.viewportWidth, 1400);
  assert.deepEqual(stored(), {
    visibility: { collection: true, content: true, inspector: true },
    collectionWidth: collectionPaneDefault + 30,
    inspectorWidth: inspectorPaneDefault,
  });
  store.resizeWorkbenchPane("collection", 10_000);
  assert.equal(store.workbenchPaneWidths.collection, collectionPaneMax, "widths clamp to the maximum");
  store.resizeWorkbenchPane("inspector", -10_000);
  assert.equal(store.workbenchPaneWidths.inspector, inspectorPaneMin, "and to the minimum");
  store.resizeWorkbenchPane("collection", -10_000);
  assert.equal(store.workbenchPaneWidths.collection, collectionPaneMin);
  assert.equal(stored().inspectorWidth, inspectorPaneMin);

  store.resetWorkbenchPane("collection");
  assert.equal(store.workbenchPaneWidths.collection, collectionPaneDefault);
  assert.equal(store.workbenchPaneWidths.inspector, inspectorPaneMin, "resetting one pane keeps the other");
  store.resetWorkbenchPane("inspector");
  assert.equal(store.workbenchPaneWidths.inspector, inspectorPaneDefault);
  assert.deepEqual(plain(store.activePaneDimensions()), {
    collectionWidth: collectionPaneDefault,
    contentWidth: 640,
    inspectorWidth: inspectorPaneDefault,
    viewportWidth: 1400,
  });
  assert.equal(stored().collectionWidth, collectionPaneDefault);
  assert.equal(stored().inspectorWidth, inspectorPaneDefault);

  // Toggling a side pane just flips and stores it.
  await store.toggleWorkbenchPane("collection");
  assert.equal(store.workbenchPaneVisibility.collection, false);
  assert.equal(stored().visibility.collection, false);
  assert.equal(calls.filter(([name]) => name === "flushWritingWebview").length, 0, "side panes need no leave guards");
  await store.toggleWorkbenchPane("collection");
  assert.equal(store.workbenchPaneVisibility.collection, true);

  // Hiding the content column saves first and asks the Language pane, which can refuse.
  store.editorFullscreen = true;
  store.canLeaveLanguageSection = async () => false;
  await store.toggleWorkbenchPane("content");
  assert.equal(store.workbenchPaneVisibility.content, true, "a refusing Language pane keeps the column");
  assert.equal(store.editorFullscreen, true);
  assert.equal(calls.filter(([name]) => name === "flushWritingWebview").length, 1);
  store.canLeaveLanguageSection = async () => true;
  await store.toggleWorkbenchPane("content");
  assert.equal(store.workbenchPaneVisibility.content, false);
  assert.equal(store.editorFullscreen, false, "hiding the content column leaves editor full screen");
  assert.equal(stored().visibility.content, false);
  // A failed save keeps the column too.
  await openEntry(fixture);
  await store.toggleWorkbenchPane("content");
  assert.equal(store.workbenchPaneVisibility.content, true, "showing it again needs no guard");
  store.updateDocumentBody("unsaved");
  fixture.project.saveEntry = async () => {
    throw new Error("disk full");
  };
  await store.toggleWorkbenchPane("content");
  assert.equal(store.workbenchPaneVisibility.content, true, "unsaved text that cannot be saved keeps the column");
  store.cancelAutoSave();

  // Dimensions for a stored location: kept only if the window is the same width.
  await store.restoreWorkspacePaneDimensions({
    collectionWidth: 260,
    contentWidth: 700,
    inspectorWidth: 300,
    viewportWidth: 1400,
  });
  assert.equal(store.restoredWorkspacePaneDimensions.contentWidth, 700);
  await store.restoreWorkspacePaneDimensions({
    collectionWidth: 260,
    contentWidth: 700,
    inspectorWidth: 300,
    viewportWidth: 1402,
  });
  assert.notEqual(
    store.restoredWorkspacePaneDimensions,
    null,
    "two pixels of difference still count as the same window",
  );
  await store.restoreWorkspacePaneDimensions({
    collectionWidth: 260,
    contentWidth: 700,
    inspectorWidth: 300,
    viewportWidth: 1000,
  });
  assert.equal(store.restoredWorkspacePaneDimensions, null, "another window width drops them");
  await store.restoreWorkspacePaneDimensions({
    collectionWidth: 260,
    contentWidth: 700,
    inspectorWidth: 300,
    viewportWidth: 0,
  });
  assert.equal(store.restoredWorkspacePaneDimensions, null);

  store.restoredWorkspacePaneDimensions = {
    collectionWidth: 111,
    contentWidth: 222,
    inspectorWidth: 333,
    viewportWidth: 1400,
  };
  store.collectionPaneElement = { getBoundingClientRect: () => ({ width: 301.4 }) };
  store.contentPaneElement = { getBoundingClientRect: () => ({ width: 0 }) };
  assert.deepEqual(plain(store.currentWorkspacePaneDimensions()), {
    collectionWidth: 301,
    contentWidth: 222,
    inspectorWidth: 333,
    viewportWidth: 1400,
  });
  store.destroy();
}

// ---- Effects: the section-keyed reloads -----------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store, state } = fixture;
  assert.equal(store.collectionQuery.section, "lore");
  assert.equal(
    JSON.parse(storage.get("daena:collection-query:lore")).section,
    "lore",
    "the first run persists the query",
  );

  storage.set(
    "daena:collection-query:timeline",
    JSON.stringify({ textSearch: "stored", pageSize: 100, excludedTypes: ["x"], sortField: "updated_at" }),
  );
  storage.set(
    workbenchLayoutStorageKey("timeline"),
    JSON.stringify({
      visibility: { collection: false, content: true, inspector: true },
      collectionWidth: 300,
      inspectorWidth: 350,
    }),
  );
  store.expandedGroups = new Set(["open"]);
  store.restoredWorkspacePaneDimensions = {
    collectionWidth: 1,
    contentWidth: 2,
    inspectorWidth: 3,
    viewportWidth: 1400,
  };
  store.collectionQuery.page = 4;
  state.section = "timeline";
  flushSync();
  assert.equal(store.collectionQuery.section, "timeline");
  assert.equal(store.collectionQuery.textSearch, "stored", "the section's stored query is loaded");
  assert.equal(store.collectionQuery.pageSize, 100);
  assert.equal(store.collectionQuery.page, 0);
  assert.deepEqual(store.collectionQuery.excludedTypes, ["x"]);
  assert.equal(store.expandedGroups.size, 0, "open groups are cleared");
  assert.equal(store.collectionQueryRestoring, true, "the restoring flag stays up until the next tick");
  assert.equal(store.workbenchPaneVisibility.collection, false, "the section's pane layout is loaded");
  assert.deepEqual(plain(store.workbenchPaneWidths), { collection: 300, inspector: 350 });
  assert.equal(store.restoredWorkspacePaneDimensions, null, "restored pane dimensions are dropped");
  assert.equal(JSON.parse(storage.get("daena:collection-query:timeline")).textSearch, "stored");
  assert.equal(
    JSON.parse(storage.get("daena:collection-query:lore")).section,
    "lore",
    "the old section's key is left alone",
  );
  await svelteTick();
  assert.equal(store.collectionQueryRestoring, false);

  // Edits to the query are persisted under the current section's key.
  store.collectionQuery.textSearch = "typed";
  flushSync();
  assert.equal(JSON.parse(storage.get("daena:collection-query:timeline")).textSearch, "typed");
  assert.equal(JSON.parse(storage.get("daena:collection-query:lore")).textSearch, "");

  // Effects stop with the store: a later section change is not seen.
  store.destroy();
  state.section = "lore";
  flushSync();
  assert.equal(store.collectionQuery.section, "timeline");
}

// ---- Effects: the scope reset and the collection fetch --------------------------------------------------------
{
  timers.clear();
  const requests = [];
  const fixture = createFixture({
    project: {
      queryEntities: (query) => {
        const request = { query, ...deferred() };
        requests.push(request);
        return request.promise;
      },
    },
  });
  const { store, state, calls } = fixture;
  const page = (items, extra = {}) => ({
    items,
    total: items.length,
    offset: 0,
    limit: 50,
    has_more: false,
    type_counts: [],
    ...extra,
  });
  assert.deepEqual(plain(store.collectionPage), plain(emptyEntityPage()));

  // Not ready: no query, an empty page.
  assert.equal(store.collectionLoading, false);
  assert.equal(timers.size, 0);

  state.ready = true;
  state.projectInfo = { root: "/project" };
  flushSync();
  assert.equal(store.collectionLoading, true, "an open project starts loading");
  assert.equal([...timers.values()].at(-1).ms, 0, "an empty search fetches without delay");
  fireTimers();
  assert.deepEqual(plain(requests[0].query), {
    entityTypes: ["daena.lore:character"],
    excludedEntityTypes: [],
    sortField: "name",
    sortDirection: "asc",
    offset: 0,
    limit: 50,
  });
  requests[0].resolve(page([entity("a"), entity("b")]));
  await tick();
  assert.deepEqual(
    store.collectionPage.items.map((item) => item.id),
    ["a", "b"],
  );
  assert.equal(store.collectionLoading, false);
  assert.equal(
    calls.filter(([name]) => name === "upsertEntityInCache").length,
    2,
    "fetched entries enter the entity cache",
  );

  // A search is debounced, and a newer request makes an older answer irrelevant.
  store.collectionQuery.textSearch = "  ann ";
  flushSync();
  assert.equal(store.collectionPage.items.length, 0, "a new scope empties the page");
  assert.equal([...timers.values()].at(-1).ms, 180, "a search waits 180ms");
  assert.equal(requests.length, 1, "nothing is asked before the debounce ends");
  store.collectionQuery.textSearch = "annie";
  flushSync();
  assert.equal(timers.size, 1, "retyping cancels the pending timer");
  fireTimers();
  assert.equal(requests[1].query.query, "annie");
  store.collectionQuery.sortDir = "desc";
  flushSync();
  fireTimers();
  assert.equal(requests[2].query.sortDirection, "desc");
  requests[2].resolve(page([entity("c")]));
  await tick();
  requests[1].resolve(page([entity("stale")]));
  await tick();
  assert.deepEqual(
    store.collectionPage.items.map((item) => item.id),
    ["c"],
    "an answer to an older request is dropped",
  );
  assert.equal(store.collectionLoading, false);

  // A scope change resets the page index; a plain page change does not.
  store.collectionQuery.page = 2;
  flushSync();
  assert.equal(store.collectionQuery.page, 2);
  fireTimers();
  assert.equal(requests[3].query.offset, 100, "the page index becomes an offset");
  store.collectionQuery.pageSize = 25;
  flushSync();
  assert.equal(store.collectionQuery.page, 0, "changing the page size goes back to the first page");
  // While a stored query is being applied the scope may change without a reset.
  store.collectionQueryRestoring = true;
  store.collectionQuery.page = 3;
  store.collectionQuery.excludedTypes = ["x"];
  flushSync();
  assert.equal(store.collectionQuery.page, 3, "the restoring flag keeps the page");
  store.collectionQueryRestoring = false;
  fireTimers();

  // Asking for a page past the end moves to the last page instead of showing nothing.
  const before = requests.length;
  const last = requests.at(-1);
  last.resolve(page([], { total: 120 }));
  await tick();
  assert.equal(store.collectionQuery.page, 4, "120 entries at 25 per page end on page index 4");
  assert.equal(requests.length, before, "the correction does not itself resolve a page");

  // Failures show an error and an empty page.
  flushSync();
  fireTimers();
  requests.at(-1).reject(new Error("boom"));
  await tick();
  assert.notEqual(store.collectionError, "");
  assert.equal(store.collectionPage.items.length, 0);
  assert.equal(store.collectionLoading, false);

  // The Writing manuscript outline loads itself, so the page fetch stays quiet there.
  state.section = "writing";
  flushSync();
  const asked = requests.length;
  fireTimers();
  assert.equal(requests.length, asked, "grouped manuscripts without a search ask nothing");
  assert.equal(store.collectionLoading, false);
  state.writingView = "scenes";
  flushSync();
  fireTimers();
  assert.equal(requests.length, asked + 1, "other Writing tabs fetch their page");

  // Closing the project empties the page and stops loading.
  state.ready = false;
  flushSync();
  assert.equal(store.collectionLoading, false);
  assert.equal(store.collectionPage.items.length, 0);
  store.destroy();
}

// ---- Groups, outline, kinship and appearance labels -----------------------------------------------------------------
{
  const fixture = createFixture();
  const { store } = fixture;
  store.toggleGroup("person");
  assert.deepEqual([...store.expandedGroups], ["person"]);
  const before = store.expandedGroups;
  store.toggleGroup("place");
  assert.notEqual(store.expandedGroups, before, "toggling replaces the set so the template updates");
  store.toggleGroup("person");
  assert.deepEqual([...store.expandedGroups], ["place"]);
  store.toggleManuscriptOutlineNode("m1");
  store.toggleManuscriptOutlineNode("m2");
  store.toggleManuscriptOutlineNode("m1");
  assert.deepEqual([...store.manuscriptExpanded], ["m2"]);

  assert.equal(store.kinshipRefreshEpoch, 0);
  store.bumpKinshipRefresh();
  assert.equal(store.kinshipRefreshEpoch, 1);

  store.writingAppearanceLabels = { x: "y" };
  await store.refreshWritingAppearanceLabels();
  assert.deepEqual(plain(store.writingAppearanceLabels), {}, "no selection, no labels");
  await openEntry(fixture);
  store.writingAppearanceLabels = { x: "y" };
  await store.refreshWritingAppearanceLabels();
  assert.deepEqual(plain(store.writingAppearanceLabels), {}, "no Writing links, no labels");
  store.destroy();
}

// ---- Which surface is open: derived gates and the surface key ---------------------------------------------------------
{
  const { store } = createFixture();
  assert.equal(store.projectHomeOpen, true, "a store starts on the home page");
  assert.equal(store.section, "lore");
  assert.equal(store.specializedSurfaceKey(), null);
  assert.equal(store.projectionSandboxKey(), null);
  assert.equal(store.projectionSandboxOpen(), false);

  store.sandboxView = { plugin: { id: "daena.lore" }, view: { id: "graph" }, renderer: "webview" };
  assert.equal(store.projectionSandboxKey(), "lore");
  assert.equal(store.projectionSandboxOpen("lore"), true);
  assert.equal(store.projectionSandboxOpen("timeline"), false);
  assert.equal(store.specializedSurfaceKey(), "workspace:lore:graph", "a projection is matched before a plain sandbox");
  assert.equal(store.currentWorkspaceLocationView(), "graph");
  store.sandboxView = { plugin: { id: "p" }, view: { id: "v" }, renderer: "webview" };
  assert.equal(store.projectionSandboxKey(), null);
  assert.equal(store.specializedSurfaceKey(), "plugin:p:v");
  // The maps editor is a sandbox view too, but not a scrollable surface.
  store.sandboxView = { plugin: { id: "m" }, view: { id: "maps" }, renderer: "maps" };
  assert.equal(store.specializedSurfaceKey(), null);
  assert.equal(store.mapSurfaceOpen, false, "the maps surface needs the maps section");
  store.section = "maps";
  assert.equal(store.mapSurfaceOpen, true);
  store.sandboxView = { plugin: { id: "m" }, view: null, renderer: "maps" };
  assert.equal(store.mapSurfaceOpen, false, "the welcome screen is not an open map");
  store.sandboxView = null;

  store.section = "writing";
  store.writingView = "reference";
  assert.equal(store.currentWorkspaceLocationView(), "reference");
  store.selected = entity("w", { entity_type: "daena.writing:manuscript" });
  assert.equal(store.writingEntrySelected, true, "a manuscript is edited in the Writing webview");
  store.selected = entity("c", { entity_type: "daena.lore:character" });
  assert.equal(store.writingEntrySelected, false);
  store.section = "lore";
  store.selected = entity("w", { entity_type: "daena.writing:manuscript" });
  assert.equal(store.writingEntrySelected, false, "only in the Writing section");
  store.selected = null;
  store.loreWikiOpen = true;
  assert.equal(store.currentWorkspaceLocationView(), "wiki");
  assert.equal(store.specializedSurfaceKey(), "workspace:lore:wiki");
  store.destroy();
}

// ---- Effects: the wiki closes with Lore, the Writing and Timeline tabs are clamped -----------------------------------
{
  const fixture = createFixture();
  const { store, state } = fixture;
  store.section = "lore";
  store.loreWikiOpen = true;
  store.loreWikiEntityId = "a";
  flushSync();
  assert.equal(store.loreWikiOpen, true, "the wiki stays open in Lore");
  store.section = "timeline";
  flushSync();
  assert.equal(store.loreWikiOpen, false, "leaving Lore closes the wiki");
  assert.equal(store.loreWikiEntityId, null);

  // The tab clamp waits for installed modules, then falls back to the first tab that exists.
  state.collectionTabs = {
    writing: [{ id: "manuscripts" }, { id: "reference" }],
    timeline: [{ id: "events" }, { id: "calendars" }],
  };
  store.writingView = "gone";
  store.timelineView = "also-gone";
  flushSync();
  assert.equal(store.writingView, "gone", "nothing is clamped before the modules are known");
  state.moduleCount = 3;
  flushSync();
  assert.equal(store.writingView, "manuscripts");
  assert.equal(store.timelineView, "events");
  store.writingView = "reference";
  flushSync();
  assert.equal(store.writingView, "reference", "a tab that exists is kept");

  // The Language list is refreshed once per visit and opens the first entry.
  state.ready = true;
  state.projectInfo = { root: "/project" };
  state.collectionEntities = [entity("a")];
  const epoch = store.collectionRefreshEpoch;
  store.section = "language";
  flushSync();
  assert.equal(store.collectionRefreshEpoch, epoch + 1, "arriving in Language refreshes the list");
  assert.equal(store.languageListRefreshed, true);
  await tick();
  assert.equal(store.selected?.id, "a", "the first language opens when none is selected");
  store.bumpCollectionRefresh();
  flushSync();
  assert.equal(store.languageListRefreshed, true, "staying keeps the flag, so the list is not refreshed again");
  store.section = "lore";
  flushSync();
  assert.equal(store.languageListRefreshed, false, "leaving resets it for the next visit");
  store.clearSelection();
  store.destroy();
}

// ---- Effects: the pending collection scroll waits for the list, then applies once -------------------------------------
{
  const fixture = createFixture();
  const { store } = fixture;
  const list = { scrollTop: 0 };
  store.collectionListElement = list;
  store.projectHomeOpen = false;
  store.queueCollectionScroll("lore", 240);
  store.hostView = { plugin: { id: "p" }, view: { id: "v" } };
  flushSync();
  assert.notEqual(store.pendingCollectionScroll, null, "a full surface hides the list: the scroll waits");
  store.hostView = null;
  store.collectionLoading = true;
  flushSync();
  assert.notEqual(store.pendingCollectionScroll, null, "a loading list: the scroll waits");
  store.collectionLoading = false;
  flushSync();
  assert.equal(store.pendingCollectionScroll, null, "the queued scroll is taken");
  await svelteTick();
  assert.equal(store.collectionListElement.scrollTop, 240);
  assert.equal(store.collectionScrollBySection.lore, 240);
  store.destroy();
}

// ---- Location capture and restore: a workspace location round-trips ------------------------------------------------------
{
  window.requestAnimationFrame = (callback) => callback();
  const fixture = createFixture();
  const { store, state, calls } = fixture;
  state.ready = true;
  state.projectInfo = { root: "/project" };
  state.entities = [entity("a"), entity("b"), entity("tree-root")];
  state.collectionTabs = {
    writing: [
      { id: "manuscripts", entityTypes: ["daena.writing:manuscript"] },
      { id: "reference", entityTypes: ["daena.writing:reference-page"] },
    ],
    timeline: [{ id: "events", entityTypes: [] }],
  };
  state.moduleCount = 2;
  flushSync();

  // A Writing location with a tab, an entry, a query, open groups, scroll and pane widths.
  await store.switchSection("writing");
  await store.switchWritingView("reference");
  await store.selectEntity(state.entities[0], false);
  timers.clear();
  store.collectionQuery.textSearch = "ann";
  store.collectionQuery.sortDir = "desc";
  store.collectionQuery.pageSize = 25;
  store.collectionQuery.excludedTypes = ["x", "a"];
  store.expandedGroups = new Set(["place", "person"]);
  flushSync();
  store.collectionQuery.page = 0;
  store.collectionScrollBySection.writing = 120;
  store.restoredWorkspacePaneDimensions = {
    collectionWidth: 301,
    contentWidth: 702,
    inspectorWidth: 333,
    viewportWidth: 1400,
  };
  const writing = store.currentShellLocation();
  assert.equal(writing.kind, "workspace");
  assert.equal(writing.section, "writing");
  assert.equal(writing.view, "reference");
  assert.equal(writing.writingView, "reference");
  assert.equal(writing.entityId, "a");
  assert.deepEqual(plain(writing.collection), {
    query: {
      textSearch: "ann",
      sortField: "name",
      sortDir: "desc",
      pageSize: 25,
      page: 0,
      excludedTypes: ["a", "x"],
      viewMode: "grouped",
    },
    expandedGroups: ["person", "place"],
    scrollTop: 120,
  });
  assert.deepEqual(plain(writing.panes), {
    collectionWidth: 301,
    contentWidth: 702,
    inspectorWidth: 333,
    viewportWidth: 1400,
  });
  const snapshot = plain(writing);

  // Go somewhere else: the home page, with no entry open.
  await store.openProjectHome();
  assert.equal(store.projectHomeOpen, true);
  assert.deepEqual(store.currentShellLocation(), { kind: "home" });
  store.clearSelection();
  store.writingView = "manuscripts";
  store.section = "lore";
  store.collectionQuery.textSearch = "";
  flushSync();
  calls.length = 0;

  assert.equal(await store.restoreShellLocation(writing), true, "the restored location is the one that was captured");
  assert.deepEqual(
    plain(store.currentShellLocation()),
    snapshot,
    "capture, restore and capture again is a fixed point",
  );
  assert.equal(store.section, "writing");
  assert.equal(store.writingView, "reference");
  assert.equal(store.projectHomeOpen, false);
  assert.equal(store.selected?.id, "a");
  assert.equal(store.collectionQuery.textSearch, "ann");
  assert.deepEqual([...store.expandedGroups].sort(), ["person", "place"]);
  assert.equal(store.collectionScrollBySection.writing, 120);
  assert.deepEqual(plain(store.restoredWorkspacePaneDimensions), plain(writing.panes));
  const flags = calls.filter(([name]) => name === "setShellNavigationRestoring");
  assert.deepEqual(flags, [
    ["setShellNavigationRestoring", true],
    ["setShellNavigationRestoring", false],
  ]);
  assert.deepEqual(calls[0], ["setShellNavigationRestoring", true], "the restore flag goes up first");
  assert.deepEqual(calls.at(-1), ["setShellNavigationRestoring", false], "and comes down last");
  assert.ok(
    calls.some(([name]) => name === "recordShellDeparture"),
    "the switches inside a restore still hand their departures to the page, which ignores them while restoring",
  );

  // An entry that no longer exists is dropped, and the location still restores (without the entry).
  const gone = { ...plain(writing), entityId: "missing" };
  state.entities = state.entities.filter((candidate) => candidate.id !== "missing");
  assert.equal(await store.restoreShellLocation(gone), true, "a missing entry restores the rest of the location");
  assert.equal(store.selected, null);

  // Houses tree: the root comes from entityId, the session from module state.
  const tree = {
    kind: "workspace",
    section: "houses",
    view: "tree",
    entityId: "tree-root",
    writingView: store.writingView,
    timelineView: store.timelineView,
    collection: plain(writing.collection),
    panes: plain(writing.panes),
    surfaceScrollTop: 0,
    moduleState: { rootId: "tree-root", nonsense: 1 },
  };
  const nonce = store.treeRestoreNonce;
  assert.equal(await store.restoreShellLocation(tree), true);
  assert.equal(store.section, "houses");
  assert.equal(store.housesView, "tree");
  assert.equal(store.treeRootId, "tree-root");
  assert.deepEqual(plain(store.treeSession), { rootId: "tree-root", nonsense: 1 });
  assert.equal(store.treeRestoreNonce, nonce + 1, "the tree is told to rebuild");
  assert.equal(store.selected, null, "a tree location selects no entry");
  assert.deepEqual(plain(store.currentShellLocation().moduleState), { rootId: "tree-root", nonsense: 1 });
  assert.equal(store.specializedSurfaceKey(), "workspace:houses:tree");

  // Language: the pane goes to the Language bridge, which answers by setting the pane.
  const language = {
    ...tree,
    section: "language",
    view: "default",
    entityId: null,
    moduleState: { pane: "grammar" },
  };
  calls.length = 0;
  assert.equal(await store.restoreShellLocation(language), true);
  assert.deepEqual(
    calls.find(([name]) => name === "requestLanguagePane"),
    ["requestLanguagePane", "grammar"],
  );
  assert.deepEqual(plain(store.currentShellLocation().moduleState), { pane: "grammar" });
  store.clearSelection();

  // Lore wiki and the World graph reopen through openWorkspaceView.
  state.adminPlugins = [{ id: "daena.lore", enabled: true }];
  const wiki = {
    kind: "workspace",
    section: "lore",
    view: "wiki",
    entityId: null,
    writingView: store.writingView,
    timelineView: store.timelineView,
    collection: plain(writing.collection),
    panes: plain(writing.panes),
    surfaceScrollTop: 0,
    moduleState: null,
  };
  assert.equal(await store.restoreShellLocation(wiki), true);
  assert.equal(store.loreWikiOpen, true);
  assert.equal(store.currentWorkspaceLocationView(), "wiki");
  assert.equal(await store.restoreShellLocation({ ...wiki, view: "graph" }), true);
  assert.equal(store.projectionSandboxKey(), "lore");
  assert.equal(store.loreWikiOpen, false, "opening the graph closes the wiki");
  assert.equal(await store.restoreShellLocation({ ...wiki, view: "library" }), true);
  assert.equal(store.sandboxView, null);
  assert.equal(store.loreWikiOpen, false);
  delete window.requestAnimationFrame;
  store.destroy();
}

// ---- Location capture and restore: home, settings, project and plugin locations --------------------------------------------
{
  window.requestAnimationFrame = (callback) => callback();
  const fixture = createFixture();
  const { store, state, calls } = fixture;
  state.ready = true;
  state.projectInfo = { root: "/project" };
  state.pluginItems = {
    "p:v": { kind: "plugin", plugin: { id: "p" }, view: { id: "v" }, key: "p:v", renderer: "webview" },
    "h:hv": { kind: "plugin", plugin: { id: "h" }, view: { id: "hv" }, key: "h:hv", renderer: "host" },
  };
  await store.switchSection("lore");
  await store.selectEntity(state.entities[1], false);
  timers.clear();
  const workspace = plain(store.currentShellLocation());

  // Settings and the project center go through the page's own openers, which set the location flags.
  assert.equal(await store.restoreShellLocation({ kind: "settings", section: "ai" }), true);
  assert.deepEqual(plain(store.currentShellLocation()), { kind: "settings", section: "ai" });
  assert.equal(await store.restoreShellLocation({ kind: "project", section: "fields" }), true);
  assert.deepEqual(plain(store.currentShellLocation()), { kind: "project", section: "fields" });
  assert.deepEqual(
    calls.filter(([name]) => name === "openSettings" || name === "openProjectCenter"),
    [
      ["openSettings", "ai"],
      ["openProjectCenter", "fields"],
    ],
  );
  // Home, which first closes the settings surface.
  calls.length = 0;
  assert.equal(await store.restoreShellLocation({ kind: "home" }), true);
  assert.deepEqual(plain(store.currentShellLocation()), { kind: "home" });
  assert.ok(
    calls.some(([name]) => name === "clearGlobalQuery"),
    "going home clears the search box",
  );

  // A webview plugin location, with its entry and scroll.
  const element = { scrollTop: 0 };
  store.specializedSurfaceElement = element;
  const pluginLocation = {
    kind: "plugin",
    key: "p:v",
    section: "lore",
    entityId: "b",
    surfaceScrollTop: 55,
    moduleState: null,
  };
  calls.length = 0;
  assert.equal(await store.restoreShellLocation(pluginLocation), true);
  assert.equal(store.sandboxView.plugin.id, "p");
  assert.equal(store.sandboxView.renderer, "webview");
  assert.equal(store.selected?.id, "b");
  assert.equal(store.specializedSurfaceKey(), "plugin:p:v");
  assert.equal(store.specializedSurfaceElement.scrollTop, 55, "the surface scroll is restored");
  assert.deepEqual(plain(store.currentShellLocation()), plain(pluginLocation));
  assert.equal(store.projectHomeOpen, false);

  // A host surface.
  store.specializedSurfaceElement = { scrollTop: 0 };
  const hostLocation = { ...pluginLocation, key: "h:hv", surfaceScrollTop: 8 };
  delete hostLocation.moduleState;
  assert.equal(await store.restoreShellLocation(hostLocation), true);
  assert.equal(store.hostView.plugin.id, "h");
  assert.equal(store.sandboxView, null, "leaving the webview surface for a host surface clears it");
  assert.deepEqual(plain(store.currentShellLocation()), plain(hostLocation));

  // A plugin that is gone: nothing changes, not even the restore flag.
  calls.length = 0;
  assert.equal(await store.restoreShellLocation({ ...pluginLocation, key: "gone:view" }), false);
  assert.deepEqual(calls, [], "an unavailable plugin location is refused before anything runs");
  assert.equal(store.hostView.plugin.id, "h");

  // Back to the workspace location captured at the start.
  assert.equal(await store.restoreShellLocation(workspace), true);
  assert.equal(store.hostView, null);
  assert.deepEqual(plain(store.currentShellLocation()), workspace);
  delete window.requestAnimationFrame;
  store.destroy();
}

// ---- switchSection: the leave sequence and a refused leave --------------------------------------------------------------------
{
  const names = (calls) => calls.map(([name]) => name);
  const setup = async () => {
    const fixture = createFixture();
    fixture.state.ready = true;
    fixture.state.projectInfo = { root: "/project" };
    await fixture.store.switchSection("language");
    await openEntry(fixture);
    fixture.store.collectionQuery.textSearch = "kept";
    fixture.store.hostView = { plugin: { id: "p" }, view: { id: "v" } };
    fixture.calls.length = 0;
    return fixture;
  };

  // Every guard passes: flush, then the Language webview, then settings, then the plugin surface, then history.
  {
    const fixture = await setup();
    const { store, calls } = fixture;
    store.updateDocumentBody("edited");
    store.cancelAutoSave();
    calls.length = 0;
    await store.switchSection("timeline");
    assert.deepEqual(
      names(calls).filter((name) =>
        [
          "flushWritingWebview",
          "canLeaveLanguageSection",
          "dismissSettings",
          "resolveDirtyMapSession",
          "closeNativePluginWebviews",
          "recordShellDeparture",
        ].includes(name),
      ),
      [
        "flushWritingWebview",
        "canLeaveLanguageSection",
        "dismissSettings",
        "resolveDirtyMapSession",
        "closeNativePluginWebviews",
        "recordShellDeparture",
      ],
      "autosave, the Language guard, settings, the plugin surface, then the departure is recorded",
    );
    assert.equal(fixture.concurrency.saves.length, 1, "the edit was saved before anything was left");
    const departure = calls.find(([name]) => name === "recordShellDeparture")[1];
    assert.equal(departure.kind, "plugin", "the departure is the location that was left");
    assert.equal(store.section, "timeline");
    assert.equal(store.hostView, null);
    assert.equal(store.selected, null, "a new section clears the selection");
    assert.equal(store.collectionQuery.textSearch, "", "and the search");
    store.destroy();
  }

  // Each guard can refuse, and a refusal changes nothing after it.
  const refusals = [
    [
      "a failed save",
      async (fixture) => {
        fixture.project.saveEntry = async () => {
          throw new Error("disk full");
        };
        fixture.store.updateDocumentBody("edited");
        fixture.store.cancelAutoSave();
      },
      ["flushWritingWebview"],
    ],
    [
      "the Language guard",
      async (fixture) => {
        fixture.store.canLeaveLanguageSection = async () => {
          fixture.calls.push(["canLeaveLanguageSection"]);
          return false;
        };
      },
      ["flushWritingWebview", "canLeaveLanguageSection"],
    ],
    [
      "the settings guard",
      async (fixture) => {
        fixture.ctx.dismissSettings = async () => {
          fixture.calls.push(["dismissSettings"]);
          return false;
        };
      },
      ["flushWritingWebview", "canLeaveLanguageSection", "dismissSettings"],
    ],
    [
      "the map session guard",
      async (fixture) => {
        fixture.store.resolveDirtyMapSession = async () => {
          fixture.calls.push(["resolveDirtyMapSession"]);
          return false;
        };
      },
      ["flushWritingWebview", "canLeaveLanguageSection", "dismissSettings", "resolveDirtyMapSession"],
    ],
  ];
  for (const [label, refuse, expectedCalls] of refusals) {
    const fixture = await setup();
    const { store, calls } = fixture;
    await refuse(fixture);
    calls.length = 0;
    await store.switchSection("timeline");
    assert.deepEqual(
      names(calls).filter((name) => name !== "setError" && name !== "upsertEntityInCache"),
      expectedCalls,
      `${label} stops the sequence at its step`,
    );
    assert.equal(store.section, "language", `${label} leaves the section`);
    assert.equal(store.selected?.id, "a", `${label} keeps the entry`);
    assert.equal(store.collectionQuery.textSearch, "kept", `${label} keeps the search`);
    assert.equal(store.hostView?.plugin.id, "p", `${label} keeps the plugin surface`);
    assert.equal(store.projectHomeOpen, false, `${label} stays off the home page`);
    assert.ok(!names(calls).includes("recordShellDeparture"), `${label} records no departure`);
    assert.ok(!names(calls).includes("closeNativePluginWebviews"), `${label} closes no webview`);
    store.cancelAutoSave();
    store.destroy();
  }

  // Switching to the section already open is a no-op that does not even ask the guards (after the flush).
  {
    const fixture = await setup();
    fixture.store.hostView = null;
    fixture.calls.length = 0;
    await fixture.store.switchSection("language");
    assert.deepEqual(names(fixture.calls), ["flushWritingWebview"], "the same section only flushes");
    assert.equal(fixture.store.selected?.id, "a");
    fixture.store.destroy();
  }
}

// ---- Writing and Language bridges -------------------------------------------------------------------------------
{
  const fixture = createFixture();
  const { store } = fixture;
  assert.equal(await store.flushWritingWebview(), false, "a webview that is not mounted has nothing to flush");
  store.writingWebview.mounted = true;
  assert.equal(await store.flushWritingWebview(), false, "a webview that is not ready has nothing to flush");
  store.writingWebview.ready = true;
  assert.equal(store.writingLoad, null);
  assert.equal(await store.flushWritingWebview(), false, "a ready webview with no load has nothing to flush");
  store.destroy();
}
{
  const fixture = createFixture();
  const { store } = fixture;
  const posts = [];
  fixture.project.postPluginWebviewMessage = async () => {
    posts.push("post");
    throw new Error("gone");
  };
  store.languageWebview.mounted = true;
  store.languageWebview.ready = true;
  store.section = "lore";
  assert.equal(await store.canLeaveLanguageSection(), true, "outside Language the guard allows the leave");
  assert.equal(posts.length, 0, "outside Language the guard does not post");
  store.section = "language";
  assert.equal(await store.canLeaveLanguageSection(), true, "a post that fails resolves as may-leave");
  assert.equal(posts.length, 1);
  store.destroy();
}
{
  const fixture = createFixture();
  const { store } = fixture;
  const firstWriting = store.trackWritingMount();
  const secondWriting = store.trackWritingMount();
  assert.equal(store.writingWebview.mounted, true);
  assert.equal(store.writingWebview.ready, false);
  assert.equal(store.writingHostedInstance, null);
  firstWriting();
  assert.equal(store.writingWebview.mounted, true, "a stale Writing cleanup leaves the new mount mounted");
  assert.equal(store.writingWebview.ready, false, "a stale Writing cleanup leaves the new mount's ready flag");
  assert.equal(store.writingLoad, null, "a stale Writing cleanup leaves the new mount's load");
  secondWriting();
  assert.equal(store.writingWebview.mounted, false, "the current Writing cleanup still clears the mount");

  const firstLanguage = store.trackLanguageMount();
  const secondLanguage = store.trackLanguageMount();
  assert.equal(store.languageWebview.mounted, true);
  assert.equal(store.languageWebview.ready, false);
  assert.equal(store.languagePaneRequest, null);
  firstLanguage();
  assert.equal(store.languageWebview.mounted, true, "a stale Language cleanup leaves the new mount mounted");
  assert.equal(store.languageWebview.ready, false, "a stale Language cleanup leaves the new mount's ready flag");
  assert.equal(store.languageHostedInstance, null);
  secondLanguage();
  assert.equal(store.languageWebview.mounted, false, "the current Language cleanup still clears the mount");
  store.destroy();
}
{
  const fixture = createFixture();
  const { store } = fixture;
  await openEntry(fixture);
  store.writingLoad = { entityId: "a", token: "current" };
  store.documentBody = "body a";
  assert.equal(
    await store.applyWritingDraft({ entityId: "a", token: "stale", text: "from before the reload" }),
    false,
    "a draft with a stale token is ignored",
  );
  assert.equal(store.documentBody, "body a");
  assert.equal(store.hasUnsavedChanges, false);
  assert.equal(
    await store.applyWritingDraft({ entityId: "b", token: "current", text: "another entry" }),
    false,
    "a draft for another entry is ignored",
  );
  assert.equal(store.documentBody, "body a");
  store.destroy();
}
{
  // The default instance is the tab on screen at the call, not the tab that was current when the store was built.
  const fixture = createFixture();
  const posts = [];
  fixture.project.postPluginWebviewMessage = async (_pluginId, instanceId) => {
    posts.push(instanceId);
  };
  fixture.ctx.shownWebviewInstance = "later-tab";
  await fixture.store.postWritingMessage({ type: "writing.editable", editable: false });
  assert.deepEqual(posts, ["later-tab"]);
  fixture.store.destroy();
}

// ---- Map session and selection AI --------------------------------------------------------------------------------
{
  const worldMap = () => entity("m", { entity_type: "daena.maps:world-map" });
  const openMap = (store) => {
    store.selected = worldMap();
    store.sandboxView = { renderer: "maps" };
    store.mapsEditorMode = "vector";
    store.mapSaveStates = { m: { status: "dirty", detail: null } };
  };

  // No map open: allow the leave, and do not ask.
  {
    const fixture = createFixture();
    const asked = [];
    fixture.ctx.confirmDialog = async (dialog) => {
      asked.push(dialog.title);
      return false;
    };
    assert.equal(await fixture.store.resolveDirtyMapSession(), true);
    assert.deepEqual(asked, []);
    fixture.store.destroy();
  }

  // Dirty map, and the user declines the save and the discard.
  {
    const fixture = createFixture();
    const { store } = fixture;
    openMap(store);
    const asked = [];
    fixture.ctx.confirmDialog = async (dialog) => {
      asked.push(dialog.title);
      return false;
    };
    assert.equal(await store.resolveDirtyMapSession(), false);
    assert.deepEqual(asked, ["Save changes to this map before leaving?", "Discard unsaved map edits?"]);
    store.destroy();
  }

  // Dirty map, and the user saves. save() clears the dirty flag the way the editor's maps-state event does
  // while the native session saves. This function itself does not clear mapSaveStates.
  {
    const fixture = createFixture();
    const { store } = fixture;
    openMap(store);
    const saves = [];
    let dirty = true;
    fixture.ctx.confirmDialog = async () => true;
    fixture.ctx.nativeVectorSession = () => ({
      save: async () => {
        saves.push("save");
        dirty = false;
        store.mapSaveStates = { m: { status: "saved", detail: null } };
      },
      isDirty: () => dirty,
    });
    assert.equal(await store.resolveDirtyMapSession(), true);
    assert.deepEqual(saves, ["save"]);
    store.destroy();
  }

  // A selection event for another map does not change the selection.
  {
    const fixture = createFixture();
    const { store } = fixture;
    store.selected = worldMap();
    store.handleMapsSelection({ mapEntityId: "other", anchor: { kind: "point" } });
    assert.equal(store.mapSelection, null);
    store.handleMapsSelection({ mapEntityId: "m", anchor: { kind: "point" } });
    assert.deepEqual(store.mapSelection, { kind: "point" });
    store.destroy();
  }

  // closeAiRewrite while listenAiStream is still pending sets the start-cancelled flag.
  // The unlisten does not exist yet, so it is not called. When listen later resolves, the
  // function is stored and the poll still runs. The source does not call that unlisten.
  {
    const fixture = createFixture();
    const { store } = fixture;
    fixture.state.projectInfo = { root: "/project" };
    store.selected = entity("a");
    store.aiSourceSelection = "selected prose";
    store.aiInstruction = "Rewrite it.";
    const cancelled = [];
    fixture.project.aiCancelText = async (id) => {
      cancelled.push(id);
    };
    fixture.project.aiGenerateText = async () => "req-1";
    const polls = [];
    fixture.project.aiPollText = async (id) => {
      polls.push(id);
      return [];
    };
    let releaseListen;
    let markListening;
    const listening = new Promise((resolve) => {
      markListening = resolve;
    });
    let unlistenCalls = 0;
    const unlisten = () => {
      unlistenCalls += 1;
    };
    const heard = [];
    fixture.ctx.listenAiStream = (requestId) => {
      heard.push(requestId);
      markListening();
      return new Promise((resolve) => {
        releaseListen = () => resolve(unlisten);
      });
    };
    const pending = store.startAiRewrite();
    const reached = await Promise.race([listening.then(() => "listen"), pending.then(() => "done")]);
    assert.equal(reached, "listen");
    assert.equal(store.aiRequestId, "req-1");
    assert.equal(store.aiStartCancelled, false);
    store.closeAiRewrite();
    assert.equal(store.aiStartCancelled, true);
    assert.equal(unlistenCalls, 0);
    releaseListen();
    await pending;
    assert.equal(unlistenCalls, 0, "the resolved unlisten is stored, not called");
    assert.equal(store.aiUnlisten, unlisten);
    assert.deepEqual(heard, ["req-1"]);
    assert.deepEqual(cancelled, ["req-1"]);
    assert.deepEqual(polls, ["req-1"], "the poll still runs after the late listen");
    store.destroy();
  }

  // Drops an out-of-order sequence, and an event whose entity is no longer selected.
  // A stale request id is a different `ai-stream:<requestId>` channel: listenAiStream receives the
  // id, and the handler does not compare payload.requestId.
  {
    const fixture = createFixture();
    const { store } = fixture;
    store.selected = entity("a");
    store.aiSourceEntityId = "a";
    const event = (sequence, delta, requestId = "req-1") => ({
      sequence,
      requestId,
      phase: "delta",
      delta,
      output: null,
      error: null,
    });
    store.handleAiEvent(event(1, "hi"));
    assert.equal(store.aiStreamText, "hi");
    assert.equal(store.aiLastSequence, 1);
    store.handleAiEvent(event(1, "NO"));
    store.handleAiEvent(event(0, "NO"));
    assert.equal(store.aiStreamText, "hi", "an old or equal sequence is dropped");
    assert.equal(store.aiLastSequence, 1);
    store.selected = entity("b");
    store.handleAiEvent(event(2, "NO"));
    assert.equal(store.aiStreamText, "hi", "an event for an entity that is no longer selected is dropped");
    assert.equal(store.aiLastSequence, 1);
    store.destroy();
  }

  // The rewrite dialog and a running generation both block Writing edits.
  {
    const { store } = createFixture();
    assert.equal(store.writingEditable, true);
    store.aiRewriteOpen = true;
    flushSync();
    assert.equal(store.writingEditable, false);
    store.aiRewriteOpen = false;
    store.aiBusy = true;
    flushSync();
    assert.equal(store.writingEditable, false);
    store.aiBusy = false;
    flushSync();
    assert.equal(store.writingEditable, true);
    store.destroy();
  }
}

// ---- Teardown ---------------------------------------------------------------------------------------------------
{
  const { store } = createFixture();
  store.destroy();
  assert.doesNotThrow(() => store.destroy(), "destroying twice is harmless");
}

console.log("location store checks passed");
