# ADR 0035: Native physical-world plate size, transform slip, and fault morphology

- Status: Implemented generator slice
- Date: 2026-08-15
- Scope: Tectonic plate geometry in `crates/daena-physical-spike/src/tectonics.rs`

## Decision

Keep Packet 2's even Fibonacci plate sites and spherical multi-source cost
field as the first ownership pass. After that pass, reshape plates and then
classify motion. The `physical-world-v2` layout is unchanged. The generator
version is `12` because plate ownership, crust growth, and named relief all
change.

Reshape is balanced and named-seeded (`SeedDomain::PlateReshape`): some plates
split along a second interior seed constrained to the parent, and the same
number of adjacent pairs merge. Compacted plate ids stay dense. The result is
size variation without changing the requested plate count when both operations
succeed.

A subset of adjacent pairs share an Euler pole and differ in angular speed
(`SeedDomain::PlateTransformSlip`). Relative motion along that contact is
strike-slip. Ownership in a bounded contact band is walked along the pole's
small-circle flow so plates grind past each other. Empty-plate outcomes are
reverted and ids are compacted again.

Boundary cost is no longer a single low-frequency perturbation. Edge cost sums
multi-octave noise, a higher-frequency wave, and extra cost along zero-crossings
of a global fault field. Extracted boundaries are still not jittered after
ownership is finalized.

## Validation

Fixtures cover complete ownership, reciprocal adjacency, pair-reversal
invariance, non-Voronoi morphology against final plate sites, plate-area
p95/p05 inequality, material transform-boundary share, and heading changes
along plate contacts. The generator version bump is the intended
golden-fixture change for the v12 source and derived hashes in
`scripts/maps-physical.test.mjs`.
