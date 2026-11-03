import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createBrokerEditorHost, openFileChooser } from "../packages/modules/language/src/plugin-editor-host.ts";

// Fakes for the webview document, the broker RPC and the upload/read transports.

function fakeDocument() {
  const view = new EventTarget();
  const body = {
    children: [],
    appendChild(node) {
      this.children.push(node);
      return node;
    },
  };
  const inputs = [];
  return {
    inputs,
    body,
    defaultView: view,
    createElement(tagName) {
      assert.equal(tagName, "input");
      const input = new EventTarget();
      input.files = null;
      input.clicks = 0;
      input.click = () => {
        input.clicks += 1;
      };
      input.remove = () => {
        body.children = body.children.filter((node) => node !== input);
      };
      inputs.push(input);
      return input;
    },
  };
}

function choose(input, file) {
  input.files = [file];
  input.dispatchEvent(new Event("change"));
}

function fakeRpc(handlers) {
  const calls = [];
  return {
    calls,
    async call(method, payload) {
      calls.push({ method, payload });
      const handler = handlers[method];
      if (!handler) throw new Error(`unexpected ${method}`);
      return handler(payload);
    },
  };
}

const unexpectedUpload = async () => {
  throw new Error("upload not expected");
};
const unexpectedFetch = async () => {
  throw new Error("fetch not expected");
};

function makeHost({
  rpc,
  document,
  uploadChunks = unexpectedUpload,
  fetch = unexpectedFetch,
  launchEntityId = null,
  changed = [],
}) {
  return createBrokerEditorHost({
    rpc,
    fetch,
    openLink() {},
    namespace: "language",
    launchEntityId,
    uploadChunks,
    document: document ?? fakeDocument(),
    crypto: globalThis.crypto,
    onEntityChanged: (entityId) => changed.push(entityId),
  });
}

const ENTITY = { id: "lang-1", name: "Aven", deleted: false, revision: "rev-entity-1" };

function assetRecord(overrides = {}) {
  return {
    id: "a-1",
    entity_id: "lang-1",
    namespace: "language",
    filename: "map.png",
    content_hash: "sha256:00",
    size: 7,
    mime_type: "image/png",
    path: "assets/images/u-map.png",
    created_at: "2026-11-02T00:00:00Z",
    role: "attachment",
    reference_scope: "entity",
    revision: "rev-asset-1",
    ...overrides,
  };
}

// Chooser: cancel resolves null, and the dialog's click opens the chooser before any await.
{
  const doc = fakeDocument();
  const rpc = fakeRpc({});
  const host = makeHost({ rpc, document: doc });
  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  assert.equal(doc.inputs.length, 1, "the chooser input exists before the first await");
  assert.equal(doc.inputs[0].clicks, 1, "the chooser opens synchronously inside the user gesture");
  assert.equal(doc.inputs[0].type, "file");
  assert.equal(doc.body.children.length, 1, "the input is attached while the chooser is open");
  doc.inputs[0].dispatchEvent(new Event("cancel"));
  assert.equal(await pending, null, "a cancelled chooser resolves null");
  assert.equal(doc.body.children.length, 0, "the hidden input is removed after cancel");
  assert.deepEqual(rpc.calls, [], "a cancelled pick talks to the broker not at all");
}

