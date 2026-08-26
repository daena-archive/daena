# ADR 0002: Rust-owned public contracts

- Status: Accepted
- Decided: 2026-08-07
- Extended: 2026-08-10
- Consolidated: 2026-08-26

## Context

The plugin contract once existed as separately maintained Rust types, method
catalogs, JSON Schemas, TypeScript declarations, and validators. Those copies
could drift, and schema languages cannot express all of Daena's namespace and
cross-reference rules. Projects also need controlled ways to customize module
schemas without modifying installed packages.

## Decision

The Rust plugin API crate owns public contract shapes and the broker method
catalog. JSON Schemas, TypeScript declarations, and SDK build products are
generated artifacts. Contract changes begin in Rust and regenerate every
published representation. Cross-reference and semantic validation remains
authoritative in Rust; any TypeScript validation mirror is checked against a
shared conformance fixture set.

Projects may extend an opted-in module through a schema overlay. Installed
package schemas and templates remain immutable defaults. An overlay may
disable package definitions or add, edit, and remove project-specific entity
types, fields, and templates. The host validates and merges the overlay; it is
stored in authoritative runtime state and included in portable project state.
Existing data is retained when a definition is hidden or removed.

Schema-overlay mutation is a trusted-shell action. A plugin must explicitly
declare support and contribute a compatible base schema; a sandboxed plugin
cannot grant itself this ability or mutate the overlay through ordinary RPC.

## Rejected alternatives

- Hand-maintaining equivalent contract declarations in Rust, schemas, and
  TypeScript.
- Treating JSON Schema as the authority for rules it cannot fully express.
- Forking or rewriting installed plugin packages for project customization.
- Storing project-specific module data in a plugin-owned database.

## Consequences

- Generated schemas and TypeScript declarations are never edited directly.
- Generation drift and Rust/TypeScript validation parity are repository check
  responsibilities.
- Effective module manifests are the validated merge of package defaults and
  the project overlay; editors load package defaults separately when users are
  editing the overlay itself.
- Plugin upgrades can replace package defaults while preserving the separate
  project overlay and its data.

## Decision history

- 2026-08-07: Rust replaced parallel schema-first maintenance as contract
  authority.
- 2026-08-10: opted-in project schema overlays were accepted as the extension
  mechanism.
- 2026-08-26: contract generation and overlay ownership were combined because
  both depend on the same Rust-owned schema boundary.
