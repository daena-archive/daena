#!/usr/bin/env node

import assert from "node:assert/strict";
import {
  createBrowserPluginRpcTransport,
  createPluginRpcClient,
  PluginRpcException,
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

console.log("plugin browser transport tests passed");
