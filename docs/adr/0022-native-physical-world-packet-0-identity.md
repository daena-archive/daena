# ADR 0022: Native physical-world production crate and pair identity

- Status: Implemented Packet 0 boundary
- Date: 2026-08-15
- Scope: Packet 0 of the production corrective sequence in `NATIVE_MAP_GENERATOR.md`

## Decision

The pure numeric implementation is promoted to the production
`daena-physical` crate. It remains in the existing
`crates/daena-physical-spike` source directory for this migration slice so
source-path history and fixture commands remain stable. Its dependency set is
limited to pure-model support; Tauri command adaptation remains in `src-tauri`
and acceptance, descriptor validation, and identity construction remain in
`daena-core`.

Canonical physical state is a validated descriptor/source pair. The strict
`physical-world-v2` source remains immutable and is never reinterpreted. The
descriptor must carry the normalized evolution preset, reference water
inventory, and historical forcing; those values are mandatory and are never
reconstructed from current defaults.

`daena-core::maps::physical` is the sole authority for pair validation and
identity construction. It decodes source bytes before parsing descriptor
metadata, checks every duplicated provenance field independently, and reports
stable field-specific mismatch diagnostics. The identity manifest is encoded
with fixed field order, explicit integer widths, little-endian integers,
length-prefixed UTF-8 strings, and no floating-point or map values:

```text
SHA-256(
  "daena-physical-identity-v1\0"
  + u32_le(manifest byte length)
  + manifest bytes
  + u64_le(source byte length)
  + source bytes
)
```

The resulting lowercase `sha256:` value is opaque to Tauri and TypeScript.
It is included in generation status, physical acceptance responses, and
historical derived responses. Historical cache keys use this composite
identity plus the relevant derivation versions and normalized request inputs;
source content hashes remain storage/transfer hashes.

Accepted physical settings are immutable. Any identity-field change creates a
new temporary generation and accepted map rather than mutating an accepted
descriptor in place.

## v1 disposition

The current checkout contains no v1 decoder, v1 acceptance fixture, v1 source
asset, or v1 portable checkpoint fixture. ADR 0016 already records the hard
cut from `physical-world-v1` to v2, and the production validator preserves a
typed `physical.source.unsupported-version` diagnostic. Release packaging must
repeat the inventory against supported pre-release project data before calling
the no-v1-data conclusion final. If any accepted v1 data is discovered, the
release is blocked until a strict read-only adapter or an explicit
user-approved migration is provided; absent plate, crust, boundary, or
volcanic state must never be fabricated.

## Validation

Packet 0 fixtures cover deterministic identity, every manifest field,
source-byte changes, semantic JSON key reordering, mandatory water/forcing
metadata, malformed and unsupported source versions, duplicate-field
mismatches, acceptance idempotence, and identity preservation through
checkpoint reconstruction. The core validator is called by physical
acceptance, plugin physical-create commit, reopen/clean rebuild validation,
and native derived physical reads.
