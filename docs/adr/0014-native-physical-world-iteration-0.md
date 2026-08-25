# ADR 0014: Native physical-world source and iteration-0 determinism

- Status: Accepted for the iteration-0 feasibility spike
- Date: 2026-08-15
- Scope: contract decisions for the disposable spike; production provider
  registration remains deferred

## Context

Daena already stores maps as ordinary `daena.maps:map` entities. The map
descriptor, content-addressed assets, `maps:layers`, normalized anchors,
revisions, request IDs, checkpoint export, and provider dispatch are existing
contracts. FMG remains the `azgaar-fmg` provider and Native Vector Maps remain
the `daena-vector` GeoJSON provider. The physical spike must prove its numeric
and source boundaries without adding a third production descriptor branch,
RPC method, SQLite table, or frontend mutation path.

The current Native Vector provenance contract is generator version `1` in the
TypeScript generator and Rust validator. The SDK declaration and one Tauri
fixture had drifted to versions `2 | 3` and `2`; iteration 0 reconciles those
active declarations to version `1` and guards the agreement in a focused test.

## Decisions

### 1. Provider tuple and descriptor shape

The tuple reserved for the later production provider is:

| Field             | Locked value                           |
| ----------------- | -------------------------------------- |
| Provider ID       | `daena-physical`                       |
| Adapter version   | `1`                                    |
| Source format     | `physical-world-v1`                    |
| Source filename   | `world.pworld`                         |
| Source MIME       | `application/vnd.daena.physical-world` |
| Generator ID      | `daena-physical-world`                 |
| Generator version | `1`                                    |

The future descriptor keeps schema version `1` and uses the existing map
identity and asset ownership model:

```json
{
  "schemaVersion": 1,
  "provider": {
    "id": "daena-physical",
    "adapterVersion": 1,
    "sourceFormat": "physical-world-v1"
  },
  "sourceAssetId": "<map-owned asset UUID>",
  "previewAssetId": null,
  "defaultView": { "center": [0.5, 0.5], "zoom": 1 },
  "generation": {
    "id": "daena-physical-world",
    "version": 1,
    "seed": 831429,
    "retryIndex": 0,
    "settings": {}
  }
}
```

This JSON is a reserved contract, not a production schema change in iteration 0. Existing FMG and vector descriptors remain byte- and behavior-compatible.

### 2. Canonical source container

The spike source is a fixed-header little-endian binary container. It is not a
PNG, rendered GeoJSON, or collection of land polygons.

| Offset | Width | Value                                                             |
| -----: | ----: | ----------------------------------------------------------------- |
|      0 |     8 | ASCII magic `DAENAPW1`                                            |
|      8 |     2 | format version `1` (`u16`)                                        |
|     10 |     2 | header length `48` (`u16`)                                        |
|     12 |     4 | grid width (`u32`)                                                |
|     16 |     4 | grid height (`u32`)                                               |
|     20 |     8 | planet radius in metres (`u64`)                                   |
|     28 |     4 | seed (`u32`)                                                      |
|     32 |     4 | retry index (`u32`)                                               |
|     36 |     4 | target land fraction in ppm (`u32`)                               |
|     40 |     4 | sea level relative to datum in millimetres (`i32`)                |
|     44 |     4 | sample count (`u32`)                                              |
|     48 | 4 × N | signed elevation/bathymetry samples in row-major order (`i32` mm) |

The decoder requires the exact magic, version, header length, dimensions,
sample count, byte length, and bounds. It rejects truncation, trailing bytes,
unsupported dimensions, and target fractions outside `(0, 1)`. There are no
floating-point values in canonical bytes. A future incompatible source needs a
new source format or adapter version; it may not reinterpret this one.

### 3. Numeric and deterministic policy

The canonical pipeline uses integer arithmetic for seed derivation, generated
elevations, sea-level selection, serialization, and derived coordinate
quantization. The spike uses a named `splitmix64`-style derivation with
explicit `u64` wrapping and row-major traversal. No global random stream,
unordered iteration, or platform-default serialization is permitted.

Spherical diagnostics use IEEE-754 `f64` in Rust's standard library with an
explicit reduction order. Exact cell area is:

```text
R² × deltaLongitude × (sin(northLatitude) - sin(southLatitude))
```

Great-circle distance uses wrapped-longitude haversine arithmetic. These
floating-point calculations are validation and feasibility measurements in
iteration 0; they do not select a different canonical byte sequence. Any
non-finite intermediate is a hard failure. Canonical fixture hashes are
SHA-256 over the exact bytes.

Longitude wraps modulo the grid width. The first and last latitude rows are
polar bands: their outer edge terminates at one north or south pole, and all
cells in a polar row are point-adjacent through that pole. Coastline output
uses integer microdegrees in `[-180, 180]` / `[-90, 90]`. Display conversion
for the later OpenLayers host uses the existing normalized-anchor boundary and a
local, offline projection; it never changes source bytes.

### 4. Authority and derived data

The future physical source is owned by the ordinary map entity's `maps`
namespace asset. SQLite and the runtime asset store remain live authority;
portable `assets/maps/` files remain checkpoint artifacts. Numeric terrain,
provenance, physical causes, and water parameters belong in the canonical
source. Coastlines, land/ocean masks, contours, climate, hydrology, and
hillshade are derived caches keyed by source hash and derivation version.

The spike writes no project files and has no SQLite, Tauri, plugin, or RPC
dependency. It only emits a local GeoJSON rendering fixture for offline host
validation.

### 5. Initial feasibility budgets

The spike locks a default grid of `64 × 32` and a maximum grid of `128 × 64`.
The maximum source is `32,816` bytes (`48 + 128 × 64 × 4`). The derived
coastline budget is `32,768` LineString features and `65,536` positions. The
initial physical model budgets are 64 plates, 256 cratons, 64 hotspots, 1,024
basins, 2,048 rivers, 4,096 lakes, 8,192 contour features, and 16,384 vector
features. These are feasibility limits, not a claim that later geology fits
them without measurement.

The default spike fixture is bounded at 16 MiB source bytes, 4 MiB local
GeoJSON bytes, and 128 MiB estimated working memory. The maximum workload must
remain below 2 seconds of release-mode generation and below 100 ms cancellation
latency on the reference desktop before iteration 0 can be closed. Debug-mode
timings are recorded separately and are not the release budget.

### 6. Algorithm classification

| Spike component                            | Classification                    |
| ------------------------------------------ | --------------------------------- |
| Spherical cell area and haversine distance | physical/geometric calculation    |
| Seeded integer elevation field             | procedural heuristic              |
| Weighted sea-level selection               | physical accounting approximation |
| Polar/seam topology policy                 | explicit topological contract     |
| Coastline edge extraction                  | procedural vectorization          |
| Binary source codec                        | storage contract                  |

The UI and help text must describe the seeded field and its geology as a
procedural approximation until later iterations replace it with reviewed
tectonic, climate, erosion, and hydrology models.

## Consequences

This ADR makes the irreversible iteration-0 source and numeric choices
explicit while avoiding production persistence changes. It also makes the
current Native Vector version drift testable before the physical descriptor
union is extended. Later iterations may add richer canonical physical causes,
but an incompatible encoding or interpretation requires a new version.

Packaged Tauri rendering, renderer failure behavior, map teardown, cross-target
golden execution, and release-mode maximum-grid measurements remain exit-gate
evidence; they are not inferred from the pure Rust tests.
