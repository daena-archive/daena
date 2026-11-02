# Bundled plugin UI isolation remainder

Architecture and isolation rules are in
[`PLUGIN_PLATFORM.md`](../plugins/PLUGIN_PLATFORM.md). This file is the
unfinished remainder: some bundled UIs still run in the trusted webview.

## Still open

- Language still mounts in the main webview via `src/lib/modules/projections.ts`.
- Chronology opens in the `daena.timeline` plugin webview. `src-tauri/plugin-assets/timeline/index.html` loads the vis-timeline bundle. It is no longer a stub.
- Isolated HTML under `src-tauri/plugin-assets/writing/` is still a stub. There is no `plugin-assets/language/`.
- `scripts/check-plugin-isolation.mjs` still allowlists `src/lib/modules/projections.ts` because Language remains. Isolation proof is still static, not packaged Tauri webview tests on every desktop platform.
- `packages/module-api` keeps `ModuleContext` until Language leaves the main webview.

## Resume order

1. Language.
