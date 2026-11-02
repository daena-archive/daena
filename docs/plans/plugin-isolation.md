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

## Language findings

Language is blocked on one product decision: how rich text works in an
isolated webview. Everything else has a known path.

### Blocking: rich text

`panes/Overview.svelte` (the language overview document) and
`editor/CustomRuleEditor.svelte` (custom grammar rule bodies) embed the host
`src/lib/editor/RichTextEditor.svelte`. That editor reaches the trusted
project client and Tauri directly:

- asset insert uses the native file picker and `registerAssetFile`
  (`InsertAssetDialog.svelte`);
- images resolve through `readAssetBytesByPath` / `getAssetByPath`
  (`src/lib/assets/resolve.ts`);
- entity mentions search the project through the trusted client;
- links open with `@tauri-apps/plugin-opener`.

None of these are available to a plugin webview. `PLUGIN_PLATFORM.md` §5
currently describes `host.surface:daena.language/workspace@1` as host chrome
(guides, rich text, dialogs), which conflicts with moving Language out of the
trusted webview. Options:

1. Broker-backed editor. Put `RichTextEditor` behind an injected host-services
   interface with the shell behavior unchanged, then give the Language webview
   a broker-backed implementation: mentions through `search.query`, images
   through brokered asset reads, links forwarded to the shell. File-picker
   insert has no broker equivalent; it needs a new brokered method or stays
   unavailable in Language at first. Two slices.
2. Keep Language host-rendered. Remove it from this plan and record in
   `PLUGIN_PLATFORM.md` why a bundled read-write UI stays trusted.
3. Plain-text fallback. Isolate Language and edit those fields as Markdown
   text. Smaller than option 1, but drops rich-text features users have now.

### Other host imports

Besides the editor, Language imports from `src/lib/`:

- `guides/WorkspaceGuide.svelte`, `guides/persist.ts`, `guides/types.ts`;
- `dialogs.svelte` (`confirmDialog`, in Overview);
- `entity-lifecycle/archive.ts` and `entity-lifecycle/vocabulary.ts`.

These are small and can move into a shared package or be replaced locally. The
isolation check should then forbid `src/lib` imports from the Language package,
as it already does for Timeline.

### Data access

Language uses only `records.list/create/update/delete`,
`entities.get/update/delete`, `fields.set`, `fields.listRecords`, and
`documents.save`, plus `focusEntityId`, `moduleState`, and
`onModuleStateChange`. `src/lib/modules/context.ts` already builds these over a
`PluginRpcTransport`; only `trusted_module_rpc`, `module_list_manifests`, and
`maps_navigation` are Tauri-specific. The broker-backed part can move to
`packages/module-api` and run over `createBrowserPluginRpcTransport` in the
webview.

### Shell coupling

Language mounts in the workbench content pane, keyed by the selected entity,
next to the shell collection pane. Today the shell and Language share:

- the selected language (`focusEntityId`) and the active pane
  (`moduleState.pane`, driven by the shell view tabs);
- `window.__daena_canLeaveLanguage`, a synchronous guard over per-pane leave
  guards that may show Language's own confirm modal;
- `onEntityDeleted`, which refreshes the shell after a delete.

In a webview these become:

- mount parameters for entity and pane (`plugin_mount_webview` already passes
  `mapEntityId` / `linkId` this way);
- `daena.language/*` events published by the webview and forwarded to the shell,
  as `forward_maps_state_event` does for `daena.maps/state@1`, for pane changes,
  leave-guard replies, and entity mutations. The manifest must declare the
  events and `event.publish:` capabilities;
- a shell-to-webview message for pane switches and leave requests, through
  `webview.eval`, as the appearance push in `src-tauri/src/lib.rs` does. The
  leave request needs a reply with a timeout so a broken webview cannot block
  navigation.

### Styling and CSP

Language component styles are scoped and self-contained. The webview must map
shell aliases that are not theme tokens: `--theme-*` (warning, danger, success,
neutral, surface), `--font-display`, `--focus-ring`, `--shadow-sm`, and
`--control-min-height`. `panes/Grammar.svelte` sets inline `style`
attributes, so Language needs `style-src-attr 'unsafe-inline'`, like Maps and
Timeline.

### Native webview overlay

A plugin webview is a native child above the main webview. Shell overlays
(quick open, dialogs, menus) that cover the content pane render behind it. Maps
already has this limitation. Language sits in the content pane during ordinary
browsing, so it would show far more often; it needs a shell rule (hide or
unmount the child while a shell overlay is open) before or with the move.

## Resume order

1. Language: decide rich text (see Language findings), then isolate.
