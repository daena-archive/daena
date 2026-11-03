import { resolve } from "node:path";
import { build } from "vite";
import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { finalizePluginBundle } from "./minify-plugin-bundle.mjs";

const root = resolve(import.meta.dirname, "..");
const outDir = process.env.LANGUAGE_UI_OUT ?? resolve(root, "src-tauri/plugin-assets/language");

await build({
  configFile: false,
  root,
  publicDir: false,
  logLevel: "warn",
  plugins: [svelte({ configFile: false, preprocess: vitePreprocess() })],
  resolve: {
    alias: {
      $lib: resolve(root, "src/lib"),
    },
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
      entry: resolve(root, "packages/modules/language/src/plugin-main.ts"),
      formats: ["es"],
      fileName: () => "language.js",
      cssFileName: "language",
    },
    rollupOptions: {
      external: [],
      output: {
        inlineDynamicImports: true,
      },
    },
  },
});

await finalizePluginBundle(outDir, "language");
