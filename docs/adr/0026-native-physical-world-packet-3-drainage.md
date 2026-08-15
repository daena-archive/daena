# ADR 0026: Native physical-world Packet 3 drainage and stream-power erosion

- Status: Implemented Packet 3 generator slice
- Date: 2026-08-15
- Scope: Packet 3 of the production corrective sequence in `NATIVE_MAP_GENERATOR.md`

## Decision

Replace the Iteration 4 routing and erosion surrogates in
`crates/daena-physical-spike/src/evolution.rs`. The `physical-world-v2` layout
is unchanged. The generator version is `9` and `EVOLUTION_DERIVATION_VERSION`
is `2` because D-infinity routing and SI stream-power incision change the
canonical evolved elevation field and the disposable drainage products.

Priority-Flood still builds a temporary routing surface. Fill depth and flood
order are recorded. Real elevation is never written during routing. Ocean cells
seed the flood; equal-priority ties use deterministic cell index order.

Flow uses D-infinity facets on the local tangent plane. Polar-row neighbors
whose tangent offset is below one metre are dropped so a pole never uses a
zero-width longitude step. Split weights quantize to integer ppm that sum to
exactly one million, with remainder assigned to the larger unquantized weight
and lower cell index on a tie. Plateaus follow flood order. The graph is
acyclic. Accumulation is exact integer conservation of annual runoff volume.

Incision evaluates `K * Q^m * S^n` with `Q` as cubic metres per second
(`annual m³ / 31,557,600`), `m = 0.5`, and `n = 1.0`. `K` is stored as
`erodibility_e12` in the versioned Young/Mature/Old preset together with
timestep, step count, climate cadence, uplift rate, diffusivity, and the
fraction of local removable relief that may be cut in one step. Hillslope
diffusion is explicit and scaled so the outgoing lambda sum does not exceed
`0.5`. Continuing tectonic uplift is applied after incision and diffusion.
Climate and routing are recomputed every cadence step. Non-finite values,
negative discharge, and out-of-range elevations are rejected.

## Validation

Packet 3 fixtures cover Priority-Flood fill depth, exact runoff conservation,
normalized split weights, monotonic stream-power response to slope and
discharge, Young/Mature/Old work ordering, analytic planes/cones/ridges at
several longitudes, and coarse-to-finer drainage-metric convergence. The
generator version bump is the intended golden-fixture change for the v9 source
and derived hashes in `scripts/maps-physical.test.mjs`.
