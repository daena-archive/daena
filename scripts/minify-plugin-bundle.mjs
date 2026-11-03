import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { transform } from "esbuild";

// Vite's default build target (ESBUILD_MODULES_TARGET in vite/dist/node/constants.js). Using the
// same target means this pass only removes whitespace, comments and redundant syntax; it lowers
// nothing that Vite left alone.
const PLUGIN_BUNDLE_TARGET = ["es2020", "edge88", "firefox78", "chrome87", "safari14"];

// Bundles are built without source maps, and vendored code can still carry sourceMappingURL
// comments. `//#` comments end at the line break; `/*#` comments end at their closing marker.
const SOURCE_MAPPING_URL = /\/\/[#@] sourceMappingURL=[^\n]*|\/\*[#@] sourceMappingURL=[\s\S]*?\*\//g;

export function stripSourceMappingUrl(text) {
  return text.replace(SOURCE_MAPPING_URL, "");
}

// Vite's library output for `formats: ["es"]` keeps whitespace on purpose. These bundles are
// application code loaded by <script type="module">, not modules imported by other packages, so the
// whitespace is removed here. Legal comments stay inline so vendored license notices survive.
export async function minifyPluginModule(code) {
  const result = await transform(stripSourceMappingUrl(code), {
    loader: "js",
    format: "esm",
    minify: true,
    target: PLUGIN_BUNDLE_TARGET,
    charset: "utf8",
    legalComments: "inline",
  });
  return result.code;
}

// Rewrites `<name>.js` in place with the fully minified module, and strips source map references
// from `<name>.js` and `<name>.css`.
export async function finalizePluginBundle(outDir, name) {
  const jsPath = resolve(outDir, `${name}.js`);
  const cssPath = resolve(outDir, `${name}.css`);
  writeFileSync(jsPath, await minifyPluginModule(readFileSync(jsPath, "utf8")));
  writeFileSync(cssPath, stripSourceMappingUrl(readFileSync(cssPath, "utf8")));
}
