# ADR 0018: Native physical-world terrain evolution and drainage

- Status: Implemented bounded derivation slice
- Date: 2026-08-15
- Scope: Iteration 4 in `NATIVE_MAP_GENERATOR.md`

## Decision

Evolve the initial tectonic signed elevation field in a disposable pure-Rust
derivation. The final quantized signed field is then written into the existing
`physical-world-v2` elevation payload and becomes the canonical final terrain.
The v2 header and payload layout do not change. Routing surfaces, drainage
edges, accumulation, slope, and before/after relief arrays are derived
products and are never source bytes.

The routing surface uses Priority-Flood seeded by ocean cells. Depression
filling raises only the temporary routing surface to the lowest available
spill level; the natural elevation field retains its real depressions. Flow
uses a deterministic continuous-direction blend over up to four physically
downhill neighbors. Candidate weights use quantized routing-surface drop and
great-circle distance, ties use cell index, and the integer weights sum to one
million parts per million for every land source cell. Equal-height plateaus
use the Priority-Flood visitation order, which makes the graph acyclic without
inventing an elevation change.

Runoff accumulation is processed in reverse flood order. The final edge
receives the integer remainder from each source so direct runoff equals the
sum delivered to ocean outlets exactly. Uphill edges, cycles, missing land
sinks, unnormalized splits, and unaccounted runoff are hard errors with stable
`physical.hydrology.*` or `physical.numeric.*` codes.

Terrain evolution runs a fixed, versioned budget. `Young`, `Mature`, and `Old`
map to increasing step counts and stream-power work budgets; they do not
reinterpret authored historical time. Each step applies bounded stream-power
incision, boundary/volcano-derived continuing uplift, and hillslope relaxation
to a millimetre-quantized field. Downward relief loss is capped at
`25,000 mm` per step, non-finite or out-of-range values fail, and the initial
tectonic field remains available as a disposable before product.

## Native boundary and lifecycle

The trusted host exposes `project_physical_evolution` for completed temporary
jobs and `project_physical_derived_evolution` for accepted maps reopened from
canonical bytes. The reopened path deterministically regenerates the initial
tectonic field from the source identity, re-derives final climate, and rebuilds
the disposable before/after and drainage products. Deleting derived output
cannot rewrite the accepted source.

## Validation

The pure-Rust fixtures prove deterministic output, Priority-Flood visitation
coverage, acyclic reciprocal routing topology, exact normalized splits,
runoff conservation at ocean outlets, depression preservation, controlled
slope/discharge response, bounded per-step relief loss, and monotonically
increasing stream-power work across Young/Mature/Old settings. The evolved
highland-mask overlap metric preserves tectonic range orientation above the
locked 900,000 ppm fixture threshold.
The Tauri fixture proves the product boundary exposes routing, accumulation,
and before/after arrays without adding them to canonical source data. The
release benchmark records 1,205.8 ms maximum generation, 21.00 MiB peak RSS,
and remains under the 2 s / 128 MiB / 16 MiB source and derived-output
budgets. The physical source and diagnostic GeoJSON golden hashes were
intentionally refreshed for generator version 6 because final elevation now
includes the locked Mature evolution budget.

Final lakes, river vectorization, basin water balance, and native raster layer
presentation remain Iteration 5 work.