// Successful pick: entity revision, begin, chunk upload, hash, commit, and the returned pair.
{
  const doc = fakeDocument();
  const bytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 1, 2, 3]);
  const file = new File([bytes], "map.png"); // no browser type, so the extension decides
  const asset = assetRecord();
  const uploads = [];
  const transfer = { handle: "h-1", url: "/__upload/h-1/0?token=t", maxChunkBytes: 4, expiresInMs: 60000 };
  const rpc = fakeRpc({
    "entity.get": (payload) => {
      assert.deepEqual(payload, { id: "lang-1" });
      return ENTITY;
    },
    "asset.upload.begin": () => transfer,
    "asset.upload.commit": () => asset,
  });
  const host = makeHost({
    rpc,
    document: doc,
    uploadChunks: async (begun, data) => {
      uploads.push({ transfer: begun, data: [...data] });
    },
  });

  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], file);
  const result = await pending;

  assert.deepEqual(result, { asset, filename: "map.png" });
  assert.deepEqual(
    rpc.calls.map((call) => call.method),
    ["entity.get", "asset.upload.begin", "asset.upload.commit"],
    "no cancel on success",
  );
  assert.deepEqual(rpc.calls[1].payload, {
    entityId: "lang-1",
    namespace: "language",
    filename: "map.png",
    mimeType: "image/png",
    size: 7,
    expectedRevision: "rev-entity-1",
  });
  const expectedHash = `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
  assert.deepEqual(rpc.calls[2].payload, { handle: "h-1", contentHash: expectedHash });
  assert.deepEqual(uploads, [{ transfer, data: [...bytes] }]);
  assert.equal(doc.body.children.length, 0, "the hidden input is removed after a pick");
}

// The browser's own type wins over the extension guess.
{
  const doc = fakeDocument();
  const rpc = fakeRpc({
    "entity.get": () => ENTITY,
    "asset.upload.begin": () => ({ handle: "h-t", url: "/u/0?x", maxChunkBytes: 8, expiresInMs: 1 }),
    "asset.upload.commit": (payload) =>
      assetRecord({ id: "a-t", mime_type: "text/markdown", filename: payload.handle }),
  });
  const host = makeHost({ rpc, document: doc, uploadChunks: async () => {} });
  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], new File(["# hi"], "notes.md", { type: "text/markdown" }));
  await pending;
  assert.equal(rpc.calls[1].payload.mimeType, "text/markdown");
}

// No browser type and an unknown extension: the generic type is sent, never an empty one.
{
  const doc = fakeDocument();
  const rpc = fakeRpc({
    "entity.get": () => ENTITY,
    "asset.upload.begin": () => ({ handle: "h-g", url: "/u/0?x", maxChunkBytes: 8, expiresInMs: 1 }),
    "asset.upload.commit": () => assetRecord({ id: "a-g", mime_type: "application/octet-stream" }),
  });
  const host = makeHost({ rpc, document: doc, uploadChunks: async () => {} });
  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], new File([new Uint8Array([3])], "blob.zzz"));
  await pending;
  assert.equal(rpc.calls[1].payload.mimeType, "application/octet-stream");
}

// Failure after begin cancels the transfer and rethrows the original error.
for (const failing of ["upload", "commit"]) {
  const doc = fakeDocument();
  const rpc = fakeRpc({
    "entity.get": () => ENTITY,
    "asset.upload.begin": () => ({ handle: "h-fail", url: "/u/0?x", maxChunkBytes: 8, expiresInMs: 1 }),
    "asset.upload.commit": () => {
      throw new Error("commit rejected");
    },
    "asset.transfer.cancel": () => null,
  });
  const host = makeHost({
    rpc,
    document: doc,
    uploadChunks: async () => {
      if (failing === "upload") throw new Error("network down");
    },
  });
  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], new File([new Uint8Array([9])], "a.bin"));
  await assert.rejects(pending, failing === "upload" ? /network down/ : /commit rejected/);
  const methods = rpc.calls.map((call) => call.method);
  assert.equal(methods.at(-1), "asset.transfer.cancel", `${failing} failure cancels the transfer`);
  assert.deepEqual(rpc.calls.at(-1).payload, { handle: "h-fail" });
  assert.equal(
    methods.includes("asset.upload.commit"),
    failing === "commit",
    `${failing}: commit attempted only after upload`,
  );
}

// A cancel that itself fails does not mask the upload error.
{
  const doc = fakeDocument();
  const rpc = fakeRpc({
    "entity.get": () => ENTITY,
    "asset.upload.begin": () => ({ handle: "h-x", url: "/u/0?x", maxChunkBytes: 8, expiresInMs: 1 }),
    "asset.transfer.cancel": () => {
      throw new Error("broker offline");
    },
  });
  const host = makeHost({
    rpc,
    document: doc,
    uploadChunks: async () => {
      throw new Error("chunk 0 rejected");
    },
  });
  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], new File([new Uint8Array([1])], "b.bin"));
  await assert.rejects(pending, /chunk 0 rejected/);
}

// Failure before begin never opens a transfer, so there is nothing to cancel.
{
  const doc = fakeDocument();
  const rpc = fakeRpc({ "entity.get": () => null });
  const host = makeHost({ rpc, document: doc });
  const pending = host.assets.pickAndRegisterAsset({ entityId: "gone", namespace: "language" });
  choose(doc.inputs[0], new File([new Uint8Array([1])], "c.bin"));
  await assert.rejects(pending, /could not be found/);
  assert.deepEqual(
    rpc.calls.map((call) => call.method),
    ["entity.get"],
    "no begin or cancel without a transfer",
  );
}

// Reads: own-namespace assets use asset.read.begin, everything else the shared read by path.
{
  const requested = [];
  const fetched = [];
  const rpc = fakeRpc({
    "asset.read.begin": (payload) => ({ handle: "r-own", url: `/read/own/${payload.assetId}`, mimeType: "image/png" }),
    "asset.shared.read.begin": (payload) => ({
      handle: "r-shared",
      url: `/read/shared/${payload.path}`,
      mimeType: "image/jpeg",
    }),
  });
  const host = makeHost({
    rpc,
    fetch: async (url) => {
      fetched.push(url);
      requested.push(url);
      return new Response(new Uint8Array([7, 8]), { status: 200 });
    },
  });
  assert.deepEqual([...(await host.assets.readAssetBytes(assetRecord()))], [7, 8]);
  assert.deepEqual(rpc.calls.at(-1), {
    method: "asset.read.begin",
    payload: { assetId: "a-1", namespace: "language" },
  });

  const lore = assetRecord({ id: "a-2", namespace: "lore", path: "assets/images/lore.jpg" });
  assert.deepEqual([...(await host.assets.readAssetBytes(lore))], [7, 8]);
  assert.deepEqual(rpc.calls.at(-1), {
    method: "asset.shared.read.begin",
    payload: { path: "assets/images/lore.jpg" },
  });
  assert.deepEqual(fetched, ["/read/own/a-1", "/read/shared/assets/images/lore.jpg"]);

  const failing = makeHost({
    rpc,
    fetch: async () => new Response("gone", { status: 404 }),
  });
  await assert.rejects(failing.assets.readAssetBytes(assetRecord()), /status 404/, "a failed GET rejects the read");
}

// Lists and metadata updates use the language namespace and the shared list.
{
  const rpc = fakeRpc({
    "asset.list": (payload) => {
      assert.deepEqual(payload, { entityId: "lang-1", namespace: "language" });
      return [assetRecord()];
    },
    "asset.shared.list": (payload) => {
      assert.deepEqual(payload, {});
      return [assetRecord({ reference_scope: "project" })];
    },
    "asset.update": (payload) => assetRecord({ reference_scope: payload.referenceScope, revision: "rev-asset-2" }),
  });
  const host = makeHost({ rpc });
  assert.deepEqual(await host.assets.listAssets("lang-1"), [assetRecord()]);
  assert.deepEqual(await host.assets.listSharedAssets(), [assetRecord({ reference_scope: "project" })]);
  const updated = await host.assets.updateAssetMetadata("a-1", { referenceScope: "project" }, "rev-asset-1");
  assert.equal(updated.reference_scope, "project");
  assert.deepEqual(rpc.calls.at(-1), {
    method: "asset.update",
    payload: { assetId: "a-1", namespace: "language", referenceScope: "project", expectedRevision: "rev-asset-1" },
  });
}

// Image display by path: shared first, then the launch entity's own assets.
{
  const rpc = fakeRpc({
    "asset.shared.read.begin": () => {
      throw new Error("not project scoped");
    },
    "asset.list": (payload) => {
      assert.deepEqual(payload, { entityId: "launch-1", namespace: "language" });
      return [assetRecord({ id: "a-9", path: "assets/images/own.png" })];
    },
    "asset.read.begin": (payload) => {
      assert.deepEqual(payload, { assetId: "a-9", namespace: "language" });
      return { handle: "r-9", url: "/read/own-9", mimeType: "image/png" };
    },
  });
  const host = makeHost({
    rpc,
    launchEntityId: "launch-1",
    fetch: async (url) => new Response(new Uint8Array([4, 5, 6]), { status: url === "/read/own-9" ? 200 : 404 }),
  });
  assert.deepEqual([...(await host.readAssetBytesByPath("assets/images/own.png"))], [4, 5, 6]);
  assert.deepEqual(await host.getAssetByPath("assets/images/own.png"), { mime_type: "image/png" });
  assert.equal(
    rpc.calls.filter((call) => call.method === "asset.list").length,
    1,
    "the mime lookup reuses the read that the fallback already began",
  );

  // Not shared and not in the launch entity: null, no fetch.
  assert.equal(await host.readAssetBytesByPath("assets/images/missing.png"), null);
  assert.equal(await host.getAssetByPath("assets/images/missing.png"), null);

  // No launch entity: the fallback never runs.
  const unlaunched = fakeRpc({
    "asset.shared.read.begin": () => {
      throw new Error("not project scoped");
    },
  });
  const bare = makeHost({ rpc: unlaunched });
  assert.equal(await bare.readAssetBytesByPath("assets/images/own.png"), null);
  assert.deepEqual(
    unlaunched.calls.map((call) => call.method),
    ["asset.shared.read.begin"],
    "without a launch entity there is no own-asset lookup",
  );
}

// Chooser fallback: focus with no change or cancel event resolves from whatever the input holds.
{
  const doc = fakeDocument();
  const pending = openFileChooser(doc, { focusGraceMs: 5 });
  doc.defaultView.dispatchEvent(new Event("focus"));
  assert.equal(await pending, null, "focus without a file resolves null after the grace period");
  assert.equal(doc.body.children.length, 0);
}
{
  const doc = fakeDocument();
  const pending = openFileChooser(doc, { focusGraceMs: 5 });
  doc.inputs[0].files = [new File([new Uint8Array([1])], "late.bin")];
  doc.defaultView.dispatchEvent(new Event("focus"));
  const file = await pending;
  assert.equal(file?.name, "late.bin", "a file set just before focus returns is still delivered");
}
{
  const doc = fakeDocument();
  const pending = openFileChooser(doc, { focusGraceMs: 50 });
  doc.defaultView.dispatchEvent(new Event("focus"));
  choose(doc.inputs[0], new File([new Uint8Array([2])], "fast.bin"));
  assert.equal((await pending)?.name, "fast.bin", "change settles the chooser before the grace period ends");
}

// Two picks on one entity: the second waits for the first commit, then reads the revision it left.
{
  let revision = 1;
  const doc = fakeDocument();
  const changed = [];
  let uploadStarts = 0;
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  const rpc = fakeRpc({
    "entity.get": () => ({ ...ENTITY, revision: `rev-${revision}` }),
    "asset.upload.begin": (payload) => ({
      handle: `h-${payload.filename}`,
      url: `/u/${payload.filename}/0?x`,
      maxChunkBytes: 8,
      expiresInMs: 60000,
    }),
    "asset.upload.commit": (payload) => {
      revision += 1;
      return assetRecord({ id: `a-${payload.handle}`, filename: payload.handle });
    },
  });
  const host = makeHost({
    rpc,
    document: doc,
    changed,
    uploadChunks: async () => {
      uploadStarts += 1;
      if (uploadStarts === 1) await gate;
    },
  });
  const first = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  const second = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], new File([new Uint8Array([1])], "one.bin"));
  choose(doc.inputs[1], new File([new Uint8Array([2])], "two.bin"));
  await new Promise((resolve) => setTimeout(resolve, 20));
  assert.equal(
    rpc.calls.filter((call) => call.method === "asset.upload.begin").length,
    1,
    "the second pick waits while the first upload is in flight",
  );
  release();
  const [a, b] = await Promise.all([first, second]);
  assert.equal(a.filename, "one.bin");
  assert.equal(b.filename, "two.bin");
  assert.deepEqual(
    rpc.calls.map((call) => call.method),
    [
      "entity.get",
      "asset.upload.begin",
      "asset.upload.commit",
      "entity.get",
      "asset.upload.begin",
      "asset.upload.commit",
    ],
    "uploads to one entity never interleave",
  );
  assert.deepEqual(
    rpc.calls.filter((call) => call.method === "asset.upload.begin").map((call) => call.payload.expectedRevision),
    ["rev-1", "rev-2"],
    "each begin carries a fresh revision",
  );
  assert.deepEqual(changed, ["lang-1", "lang-1"], "each committed upload announces its entity");
}

// Uploads to different entities do not wait on each other.
{
  const doc = fakeDocument();
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  const rpc = fakeRpc({
    "entity.get": (payload) => ({ ...ENTITY, id: payload.id, revision: `rev-${payload.id}` }),
    "asset.upload.begin": (payload) => ({
      handle: `h-${payload.entityId}`,
      url: "/u/0?x",
      maxChunkBytes: 8,
      expiresInMs: 1,
    }),
    "asset.upload.commit": () => assetRecord(),
  });
  const host = makeHost({
    rpc,
    document: doc,
    uploadChunks: async (transfer) => {
      if (transfer.handle === "h-lang-1") await gate;
    },
  });
  const first = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  const second = host.assets.pickAndRegisterAsset({ entityId: "lang-2", namespace: "language" });
  choose(doc.inputs[0], new File([new Uint8Array([1])], "a.bin"));
  choose(doc.inputs[1], new File([new Uint8Array([2])], "b.bin"));
  await new Promise((resolve) => setTimeout(resolve, 20));
  assert.equal(
    rpc.calls.filter((call) => call.method === "asset.upload.begin").length,
    2,
    "a second entity starts while the first is still sending",
  );
  release();
  await Promise.all([first, second]);
}

// Files over 64 MiB are refused before any broker call; the limit itself is allowed through the size check.
{
  const doc = fakeDocument();
  const changed = [];
  const rpc = fakeRpc({});
  const host = makeHost({ rpc, document: doc, changed });
  const oversized = {
    name: "film.mp4",
    size: 64 * 1024 * 1024 + 1,
    type: "video/mp4",
    arrayBuffer: async () => {
      throw new Error("the file must not be read");
    },
  };
  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], oversized);
  await assert.rejects(pending, /film\.mp4 is larger than 64 MiB/);
  assert.deepEqual(rpc.calls, [], "an oversized file never reaches the broker");
  assert.equal(doc.body.children.length, 0, "the chooser input is removed");
  assert.deepEqual(changed, [], "a refused file announces nothing");

  const limit = {
    name: "limit.bin",
    size: 64 * 1024 * 1024,
    type: "",
    arrayBuffer: async () => {
      throw new Error("reached the read");
    },
  };
  const atLimit = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[1], limit);
  await assert.rejects(atLimit, /reached the read/, "exactly 64 MiB passes the size check");
}

// Entity changes are announced after a commit and after sharing, never after a failed upload.
{
  const changed = [];
  const doc = fakeDocument();
  const rpc = fakeRpc({
    "entity.get": () => ENTITY,
    "asset.upload.begin": () => ({ handle: "h-c", url: "/u/0?x", maxChunkBytes: 8, expiresInMs: 1 }),
    "asset.upload.commit": () => assetRecord(),
    "asset.update": (payload) => assetRecord({ reference_scope: payload.referenceScope, revision: "rev-asset-2" }),
  });
  const host = makeHost({ rpc, document: doc, changed, uploadChunks: async () => {} });
  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], new File([new Uint8Array([1])], "c.bin"));
  await pending;
  assert.deepEqual(changed, ["lang-1"], "a commit announces the entity");
  await host.assets.updateAssetMetadata("a-1", { referenceScope: "project" }, "rev-asset-1");
  assert.deepEqual(changed, ["lang-1", "lang-1"], "sharing announces the asset's entity");
}
{
  const changed = [];
  const doc = fakeDocument();
  const rpc = fakeRpc({
    "entity.get": () => ENTITY,
    "asset.upload.begin": () => ({ handle: "h-f", url: "/u/0?x", maxChunkBytes: 8, expiresInMs: 1 }),
    "asset.transfer.cancel": () => null,
  });
  const host = makeHost({
    rpc,
    document: doc,
    changed,
    uploadChunks: async () => {
      throw new Error("send failed");
    },
  });
  const pending = host.assets.pickAndRegisterAsset({ entityId: "lang-1", namespace: "language" });
  choose(doc.inputs[0], new File([new Uint8Array([1])], "d.bin"));
  await assert.rejects(pending, /send failed/);
  assert.deepEqual(changed, [], "a failed upload announces nothing");
}

console.log("language asset upload tests passed");
