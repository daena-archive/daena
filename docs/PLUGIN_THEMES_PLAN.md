# Plugin theme packs

## Status and purpose

Accepted. Standing policy is [ADR 0007](adr/0007-plugin-theme-packs.md). The
authoring contract is in [`PLUGIN_SDK.md`](PLUGIN_SDK.md); architecture is in
[`PLUGIN_PLATFORM_PLAN.md`](PLUGIN_PLATFORM_PLAN.md) §13. This file retains
delivery notes, contrast floors, and rejected alternatives.

This plan must stay inside the existing plugin boundary:

- [ADR 0001](adr/0001-plugin-platform-boundary.md) — isolation; untrusted code
  never runs in the trusted webview and never receives the host DOM.
- [ADR 0002](adr/0002-rust-owned-public-contracts.md) — Rust owns public
  contracts; generated SDK/schema artifacts; the host validates and merges
  contributions.
- [`PLUGIN_PLATFORM_PLAN.md`](PLUGIN_PLATFORM_PLAN.md) — declarative data is
  the preferred extension class; unknown manifest keys are rejected; minor
  host API releases may add optional fields.

It does not change the canonical project data model in `STORAGE.md`.

## Baseline (historical)

Before Phases 0–2, the trusted shell already had a two-mode appearance:

- `ThemePreference` is `light | dark | system`, stored in app-profile
  `settings.json` (`AppearanceSettings.theme`) and cached in `localStorage`.
- `applyThemePreference` sets `data-theme` / `data-theme-preference` on
  `:root`.
- Color and rail values are a Rust-owned token catalog
  (`THEME_TOKEN_IDS`, builtin light/dark maps). The trusted shell resolver
  writes them as CSS custom properties on `:root`. Shadows, fonts, and
  layout sizes stay in CSS.
- Settings copy states that appearance follows the user across projects.
  Theme application already runs on the welcome screen, before any project is
  open.

Plugins could not style the host:

- `PluginManifest` had no theme contribution; unknown keys failed closed.
- Plugin UI ran in an isolated webview with its own CSP (`style-src 'self'`
  except the maps exception). Bootstrap did not include appearance.
- Host-rendered views were JSON component trees with no styling escape hatch.
- Plugin SVGs were already painted as host-colored masks, which remains the
  correct precedent: the plugin supplies data, the host paints.

That isolation still holds. Phases 0–2 shipped the catalog, `themes` field,
Settings picker, bootstrap appearance, and sandbox sync. The shipped contract
is in `PLUGIN_SDK.md` and `PLUGIN_PLATFORM_PLAN.md` §13.

## Goal

Authors can install a plugin that contributes one or more **theme packs**.
The user picks a pack in Settings. The trusted shell applies a validated
token map. Plugin JavaScript and CSS never enter the host document.

A theme pack is presentation, not project data. It needs no namespace, no
capability grant, and no project enablement.

## Decisions

### 1. Themes are declarative token maps

A theme pack is JSON in the plugin manifest. The host parses, validates,
merges, and applies it. Plugins do not ship CSS, fonts, or images for the
host chrome, and they do not execute code to compute a theme.

This is the same contribution class as schemas, templates, catalog icon
refs, and host-rendered views.

### 2. Mode and pack are independent

| Layer | Values | Storage |
| --- | --- | --- |
| Mode | `light`, `dark`, `system` | Existing `appearance.theme` |
| Pack | builtin, or `{ pluginId, themeId }` | New `appearance.themePack` |

Mode stays a user preference. A pack supplies light and dark token maps; the
resolved mode selects which map to apply. Builtin “Warm paper / Forest night”
remains host chrome, not a plugin. The trusted shell is not a plugin.

### 3. Packs are install-scoped, not project-enabled

Installed packages are already global to the application profile. Enablement
and grants are per project. Theme packs must work with no project open, so
availability follows **installation**, not project enablement.

Selecting a pack is the consent to apply it. Installation catalogs the pack;
it does not auto-apply. A plugin cannot draw or phrase that choice.

If the selected pack’s plugin is uninstalled or fails validation, the host
falls back to builtin. The UI is never left unstyled.

### 4. One active pack, sparse maps, host fill

Exactly one pack is active. Installed packs appear in a picker; they do not
stack.

Token maps may be sparse. The host fills missing keys from the builtin map
for the resolved mode, then contrast-checks the **merged** result. Unknown
token keys are rejected.

### 5. Closed, versioned token catalog

Rust owns the token name list, the same way it owns `CATALOG_ICON_IDS` and
`TYPE_COLOR_PRESET_IDS`. `npm run gen:plugin-contract` emits the SDK enum
and JSON Schema (`THEME_TOKEN_IDS`, `BUILTIN_THEME_TOKENS`). Merge, contrast,
and pack validation stay hand-written: Rust `theme.rs`, TypeScript
`packages/plugin-sdk/src/theme.ts`, and a host apply copy in `src/lib/theme.ts`
(the host test runner cannot import SDK `.js` paths). Names match existing
CSS variables without the `--` prefix (`ink`, `surface`, `accent`,
`rail-bg`, …).

