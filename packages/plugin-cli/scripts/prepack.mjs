import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const schemaDir = join(packageRoot, "schema");
mkdirSync(schemaDir, { recursive: true });
copyFileSync(join(packageRoot, "../../schemas/plugin-manifest-v1.json"), join(schemaDir, "plugin-manifest-v1.json"));
