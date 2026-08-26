import assert from "node:assert/strict";
import { createAtlasRenderCompletionTracker } from "../src/lib/maps/atlas/render-completion.ts";

const tracker = createAtlasRenderCompletionTracker();
const completions = [];
let renderCount = 0;

tracker.watch(
  (complete) => completions.push(complete),
  () => {
    renderCount += 1;
  },
  () => true,
  () => completions.push("first-ready"),
);
tracker.watch(
  (complete) => completions.push(complete),
  () => {
    renderCount += 1;
  },
  () => true,
  () => completions.push("latest-ready"),
);

assert.equal(renderCount, 2, "each Atlas update explicitly requests a render");
completions[0]();
assert.equal(completions.includes("first-ready"), false, "a superseded render cannot clear the current busy state");
completions[1]();
assert.equal(completions.includes("latest-ready"), true, "the latest render clears the Atlas busy state");

console.log("Atlas refresh completion guard checks passed");
