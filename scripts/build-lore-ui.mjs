import { resolve } from "node:path";
import { build } from "vite";
import { finalizePluginBundle } from "./minify-plugin-bundle.mjs";

const root = resolve(import.meta.dirname, "..");
const outDir = process.env.LORE_UI_OUT ?? resolve(root, "src-tauri/plugin-assets/lore");

await build({
  configFile: false,
  root,
  publicDir: false,
  logLevel: "warn",
  resolve: {
    extensionAlias: {
      ".js": [".ts", ".js"],
    },
  },
  build: {
    outDir,
    emptyOutDir: false,
    sourcemap: false,
    cssCodeSplit: false,
    minify: true,
    chunkSizeWarningLimit: 5000,
    lib: {
      entry: resolve(root, "packages/modules/lore/src/plugin-main.ts"),
      formats: ["es"],
      fileName: () => "lore.js",
      cssFileName: "lore",
    },
    rollupOptions: {
      external: [],
      output: {
        inlineDynamicImports: true,
      },
    },
  },
});

await finalizePluginBundle(outDir, "lore");
