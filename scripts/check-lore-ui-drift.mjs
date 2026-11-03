import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-lore-ui-"));

try {
  const result = spawnSync(process.execPath, ["scripts/build-lore-ui.mjs"], {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, LORE_UI_OUT: temp },
  });
  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout || "lore UI build failed");
  }
  for (const name of ["lore.js", "lore.css"]) {
    const fresh = readFileSync(join(temp, name));
    const committed = readFileSync(resolve(root, "src-tauri/plugin-assets/lore", name));
    if (!fresh.equals(committed)) {
      throw new Error(`src-tauri/plugin-assets/lore/${name} is stale — run \`npm run build:lore-ui\``);
    }
  }
  console.log("lore plugin UI matches the Vite bundle");
} finally {
  rmSync(temp, { recursive: true, force: true });
}
