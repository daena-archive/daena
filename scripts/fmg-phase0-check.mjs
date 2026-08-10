import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { join, resolve } from "node:path";

const sourceRoot = resolve(process.argv[2] ?? "");
const expectedCommit = "3430c22204f60baa412d4657ca2f9d00c270eda9";
const expectedFixtureHash = "f1ab797dcf7e2a383b0753e965342c374e86c6ce1db666ea2a4abc8122504799";

function fail(message) {
  console.error(`FMG Phase 0 check failed: ${message}`);
  process.exitCode = 1;
}
if (!process.argv[2]) {
  fail("usage: node scripts/fmg-phase0-check.mjs /path/to/pinned-fmg");
  process.exit(2);
}
if (!existsSync(sourceRoot)) fail(`source directory does not exist: ${sourceRoot}`);
if (existsSync(sourceRoot)) {
  let commit;
  try {
    commit = execFileSync("git", ["-C", sourceRoot, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  } catch {
    fail("source is not a git checkout");
  }
  if (commit && commit !== expectedCommit) fail(`expected commit ${expectedCommit}, got ${commit}`);
  const license = join(sourceRoot, "LICENSE");
  if (!existsSync(license) || !readFileSync(license, "utf8").includes("MIT License"))
    fail("FMG MIT license notice is missing");
  const fixture = join(sourceRoot, "tests/fixtures/demo.map");
  if (!existsSync(fixture)) fail("representative demo.map fixture is missing");
  if (existsSync(fixture)) {
    const bytes = readFileSync(fixture);
    const hash = createHash("sha256").update(bytes).digest("hex");
    if (hash !== expectedFixtureHash) fail(`fixture hash mismatch: expected ${expectedFixtureHash}, got ${hash}`);
    if (bytes.length < 1024 * 1024) fail("demo.map is not a representative source fixture");
    console.log(JSON.stringify({ commit, fixtureBytes: bytes.length, fixtureSha256: hash }));
  }
  const requiredStaticPaths = ["dist", "public/modules/io/load.js", "public/modules/io/save.js"];
  for (const path of requiredStaticPaths)
    if (!existsSync(join(sourceRoot, path))) fail(`required FMG path is missing: ${path}`);
}
if (process.exitCode) process.exit();
console.log("FMG Phase 0 source, license, fixture, and static-tree checks passed");
