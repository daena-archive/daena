import assert from "node:assert/strict";
import {
  DEFAULT_LANGUAGE_PANE,
  LANGUAGE_STATE_EVENT_TYPE,
  createLanguageStatePublisher,
  isLanguagePane,
  paneStatePayload,
  parseLanguageHostMessage,
  parseLanguageMountParams,
  pickLanguageManifest,
} from "../packages/modules/language/src/plugin-protocol.ts";
import { createBrokerEditorHost, SHARED_ASSET_READ_METHOD } from "../packages/modules/language/src/plugin-editor-host.ts";

// Mount parameters: entityId and the percent-encoded pane state from SandboxView.
const launch = parseLanguageMountParams(
  "?project=p&view=language-workspace&entityId=e-1&state=%7B%22pane%22%3A%22grammar%22%7D",
);
assert.equal(launch.projectId, "p");
assert.equal(launch.entityId, "e-1");
assert.equal(launch.pane, "grammar");
assert.deepEqual(launch.moduleState, { pane: "grammar" });

const bare = parseLanguageMountParams("?project=p");
assert.equal(bare.entityId, null, "no selection means no entity");
assert.equal(bare.pane, DEFAULT_LANGUAGE_PANE);
assert.equal(bare.moduleState, null);

const unknownPane = parseLanguageMountParams("?state=%7B%22pane%22%3A%22nope%22%7D&entityId=");
assert.equal(unknownPane.pane, "overview", "unknown panes fall back to Overview");
assert.equal(unknownPane.entityId, null, "an empty entityId counts as none");

for (const broken of ["?state=%7Bnot-json", "?state=%5B1%5D", "?state=%22text%22"]) {
  const parsed = parseLanguageMountParams(broken);
  assert.equal(parsed.moduleState, null, `${broken} is ignored`);
  assert.equal(parsed.pane, "overview");
}

for (const pane of ["overview", "lexicon", "sounds", "writing", "grammar", "forms", "samples"]) {
  assert.equal(isLanguagePane(pane), true, pane);
}
assert.equal(isLanguagePane("guides"), false);
assert.equal(isLanguagePane(undefined), false);

// Host messages: only the two documented shapes are accepted.
assert.deepEqual(parseLanguageHostMessage({ type: "language.pane", pane: "sounds" }), {
  type: "language.pane",
  pane: "sounds",
});
assert.deepEqual(parseLanguageHostMessage({ type: "language.can-leave", requestId: "r-1" }), {
  type: "language.can-leave",
  requestId: "r-1",
});
assert.equal(parseLanguageHostMessage({ type: "language.pane", pane: "bogus" }), null);
assert.equal(parseLanguageHostMessage({ type: "language.can-leave", requestId: "" }), null);
assert.equal(parseLanguageHostMessage({ type: "language.can-leave" }), null);
assert.equal(parseLanguageHostMessage({ type: "other.message", pane: "overview" }), null);
assert.equal(parseLanguageHostMessage("language.pane"), null);
assert.equal(parseLanguageHostMessage(null), null);

// Pane reports derive from the workspace's module state.
assert.deepEqual(paneStatePayload({ pane: "lexicon" }), { kind: "pane", pane: "lexicon" });
assert.equal(paneStatePayload({ pane: "nope" }), null);
assert.equal(paneStatePayload(null), null);

// Publishing: one event type, payload passed through unchanged.
{
  const calls = [];
  const publish = createLanguageStatePublisher({
    async call(method, payload) {
      calls.push({ method, payload });
      return null;
    },
  });
  await publish({ kind: "leave-reply", requestId: "r-2", allowed: false });
  await publish({ kind: "entity-changed", entityId: "e-1", deleted: true });
  assert.deepEqual(calls, [
    {
      method: "event.publish",
      payload: {
        type: LANGUAGE_STATE_EVENT_TYPE,
        payload: { kind: "leave-reply", requestId: "r-2", allowed: false },
      },
    },
    {
      method: "event.publish",
      payload: {
        type: "daena.language/state@1",
        payload: { kind: "entity-changed", entityId: "e-1", deleted: true },
      },
    },
  ]);
}

// A failed publish is reported, never thrown into the edit that triggered it.
{
  const errors = [];
  const publish = createLanguageStatePublisher(
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
  const bundled = { id: "daena.language", schemas: [{ entityTypes: [{ id: "language" }] }] };
  const effective = { id: "daena.language", schemas: [{ entityTypes: [{ id: "daena.language:language" }] }] };
  assert.equal(pickLanguageManifest([{ id: "daena.timeline" }, effective], bundled), effective);
  assert.equal(pickLanguageManifest([{ id: "daena.timeline" }], bundled), bundled);
}

// Editor host: bytes and mime come from the shared-asset begin call, not Tauri.
{
  const calls = [];
  const fetched = [];
  const opened = [];
  const bytes = new Uint8Array([1, 2, 3, 4]);
  const host = createBrokerEditorHost({
    rpc: {
      async call(method, payload) {
        calls.push({ method, payload });
        if (method !== SHARED_ASSET_READ_METHOD) throw new Error(`unexpected ${method}`);
        if (payload.path === "assets/missing.png") throw new Error("not project scoped");
        return { handle: "h", url: `/__asset/${payload.path}`, size: 4, mimeType: "image/png" };
      },
    },
    async fetch(url) {
      fetched.push(url);
      return new Response(bytes, { status: 200 });
    },
    openLink: (href) => opened.push(href),
  });

  assert.equal(typeof host.assets?.pickAndRegisterAsset, "function", "the broker host offers asset management");
  const read = await host.readAssetBytesByPath("assets/a.png");
  assert.deepEqual([...read], [1, 2, 3, 4]);
  assert.deepEqual(fetched, ["/__asset/assets/a.png"]);
  assert.deepEqual(await host.getAssetByPath("assets/a.png"), { mime_type: "image/png" });
  assert.equal(
    calls.filter((call) => call.payload.path === "assets/a.png").length,
    1,
    "metadata reuses the read that already happened",
  );

  assert.equal(await host.readAssetBytesByPath("assets/missing.png"), null, "failed begin resolves to null");
  assert.equal(await host.getAssetByPath("assets/missing.png"), null);

  await host.openExternal("https://example.com/a");
  assert.deepEqual(opened, ["https://example.com/a"]);
}

{
  const host = createBrokerEditorHost({
    rpc: {
      async call() {
        return { url: "/__asset/x" };
      },
    },
    async fetch() {
      return new Response("nope", { status: 404 });
    },
    openLink() {},
  });
  assert.equal(await host.readAssetBytesByPath("assets/x.png"), null, "non-OK responses resolve to null");
  assert.deepEqual(await host.getAssetByPath("assets/x.png"), { mime_type: undefined });
}

{
  const host = createBrokerEditorHost({
    rpc: {
      async call() {
        throw new Error("transport down");
      },
    },
    async fetch() {
      throw new Error("never reached");
    },
    openLink() {},
  });
  assert.equal(await host.readAssetBytesByPath("assets/x.png"), null);
  assert.equal(await host.getAssetByPath("assets/x.png"), null);
}

console.log("language plugin source tests passed");
