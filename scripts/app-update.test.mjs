import assert from "node:assert/strict";
import {
  formatUpdateMessage,
  normalizeUpdateChannelPreference,
  readUpdateChannelPreference,
} from "../src/lib/appUpdate.ts";

assert.equal(normalizeUpdateChannelPreference("alpha"), "alpha");
assert.equal(normalizeUpdateChannelPreference("stable"), "stable");
assert.equal(normalizeUpdateChannelPreference("nope"), "auto");

const storage = new Map();
readUpdateChannelPreference({
  getItem: (key) => storage.get(key) ?? null,
});
storage.set("daena-update-channel", "beta");
assert.equal(
  readUpdateChannelPreference({
    getItem: (key) => storage.get(key) ?? null,
  }),
  "beta",
);

assert.equal(
  formatUpdateMessage({
    current: "0.1.0",
    latest: "v0.1.0-alpha.2",
    newer: true,
    htmlUrl: "https://example.com",
    releaseChannel: "alpha",
    latestPrerelease: true,
    updateChannelPreference: "alpha",
  }),
  "Update available: v0.1.0-alpha.2 (alpha)",
);

assert.equal(
  formatUpdateMessage({
    current: "0.1.0-alpha.1",
    latest: "v0.1.0-alpha.1",
    newer: false,
    htmlUrl: "https://example.com",
    releaseChannel: "alpha",
    latestPrerelease: true,
    updateChannelPreference: "auto",
  }),
  "You're up to date (0.1.0-alpha.1, alpha channel)",
);

console.log("app update checks passed");
