# Bundled plugin UI isolation remainder

Architecture and isolation rules are in
[`PLUGIN_PLATFORM.md`](../plugins/PLUGIN_PLATFORM.md). This file is the
unfinished remainder.

## Shipped

- Chronology runs in the `daena.timeline` plugin webview
  (`src-tauri/plugin-assets/timeline/`, built by `npm run build:timeline-ui`).
- Language runs in the `daena.language` plugin webview inside the workbench
  content pane (`src-tauri/plugin-assets/language/`, built by
  `npm run build:language-ui`). The shell and the webview talk through mount
  parameters, `plugin_webview_post` host messages, and forwarded
  `daena.language/state@1` events; the pure shell side is
  `src/lib/modules/language-bridge.ts`.
- The shared rich-text editor reaches the host only through `EditorHost`
  (`src/lib/editor/host.ts`). The shell installs `shell-host.ts`; the Language
  webview installs a broker host that reads images with
  `asset.shared.read.begin` and hands links to the shell.
- `createBrokerModuleContext` in `packages/module-api` is the one
  `ModuleContext` implementation; the shell wraps it with the trusted
  transport, plugin webviews with the browser transport.
- Embedded plugin webviews are hidden while a shell overlay covers them, and
  toasts move clear of them.

## Still open

- The Lore graph projection still mounts in the main webview through
  `src/lib/modules/projections.ts`, which stays on the
  `scripts/check-plugin-isolation.mjs` allowlist.
- `src-tauri/plugin-assets/writing/` is still a stub.
- Language's rich-text editor cannot insert or upload assets: there is no
  brokered file-picker upload. It shows images only for project-shared assets
  (`reference_scope: "project"`).
- Guide dismissals for Language now live in the plugin webview's own
  `localStorage`, so earlier dismissals made in the main webview do not carry
  over.
- The plugin bundles are unminified (Language is about 2.8 MB).
- Isolation proof is still static and fake-broker rendering, not packaged
  Tauri webview tests on every desktop platform. The Language content-pane
  layout, overlay hiding, and toast placement need a run in the real app.

## Resume order

1. Run the desktop app and verify Language in the content pane: layout at
   narrow and short windows, pane switching from history, leave guards,
   overlays, toasts, link opening, and shared images.
2. Decide whether the Lore graph projection moves to a plugin webview.
3. Brokered asset upload for plugin editors, if Language should insert assets.