v1 values are parsed colors only: `#rgb`, `#rrggbb`, `#rrggbbaa`. No raw CSS
strings, `url()`, `expression()`, `@import`, fonts, radii, spacing, or
shadows-as-css.

Adding a token later is a minor host API change. Removing or renaming one is
major.

### 6. Manifest field, not a new kind or manifest version

Add `themes` with `#[serde(default)]` on `PluginManifest`. This is the same
optional-field pattern as `records`. It is a minor host API addition.
Manifest version stays 1. Do not introduce a parallel extension block or a
`theme` plugin kind.

A pack-only plugin may be `kind: "declarative"` with empty entrypoints and
empty capabilities.

### 7. Sandboxed UIs observe appearance; they do not grant it

Appearance is ambient chrome, not project data. A plugin session may read the
resolved appearance without a capability.

- `PluginBootstrap` gains an `appearance` object (preference, resolved mode,
  pack ref, complete merged tokens).
- RPC `appearance.get` is session-scoped with no capability
  (`Static([])`).
- The host pushes appearance changes to open plugin sessions on a host-owned
  channel. This is not `event.subscribe` and does not require a grant.
- Optional feature `appearance@1` is advertised through existing
  `optionalFeatures` / feature discovery.

Plugin webviews may use the same token names in their own CSS. They still
cannot reach the host DOM.

## Manifest contribution

```json
{
  "themes": [
    {
      "id": "parchment",
      "name": "Parchment",
      "tokens": {
        "light": { "accent": "#b4773f", "canvas": "#f7f6f2" },
        "dark": { "accent": "#c58a4a", "canvas": "#0e1714" }
      }
    }
  ]
}
```

Rules:

- `id` is unique within the package; the globally addressable id is
  `pluginId/themeId`.
- `name` is author-facing and host-rendered.
- `tokens.light` and `tokens.dark` are both required when `themes` is
  non-empty. Each map may omit keys.
- Cross-reference validation lives in `validate_manifest` and is mirrored in
  TypeScript `validatePluginManifest`, with dual-validator fixtures.

## Settings and apply path

`AppearanceSettings` gains:

```text
themePack: Option<ThemePackRef>   // { pluginId, themeId }
```

`updateChannel` stays on `AppearanceSettings`. Settings UI keeps Light / Dark
/ System and adds a pack picker: Default plus installed packs, with swatches
from `accent`, `surface`, and `canvas` for the resolved mode. If the persisted
pack is missing, Default is not marked selected; an unavailable row is shown
and builtin tokens stay applied until the user picks Default or a live pack.

Resolver (trusted shell only):

1. Resolve mode from preference and `prefers-color-scheme`.
2. Load the selected pack from the **installed** catalog, or builtin.
3. Merge pack tokens over the builtin map for that mode.
4. Contrast-check the merged map (see below).
5. Set `data-theme` and write custom properties on `document.documentElement`.
6. Cache the merged light and dark maps in `localStorage` (`daena-theme-pack`)
   so `theme-init.js` can paint the welcome screen before the plugin catalog
   loads. Clear the cache when applying builtin (explicit Default or fallback).

CSS custom properties on `:root` are outputs of this resolver, not the
source of truth.

Contrast pairs and floors (measured on the current builtin):

| Pair | Floor | Notes |
| --- | --- | --- |
| `ink` / `surface` | 4.5 | Body text. |
| `ink` / `canvas` | 4.5 | Body text. |
| `rail-text` / `rail-bg` | 3.0 | Chrome. |
| `on-accent` / `accent` | not gated | Builtin is 3.68 light / 2.93 dark. Do not restyle in Phase 0. Revisit if a pack changes `accent`. |

Eight-digit hex with alpha other than `ff` fails contrast (ratio would ignore
transparency). Opaque `#rrggbb` and `#rrggbbff` are equivalent.

Hardcoded colors outside the catalog are **not** a Phase 0 exit. Remaining
hex in `+page.svelte` after token extraction:

- `var(--token, #hex)` fallbacks — unused once `:root` variables are set;
  follow-on CSS cleanup.
- Welcome illustration (`.marker-*`, hero overlays) — scene paint, not chrome.
- A few controls (`.primary-button` `#fff` / `#263d30` / `#2b4535`) that
  should later use tokens; changing them is not a visual no-op.

## Validation

Package validation is in `validate_manifest` and TypeScript
`validatePluginManifest`, with dual-validator fixtures. The same merge and
contrast rules run again at apply; a missing or unreadable pack leaves
builtin tokens active.

At package validation (CLI and installer) and again at apply:

