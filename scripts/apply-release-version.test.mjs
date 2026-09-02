import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  applyReleaseVersion,
  assertTagMatchesManifestCores,
  cargoPackageVersion,
  normalizeReleaseVersion,
  setCargoLockPackageVersion,
  setCargoPackageVersion,
  versionCore,
} from "./apply-release-version.mjs";

assert.equal(normalizeReleaseVersion("v0.1.0-alpha.2"), "0.1.0-alpha.2");
assert.equal(versionCore("v0.1.0-alpha.2"), "0.1.0");

assert.equal(cargoPackageVersion('[package]\nname = "daena"\nversion = "0.1.0"\n\n[lib]\n'), "0.1.0");
assert.match(
  setCargoPackageVersion('[package]\nversion = "0.1.0"\n\n[lib]\n', "0.1.0-alpha.2"),
  /version = "0.1.0-alpha.2"/,
);
assert.match(
  setCargoLockPackageVersion('[[package]]\nname = "daena"\nversion = "0.1.0"\n', "daena", "0.1.0-alpha.2"),
  /version = "0.1.0-alpha.2"/,
);

assert.throws(() =>
  assertTagMatchesManifestCores("v0.2.0-alpha.1", {
    package: "0.1.0",
    tauri: "0.1.0",
    cargo: "0.1.0",
  }),
);

const root = fs.mkdtempSync(path.join(os.tmpdir(), "daena-version-"));
fs.mkdirSync(path.join(root, "src-tauri"));
fs.writeFileSync(path.join(root, "package.json"), `${JSON.stringify({ name: "daena", version: "0.1.0" }, null, 2)}\n`);
fs.writeFileSync(
  path.join(root, "src-tauri/tauri.conf.json"),
  `${JSON.stringify({ productName: "Daena", version: "0.1.0" }, null, 2)}\n`,
);
fs.writeFileSync(path.join(root, "src-tauri/Cargo.toml"), '[package]\nname = "daena"\nversion = "0.1.0"\n');
fs.writeFileSync(path.join(root, "src-tauri/Cargo.lock"), '[[package]]\nname = "daena"\nversion = "0.1.0"\n');

assert.equal(applyReleaseVersion(root, "v0.1.0-alpha.2"), "0.1.0-alpha.2");
assert.equal(JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version, "0.1.0-alpha.2");
assert.equal(
  JSON.parse(fs.readFileSync(path.join(root, "src-tauri/tauri.conf.json"), "utf8")).version,
  "0.1.0-alpha.2",
);
assert.equal(cargoPackageVersion(fs.readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8")), "0.1.0-alpha.2");
fs.rmSync(root, { recursive: true, force: true });

console.log("apply release version checks passed");
