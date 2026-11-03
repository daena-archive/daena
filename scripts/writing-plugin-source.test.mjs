import assert from "node:assert/strict";
import fs from "node:fs";
import {
  DEFAULT_WRITING_MODE,
  WRITING_DRAFT_INLINE_LIMIT_BYTES,
  WRITING_STATE_EVENT_TYPE,
  createWritingStatePublisher,
  isWritingMode,
  isWritingShortcut,
  parseWritingDraft,
  parseWritingHostMessage,
  parseWritingMountParams,
  parseWritingStatePayload,
  pickWritingManifest,
} from "../packages/modules/writing/src/plugin-protocol.ts";
import { entityIdsInMarkdown, loadWritingDocument } from "../packages/modules/writing/src/document-source.ts";
import { parseWritingStateEvent } from "../src/lib/modules/writing-bridge.ts";

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
assert.deepEqual(parseWritingHostMessage({ type: "writing.load", entityId: "e1", revision: "r1" }), {
  type: "writing.load",
  entityId: "e1",
  revision: "r1",
  editable: false,
  placeholder: "",
  mode: "read",
});
assert.deepEqual(
  parseWritingHostMessage({
    type: "writing.load",
    entityId: "e1",
    revision: "",
    editable: true,
    placeholder: "Write…",
    mode: "edit",
  }),
  { type: "writing.load", entityId: "e1", revision: "", editable: true, placeholder: "Write…", mode: "edit" },
  "a new entity has the empty revision",
);
for (const bad of [
  { type: "writing.load", revision: "r1" },
  { type: "writing.load", entityId: "", revision: "r1" },
  { type: "writing.load", entityId: "x".repeat(129), revision: "r1" },
  { type: "writing.load", entityId: "e1" },
  { type: "writing.load", entityId: "e1", revision: 4 },
  { type: "writing.load", entityId: "e1", revision: "r1", editable: "yes" },
  { type: "writing.load", entityId: "e1", revision: "r1", placeholder: 3 },
  { type: "writing.load", entityId: "e1", revision: "r1", mode: "preview" },
]) {
  assert.equal(parseWritingHostMessage(bad), null, JSON.stringify(bad));
}
assert.equal(
  parseWritingHostMessage({ type: "writing.load", entityId: "e1", revision: "r", placeholder: "p".repeat(500) })
    .placeholder.length,
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
});
assert.equal(parseWritingHostMessage({ type: "writing.conflict", entityId: "e1", message: 5 }), null);
assert.equal(parseWritingHostMessage({ type: "writing.conflict" }), null);
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
assert.equal(parseWritingHostMessage({ type: "language.pane", pane: "overview" }), null);
assert.equal(parseWritingHostMessage("writing.load"), null);
assert.equal(parseWritingHostMessage(null), null);
assert.equal(parseWritingHostMessage([]), null);

// State payloads: the webview parser and the shell bridge accept and reject the same things.
{
  const inline = { entityId: "e1", baseRevision: "r1", text: "Hello" };
  const accepted = [
    { kind: "ready", entityId: null },
    { kind: "ready", entityId: "e1" },
    { kind: "draft", draft: inline },
    { kind: "draft", draft: { entityId: "e1", baseRevision: "r1", transfer: "t-1" } },
    { kind: "flush-reply", requestId: "f-1", draft: null },
    { kind: "flush-reply", requestId: "f-1", draft: inline },
    { kind: "open-link", href: "https://example.com" },
    { kind: "open-entity", entityId: "e2" },
    { kind: "fullscreen", enabled: false },
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
    { kind: "draft", draft: { ...inline, transfer: "t-1" } },
    { kind: "draft", draft: { ...inline, text: "a".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES + 1) } },
    { kind: "flush-reply", draft: null },
    { kind: "flush-reply", requestId: "f-1" },
    { kind: "open-link", href: 1 },
    { kind: "open-entity", entityId: "" },
    { kind: "fullscreen", enabled: "true" },
    { kind: "request-rename" },
    { kind: "shortcut", shortcut: "save" },
  ];
  for (const payload of rejected) {
    assert.equal(parseWritingStatePayload(payload), null, `webview rejects ${JSON.stringify(payload)}`);
    assert.equal(parseWritingStateEvent(payload), null, `shell rejects ${JSON.stringify(payload)}`);
  }
  assert.equal(parseWritingDraft({ ...inline, text: "é".repeat(WRITING_DRAFT_INLINE_LIMIT_BYTES) }), null);
}

// Publishing: one event type, payload passed through unchanged.
{
  const calls = [];
  const publish = createWritingStatePublisher({
    async call(method, payload) {
      calls.push({ method, payload });
      return null;
    },
  });
  await publish({ kind: "ready", entityId: null });
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
  await publish({ kind: "open-link", href: "https://example.com" });
  assert.equal(errors.length, 1);
  assert.equal(errors[0].message, "broker offline");
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
  assert.match(workspace, /message\.type === "writing\.load"/);
  assert.doesNotMatch(workspace, /document\.save|documents\.save|entity\.update/, "slice 1 never writes");
}

// Bundle markers: the committed bundle is the broker-only build of this entry.
{
  const bundle = fs.readFileSync("src-tauri/plugin-assets/writing/writing.js", "utf8");
  assert.ok(bundle.length > 512, "the writing bundle is built");
  assert.match(bundle, /(["'`])\/__rpc\1/, "the bundle talks to the broker endpoint");
  assert.match(bundle, /daena\.writing\/state@1/, "the bundle publishes the Writing state event");
  assert.match(bundle, /writing-root/, "the bundle mounts the Writing root");
  assert.match(bundle, /asset\.shared\.read\.begin/, "the bundle reads images through the broker host");
  assert.doesNotMatch(
    bundle,
    /__TAURI_INTERNALS__|@tauri-apps|trusted_module_rpc/,
    "no Tauri or trusted RPC in the bundle",
  );
}

console.log("writing plugin source tests passed");
