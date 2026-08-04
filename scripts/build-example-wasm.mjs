#!/usr/bin/env node

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

// (module (func (export "run") (result i32) i32.const 0))
const minimalModule = Buffer.from("0061736d010000000105016000017f030201000707010372756e00000a0601040041000b", "hex");
new WebAssembly.Module(minimalModule);
const output = resolve("examples/plugins/wasm-service/dist/service.wasm");
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, minimalModule);
console.log(`wrote ${output}`);
