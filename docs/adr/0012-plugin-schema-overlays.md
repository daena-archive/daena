# ADR 0012: Plugin schema overlays

- Status: Accepted
- Date: 2026-08-10
- Updated: 2026-08-10

## Context

Bundled modules ship packaged entity types, fields, and templates in their
`packages/modules/*/manifest.json` files. Authors need project-specific types
without forking plugin packages or rewriting immutable package defaults.

Lore was the first consumer; Timeline and Writing Studio use the same schema
and template shape. Installed third-party plugins should opt in the same way.

## Decision

Projects may store a **module schema overlay** for plugins that opt in:

- Opt-in capability: `schema.overlay` (declared on the packaged manifest).
- The plugin must also contribute at least one schema entity type.
- Package schemas/templates remain the immutable defaults.
- The overlay may **disable** builtin entity types, fields, and templates, and
  may **add/edit/remove** custom entity types, fields, and templates.
- The host merges package + overlay when listing effective module manifests for
  create/edit UI.
- Overlay JSON is persisted in runtime SQLite (`module_schema_overlays`) and
  round-tripped through portable `plugins/{module_id}.json` as `schemaOverlay`.
- Overlay mutation is a trusted-shell host action, not a sandboxed plugin RPC.
- Schema settings loads **package-only** schemas/templates for the editor
  (`module_schema_editor_load`) so customs never appear as builtins.

Bundled Lore, Timeline, and Writing Studio declare `schema.overlay`. Maps does
not, and remains out of scope until it opts in.

Existing entity field values for disabled or removed definitions are retained;
the UI simply stops offering those definitions until restored.

## Consequences

- Create and field forms see effective schemas without a parallel path.
- Plugin upgrades keep package defaults; overlays continue to apply on top.
- Any installed plugin (bundled or third-party) can offer schema customization by
  declaring `schema.overlay` and shipping schemas.
- Rust validates overlay payloads against the packaged module manifest before
  persistence.
