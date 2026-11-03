import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-language-ui-"));

try {
  const result = spawnSync(process.execPath, ["scripts/build-language-ui.mjs"], {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, LANGUAGE_UI_OUT: temp },
  });
  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout || "language UI build failed");
  }
  for (const name of ["language.js", "language.css"]) {
    const fresh = readFileSync(join(temp, name));
    const committed = readFileSync(resolve(root, "src-tauri/plugin-assets/language", name));
    if (!fresh.equals(committed)) {
      throw new Error(`src-tauri/plugin-assets/language/${name} is stale — run \`npm run build:language-ui\``);
    }
  }
  console.log("language plugin UI matches the Vite bundle");
} finally {
  rmSync(temp, { recursive: true, force: true });
}
