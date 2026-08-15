# ADR 0019: Native physical-world current hydrology and geography

- Status: Implemented bounded derivation slice
- Date: 2026-08-15
- Scope: Iteration 5 in `NATIVE_MAP_GENERATOR.md`

## Decision

Derive current lakes, rivers, watersheds, islands, final coastline, and renderer
products from the immutable Iteration 4 elevation field, current climate, and
acyclic drainage graph. The result is a disposable `HydrologyField`; it is not
added to the locked `physical-world-v2` header or payload. Accepting a map
therefore stores exactly the same canonical source bytes while the trusted
native host can regenerate current geography after restart or cache deletion.
Acceptance also creates a separate empty GeoJSON authored-overlay asset and
locked physical layer definitions. The overlay is the only editable vector
asset for a physical map; replacing it cannot mutate or bypass validation of
the canonical `.pworld` source.

The basin hierarchy uses deterministic local minima and lowest valid spill
edges. Each basin records its minimum, spill point/elevation, volume to spill,
parent/destination, direct precipitation, runoff inflow, evaporation, storage,
outflow, and one of `dry`, `endorheic`, `active`, `overflowing`, or `merged`.
Endorheic storage is bounded; overflowing child basins carry their excess to a
parent or the ocean.

Water balance uses integer derived volumes at the public boundary. A bounded
fixed-point loop adjusts ocean level against the reference ocean inventory and
inland basin storage. The declared tolerance is 5,000 ppm of the reference
inventory, and non-convergence or excess balance is a stable
`physical.water.non-convergent` error.

Rivers select one deterministic primary downstream edge per channel cell from
the normalized Iteration 4 graph. This prevents ordinary bifurcation while
retaining junctions, lake entry, spill destinations, mouths, and endorheic
termination. Tributary segments may therefore terminate at a `junction`
destination; only the downstream continuation is a terminal river mouth.
Overflowing basins also receive one explicit `spillOutlet` segment from their
lake spill cell to the recorded ocean or parent-basin destination. That short
segment represents the water-surface spill over a sill rather than an
additional terrain-routing edge. Channels are simplified inside their routed
cell corridor and carry Strahler order. Watershed groups, lake polygons, and
water-aware coastline segments use the same wrapped grid and pole policy as
tectonic products.

The native preview exposes locked Exposed land, Ocean, Continental shelves,
Islands, Lakes, Rivers, Watersheds, and Bathymetric contours vector layers plus
a Hillshade raster overlay. Bathymetry, slopes, water levels, watershed IDs,
island IDs, basin metrics, river order, and water-balance metrics are available
from temporary generation jobs and from deterministic reopened-map derivation.

## Validation

The pure-Rust fixtures prove bounded basin and river geometry, deterministic
water balance within tolerance, watershed output, and current-world GeoJSON
layer production. Tauri tests prove the hydrology product boundary exposes
water levels, hillshade, bathymetry, watershed IDs, island IDs, lake masks,
basin metrics, and convergence without adding derived bytes to the canonical
source. Core acceptance tests also verify the separate overlay asset survives
clean rebuild and that locked physical layers cannot be changed through
vector-layer mutations.

Final hydrology remains current-world derivation only. Historical climate,
land ice, epoch playback, and cache-keyed geographic history remain Iteration
6 work.
