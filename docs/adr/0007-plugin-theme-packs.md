# ADR 0007: Plugin theme packs

- Status: Accepted
- Decided: 2026-09-09

## Context

The trusted shell already owns light / dark / system appearance. Plugins needed
a way to contribute alternate chrome palettes without receiving the host DOM or
running in the trusted webview.

## Decision

Theme packs are declarative token maps in the plugin manifest. The host parses,
validates, merges, and paints them. Plugin CSS, fonts, images, and JavaScript
never enter the host document.

Mode (`light` | `dark` | `system`) stays app-profile `appearance.theme`. Pack
selection is a separate `appearance.themePack` ref. Builtin Warm paper / Forest
night remains host chrome, not a plugin.

Packs are install-scoped, not project-enabled, so they work with no project
open. Installation catalogs a pack; it does not auto-apply. Selecting a pack in
Settings is the consent. Exactly one pack is active. Maps may be sparse; the
host fills from the builtin map for the resolved mode and contrast-checks the
merge. Unknown keys and non-color values fail closed.

Rust owns the closed token catalog (`THEME_TOKEN_IDS`) and builtin maps.
`themes` is an optional defaulted manifest v1 field, not a new kind,
capability, or manifest version.

Sandboxed sessions may read the resolved appearance without a grant
(`plugin.bootstrap.appearance`, `appearance.get`, host push, optional feature
`appearance@1`). They cannot grant or mutate host appearance.

## Rejected alternatives

- Plugin CSS in the host webview, or sandboxed JavaScript that mutates host
  styles.
- Gating packs on project enablement.
- Auto-apply on install.
- Manifest v2 or a versioned extension block.
- Requiring complete token maps.
- `event.subscribe` for theme changes.
- Shipping the builtin look as a bundled plugin.

## Consequences

- Settings offers a pack picker. Missing or unreadable packs keep builtin
  tokens applied until the user picks Default or a live pack.
- Welcome-screen paint uses cached merged maps. Sandbox webviews receive a
  host-owned apply script; Wasm sessions poll `appearance.get`.
- Adding a token is a minor host API change; removing or renaming one is major.

## Decision history

- 2026-09-09: accepted after the host catalog, Settings picker, and sandbox
  sync shipped.
