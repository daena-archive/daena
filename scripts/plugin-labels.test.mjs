#!/usr/bin/env node

import assert from "node:assert/strict";
import { capabilityDescription, capabilityLabel } from "../src/lib/plugins/labels.ts";

assert.equal(capabilityLabel("entity.read"), "Read entities");
assert.equal(capabilityLabel("schema.overlay"), "Customize types and fields");
assert.equal(capabilityLabel("ai.text.generate-structured"), "Request structured text assistance");
assert.equal(capabilityLabel("record.write:self"), "Create and edit own records");
assert.equal(
  capabilityLabel("service.call:daena.timeline.resolve-date"),
  "Call the daena.timeline.resolve-date service",
);
assert.equal(capabilityLabel("host.surface:daena.maps/editor@1"), "Use the daena.maps/editor@1 host surface");
assert.equal(
  capabilityLabel("event.publish:com.example.weather/forecast-updated@1"),
  "Publish com.example.weather/forecast-updated@1 events",
);
assert.equal(capabilityLabel("network:https://example.com"), "Make HTTPS requests to https://example.com");
assert.equal(capabilityLabel("clipboard.write"), "Write to the clipboard");
assert.equal(capabilityDescription("entity.delete").includes("confirm"), true);
assert.equal(capabilityDescription("ai.text.generate").includes("network"), true);
assert.notEqual(capabilityLabel("unknown.capability"), "");
assert.equal(capabilityLabel("unknown.capability"), "unknown.capability");

console.log("plugin capability labels checks passed");
