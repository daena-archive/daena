import { resolve } from "node:path";
import { build } from "vite";
import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte";

const root = resolve(import.meta.dirname, "../..");

/**
 * Bundles the runes and legacy parents with the real RichTextEditor into one ES module
 * built for the client Svelte runtime, so teardown can run outside the app.
 */
export async function buildRichTextEditorHarness(outDir) {
  await build({
    configFile: false,
    root,
    publicDir: false,
    logLevel: "warn",
    plugins: [svelte({ configFile: false, preprocess: vitePreprocess() })],
    resolve: {
      alias: { $lib: resolve(root, "src/lib") },
      extensionAlias: { ".js": [".ts", ".js"] },
    },
    build: {
      outDir,
      emptyOutDir: true,
      sourcemap: false,
      minify: false,
      lib: {
        entry: resolve(root, "scripts/rich-text-editor-teardown/entry.ts"),
        formats: ["es"],
        fileName: () => "harness.js",
      },
      rollupOptions: { output: { inlineDynamicImports: true } },
    },
  });
  return resolve(outDir, "harness.js");
}
