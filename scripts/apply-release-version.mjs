import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z.-]+))?(?:\+([0-9A-Za-z.-]+))?$/;

export function normalizeReleaseVersion(raw) {
  const trimmed = String(raw ?? "")
    .trim()
    .replace(/^[vV]/, "");
  if (!SEMVER.test(trimmed)) {
    throw new Error(`Invalid release version '${raw}'. Expected semver, optionally prefixed with v.`);
  }
  return trimmed;
}

export function versionCore(version) {
  return normalizeReleaseVersion(version).split(/[-+]/, 1)[0];
}

export function cargoPackageVersion(toml) {
  let inPackage = false;
  for (const line of toml.split(/\r?\n/)) {
    if (/^\[package\]\s*$/.test(line)) {
      inPackage = true;
      continue;
    }
    if (inPackage && /^\s*\[/.test(line)) {
      break;
    }
    const match = inPackage ? line.match(/^version\s*=\s*"([^"]+)"/) : null;
    if (match) return match[1];
  }
  throw new Error("Could not read [package].version from Cargo.toml.");
}

export function setCargoPackageVersion(toml, version) {
  const eol = toml.includes("\r\n") ? "\r\n" : "\n";
  let inPackage = false;
  const lines = toml.split(/\r?\n/);
  let replaced = false;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^\[package\]\s*$/.test(line)) {
      inPackage = true;
      continue;
    }
    if (inPackage && /^\s*\[/.test(line) && !/^\[package\]/.test(line)) {
      break;
    }
    if (inPackage && /^version\s*=/.test(line)) {
      lines[i] = `version = "${version}"`;
      replaced = true;
      break;
    }
  }
  if (!replaced) {
    throw new Error("Could not update [package].version in Cargo.toml.");
  }
  return lines.join(eol);
}

export function setCargoLockPackageVersion(lockfile, name, version) {
  const eol = lockfile.includes("\r\n") ? "\r\n" : "\n";
  const lines = lockfile.split(/\r?\n/);
  for (let i = 0; i < lines.length; i += 1) {
    if (lines[i] === `name = "${name}"` && lines[i + 1]?.startsWith("version = ")) {
      lines[i + 1] = `version = "${version}"`;
      return lines.join(eol);
    }
  }
  throw new Error(`Could not update package '${name}' version in Cargo.lock.`);
}

export function readManifestVersions(root) {
  const packageJson = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
  const tauri = JSON.parse(fs.readFileSync(path.join(root, "src-tauri/tauri.conf.json"), "utf8"));
  const cargo = cargoPackageVersion(fs.readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8"));
  return {
    package: String(packageJson.version),
    tauri: String(tauri.version),
    cargo,
  };
}

export function assertTagMatchesManifestCores(tag, versions) {
  const expectedCore = versionCore(tag);
  for (const [name, value] of Object.entries(versions)) {
    const actualCore = versionCore(value);
    if (actualCore !== expectedCore) {
      throw new Error(
        `Tag '${tag}' core '${expectedCore}' does not match ${name} version '${value}' core '${actualCore}'.`,
      );
    }
  }
}

export function applyReleaseVersion(root, rawVersion) {
  const version = normalizeReleaseVersion(rawVersion);
  const versions = readManifestVersions(root);
  assertTagMatchesManifestCores(version, versions);

  const packagePath = path.join(root, "package.json");
  const tauriPath = path.join(root, "src-tauri/tauri.conf.json");
  const cargoPath = path.join(root, "src-tauri/Cargo.toml");
  const lockPath = path.join(root, "src-tauri/Cargo.lock");

  const packageJson = JSON.parse(fs.readFileSync(packagePath, "utf8"));
  packageJson.version = version;
  fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

  const tauri = JSON.parse(fs.readFileSync(tauriPath, "utf8"));
  tauri.version = version;
  fs.writeFileSync(tauriPath, `${JSON.stringify(tauri, null, 2)}\n`);

  fs.writeFileSync(cargoPath, setCargoPackageVersion(fs.readFileSync(cargoPath, "utf8"), version));
  fs.writeFileSync(lockPath, setCargoLockPackageVersion(fs.readFileSync(lockPath, "utf8"), "daena", version));
  return version;
}

function usage() {
  return "Usage: deno run --allow-read --allow-write --allow-env scripts/apply-release-version.mjs <check|apply> <v0.1.0-alpha.2>";
}

function main(argv) {
  const [, , command, tag] = argv;
  if ((command !== "check" && command !== "apply") || !tag) {
    throw new Error(usage());
  }
  const root = process.cwd();
  const versions = readManifestVersions(root);
  assertTagMatchesManifestCores(tag, versions);
  if (command === "apply") {
    const version = applyReleaseVersion(root, tag);
    process.stdout.write(`${version}\n`);
  }
}

const isMain =
  import.meta.main === true ||
  (Boolean(process.argv[1]) && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url));
if (isMain) {
  try {
    main(process.argv);
  } catch (cause) {
    console.error(cause instanceof Error ? cause.message : cause);
    process.exit(1);
  }
}
