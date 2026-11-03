#!/usr/bin/env node

import assert from "node:assert/strict";
import {
  createBrowserPluginRpcTransport,
  createPluginRpcClient,
  MAX_TEXT_TRANSFER_BYTES,
  PluginRpcException,
  transferText,
} from "../packages/plugin-sdk/dist/index.js";

function response(status, value) {
  return { ok: status >= 200 && status < 300, status, text: async () => JSON.stringify(value) };
}

function fakeFetch() {
  const calls = [];
  let bootstrapCount = 0;
  const fetch = async (_endpoint, init) => {
    const body = JSON.parse(init.body);
    calls.push(body);
    if (body.op === "bootstrap") {
      bootstrapCount += 1;
      return response(200, {
        rpcVersion: 1,
        pluginId: body.pluginId,
        sessionId: `session-${bootstrapCount}`,
        projectId: body.projectId,
        version: "1.0.0",
        hostApi: ">=1.0.0 <2.0.0",
        grantedCapabilities: ["entity.read"],
        optionalFeatures: [],
      });
    }
    if (body.request.method === "entity.list") {
      return response(200, {
        rpcVersion: 1,
        requestId: body.request.requestId,
        ok: true,
        result: [
          {
            id: "entity-1",
            name: "Entity 1",
            entity_type: "place",
            deleted: false,
            created_at: "2026-01-01T00:00:00Z",
            updated_at: "2026-01-01T00:00:00Z",
            revision: "revision-1",
          },
        ],
      });
    }
    if (body.request.method === "entity.query") {
      return response(200, {
        rpcVersion: 1,
        requestId: body.request.requestId,
        ok: true,
        result: {
          items: [
            {
              id: "entity-1",
              name: "Entity 1",
              entityType: "place",
              deleted: false,
              createdAt: "2026-01-01T00:00:00Z",
              updatedAt: "2026-01-01T00:00:00Z",
              revision: "revision-1",
            },
          ],
          total: 1,
          offset: 0,
          limit: 25,
          hasMore: false,
          typeCounts: [{ entityType: "place", count: 1 }],
        },
      });
    }
    return response(200, {
      rpcVersion: 1,
      requestId: body.request.requestId,
      ok: false,
      error: { code: "capability.denied", message: "not granted", retryable: false },
    });
  };
  return {
    fetch,
    calls,
    get bootstrapCount() {
      return bootstrapCount;
    },
  };
}

const first = fakeFetch();
const client = createPluginRpcClient(
  createBrowserPluginRpcTransport({
    pluginId: "com.example.plugin",
    projectId: "project-1",
    fetch: first.fetch,
  }),
);
const bootstrap = await client.bootstrap();
assert.equal(bootstrap.sessionId, "session-1");
assert.deepEqual(await client.listEntities(), [
  {
    id: "entity-1",
    name: "Entity 1",
    entityType: "place",
    deleted: false,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    revision: "revision-1",
  },
]);
assert.deepEqual(await client.queryEntities({ query: "Entity", limit: 25 }), {
  items: [
    {
      id: "entity-1",
      name: "Entity 1",
      entityType: "place",
      deleted: false,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
      revision: "revision-1",
    },
  ],
  total: 1,
  offset: 0,
  limit: 25,
  hasMore: false,
  typeCounts: [{ entityType: "place", count: 1 }],
});
assert.equal(first.bootstrapCount, 1);
assert.equal(first.calls[1].request.sessionId, "session-1");
const requestId = first.calls[1].request.requestId;
assert.ok(
  requestId === "com.example.plugin-1" ||
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(requestId),
  "requestId is opaque: pluginId counter or UUID",
);

await assert.rejects(
  client.call("entity.write", {}),
  (error) => error instanceof PluginRpcException && error.code === "capability.denied",
);

const concurrent = fakeFetch();
const concurrentClient = createPluginRpcClient(
  createBrowserPluginRpcTransport({
    pluginId: "com.example.plugin",
    projectId: "project-1",
    fetch: concurrent.fetch,
  }),
);
await Promise.all([concurrentClient.listEntities(), concurrentClient.listEntities()]);
assert.equal(concurrent.bootstrapCount, 1, "concurrent calls share one handshake");

const mismatch = createPluginRpcClient(
  createBrowserPluginRpcTransport({
    pluginId: "com.example.plugin",
    projectId: "project-1",
    fetch: async () =>
      response(200, {
        rpcVersion: 1,
        requestId: "wrong",
        ok: true,
        result: [],
      }),
  }),
);
await assert.rejects(mismatch.listEntities(), /plugin bootstrap response is invalid/);

