# Bundled plugin UI isolation remainder

Architecture and isolation rules are in
[`PLUGIN_PLATFORM.md`](../plugins/PLUGIN_PLATFORM.md). This file is the
unfinished remainder.

## Shipped

- Chronology runs in the `daena.timeline` plugin webview
  (`src-tauri/plugin-assets/timeline/`, built by `npm run build:timeline-ui`).
- The Lore World graph runs in the `daena.lore` plugin webview
  (`src-tauri/plugin-assets/lore/`, built by `npm run build:lore-ui`). The
  shell opens Chronology and the graph through one `PROJECTION_SANDBOXES`
  table; the trusted projection path (`projections.ts`, `ProjectionView`) is
  gone, and `scripts/check-plugin-isolation.mjs` has no runtime-import
  allowlist left.
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
- Plugin editors can upload assets: `asset.upload.begin` / `asset.upload.commit`
  (chunked, hash-checked, own namespace only) and `asset.shared.list`. The
  Language editor attaches uploads to the selected language through a hidden
  `<input type="file">`, one upload at a time per entity.
- The plugin bundles are fully minified after the Vite build
  (`scripts/minify-plugin-bundle.mjs`).

## Still open

- `src-tauri/plugin-assets/writing/` is still a stub.
- Asset upload transfers expire 60 s after `asset.upload.begin` and are not
  refreshed per chunk; the SDK caps uploads at 64 MiB.
- `RichTextEditor` writes Svelte state from ProseMirror blur, so destroying a
  focused editor inside a runes flush throws `state_unsafe_mutation`. Language's
  Overview no longer reloads into a loading state after autosave, which was the
  known trigger; other surfaces that unmount a focused editor would hit it.
- Guide dismissals for Language now live in the plugin webview's own
  `localStorage`, so earlier dismissals made in the main webview do not carry
  over.
- Isolation proof is still static and fake-broker rendering, not packaged
  Tauri webview tests on every desktop platform. The Language content-pane
  layout, the projection webviews, overlay hiding, and toast placement need a
  run in the real app, windowed and fullscreen.

## Resume order

1. Run the desktop app, windowed and fullscreen, and verify Language in the
   content pane (layout at narrow and short windows, pane switching from
   history, leave guards, overlays, toasts, link opening, shared images, the
   native file chooser for asset upload) and the Chronology and World graph
   webviews (layout, Back, Show on map). Also insert an image in a main-window
   editor, which is unverified since the editor host change.
2. Make `RichTextEditor` safe to destroy while focused.
