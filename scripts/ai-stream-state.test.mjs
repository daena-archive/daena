import assert from "node:assert/strict";
import { reduceTextGenerationEvent } from "../src/lib/ai/stream.ts";

function event(phase, overrides = {}) {
  return {
    sequence: 0,
    requestId: "request",
    phase,
    delta: null,
    output: null,
    error: null,
    ...overrides,
  };
}

const initial = {
  streamText: "",
  proposal: "",
  progressMessage: "Preparing model…",
};

const reasoning = reduceTextGenerationEvent(initial, event("reasoning"));
assert.equal(reasoning.progressMessage, "Model is thinking…");
assert.equal(reasoning.streamText, "");
assert.equal(reasoning.proposal, "");

const partial = reduceTextGenerationEvent(reasoning, event("delta", { delta: "Partial proposal" }));
assert.equal(partial.progressMessage, "Writing proposal…");
assert.equal(partial.streamText, "Partial proposal");

const timedOut = reduceTextGenerationEvent(partial, event("deadline_exceeded", { error: "DeadlineExceeded" }));
assert.equal(timedOut.proposal, "Partial proposal");
assert.equal(timedOut.progressMessage, "");

const completed = reduceTextGenerationEvent(
  { ...partial, streamText: "Complete streamed proposal" },
  event("completed", { output: "Short" }),
);
assert.equal(completed.proposal, "Complete streamed proposal");

console.log("AI stream state checks passed");