- every key is in the closed catalog;
- every value parses as a color;
- both mode maps exist;
- ids are unique in-package;
- `name` is 1–128 characters (same rule in schema, Rust, and TypeScript);
- merged contrast meets the host threshold.

Failures fail closed: the package does not install, or the pack is not
applied and builtin remains active.

No `url()`, external fonts, or host CSS attachment. Plugin CSP is unchanged.

## Explicit non-goals (v1)

- Arbitrary CSS or layout restyles of host chrome.
- Fonts, images, radii, spacing, or motion tokens.
- Remapping entity-type color presets.
- Auto-apply on install.
- Stacking multiple packs.
- Per-project world skins.
- A new plugin kind, capability, or manifest version.

## Follow-on (not v1)

A project-scoped `themePack` overlay could let a world look different while
open, still as host-applied JSON. That conflicts with current Settings copy
(“follows you across projects”) and with welcome-screen theming, so it waits
until the app-global pack path is real.

Fonts and density tokens can join the catalog later as parsed structured
values, never as CSS strings.

## Delivery

### Phase 0 — Tokenize the host (implemented)

Extract the builtin light/dark maps into a Rust-owned catalog and a host
resolver. Apply builtin tokens through that resolver. Visual result is a
no-op if the maps match the former `:root` CSS.

The closed token catalog and builtin maps are generated into
`schemas/theme-tokens-v1.json` and `packages/plugin-sdk/src/generated.ts`
(`THEME_TOKEN_IDS`, `BUILTIN_THEME_TOKENS`). That is Decision 5 scaffolding
and matches `CATALOG_ICON_IDS`: the trusted shell already imports generated
contract constants (`src/lib/entity-icons/catalog.ts`,
`src/lib/entity-colors/presets.ts`). It is not a theme contribution, picker,
`appearance.get`, or bootstrap field.

No `themes` manifest field. Settings still only offers mode.

Exit: resolver tests; Settings still only offers mode; welcome screen and
project chrome both consume the resolver.

Out of slice: `src/lib/maps/atlas/constants.ts` and other non-theme diffs.

### Phase 1 — Manifest and Settings (implemented)

Add `themes` to the contract, validate (Rust + TypeScript dual fixtures),
persist `themePack`, list installed packs in Appearance, apply/fallback.

Missing or unreadable packs apply builtin tokens without leaving the shell
unstyled. The persisted ref is kept until the user picks Default or a live pack.
Welcome-screen paint uses the cached merged maps when present.

Exit: dual-validator fixtures; uninstall/corrupt pack falls back to builtin;
`daena-plugin validate` rejects unknown keys and illegal color strings.

### Phase 2 — Sandbox sync (implemented)

Bootstrap `appearance`, `appearance.get` (`Static([])`), live host push via
plugin-webview eval of a host-owned apply script, `appearance@1` on
`optionalFeatures`. Example pack-only plugin: `examples/plugins/theme`.
Ink Tools consumes the same CSS variables; the host paints tokens before
plugin script runs so the sandbox does not flash builtin.

Host push targets `plugin:` webviews (DOM). Wasm/service sessions have no
document; they read `appearance.get`. Sandbox bootstrap still requires an
open project; welcome-screen theming is host-only. Init-script snapshots
seed light/dark from app settings; `system` resolves on the first frontend
sync.

Exit: a sandboxed UI restyles with host tokens without flashing the wrong
mode; no plugin CSS appears in the host document.

### Phase 3 — Record the decision (implemented)

Folded into `PLUGIN_SDK.md` and `PLUGIN_PLATFORM_PLAN.md`. Standing policy is
[ADR 0007](adr/0007-plugin-theme-packs.md). Also added short pointers in
`ARCHITECTURE.md` and ADR 0001 (beyond the named fold-in; consistent, not a
contract change).

## Rejected alternatives

| Alternative | Why rejected |
| --- | --- |
| Plugin CSS in the host webview | Breaks ADR 0001 and host CSP. |
| Sandboxed JS that mutates host styles | Untrusted code in trusted chrome. |
| Gate packs on project enablement | Breaks welcome-screen theming; appearance is app-profile state. |
| Auto-apply the first installed pack | The plugin would own a host permission surface. |
| Manifest v2 or a versioned extension block | Optional fields already land as defaulted keys (`records`). |
| Require complete token maps | Hostile to authors; merge plus contrast is enough. |
| `event.subscribe` for theme changes | Appearance is not a data grant; bootstrap plus host push is enough. |
| Builtin look as a bundled plugin | The trusted shell is not a plugin. |

## Vertical slice (when implementing)

Phase 0 first: host tokens as data. Until that exists, plugins have nothing
safe to contribute. Phase 1 is the first user-visible plugin theme. Phase 2
is required before sandboxed module UIs can match the chosen pack. Do not
stage, commit, or push unless explicitly asked.