// Text transfers: the client sends the documented method names and payloads, and transferText runs the whole
// begin, PUT, commit sequence, cancelling a transfer that fails.
{
  const rpcCalls = [];
  const puts = [];
  let putStatus = 200;
  let commitFails = false;
  const textFetch = async (_endpoint, init) => {
    const body = JSON.parse(init.body);
    if (body.op === "bootstrap") {
      return response(200, {
        rpcVersion: 1,
        pluginId: body.pluginId,
        sessionId: "session-text",
        projectId: body.projectId,
        version: "1.0.0",
        hostApi: ">=1.0.0 <2.0.0",
        grantedCapabilities: ["text.transfer"],
        optionalFeatures: [],
      });
    }
    const { method, payload, requestId } = body.request;
    rpcCalls.push({ method, payload });
    const reply = (result) => response(200, { rpcVersion: 1, requestId, ok: true, result });
    if (method === "text.transfer.begin") {
      return reply({ transferId: "tx-1", url: "/__asset/tx-1/0?sessionId=session-text", maxChunkBytes: 4, expiresInMs: 60000 });
    }
    if (method === "text.transfer.commit") {
      if (commitFails) {
        return response(200, {
          rpcVersion: 1,
          requestId,
          ok: false,
          error: { code: "core.error", message: "text transfer is not valid UTF-8", retryable: false },
        });
      }
      return reply({ transferId: payload.transferId, size: 10 });
    }
    if (method === "asset.transfer.cancel") return reply(null);
    throw new Error(`unexpected ${method}`);
  };
  const client = createPluginRpcClient(
    createBrowserPluginRpcTransport({ pluginId: "daena.writing", projectId: "project-1", fetch: textFetch }),
  );
  const putFetch = async (url, init) => {
    puts.push({ url, body: init.body });
    return { ok: putStatus === 200, status: putStatus };
  };

  assert.deepEqual(await client.beginTextTransfer(10), {
    transferId: "tx-1",
    url: "/__asset/tx-1/0?sessionId=session-text",
    maxChunkBytes: 4,
    expiresInMs: 60000,
  });
  assert.deepEqual(await client.commitTextTransfer("tx-1"), { transferId: "tx-1", size: 10 });
  assert.deepEqual(rpcCalls, [
    { method: "text.transfer.begin", payload: { size: 10 } },
    { method: "text.transfer.commit", payload: { transferId: "tx-1" } },
  ]);

  // "héllo ✓" is 10 bytes; the host's chunk limit splits it, even inside a character.
  rpcCalls.length = 0;
  assert.equal(await transferText(client, "héllo ✓", putFetch), "tx-1");
  assert.deepEqual(
    rpcCalls.map((call) => call.method),
    ["text.transfer.begin", "text.transfer.commit"],
  );
  assert.deepEqual(rpcCalls[0].payload, { size: 10 }, "the size is the UTF-8 byte length");
  assert.deepEqual(
    puts.map((put) => put.url),
    [
      "/__asset/tx-1/0?sessionId=session-text",
      "/__asset/tx-1/1?sessionId=session-text",
      "/__asset/tx-1/2?sessionId=session-text",
    ],
    "chunks are PUT in order to the begun URL",
  );
  assert.equal(
    new TextDecoder().decode(Buffer.concat(puts.map((put) => put.body))),
    "héllo ✓",
    "the chunks add up to the text",
  );

  // A failed chunk or commit cancels the transfer and rethrows.
  rpcCalls.length = 0;
  putStatus = 409;
  await assert.rejects(transferText(client, "héllo ✓", putFetch), /asset upload failed: 409/);
  assert.deepEqual(
    rpcCalls.map((call) => call.method),
    ["text.transfer.begin", "asset.transfer.cancel"],
  );
  assert.deepEqual(rpcCalls[1].payload, { handle: "tx-1" });
  putStatus = 200;
  rpcCalls.length = 0;
  commitFails = true;
  await assert.rejects(transferText(client, "héllo ✓", putFetch), /not valid UTF-8/);
  assert.deepEqual(
    rpcCalls.map((call) => call.method),
    ["text.transfer.begin", "text.transfer.commit", "asset.transfer.cancel"],
  );

  // Empty and oversized text never reach the host.
  rpcCalls.length = 0;
  await assert.rejects(transferText(client, "", putFetch), /needs text/);
  await assert.rejects(transferText(client, "a".repeat(MAX_TEXT_TRANSFER_BYTES + 1), putFetch), /exceeds SDK transfer limit/);
  assert.deepEqual(rpcCalls, []);
  assert.equal(MAX_TEXT_TRANSFER_BYTES, 32 * 1024 * 1024);
}

console.log("plugin browser transport tests passed");
