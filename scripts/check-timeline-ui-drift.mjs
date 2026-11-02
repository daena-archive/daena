import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const temp = mkdtempSync(join(tmpdir(), "daena-timeline-ui-"));

try {
  const result = spawnSync(process.execPath, ["scripts/build-timeline-ui.mjs"], {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, TIMELINE_UI_OUT: temp },
  });
  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout || "timeline UI build failed");
  }
  for (const name of ["timeline.js", "timeline.css"]) {
    const fresh = readFileSync(join(temp, name));
    const committed = readFileSync(resolve(root, "src-tauri/plugin-assets/timeline", name));
    if (!fresh.equals(committed)) {
      throw new Error(`src-tauri/plugin-assets/timeline/${name} is stale — run \`npm run build:timeline-ui\``);
    }
  }
  console.log("timeline plugin UI matches the Vite bundle");
} finally {
  rmSync(temp, { recursive: true, force: true });
}
