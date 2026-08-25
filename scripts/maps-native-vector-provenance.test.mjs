import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

const core = read("crates/daena-core/src/maps/vector.rs");
assert.match(core, /generation\.version[\s\S]{0,120}must be 1/);
assert.match(core, /Some\(1\)/);
assert.match(core, /daena-landmass/);

const sdk = read("packages/plugin-sdk/src/maps.ts");
assert.match(sdk, /version: 1;/);
assert.doesNotMatch(sdk, /version: 2 \| 3/);
assert.match(sdk, /daena-landmass/);

const generatedSdk = read("packages/plugin-sdk/dist/maps.d.ts");
assert.match(generatedSdk, /version: 1;/);
assert.doesNotMatch(generatedSdk, /version: 2 \| 3/);

const tauriTests = read("src-tauri/src/tests.rs");
const vectorCreate = tauriTests.slice(tauriTests.indexOf("fn maps_vector_create_rpc_round_trips"));
assert.match(vectorCreate, /"version": 1/);
assert.doesNotMatch(vectorCreate, /"version": 2/);

console.log("native vector provenance declarations agree on generator version 1");
