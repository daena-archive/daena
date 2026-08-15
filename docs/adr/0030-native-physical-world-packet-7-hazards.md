# ADR 0030: Native physical-world Packet 7 hazard rate semantics

- Status: Implemented Packet 7 generator slice
- Date: 2026-08-15
- Scope: Packet 7 of the production corrective sequence in `NATIVE_MAP_GENERATOR.md`

## Decision

Replace ADR 0021's truncated strongest-source relative field in
`crates/daena-physical-spike/src/hazards.rs` with a complete annual-rate field.
`HAZARD_DERIVATION_VERSION` is `3`. The `physical-world-v2` layout and generator
version `11` are unchanged.

Each boundary segment contributes an annual generated earthquake rate from its
class, relative speed, and geodesic length. A distance cutoff is not used: every source contributes to every cell because
the spherical maximum angle still yields a material decay. The field kernel
quantizes geodesic angle from a unit-vector dot product so production grids
stay inside the generation budget while evaluating the complete source set.
Display GeoJSON still ranks at most `MAX_HAZARD_FEATURES` cells, but only after
the complete field and metrics exist.

Volcanic origin, activity class, and eruptions per model year are a versioned
interpretation of immutable v2 centers and divergent-boundary causes
(`VOLCANIC_SOURCE_DERIVATION_VERSION = 1`). The next source format that stores
volcanic records may persist those properties; v2 does not.

Event materialization version `2` draws `N ~ Poisson(Λ)` with
`Λ = total_annual_rate * interval_years`, then samples location from the
normalized full-field rate mass. Earthquakes use the bounded Gutenberg-Richter
model; eruptions use `persistent-rate-v2` and record a sampled center id.
`hazardSeed` remains an explicit input independent from geography. Changing the
display feature cap does not change field hashes or event samples.

## Validation

Packet 7 fixtures cover analytic single-source decay, source superposition,
spatial-index omission within the locked nano-event bound, boundary reversal
and seam symmetry, rate-mass conservation, render-cap independence, Poisson
mean over a fixed ensemble, magnitude-frequency ordering, and deterministic
replay. Canonical source hashes are unchanged; derived GeoJSON hashes update
because hazard sample properties now include annual rates from the complete
field.
