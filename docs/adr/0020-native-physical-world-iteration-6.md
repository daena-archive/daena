# ADR 0020: Native physical-world historical climate playback

- Status: Implemented
- Date: 2026-08-15
- Scope: Iteration 6 in `NATIVE_MAP_GENERATOR.md`

## Decision

Historical playback is a disposable interpretation of the accepted final
physical field. It never edits the `physical-world-v2` source, final
elevations, plate assignments, boundary classifications, or volcanic centers.
Each accepted physical generation descriptor persists a versioned
`historicalForcing` object in its settings. Older descriptors remain valid and
use the deterministic forcing defaults derived from their seed and retry index
until they are regenerated.

The forcing is an explicitly bounded low-frequency triangle wave over an
integer physical offset. It is a useful geographic playback heuristic, not an
Earth-orbit or paleoclimate simulation. The parameters include temperature
amplitude, period, phase, land-ice amplitude, ice response time, and thermal
expansion coefficient. The reference epoch is zero offset; no floating sea-ice
reservoir is modeled or added to the sea-level inventory.

At an epoch, the model derives an equilibrium land-ice volume from the current
temperature and an actual land-ice volume from the lagged temperature. Thermal
expansion is signed and bounded. The effective ocean inventory is:

```text
reference water - lagged land ice + thermal expansion
```

The derived sea level first selects a bounded ocean-volume candidate on the
immutable terrain. Climate, drainage, basin storage, sea level, rivers, lakes,
coastline, shelves, bathymetry, and island products are then recomputed from
that epoch field. The existing Iteration 5 basin solve remains the inland-water
solver; historical conservation reports the effective ocean/inland total plus
land ice minus signed thermal expansion against the reference inventory.

Epoch responses use a session-local bounded cache of at most 16 products. The
cache key contains the canonical source SHA-256, historical derivation version,
normalized integer epoch, and persisted forcing signature. A missing or stale
entry recomputes from the canonical source. The cache is disposable, is
cleared by restart/eviction, and never writes or rewrites source assets.

The historical API accepts physical offsets from `-10,000,000` to `+10,000,000`
years through `MAX_HISTORICAL_OFFSET_YEARS`. The accepted native map exposes a
deliberately narrower physical-only integer-years slider from `-100,000` to
`+100,000`; the UI bound is a usability limit, not the derivation contract.
Slider input is debounced by 180 ms. A per-map generation token cancels
superseded pure-Rust work at progress checkpoints, and the Tauri layer emits
`physical-historical-progress` events correlated by map and request ID so the
UI can show the current derivation phase and counters. The UI applies only the
newest response. The Tauri response includes an explicit
`physical-offset-years` chronology mapping contract whose reference is the
accepted source. Any future Timeline integration must map through Daena's
shared chronology/date-precision contract instead of inventing absent calendar
components or JavaScript timestamps.

The native editor loads a physical map through the epoch-zero derived command;
epoch zero is an exact replay of the accepted reference climate, drainage, and
hydrology. The older GeoJSON and hydrology commands remain available because
the standalone physical-map preview still consumes them.

## Validation

The physical crate tests prove deterministic forcing, the inclusive API bound,
zero-offset reference replay equality, cooling/warming land-ice and
thermal-expansion signs, lagged history, water conservation within the existing
hydrology tolerance, land/shelf exposure and island-connectivity changes,
repeated derived equality, and byte-level preservation of the canonical field.
Tauri tests cover chronology and derived-hash provenance, disposable cache
clearing, superseded-request cancellation, and the typed generation/command
boundary. The native editor subscribes to correlated phase events and renders
phase/counter status; cache hits complete without emitting derivation phases.
The Svelte contract check covers the accepted-map client method and playback
control. The existing hydrology derivation supplies the epoch-specific shelf,
coastline, lake, river, watershed, and island connectivity products.

The cache is intentionally session-local rather than disk-persistent: explicit
cache clearing, eviction, restart, or a source/forcing key change recomputes
the same products without touching canonical assets. Timeline remains an
optional consumer of the explicit physical-offset chronology mapping; no date
components are fabricated when it is absent.
