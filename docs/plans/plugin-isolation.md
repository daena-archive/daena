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
- Verified in the real macOS app, windowed and fullscreen: Language in the
  content pane (layout, pane switching from history, leave guards, toasts,
  link opening, shared images, the native file chooser), the Chronology and
  World graph webviews (layout, Back, Show on map), image insert in a
  main-window editor, and switching entities right after typing. That run
  found and fixed:
  - Show on map: activating Maps replaced the native
    `daena.maps/navigation@1` handler with a failing WASM stub.
  - Placement: in windowed mode the main webview spans the title bar and
    WebKit starts the page 32pt down, so the content view is taller than the
    CSS viewport. `scale_plugin_bounds` takes zoom from the width ratio and
    applies the extra height as a top inset. `SandboxView` also re-syncs when
    its placeholder moves or an ancestor scrolls, not only when it resizes.
  - The Languages layout runs flush to the window's bottom edge, since a
    native webview cannot scroll with the page.
  - Anchored popovers (project switcher, status center) hide a plugin
    webview only when they overlap it; modal dialogs still hide all of them.
  - Plugin editors get entity search through `EditorHost.searchEntities`.
- Entity links open from any rich-text editor through `EditorHost.openEntity`
  (an Open button beside Edit reference, or Cmd/Ctrl+click). The Language
  webview asks the shell with an `open-entity` state message, which goes
  through the Language leave guard.

## Still open

- Writing editing and saving still run in the main webview by default. Slices 1 and 2 of
  [`writing-isolation.md`](writing-isolation.md) ship the webview (`src-tauri/plugin-assets/writing/`,
  `npm run build:writing-ui`) with editing and shell-side saving behind `localStorage["daena:writing-webview"] = "1"`.
- The SDK caps uploads at 64 MiB.
- `RichTextEditor` is safe to destroy while focused: blur during a reactive
  update never calls `onChange`, and destroying the editor drops edits still in
  its 120 ms debounce. Parents must call `flushPendingChanges()` before they
  unmount or navigate (the shell does this in `flushAutoSave`). Focused inputs
  inside the editor's own dialogs are not covered.
- Guide dismissals for Language now live in the plugin webview's own
  `localStorage`, so earlier dismissals made in the main webview do not carry
  over.
- Automated isolation proof is still static and fake-broker rendering. The
  real-app check above was manual and macOS only; Windows and Linux webview
  placement is unverified.
- Do not call Tauri dispatcher methods (`set_position`, `set_size`, `show`,
  `run_on_main_thread`) inside `Webview::with_webview`. In tauri-runtime-wry
  2.11.4 the runtime holds the webview's `window_id` mutex while the closure
  runs, and on the main thread the closure runs synchronously, so such a call
  deadlocks the app.
- Two clippy warnings predate this work (`host.rs` useless conversion,
  `plugin_webview_url` argument count), so workspace clippy with
  `-D warnings` fails.

## Resume order

1. Isolate Writing Studio into `src-tauri/plugin-assets/writing/`, the last
   major module still in the main webview.
2. Clear the two pre-existing clippy warnings.
