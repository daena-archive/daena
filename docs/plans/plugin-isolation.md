# Bundled plugin UI isolation remainder

Architecture and isolation rules are in
[`PLUGIN_PLATFORM.md`](../plugins/PLUGIN_PLATFORM.md). This file is the
unfinished remainder: some bundled UIs still run in the trusted webview.

## Still open

- Timeline chronology and Language still mount in the main webview via
  `src/lib/modules/projections.ts`.
- Isolated HTML under `src-tauri/plugin-assets/{timeline,writing}/` is still a
  stub projection, not vis-timeline / Writing UIs. There is no
  `plugin-assets/language/`.
- `scripts/check-plugin-isolation.mjs` still allowlists
  `src/lib/modules/projections.ts`. Timeline and Language remain on that
  allowlist. Isolation proof is still static, not packaged Tauri webview tests
  on every desktop platform.
- `packages/module-api` keeps `ModuleContext` until those UIs leave the main
  webview.

## Resume order

1. Timeline chronology isolation. Calendar date and era-scope helpers already
   live in the timeline package so they are not private host imports when
   Timeline moves.
2. Language.
