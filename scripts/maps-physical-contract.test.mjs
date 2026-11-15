import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { physicalContract } from "../src/lib/project/physical-contract.ts";

const root = resolve(import.meta.dirname, "..");
const fixture = JSON.parse(readFileSync(resolve(root, "src/lib/project/physical-contract.fixture.json"), "utf8"));

function shape(value) {
  if (Array.isArray(value)) {
    return { kind: "array", items: value.length === 0 ? { kind: "any" } : shape(value[0]) };
  }
  if (value && typeof value === "object") {
    const fields = {};
    for (const key of Object.keys(value).sort()) fields[key] = shape(value[key]);
    return { kind: "object", fields };
  }
  if (value === null) return { kind: "null" };
  if (typeof value === "number") return { kind: "number" };
  if (typeof value === "boolean") return { kind: "boolean" };
  if (typeof value === "string") return { kind: "string" };
  throw new Error(`unsupported contract mirror value: ${typeof value}`);
}

for (const [name, expected] of Object.entries(fixture)) {
  const actual = shape(physicalContract[name]);
  assert.deepEqual(
    actual,
    expected,
    `${name} in src/lib/project/types.ts does not match the Rust JSON; regenerate the fixture only when the Rust shape is intentional`,
  );
}

console.log(`physical contract fixture matches ${Object.keys(fixture).length} Rust JSON shapes`);
