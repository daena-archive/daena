import { resolve } from "node:path";
import { build } from "vite";
import { finalizePluginBundle } from "./minify-plugin-bundle.mjs";

const root = resolve(import.meta.dirname, "..");
const outDir = process.env.TIMELINE_UI_OUT ?? resolve(root, "src-tauri/plugin-assets/timeline");

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
      entry: resolve(root, "packages/modules/timeline/src/plugin-main.ts"),
      formats: ["es"],
      fileName: () => "timeline.js",
      cssFileName: "timeline",
    },
    rollupOptions: {
      external: [],
      output: {
        inlineDynamicImports: true,
      },
    },
  },
});

await finalizePluginBundle(outDir, "timeline");
