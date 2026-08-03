# Worldbuilder version-one architecture

This document turns the MVP in `docs/PLAN.md` into the contract implemented by
the host and bundled modules. The contract is intentionally small: the core
owns durable data and modules own meaning and presentation.

The third-party plugin platform contract is defined in
[`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md). Its Phase 0 schemas,
Rust API types, generated SDK types, and ADRs are authoritative for future
runtime plugins; this document describes the current bundled-module behavior
until the later conversion phases are complete.

## Canonical data

Each project is a portable directory containing `project.json`, the canonical
`worldbuilder.sqlite` database, and an `assets/` tree divided into images,
videos, maps, and other files. JSON snapshots remain an interoperability and
recovery format; optional Git operations are explicit user actions.

Every entity has an immutable UUID, a
display name, an optional type, timestamps, and a soft-delete marker. Prose is
stored as documents attached to an entity. Structured fields are namespaced by
module and validated by that module's schema contribution. Relationships store
source ID, target ID, relationship type, and optional metadata. Assets are
registered by SHA-256 content hash and project-relative path; modules store
references to assets, never copies of project files.

Names and other presentation properties are mutable; IDs are not. A reference
always stores an ID, so renaming an entity cannot break links. Deleting an
entity is a core operation that preserves an audit/tombstone record and leaves
module data available for recovery or export.

## Module boundary

Modules are build-time registered packages. A manifest declares an ID, version,
display name, capabilities, schemas, templates, views, commands, and
migrations. The host passes a typed `ModuleContext` containing
entity/query/document/field/relationship/asset operations and a
mount point; it never passes a database handle, filesystem handle, or raw Tauri
invoke function. Views return a cleanup callback and must release subscriptions
when unmounted.

Module-owned fields and assets use module namespaces and entity IDs. Disabling a
module changes visibility and unregisters its commands/views, but does not
delete its rows, documents, or assets. Enabled state is persisted in the
project database so reopening a project does not silently re-enable a module.

## Migrations

Migrations are declarative operations (`create namespace`, `add field`,
`rename field`, and `drop field` with an explicit backup policy). A migration
has a unique ID, source and target versions, and a deterministic operation
list. The core validates that versions are contiguous, operations are scoped
to the declaring module, and destructive operations have a recovery policy.

Before applying a migration, the core creates a JSON backup, validates the
complete plan, and executes it in one SQLite transaction. On any error it rolls
back and leaves the previous module version active. Successful migrations
record their ID and serialized checksum so they cannot run twice or be
replaced silently.

## Capabilities and versioning

Capabilities are explicit strings such as `entity.read`, `entity.write`,
`asset.read`, and `search.query`. The host grants only declared capabilities;
unknown capabilities and undeclared operations fail closed. The public API
uses semantic versions. API major versions are incompatible; module manifest
major versions must match the host-supported range. Stored data versions are
separate from package versions and are advanced only by migrations.

## MVP sequencing

1. Ship this contract and the typed API package.
2. Implement the Rust project store and narrow Tauri commands.
3. Build the Svelte host and registry.
4. Build Lore, then Timeline, using only the public context.
5. Validate the Eldermere example through export/import, migration rollback,
   disablement, renames, and search-index rebuild.
