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

function bootstrapValue(body, sessionId) {
  return {
    rpcVersion: 1,
    pluginId: body.pluginId,
    sessionId,
    projectId: body.projectId,
    version: "1.0.0",
    hostApi: ">=1.0.0 <2.0.0",
    grantedCapabilities: ["entity.read"],
    optionalFeatures: [],
  };
}

function rpcResult(requestId, result) {
  return { rpcVersion: 1, requestId, ok: true, result };
}

function rpcError(requestId, code) {
  return {
    rpcVersion: 1,
    requestId,
    ok: false,
    error: { code, message: code, retryable: false },
  };
}

function createHarness(decide) {
  const calls = [];
  let bootstrapCount = 0;
  const fetch = async (_endpoint, init) => {
    const body = JSON.parse(init.body);
    calls.push(body);
    if (body.op === "bootstrap") {
      bootstrapCount += 1;
      return response(200, bootstrapValue(body, `session-${bootstrapCount}`));
    }
    return response(200, await decide(body));
  };
  const client = createPluginRpcClient(
    createBrowserPluginRpcTransport({
      pluginId: "com.example.plugin",
      projectId: "project-1",
      fetch,
    }),
  );
  return {
    client,
    calls,
    get bootstrapCount() {
      return bootstrapCount;
    },
  };
}

async function renews(code) {
  let attempts = 0;
  const harness = createHarness(async (body) => {
    attempts += 1;
    const requestId = body.request.requestId;
    if (attempts === 1) return rpcError(requestId, code);
    return rpcResult(requestId, { renewed: true });
  });
  assert.deepEqual(await harness.client.call("entity.get", { id: "1" }), { renewed: true });
  assert.equal(harness.bootstrapCount, 2, `${code} bootstraps once more`);
  const rpcs = harness.calls.filter((call) => call.op === "rpc");
  assert.equal(rpcs.length, 2);
  assert.equal(rpcs[0].request.sessionId, "session-1");
  assert.equal(rpcs[1].request.sessionId, "session-2");
  assert.equal(rpcs[0].request.requestId, rpcs[1].request.requestId);
  assert.equal(attempts, 2);
}

await renews("session.expired");
await renews("session.revoked");
await renews("session.stale");

for (const code of ["session.invalid", "session.origin", "capability.denied"]) {
  let attempts = 0;
  const harness = createHarness(async (body) => {
    attempts += 1;
    return rpcError(body.request.requestId, code);
  });
  await assert.rejects(
    harness.client.call("entity.get", {}),
    (error) => error instanceof PluginRpcException && error.code === code,
  );
  assert.equal(harness.bootstrapCount, 1, `${code} does not bootstrap again`);
  assert.equal(attempts, 1, `${code} is not retried`);
}

{
  let attempts = 0;
  const harness = createHarness(async (body) => {
    attempts += 1;
    const code = attempts === 1 ? "session.expired" : "capability.denied";
    return rpcError(body.request.requestId, code);
  });
  await assert.rejects(
    harness.client.call("entity.get", {}),
    (error) => error instanceof PluginRpcException && error.code === "capability.denied",
  );
  assert.equal(harness.bootstrapCount, 2, "a failed retry bootstraps once");
  assert.equal(attempts, 2, "a failed retry is not tried again");
}

{
  let waiting = 0;
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  let opened;
  const bothWaiting = new Promise((resolve) => {
    opened = resolve;
  });
  const harness = createHarness(async (body) => {
    const requestId = body.request.requestId;
    if (body.request.sessionId === "session-1") {
      waiting += 1;
      if (waiting === 2) opened();
      await gate;
      return rpcError(requestId, "session.expired");
    }
    return rpcResult(requestId, { sessionId: body.request.sessionId });
  });
  const pending = Promise.all([
    harness.client.call("entity.get", { id: "a" }),
    harness.client.call("entity.get", { id: "b" }),
  ]);
  await bothWaiting;
  release();
  assert.deepEqual(await pending, [{ sessionId: "session-2" }, { sessionId: "session-2" }]);
  assert.equal(harness.bootstrapCount, 2, "concurrent expiry shares one renewal");
  const retries = harness.calls.filter((call) => call.op === "rpc" && call.request.sessionId === "session-2");
  assert.equal(retries.length, 2);
}

{
  let releaseSlow;
  const slowGate = new Promise((resolve) => {
    releaseSlow = resolve;
  });
  let markSlow;
  const slowEntered = new Promise((resolve) => {
    markSlow = resolve;
  });
  let slowCount = 0;
  const harness = createHarness(async (body) => {
    const requestId = body.request.requestId;
    if (body.request.method === "slow") {
      slowCount += 1;
      if (slowCount === 1) {
        markSlow();
        await slowGate;
        return rpcError(requestId, "session.expired");
      }
      return rpcResult(requestId, { sessionId: body.request.sessionId });
    }
    if (body.request.sessionId === "session-1") return rpcError(requestId, "session.revoked");
    return rpcResult(requestId, { sessionId: body.request.sessionId });
  });
  const slow = harness.client.call("slow", {});
  await slowEntered;
  assert.deepEqual(await harness.client.call("fast", {}), { sessionId: "session-2" });
  assert.equal(harness.bootstrapCount, 2);
  releaseSlow();
  assert.deepEqual(await slow, { sessionId: "session-2" });
  assert.equal(harness.bootstrapCount, 2, "a late failure reuses the renewed session");
}

{
  let posts = 0;
  const client = createPluginRpcClient(
    createBrowserPluginRpcTransport({
      pluginId: "com.example.plugin",
      projectId: "project-1",
      fetch: async () => {
        posts += 1;
        return response(200, { rpcVersion: 1 });
      },
    }),
  );
  await assert.rejects(client.bootstrap(), /plugin bootstrap response is invalid/);
  assert.equal(posts, 1, "an explicit bootstrap is not retried");
}
