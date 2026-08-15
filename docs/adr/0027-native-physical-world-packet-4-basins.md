# ADR 0027: Native physical-world Packet 4 nested basins and coupled water

- Status: Implemented Packet 4 generator slice
- Date: 2026-08-15
- Scope: Packet 4 of the production corrective sequence in `NATIVE_MAP_GENERATOR.md`

## Decision

Replace the Iteration 5 raw-sink basin model and ocean-level-only fixed point
in `crates/daena-physical-spike/src/hydrology.rs`. The `physical-world-v2`
layout is unchanged. The generator version is `10` and
`HYDROLOGY_DERIVATION_VERSION` is `3` because nested Fill-Spill-Merge labeling
and coupled lake/ocean storage change disposable hydrology products and the
derived coastline hashes.

Depression nodes are built by processing land cells in `(elevation, index)`
order against an ocean mask. Equal-saddle ties use that order. Each node
records exclusive cells, a subtree volume-versus-level curve from exact
spherical cell areas, the spill saddle, parent, children, and ocean
destination. A parent exists only after children meet at a shared saddle. Lake
cells never occupy terrain at or above the solved water level.

Basin water is solved bottom-up. Inflow is routed runoff plus child overflow.
Lake precipitation and evaporation are `depth * water-surface area`. Evaporation
uses versioned open-water depth, derived temperature, and maritime dryness; it
is not a constant fraction of precipitation. Overflowing children that have
reached a parent saddle are `merged`. Land ice remains a cell inventory lock
from the remaining liquid pool.

The outer water loop rebuilds ocean connectivity and inland storage, then
bisection-solves sea level to millimetre quantization against remaining ocean
water. The residual tolerance is the volume of one millimetre over the current
ocean area, plus per-cell rounding, not a percentage of inventory. Inland
storage is scaled only when it would exceed the inventory after a minimum
ocean is reserved. Rivers still select one downstream channel; spill outlets
are emitted for overflowing nodes that are not merged children.

## Validation

Packet 4 fixtures cover conserved water balance, nested parent/child
depressions, coastal capture when sea level rises, chained overflow or
endorheic representation, deterministic rebuild, and land-ice freeze/thaw
rules. The generator version bump is the intended golden-fixture change for
the v10 source and derived hashes in `scripts/maps-physical.test.mjs`.
