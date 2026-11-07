import assert from "node:assert/strict";

import { tabDropIndex } from "../src/lib/shell/tab-drag.ts";

// Centres sit at 50, 150, 250, and 350.
const rects = [
  { left: 0, width: 100 },
  { left: 100, width: 100 },
  { left: 200, width: 100 },
  { left: 300, width: 100 },
];

assert.equal(tabDropIndex(rects, 40, 3), 0, "moving left, past the first centre, lands at the start");
assert.equal(tabDropIndex(rects, 100, 2), 1, "moving left lands before the first centre to the right of the pointer");
assert.equal(tabDropIndex(rects, 250, 0), 2, "moving right lands before the first centre to the right of the pointer");
assert.equal(tabDropIndex(rects, 150, 1), 1, "a pointer on the dragged tab's centre keeps its index");
assert.equal(tabDropIndex(rects, -20, 2), 0, "a pointer past the left end clamps to the start");
assert.equal(tabDropIndex(rects, 10_000, 0), 3, "a pointer past the right end clamps to the last index");
assert.equal(tabDropIndex(rects, 10_000, 3), 3, "dragging the last tab past the end stays on itself");
assert.equal(tabDropIndex([{ left: 0, width: 80 }], 10, 0), 0, "a single tab stays at 0");
assert.equal(tabDropIndex([{ left: 0, width: 80 }], 10_000, 0), 0, "a single tab stays at 0 past its end");
assert.equal(tabDropIndex([], 0, 0), 0, "no tabs yields 0");

console.log("tab drag checks passed");
