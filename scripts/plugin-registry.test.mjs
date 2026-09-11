#!/usr/bin/env node

import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { compileIndex } from "./plugin-registry-compile.mjs";

const emptyRoot = mkdtempSync(join(tmpdir(), "daena-plugin-registry-empty-"));
const root = mkdtempSync(join(tmpdir(), "daena-plugin-registry-"));
try {
  mkdirSync(join(emptyRoot, "publishers"));
  const empty = compileIndex(emptyRoot);
  assert.equal(empty.trust.schemaVersion, 1);
  assert.deepEqual(empty.trust.publishers, {});
  assert.deepEqual(empty.catalog.plugins, []);

  mkdirSync(join(root, "publishers", "com.example"), { recursive: true });
  writeFileSync(
    join(root, "publishers", "com.example", "publisher.json"),
    `${JSON.stringify({
      schemaVersion: 1,
      id: "com.example",
      keys: [{ keyId: "2026-09", publicKey: Buffer.alloc(32, 7).toString("base64") }],
      sourceRepo: "https://github.com/example/daena-plugins",
      plugins: ["com.example", "com.example.genealogy"],
    })}\n`,
  );
  writeFileSync(
    join(root, "revocations.json"),
    `${JSON.stringify({ schemaVersion: 1, keys: [], digests: [], packages: [] })}\n`,
  );
  const compiled = compileIndex(root);
  assert.equal(compiled.trust.publishers["com.example"].keys[0].keyId, "2026-09");
  assert.equal("sourceRepo" in compiled.trust.publishers["com.example"], false);

  const bad = join(root, "publishers", "com.example", "publisher.json");
  writeFileSync(
    bad,
    `${JSON.stringify({
      schemaVersion: 1,
      id: "com.example",
      keys: [{ keyId: "2026-09", publicKey: Buffer.alloc(32, 7).toString("base64") }],
      sourceRepo: "https://github.com/example/daena-plugins",
      plugins: ["com.other.plugin"],
      extra: true,
    })}\n`,
  );
  assert.throws(() => compileIndex(root), /unknown field extra|must be com.example/);

  writeFileSync(
    bad,
    `${JSON.stringify({
      schemaVersion: 1,
      id: "com.example",
      keys: [{ keyId: "2026-09", publicKey: Buffer.alloc(32, 7).toString("base64") }],
      sourceRepo: "https://github.com/example/daena-plugins",
      plugins: ["com.other.plugin"],
    })}\n`,
  );
  assert.throws(() => compileIndex(root), /com.example\.\*/);

  writeFileSync(
    join(root, "revocations.json"),
    `${JSON.stringify({
      schemaVersion: 1,
      keys: [{ publisher: "com.example", keyId: "old" }],
      digests: [],
      packages: [],
    })}\n`,
  );
  writeFileSync(
    bad,
    `${JSON.stringify({
      schemaVersion: 1,
      id: "com.example",
      keys: [{ keyId: "2026-09", publicKey: Buffer.alloc(32, 7).toString("base64") }],
      sourceRepo: "https://github.com/example/daena-plugins",
      plugins: ["com.example.genealogy"],
    })}\n`,
  );
  assert.throws(() => compileIndex(root), /publicKey/);
  console.log("plugin registry compile checks passed");
} finally {
  rmSync(emptyRoot, { recursive: true, force: true });
  rmSync(root, { recursive: true, force: true });
}
