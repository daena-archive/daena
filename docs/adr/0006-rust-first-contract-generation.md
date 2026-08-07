# ADR 0006: Rust-first contract generation

- Status: Accepted
- Date: 2026-08-07

## Context

The plugin platform contract was maintained in five parallel representations:
Rust types plus `validate_manifest`, the RPC method catalog, `generated.ts`,
the TypeScript `validatePluginManifest`, and the `schemas/*.json` files. Only
the Rust side was load-bearing. Reconnaissance found that RPC payload types
existed nowhere in Rust, the method catalog disagreed in three places (schema
`x-methods`, TS `BrokerMethodPayloads`, and the Rust dispatch `match`), and
`generated.ts` was hand-maintained despite a header claiming it was generated.

The original Phase 0 direction in `PLUGIN_PLATFORM_PLAN.md` was schema-first:
"one versioned JSON Schema owned by the Rust plugin API crate. Rust types and
TypeScript declarations are generated from that schema." That direction was
never implemented; all three copies were hand-written and kept in sync by hand.
JSON Schema also cannot express the cross-reference rules that `validate_manifest`
enforces (namespace ownership, migration contiguity, template-field typing), so
generation can only ever cover shapes, never rules.

## Decision

Reverse the direction: Rust is the single source of truth. The `daena-plugin-api`
crate owns the contract types and the RPC method catalog; the versioned JSON
Schemas and the TypeScript declarations are generated build artifacts, produced
by `npm run gen:plugin-contract`. Cross-reference rules stay handwritten in
Rust's `validate_manifest` and are mirrored in the TypeScript
`validatePluginManifest`, with parity enforced by a dual-validator conformance
test that runs both validators over a shared fixture battery.

Rationale:

- Rust is already the enforcement boundary; the schemas and TS types exist to
  serve it, not the other way around.
- Schema-first codegen (e.g. `typify`) is immature for this contract's needs.
- Generating schemas from typed structs preserves exact wire names through serde
  `rename`/`rename_all`, keeping `generated.ts` identical to what the host sends.
- Rules cannot be generated, so keeping them as the enforceable Rust code and
  conformance-testing the TS mirror is the closest possible parity.

A drift guard (`check:plugin-contract`) regenerates everything into a temp
directory and byte-diffs the committed schemas, `generated.ts`, and SDK `dist`
artifacts; any contract change that is not followed by a regen fails the check.

## Consequences

- `schemas/*.json` and `packages/plugin-sdk/src/generated.ts` are build outputs;
  they must not be hand-edited. Contract changes are made in Rust, then
  regenerated.
- The dual-validator conformance test is the guard for rule parity between Rust
  and TypeScript validators.
- The schema-first (`typify`-style) codegen direction is not adopted.
- Committing a contract change without regenerating artifacts is caught by the
  drift guard rather than silently shipped.
