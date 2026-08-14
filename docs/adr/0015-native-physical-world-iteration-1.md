# ADR 0015: Native physical-world durable single-map slice

- Status: Superseded by ADR 0016 hard-cut v2 source contract
- Date: 2026-08-15
- Scope: Iteration 1 of `NATIVE_MAP_GENERATOR.md`

## Decision

This ADR records the completed v1 vertical slice as historical context. The
active physical provider no longer preserves or reads this source contract;
ADR 0016 replaces it with `physical-world-v2` without a compatibility path.

The physical provider is now a production descriptor branch of the existing
`daena.maps:map` entity. It uses the iteration-0 source tuple unchanged:

- provider `daena-physical`
- adapter version `1`
- source format `physical-world-v1`
- source MIME `application/vnd.daena.physical-world`
- source filename `world.pworld`

The pure generator remains isolated from Tauri, SQLite, Svelte, and the plugin
host. Its phase-1 API adds typed errors, progress and cancellation hooks, a
validation report, deterministic source generation, and a derived coastline
preview. The fixed source bytes remain the authority; the reference water
inventory is recomputed from those bytes and persisted in the versioned
`generation.settings.referenceWaterInventoryM3` field.

Generation is a temporary project-bound job. It does not acquire a core or
database lock while CPU work runs. The trusted shell exposes start, status,
cancel, preview, and accept operations. Acceptance revalidates the source and
provenance, then creates the map entity, physical source asset, descriptor, and
empty initial layers field through one request-id-aware core mutation.

The plugin RPC catalog also exposes typed `maps.physical.create.begin` and
`maps.physical.create.commit` transfer methods. The commit path uses the same
core acceptance operation, so plugin transfers cannot create a second storage
or identity route.

## Validation

Focused checks for this slice are:

```text
rtk cargo test --manifest-path crates/daena-physical-spike/Cargo.toml --locked --offline
rtk cargo test --manifest-path crates/daena-core/Cargo.toml --offline physical_map_acceptance_is_atomic_and_request_idempotent
rtk cargo check --manifest-path src-tauri/Cargo.toml --offline
rtk npm run check
rtk npm run check:maps:physical
```

The pure and core checks cover deterministic generation, cancellation,
provenance validation, idempotent acceptance, checkpoint rebuild, and source
recovery after deleting `.daena`. Native Tauri rendering and cross-target
release-golden execution remain the final desktop exit-gate evidence.
