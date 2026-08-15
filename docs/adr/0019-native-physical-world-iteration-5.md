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

The basin hierarchy is a bounded raw-field equivalent rather than a full nested
Fill-Spill-Merge tree. Each land cell traces downhill to a deterministic local
sink. Each sink basin records its minimum and lowest valid spill edge, along
with volume to spill, parent/destination, direct precipitation, runoff inflow,
evaporation, storage, outflow, and one of `dry`, `endorheic`, `active`,
`overflowing`, or `merged`. Nested sub-depressions that are not represented by
separate downhill sinks are not emitted as separate lakes. Endorheic storage is
bounded; overflowing child basins carry their excess to a parent or the ocean.
The `merged` status means that excess outflow is carried into the parent basin;
it does not union the two basin topologies.

Water balance uses integer derived volumes at the public boundary. Evaporation
is a fixed `280,000 ppm` (`28%`) fraction of each basin's direct precipitation;
it is not state-dependent on water level, area, temperature, or humidity. Basin
storage is solved once from the initial sea level. A bounded ocean-level-only
fixed-point loop, limited to 24 iterations, adjusts ocean level against the
reference inventory and that frozen inland storage. The declared tolerance is
5,000 ppm of the reference inventory, and non-convergence or excess balance is
a stable `physical.water.non-convergent` error.

Rivers select one deterministic primary downstream edge per channel cell from
the normalized Iteration 4 graph. This prevents ordinary bifurcation while
retaining junctions, lake entry, spill destinations, mouths, and endorheic
termination. Tributary segments may therefore terminate at a `junction`
destination; only the downstream continuation is a terminal river mouth.
Overflowing basins also receive one explicit `spillOutlet` segment from their
lake spill cell to the recorded ocean or parent-basin destination. That short
segment represents the water-surface spill over a sill rather than an
additional terrain-routing edge. Channels are simplified inside their routed
cell corridor and carry Strahler order. Watershed groups and lake polygons use
the same wrapped grid and pole policy as tectonic products. Coastline and
bathymetry outputs are deterministic per-cell-edge boundary segments over
wrapped neighbors and fixed depth thresholds; they are not marching-squares
isolines.

The native preview exposes locked Exposed land, Ocean, Continental shelves,
Islands, Lakes, Rivers, Watersheds, and Bathymetric contours vector layers plus
a Hillshade raster overlay. Bathymetry, slopes, water levels, watershed IDs,
island IDs, basin metrics, river order, and water-balance metrics are available
from temporary generation jobs and from deterministic reopened-map derivation.

## Validation

The pure-Rust fixtures prove bounded sink/spill basin and river geometry,
deterministic water balance within tolerance, the ocean-level-only fixed-point
solve against frozen inland storage, watershed output, and current-world
GeoJSON layer production. Tauri tests prove the hydrology product boundary
exposes water levels, hillshade, bathymetry, watershed IDs, island IDs, lake
masks, basin metrics, and convergence without adding derived bytes to the
canonical source. Core acceptance tests also verify the separate overlay asset
survives clean rebuild and that locked physical layers cannot be changed
through vector-layer mutations.

Final hydrology remains current-world derivation only. Historical climate,
land ice, epoch playback, and cache-keyed geographic history remain Iteration
6 work.
