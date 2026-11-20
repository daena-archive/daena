import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { Window } from "happy-dom";
import { build } from "vite";
import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte";

const root = resolve(import.meta.dirname, "..");
const output = await mkdtemp(resolve(tmpdir(), "daena-wiki-test-"));
const window = new Window({ url: "http://localhost" });
Object.assign(globalThis, { window, document: window.document });
for (const key of Object.getOwnPropertyNames(window)) {
  if (!(key in globalThis) || key === "localStorage") {
    Object.defineProperty(globalThis, key, { configurable: true, get: () => window[key] });
  }
}

try {
  await build({
    configFile: false,
    root,
    publicDir: false,
    logLevel: "warn",
    plugins: [
      svelte({ configFile: false, preprocess: vitePreprocess() }),
      {
        name: "wiki-test-entry",
        resolveId(id) { if (id === "wiki-test-entry" || id === resolve(root, "wiki-test-entry")) return resolve(root, "wiki-test-entry"); },
        load(id) {
          if (id !== resolve(root, "wiki-test-entry")) return;
          return `
            export { mount, unmount, flushSync } from "svelte";
            export { default as Wiki } from "$lib/lore/WikiView.svelte";
            export { project } from "$lib/project/client";
            export { setDisplayLanguage } from "$lib/i18n";
          `;
        },
      },
    ],
    resolve: { alias: { $lib: resolve(root, "src/lib") }, extensionAlias: { ".js": [".ts", ".js"] } },
    build: {
      outDir: output,
      emptyOutDir: true,
      lib: { entry: "wiki-test-entry", formats: ["es"], fileName: () => "wiki.mjs" },
      rollupOptions: { output: { inlineDynamicImports: true } },
      chunkSizeWarningLimit: 5000,
    },
  });
  const { Wiki, project, setDisplayLanguage, mount, unmount, flushSync } = await import(
    pathToFileURL(resolve(output, "wiki.mjs")).href
  );
  const entity = { id: "test-person", name: "Knowledge base", entity_type: "person", updated_at: 0 };
  project.getEntity = async () => entity;
  project.queryEntities = async () => ({
    items: [entity], total: 1, offset: 0, has_more: false, type_counts: [{ entity_type: "person", count: 1 }],
  });
  for (const method of ["listAssets", "listDocuments", "listFields", "listRelationships", "listMapLocations"]) {
    project[method] = async () => [];
  }
  const target = document.createElement("div");
  document.body.append(target);
  setDisplayLanguage("ja");
  const app = mount(Wiki, { target, props: {
    manifest: { id: "daena.lore", name: "Lore", schemas: [{ fields: [], entityTypes: [{ id: "person", name: "Person" }] }], namespaces: ["lore"] },
    projectId: "fixture", initialEntityId: entity.id, aiEnabled: false, imageProvider: {}, textProvider: {},
  } });
  for (let i = 0; i < 100 && !target.textContent.includes("公開ページ 1件"); i++) {
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  assert.equal(target.querySelector(".article-heading h1")?.textContent, entity.name);
  assert.equal(target.querySelector(".article-type")?.textContent, "人物");
  assert.equal(target.querySelector(".sidebar-search input")?.getAttribute("placeholder"), "資料集を検索");
  for (const text of ["世界設定の資料集", "公開ページ 1件", "記事を書く", "関連ページ", "詳細はまだ登録されていません。"]) {
    assert.ok(target.textContent.includes(text), text);
  }
  setDisplayLanguage("en");
  flushSync();
  assert.equal(target.querySelector(".article-heading h1")?.textContent, entity.name);
  assert.equal(target.querySelector(".article-type")?.textContent, "Person");
  assert.equal(target.querySelector(".sidebar-search input")?.getAttribute("placeholder"), "Search the knowledge base");
  assert.ok(target.textContent.includes("Write article"));
  await unmount(app);
  console.log("Wiki renders translated UI and preserves author text across live language changes");
} finally {
  await window.happyDOM.close();
  await rm(output, { recursive: true, force: true });
}
