# Daena Native Physical Map Generator implementation plan

## Status and authority

This document is the implementation plan for Daena's native physical-world
generator. It turns the physical-model requirements into bounded, sequential
iterations for implementation agents.

It is subordinate to:

- [`ARCHITECTURE.md`](./ARCHITECTURE.md), which defines core, host, module, and
  plugin boundaries;
- [`STORAGE.md`](./STORAGE.md), which defines SQLite runtime authority,
  content-addressed assets, portable checkpoints, and rebuild behavior;
- [`MAP_INTEGRATION_PLAN.md`](./MAP_INTEGRATION_PLAN.md), which defines shared
  map entities, anchors, navigation, hierarchy, and semantic overlays; and
- [`NATIVE_MAP_INTEGRATION.md`](./NATIVE_MAP_INTEGRATION.md), which defines the
  existing editable `daena-vector` provider.

If this plan conflicts with one of those authorities, stop the active
iteration and reconcile the contract before writing more code.

The physical generator is additive. It does not migrate, reinterpret, or
silently regenerate existing `azgaar-fmg`, `daena-vector`, or image maps.
Existing accepted Native Vector Map sources remain canonical GeoJSON and remain
editable under their current provider contract.

## How agents must execute this plan

1. Work on one iteration at a time. Do not begin the next iteration until every
   item in the current exit gate has direct evidence.
2. At the start of an iteration, inspect the current worktree and the live
   contracts named in that iteration. Preserve unrelated work.
3. Write a short vertical-slice plan before editing. Name the production path,
   persisted contract, tests, and rendered boundary that will change.
4. Keep generator behavior deterministic and bounded. Do not replace defined
   metrics with subjective visual tuning or repeatedly adjust heuristics after
   the exit gate passes.
5. Add the smallest fixtures that prove the contract. Never refresh a golden
   fixture merely to make a changed algorithm pass; explain the intended
   generator-version change first.
6. Run focused checks after each meaningful slice. Run the iteration's full
   gate after the final edit, followed by `rtk git diff --check` and
   `rtk git status --short`.
7. Treat a build, type check, or unit test as insufficient proof of native
   rendering, cancellation, persistence, restart behavior, or clean-checkpoint
   recovery. Exercise those boundaries directly where the exit gate names
   them.
8. Stop at the exit gate and report remaining deferred work. Do not continue
   into the next iteration without a new instruction.

## Product outcome

An author can configure a world, generate exactly one complete physical map in
the Rust backend, inspect it, accept it, and reopen it later with the same
physical geography. A reroll discards the current temporary result and runs
the pipeline once with another deterministic seed; it does not generate or
score a hidden batch of candidates.

The accepted world has one immutable, signed elevation/bathymetry field. Land,
ocean, coastlines, islands, lakes, rivers, land ice, climate, relief, and
hazards derive from that field and its persisted physical causes. Authored countries,
cultures, settlements, routes, borders, annotations, and entity links remain
normal Daena layers and shared records above the immutable physical base.

The full plan is complete when:

- generation is deterministic from generator version, seed, retry index, and
  settings on every supported desktop target;
- one generation request creates at most one temporary world and acceptance
  commits exactly one normal `daena.maps:map` entity;
- the canonical source contains numeric signed elevation, not a PNG, rendered
  GeoJSON, or a collection of land polygons;
- physical calculations use spherical distance, exact spherical cell area,
  longitude wrapping, and an explicit pole policy;
- current and historical geography satisfy the declared water, terrain,
  hydrology, and topology invariants;
- cancellation, failure, or invalid generation creates no entity, asset, or
  portable generation;
- a clean checkpoint reconstructs the same map after deleting `.daena/`;
- derived caches can be removed and rebuilt without changing canonical data;
  and
- the packaged application remains offline, responsive, and within measured
  CPU, memory, source-size, and derived-data budgets.

## Current checkout baseline

The current checkout already has two distinct map-generation/editing paths:

- FMG is an isolated provider-owned child webview.
- `daena-vector` is a trusted, in-window MapLibre/Terra Draw editor whose
  canonical source is GeoJSON. Its current TypeScript landmass generator
  creates six candidates and accepts one as editable base polygons.

The physical generator must not be fitted into the `daena-vector` GeoJSON
source format. That would make a derived coastline authoritative and discard
the numeric terrain, bathymetry, tectonic, and water state required here.

At the time of this rewrite, the focused native-vector suite passes, but its
provenance declarations are not aligned: the TypeScript generator and Rust
validator use version `1`, while `packages/plugin-sdk/src/maps.ts` declares
versions `2 | 3`. Iteration 0 must reconcile that pre-existing contract drift
and add coverage that would catch it before extending the map descriptor union.

The implementation therefore adds a third provider variant, provisionally
locked as:

```json
{
  "schemaVersion": 1,
  "provider": {
    "id": "daena-physical",
    "adapterVersion": 2,
    "sourceFormat": "physical-world-v2"
  },
  "sourceAssetId": "<asset UUID>",
  "previewAssetId": null,
  "defaultView": {
    "center": [0.5, 0.5],
    "zoom": 1
  },
  "generation": {
    "id": "daena-physical-world",
    "version": 6,
    "seed": 831429,
    "retryIndex": 0,
    "settings": {}
  }
}
```

Iteration 0 must confirm the exact provider tuple, MIME type, filename,
encoding, and descriptor schema in an ADR before they become stored contracts.
Later iterations must not change those values in place. An incompatible source
or algorithm change requires a new adapter or generator version.

When the first physical vertical slice passes its exit gate, the Create Map
surface replaces the current six-candidate landmass action with the single-map
physical flow. `daena-vector` remains available for image import and authored
vector maps; its already-accepted generated maps remain readable and editable.

## Non-negotiable architecture decisions

### Shared map identity, separate provider source

A physical world is a normal `daena.maps:map` entity. It reuses existing map
navigation, hierarchy, normalized anchors, semantic overlays, relationships,
asset ownership, revisions, request IDs, checkpoint export, and recovery.

It does not introduce:

- a physical-map identity table;
- a second project root or direct portable-file writer;
- a private Maps database;
- frontend-owned canonical data;
- provider-specific entity identity; or
- a plugin-only persistence route.

The source asset belongs to the map entity in the existing `maps` namespace.
SQLite and the content-addressed runtime asset store are live authority. The
portable copy under `assets/maps/` is a generated checkpoint artifact.

### Rust owns generation and physical derivation

All large numeric-field work runs in Rust:

- seeded random generation;
- spherical grid and geodesic calculations;
- tectonic plates and synthetic motion;
- crust, elevation, and bathymetry;
- climate, runoff, and erosion;
- sea-level and water-volume solving;
- depression analysis and hydrology;
- contour extraction and vectorization;
- historical geography and hazards; and
- physical validation.

The Svelte/Tauri layer owns configuration, progress, cancellation, temporary
result presentation, explicit acceptance/reroll, time selection, interaction,
and MapLibre rendering. Frontend validation may improve feedback but never
bypasses Rust validation.

Put the pure model in a Rust crate or module that has no Tauri, SQLite, UI, or
plugin-host dependency. `daena-core` adapts validated model bytes to normal map
entities/assets and transactions. `src-tauri` adapts long-running jobs to the
trusted shell and must run CPU work away from the async/UI thread. Host
generation and historical derivation run inside `spawn_blocking`. The numeric
path is sequential; unordered parallel reductions are not used in the canonical
pipeline.

### Canonical physical truth and derived presentation

The single canonical source asset stores the information required to define
the accepted physical world:

- versioned grid and planet parameters;
- signed final elevation/bathymetry samples as numeric data;
- tectonic plate definitions and synthetic motion;
- plate ownership, crust type, boundary geometry, and boundary classification;
- persistent volcanic centers and hotspots;
- total water inventory and current/reference physical parameters;
- climate-history parameters needed for historical derivation;
- generator ID/version, seed, retry index, and settings; and
- any other value that cannot be reproduced from those fields without rerunning
  the accepted generator version.

Temporary generation fields such as uplift, crust age, rainfall, runoff,
slope, flow direction, flow accumulation, depression work queues, erosion rate,
ice work state, and intermediate terrain do not become canonical unless a
later reviewed feature truly needs them.

Derived output includes:

- land and ocean masks;
- coastlines, land polygons, islands, and exposed shelves;
- bathymetric contours, hillshade, slope, and relief;
- temperature, precipitation, runoff, and climate classes;
- rivers, lakes, watersheds, and drainage basins;
- land-ice cells, water-equivalent thickness, and ice polygons; and
- earthquake and volcanic hazard fields.

MapLibre consumes derived GeoJSON and bounded derived raster products. These
representations may be cached using the canonical source hash, derivation
version, and epoch. They are never authoritative and must be safely deletable.
No physical layer may exist only as a PNG or JPEG.

Physical maps render in MapLibre with `projection: "globe"`. Authored
`daena-vector` maps remain Web Mercator with `maxPitch: 0`. The physical
hillshade raster classifies the largest connected below-sea-level component as
ocean, hides inland water smaller than eight cells so continents do not
speckle, and paints land ice over remaining land. Diagnostic vector layers,
including `ice`, start hidden. Changing layer visibility or historical year
updates GeoJSON and raster sources in place and must not reset camera location
or zoom.

### Immutable physical base, editable authored overlays

After acceptance, the physical source is immutable. Daena does not continue
tectonic or erosional evolution, and the first product does not let authors
paint, sculpt, or replace the generated elevation field.

Reroll creates a new temporary world. It never mutates an accepted source.
Normal Daena vector, raster, semantic, and relationship-backed layers may be
created above the physical map without flattening or rewriting the source.

### One visible result, no hidden candidate scoring

One request executes one pipeline and produces one temporary result. A hard
invalid result may retry internally with a deterministic retry index derived
from the original seed, but:

- retries are bounded;
- the retry index is recorded in provenance;
- every failed attempt is validated for a named hard invariant;
- retries are not a visual-quality tournament; and
- the user sees and accepts only one result.

If all retries fail, generation ends with a typed diagnostic and no durable
mutation.

### Determinism is a stored contract

Generation is a function of:

```text
generator version + seed + retry index + normalized settings
```

Each subsystem receives a named derived seed, at minimum:

```text
plates
continental-crust
plate-motion
tectonic-relief
hotspots
terrain-detail
climate
erosion
hydrology
hazards
```

Do not use one global sequential random stream. Adding a random draw to a later
subsystem must not alter earlier output. Do not use nondeterministic hash-map
iteration, unordered parallel reductions, platform-default transcendentals, or
`Math.random` in the canonical pipeline.

Iteration 0 must choose and prove the deterministic numeric policy: arithmetic
implementation, reduction order, quantization points, NaN/overflow handling,
and canonical byte encoding. Exact source hashes across supported targets are
the acceptance standard unless the ADR explicitly narrows the supported
target set before production data exists.

## Physical model contract

### One signed elevation field

The fundamental field is:

```text
(longitude, latitude) -> signed elevation in metres relative to datum
```

Positive values are terrain above the datum and negative values are seafloor.
There is no separate land-height and ocean-depth model.

At sea level `s` and terrain elevation `z`:

```text
land        when z > s
coastline   where z = s
ocean depth max(0, s - z)
```

Islands are connected components of `z > s`. Lakes are water-filled drainage
depressions. Rivers are extracted from accumulated runoff. None is placed as
independent decorative geometry.

### Spherical grid

A longitude/latitude raster is the initial storage and computation structure,
but calculations treat it as a sphere.

For longitude step `delta_lon`, latitude edges `lat_s` and `lat_n`, planet
radius `R`, and a cell-center latitude `lat`:

```text
horizontal width = R * cos(lat) * delta_lon
vertical height   = R * delta_lat
cell area         = R^2 * delta_lon * (sin(lat_n) - sin(lat_s))
```

Angles are in radians. Exact spherical cell area is required for land
percentage, water volume, precipitation volume, runoff, drainage area, and
erosion inputs.

Longitude wraps. Column `0` and the final column are neighbors. Calculations
must have no antimeridian seam. Iteration 0 must define pole-cell adjacency and
sampling explicitly; no production algorithm may silently treat the first and
last latitude rows as ordinary flat edges.

Great-circle distance uses wrapped longitude difference and a numerically
stable haversine or equivalent spherical formula. Use it for plate influence,
craton growth, hotspot/hazard distance, and geographic validation.

### Causal dependency order

The implementation must preserve this dependency order:

```text
seed/settings
  -> spherical plates and synthetic motion
  -> continental/oceanic crust and classified boundaries
  -> tectonic deformation and initial signed elevation
  -> initial climate and runoff
  -> erosion and final signed elevation
  -> water inventory and reference sea level
  -> depression hierarchy and drainage
  -> land ice locked from the global pool, then inland water and ocean level
  -> rivers, lakes, coastline, and derived climate products
  -> physical validation
  -> temporary result
  -> explicit acceptance
```

Historical derivation starts from the accepted final terrain and persistent
physical parameters. It never moves plates or changes terrain:

```text
climate forcing at t
  -> temperature offset and thermal expansion
  -> climate field (per-cell temperature and precipitation)
  -> land ice on freezing land cells, subtracted from the global pool
  -> remaining liquid inventory
  -> sea level, lakes, rivers, and coastline
```

Hazards derive separately from accepted plate boundaries and volcanic centers.

### Tectonics and initial terrain

Use approximately even spherical sampling for plate seeds, such as spherical
best-candidate or Poisson-disc sampling. Spherical Voronoi ownership is an
acceptable scaffold, but boundaries must be deterministically irregularized
without introducing a longitude seam.

Each plate stores a rotation axis and angular speed. At surface position `p`,
derive synthetic velocity from `R * cross(angular_velocity, p)`. At a shared
boundary, split relative velocity into normal and tangential components and
classify it as convergent, divergent, or transform using versioned thresholds.
The plates do not move after generation.

Continents originate as continental crust grown from multiple related cratons,
not as land polygons or a thresholded noise field. Growth cost combines
geodesic distance, low-frequency correlated variation, plate membership,
related-craton attraction, and unrelated-group repulsion. Detached continental
terranes may occur. Continental crust normally exceeds the desired exposed
land area so shelves and drowned continental regions exist.

Initial signed elevation combines:

```text
crust baseline
+ optional simplified isostatic contribution
+ continental collision uplift
+ subduction trench and inland volcanic-arc terms
+ oceanic-island-arc and ridge terms
+ continental rift subsidence and shoulders
+ transform-boundary minor relief
+ hotspot uplift
+ restrained multiscale terrain detail
```

Boundary influence decays smoothly with geodesic distance, initially using a
versioned bell-shaped kernel. Convergence strength controls collision uplift.
Oceanic-continental subduction produces a trench, an uplifted margin, and an
inland-offset volcanic arc. Divergent oceanic boundaries create ridges and a
bounded age/depth relationship. Continental divergence creates a rift and
weaker shoulders. Transform boundaries never produce collision-scale relief.

Noise may modulate resistance, boundary irregularity, mountain texture,
rainfall, and local relief. Its amplitude stays below structural tectonic
relief at continental scales. Noise never directly decides whether a continent
exists.

### Climate, runoff, and terrain evolution

The current-climate model needs enough structure for wet/dry regions,
continental drying, orographic precipitation, and rain shadows; it is not a
full atmospheric or ocean-circulation simulation.

Temperature combines global temperature, latitude cooling, altitude cooling,
and maritime moderation. Moisture moves through broad prevailing-wind bands,
gains over ocean, decays with travel distance, precipitates while moving
uphill, and loses the precipitated amount. All transport steps use physical
distance and fixed traversal/convergence rules.

Local runoff is precipitation times an effective runoff coefficient. Runoff
volume multiplies local runoff by exact cell area.

Terrain evolution is specified conceptually by a stream-power-style model:

```text
incision = erodibility * discharge^m * slope^n
new terrain = old terrain + limited active uplift - incision
              + hillslope relaxation
```

Initial calibration begins near `m = 0.5` and `n = 1.0`, but these are
versioned empirical parameters, not literal constants of nature. Iteration 4
does not evaluate that power law literally. Its versioned bounded v1 surrogate
uses accumulated runoff volume as discharge and applies:

```text
discharge_scale = clamp(
  ln(1 + discharge_m3_per_year) / ln(1 + 1,000,000,000), 0, 1
)
slope_scale = slope_ppm / 1,000,000
incision_mm = round(
  (stream_power_ppm / 1,000,000) * 30,000
  * discharge_scale * slope_scale
)
```

This preserves a bounded monotonic response to discharge and slope; it is not
a calibrated `discharge^0.5 * slope^1.0` evaluation. `stream_power_ppm` and
the evolution budgets are versioned empirical controls. Replacing this
surrogate with a literal power law requires a new derivation decision and
version. `Young`, `Mature`, and `Old` settings select coherent iteration
counts, relaxation, and tectonic persistence; they are not literal geological
ages.

For erosion routing, use Priority-Flood or an equivalent algorithm to build a
temporary drainage-compatible surface. Never overwrite real depressions in
the canonical elevation field.

### Water inventory, sea level, and hydrology

The reference land-coverage setting uses spherical cell area to solve an
initial sea level. That result establishes the initial total water inventory;
land coverage is not enforced by editing terrain or counting raster cells.

For proposed sea level `s`:

```text
ocean volume = sum(cell area * max(0, s - terrain elevation))
```

Solve the monotonic volume equation with bounded root finding and explicit
tolerance/iteration limits.

Current total water is conserved among ocean water, land-based ice, and
persistent inland water. Hydrology derivation version 2 locks land ice from
the same inventory before the ocean-level solve. A land cell becomes ice when
its elevation is above sea level, its temperature is below 0 °C, and its
precipitation is at least 80 mm/year. Water-equivalent thickness scales with
coldness and precipitation, is capped, and is dropped for isolated patches
smaller than eight cells on production-width grids. That volume leaves the
liquid pool. When the cell is at or above 0 °C, it holds no ice and the water
remains in (or returns to) the pool. Ice does not flow, does not alter the
canonical elevation field, and does not interact with land. There is no sea
ice; ocean cells stay in the ocean reservoir.

Iteration 5 then uses a bounded, one-directional ocean-level fixed-point solve:
derive basin assignments and inland storage at the initial sea level, skip
inland storage for basins whose minimum cell is ice, then adjust only ocean
level against that frozen inland storage plus remaining liquid inventory until
the declared tolerance is met.

Flow routing uses D-infinity or another reviewed continuous-direction method;
basic D8 is not acceptable because it produces visible grid alignment.
Accumulation weights sum to one, use physical runoff volume, and form an
acyclic downstream graph.

The final Iteration 5 depression hierarchy is a bounded raw-field equivalent
of a depression hierarchy, not a full nested Fill-Spill-Merge tree. Each land
cell traces downhill to a deterministic local sink, and each sink basin uses
its lowest valid spill edge. The hierarchy records basin minima, spill
points/elevations, volume-to-spill, parents, and downstream basin/ocean
destinations. A basin's water balance includes inflow, direct precipitation,
fixed evaporation equal to 28% of direct precipitation, and outlet discharge.
Outcomes include dry, endorheic, overflowing, and merged lakes; `merged` means
that excess outflow is carried into a parent basin, not that basin topologies
are unioned. Nested sub-depressions that are not represented by separate
downhill sinks are not emitted as separate lakes.

River channels cross a normalized discharge threshold. Their topology obeys:

- downstream elevation does not increase on the routing surface;
- tributaries may merge, while ordinary rivers do not split;
- every terminal reaches ocean, lake, or a legitimate endorheic basin;
- lake outlets start at calculated spill points;
- mouths meet the derived ocean boundary; and
- the graph has no cycles.

Vectorization preserves sources, junctions, lake entries/exits, and mouths.
Simplification may not move a river outside its drainage corridor. Strahler
order and discharge drive rendering hierarchy. Do not add synthetic sine-wave
meanders.

### Historical climate and sea level

Historical sea level derives from smooth, low-frequency climate forcing, not
from an independently random curve. Persist the oscillation amplitudes,
periods, and phase offsets or an equivalently sufficient versioned model.

Each epoch applies a global temperature offset to the climate field, then
re-derives hydrology. Land ice is the hydrology cell product described above:
colder climate freezes more land water out of the pool and lowers sea level;
warmer climate melts it back. Floating sea ice is excluded. Thermal expansion
applies a bounded effective coefficient to the liquid inventory. The ADR 0020
logistic `land_ice_equilibrium_m3` value remains a forcing-lag diagnostic; it
is not subtracted from inventory before hydrology. Reported `land_ice_m3` and
effective ocean water come from the hydrology solve.

Every epoch then resolves sea level, lake storage, climate, and hydrology from
the immutable final terrain.

The first historical contract uses an integer physical offset from a persisted
reference epoch. Integration with Timeline must use Daena's shared calendar and
date-precision contract; it may not invent missing month/day/time components or
hardcode JavaScript timestamp semantics.

Iteration 6 implements this contract as a versioned `historicalForcing` object
inside accepted physical generation metadata and an integer `epochOffsetYears`
derived-product command. The API accepts offsets through ±10,000,000 years;
the current native control intentionally exposes the narrower ±100,000-year
usability range. Historical derivation emits correlated phase/counter events
for the native UI, while a cache hit may complete without phase events. The
native editor loads physical maps through exact epoch-zero replay; the
standalone physical preview retains its separate GeoJSON/hydrology commands.
The current native control is intentionally physical years only; it does not
silently reinterpret or fabricate shared Timeline date components.

### Hazards and materialized history

Earthquake hazard combines a background rate with boundary type, relative plate
speed, and distance decay. Volcanic hazard derives from subduction arcs,
hotspots, rifts, and spreading systems.

Optional event generation samples bounded earthquake and eruption histories
from those persistent hazard fields using named, versioned statistical models.
Once an event is materialized, it becomes normal durable Daena data linked to
the map and, where appropriate, Timeline/Lore entities. It is not a derived
cache and is never regenerated because a hazard algorithm changes.

## User-facing settings and provenance

Keep the initial settings high level:

| Setting | Meaning |
| --- | --- |
| Seed | Reproducible base input |
| Land coverage | Reference exposed land fraction |
| Major landmasses | Approximate continental grouping |
| World age | Erosion maturity preset |
| Tectonic activity | Coherent uplift, rift, and volcanic strength |
| Hydrology | Coherent runoff/river/lake abundance preset |
| Island activity | Volcanic-arc, hotspot, and fragment tendency |
| Climate variability | Historical forcing and sea-level range preset |

Raw plate counts, densities, speeds, kernels, erosion coefficients, runoff
coefficients, thresholds, response times, and numerical tolerances remain
versioned implementation parameters. Presets must map to complete parameter
groups in Rust; the frontend does not assemble physical coefficients.

Provenance records generator ID/version, seed, retry index, normalized settings,
planet parameters, and canonical source schema version. Accepted bytes remain
authoritative even if later generator versions change.

## Job, acceptance, and failure contract

Generation is expensive and must not hold a SQLite transaction or the project
session lock while computing.

The trusted host flow is:

```text
normalize settings and request ID
  -> start project/session-bound Rust job
  -> stream bounded phase progress
  -> validate complete physical result
  -> write validated temporary source and derived preview
  -> present exactly one result
  -> accept: install runtime asset, then commit entity/descriptor/asset/fields
     atomically with an idempotency receipt
  -> reroll/cancel/close: delete temporary state
```

Temporary handles are project-bound, session-bound, unguessable, expiring, and
single-purpose. Acceptance revalidates size, hash, source schema, provenance,
and physical invariants. A crash or failure produces either no accepted map or
one complete accepted map; never an entity with missing bytes or mixed source
metadata.

Progress phases are stable user-facing categories, not an event for every
numeric iteration:

```text
Building tectonic structure
Building terrain
Calculating climate
Eroding landscape
Calculating water
Building rivers and lakes
Preparing geography
Validating world
```

Cancellation is checked at bounded intervals inside every long loop. Project
close, workspace switch, app shutdown, or a newer reroll cancels the job and
reclaims temporary files/memory. A late completion event may not attach to a
new project or superseding request.

Use typed errors with stable codes. The initial vocabulary should distinguish:

```text
physical.generator.invalid-settings
physical.generator.unsupported-version
physical.generator.cancelled
physical.generator.retry-exhausted
physical.source.invalid
physical.source.unsupported-version
physical.numeric.non-finite
physical.numeric.non-convergent
physical.water.non-convergent
physical.hydrology.cycle
physical.geometry.invalid
physical.limit.exceeded
physical.renderer.unavailable
asset.revision-conflict
```

Diagnostics name the stage and bounded location/index where useful. They do not
include complete source bytes, internal runtime paths, or SQLite details.

## Resource and numerical budgets

Iteration 0 must measure and lock budgets before production generation. At
minimum define:

- supported grid sizes and one default generation size;
- maximum plate, craton, hotspot, basin, river, lake, contour, and vector
  feature counts;
- canonical source and derived-output byte limits;
- peak generation and open-map memory limits;
- maximum wall time for default and maximum presets on reference hardware;
- progress and cancellation latency;
- maximum erosion, climate, water fixed-point, and root-finding iterations;
- tolerances for land coverage, water balance, lake/sea-level convergence, and
  geometric simplification; and
- MapLibre source/update and frame-time budgets for the default world.

Every iterative calculation defines timestep, convergence criterion, maximum
iterations, acceptable error, finite-value checks, and failure behavior.
Erosion may not remove an unreasonable fraction of local relief in one step.
Root finding stays bracketed. Hydrology is checked acyclic. Over-budget or
unstable worlds fail rather than emitting corrupted geography.

Increasing resolution is not an acceptable substitute for a weak algorithm.
Start at a moderate measured resolution and add a level-of-detail strategy only
after profiling proves it necessary.

The production lock after ADR 0024 is:

| Bound | Value |
| --- | ---: |
| Production grid (default and maximum) | 384 × 192 |
| Canonical source | 128 MiB |
| Derived GeoJSON | 256 MiB |
| Generation wall time | 8 s |
| Working memory | 128 MiB |

`256 x 128` and `512 x 256` remain supported preview grids. `1024 x 512` and
`2048 x 1024` remain preview-only. The former 16 MiB host ceiling is not a
`physical-world-v2` layout constraint and is not retained. Measured fixture
bytes and hashes live in [`docs/maps/physical-map-budgets.md`](./maps/physical-map-budgets.md)
and the golden gate.

## Verification strategy

Maintain a small fixed fixture matrix that covers:

- low, medium, and high land coverage;
- one and several major landmasses;
- low and high tectonic/island activity;
- young, mature, and old terrain;
- wet and dry hydrology presets;
- at least one seam-stressing and one pole-stressing seed; and
- at least one hard-invalid fixture for each retry/failure path.

For each applicable fixture record canonical source hash, key field hashes,
derived geometry hashes, invariant metrics, and generator/derivation versions.
Visual snapshots support rendered regression checks but do not replace numeric
metrics.

Every completed iteration adds or extends:

- focused Rust unit tests for numeric primitives and invariants;
- deterministic golden fixtures on supported targets;
- property tests for finite values, ranges, wrapping, and graph topology;
- fuzz tests for source parsing and derived-geometry validation where the
  relevant parser exists;
- core integration tests for atomic acceptance, idempotency, cancellation,
  checkpoint export, and clean rebuild;
- contract drift tests across Rust schemas, RPC, SDK, test host, and frontend;
  and
- rendered native-app checks for progress, cancellation, MapLibre behavior,
  reopen, teardown, and epoch changes.

The focused repository gate should be exposed as one stable command, for
example `rtk npm run check:maps:physical`, which orchestrates the relevant Rust,
contract, and frontend fixture checks. Continue to use the explicit Cargo
manifest for Rust commands.

## Iteration 0: contract, determinism, and feasibility spike

### Goal

Resolve irreversible storage/numeric decisions and prove that the proposed
model can run deterministically within desktop budgets before production
contracts or UI are added.

### Required work

1. Audit current map descriptor, asset, layer, location, revision, binary
   transfer, checkpoint, provider dispatch, and native-vector creation paths.
   Record what is reused and the exact old six-candidate entry points that will
   eventually be retired. Reconcile the current native-vector provenance
   version drift across Rust, TypeScript, generated SDK declarations, fixtures,
   and focused tests before adding a physical provider variant.
2. Add an ADR that locks:
   - `daena-physical` provider identity and adapter/source versions;
   - source filename and MIME type;
   - canonical container format, integer/float widths, byte order,
     quantization, strict decoder rules, and version-upgrade policy;
   - the division between canonical fields, normal project records, and
     derived caches;
   - spherical grid, longitude seam, pole adjacency, and display projection;
   - deterministic random derivation, math implementation, traversal/reduction
     order, and hash-fixture policy; and
   - generation-job, temporary-result, cancellation, and atomic-acceptance
     boundaries.
3. Build a disposable pure-Rust spike, not connected to project mutation, that
   creates a versioned spherical field, calculates exact cell areas and
   geodesic distances, solves sea level for a target land fraction, extracts a
   wrapped coastline, encodes/decodes the proposed source, and emits bounded
   GeoJSON for the renderer.
4. Run the same golden input on every supported target available in CI. Compare
   canonical bytes, not only approximate metrics. Eliminate platform drift or
   explicitly block the unsupported target before accepting the ADR.
5. Render the local derived output in the real Tauri host with network disabled.
   Prove the selected MapLibre projection, antimeridian continuity, pole
   behavior, worker/CSP setup, teardown, and WebGL failure diagnostic.
6. Measure source size, derived size, wall time, peak memory, render load time,
   frame behavior, and cancellation latency at proposed default and maximum
   grids. Lock initial budgets and the fixture matrix in the ADR or a referenced
   test document.
7. Classify each equation/algorithm in the design notes as a physical
   approximation, empirical geomorphological model, or procedural heuristic so
   UI/help text does not present heuristics as literal geology.

### Exit gate

- The ADR is reviewed and contains no unresolved source-format, determinism,
  spherical-grid, provider, or authority decision.
- Existing native-vector provenance declarations agree across Rust, frontend,
  SDK declarations, and fixtures, and a focused drift check fails when they do
  not.
- The spike round-trips canonical physical bytes exactly and produces identical
  golden hashes on every supported target tested.
- Cell-area totals approximate `4 * pi * R^2` within the declared tolerance;
  seam and pole fixtures have no duplicate/missing neighbor or contour break.
- Target land fraction and ocean volume solve monotonically within locked
  tolerances and iteration limits.
- A packaged development build renders the fixture offline, handles WebGL
  failure, and releases all map/worker/listener resources after repeated open
  and close.
- Default and maximum workloads fit the recorded CPU, memory, source-size,
  derived-size, and cancellation budgets.
- No production RPC, stored descriptor, or project migration has been added.

## Iteration 1: durable single-map vertical slice

### Goal

Deliver the smallest end-to-end physical map: one Rust-generated signed field,
one temporary preview, one explicit acceptance, one canonical physical asset,
and deterministic reopen/rebuild rendering.

This slice is intentionally geologically simple, but none of its storage,
numeric, job, or rendering foundations are throwaway.

### Required work

1. Add the pure Rust physical-generator crate/module with versioned settings,
   named subsystem seeds, spherical-grid primitives, strict source codec,
   typed errors, validation reports, and cancellation/progress hooks. It must
   not depend on Tauri, SQLite, Svelte, or the plugin host.
2. Implement a deterministic initial crust/elevation scaffold sufficient to
   exercise positive terrain and negative seafloor. Use low-frequency
   structural fields and restrained detail; do not port the existing
   polygon-first TypeScript candidate algorithm.
3. Solve reference sea level from spherical-area land coverage, derive the
   land/ocean mask and coastline from the signed field, establish and persist
   the reference total water inventory, and validate finite ranges, seam
   continuity, geometry validity, and target-area tolerance.
4. Extend Rust map descriptor/source validation with the locked physical
   provider variant. Keep schema version `1` if additions are backward
   compatible with the current map union; do not mutate existing provider
   variants.
5. Extend the Rust-first RPC catalog and generated JSON Schema, SDK, test-host,
   manifest, and conformance fixtures. Register the new host surface through a
   typed provider dispatch table instead of adding another fallback ternary in
   the route.
6. Add trusted Tauri job start/status/cancel/accept operations. Generate outside
   database locks, bind temporary results to project/session/request, and use
   the existing runtime-asset installation plus mutation receipt patterns.
7. On acceptance, create the map entity, source asset, descriptor, and initial
   layer field in one core mutation. Revalidate temporary bytes and provenance
   immediately before commit. A retry with the same request ID returns the
   first result; reuse with different input conflicts.
8. Add a physical-map host surface that reads the source, obtains/rebuilds the
   coastline/land/ocean derivation, and renders through local MapLibre sources.
   Keep authored overlay support on the existing Maps contracts.
9. Replace the six-candidate Create Map action with the single-result physical
   flow only after the new flow passes focused checks. Preserve image import
   and editing of existing `daena-vector` maps.
10. Add checkpoint export/import and clean-rebuild coverage for the new source
   MIME/descriptor. Derived render data must rebuild when absent.

### Exit gate

- The same settings, seed, and retry index produce the same canonical source
  and coastline hashes on all supported targets.
- The UI shows one progressing result. Reroll replaces only that temporary
  result; cancel, validation failure, close, or restart leaves no entity,
  source asset, receipt, or portable generation.
- Acceptance creates exactly one normal map entity and one canonical physical
  source asset atomically. Repeating accept is idempotent.
- The accepted map opens without rerunning generation, survives app restart,
  and reconstructs equivalently after a clean checkpoint and deletion of
  `.daena/`.
- Deleting derived state changes neither the source hash nor rendered
  coastline/land hashes after rebuild.
- The old six-candidate creation action is no longer user-accessible, while
  existing `daena-vector` maps and image import still work.
- Native progress, cancel, reroll, accept, reopen, and teardown behavior is
  verified in the Tauri app, not inferred from unit tests.

## Iteration 2: tectonic scaffold, crust, and causal relief

### Goal

Replace the simple elevation scaffold with a coherent tectonic world whose
continents, shelves, mountains, trenches, rifts, ridges, volcanic arcs,
hotspots, seamounts, and islands share one physical cause chain.

### Required work

1. Generate approximately even spherical plate seeds and deterministic plate
   ownership with explicit seam/pole handling. Irregularize the scaffold
   without breaking complete ownership or creating sliver gaps.
2. Assign each plate a synthetic rotation axis and angular speed. Derive
   boundary-relative motion and classify every boundary segment as convergent,
   divergent, or transform with versioned thresholds and tie rules.
3. Grow continental crust from related cratons using geodesic costs and
   correlated low-frequency variation. Persist crust type and required plate
   metadata. Ensure continental crust includes submerged shelves rather than
   equalling the land mask.
4. Build initial elevation from crust baseline and tectonic terms:
   continental collision, subduction trenches and offset arcs, oceanic arcs,
   rifts and shoulders, spreading ridges with bounded age/depth, transforms,
   hotspots, and restrained terrain detail.
5. Store persistent boundary and volcanic-center metadata required by later
   hazard/history iterations. Keep uplift, influence, and crust-age work fields
   temporary unless the source ADR classifies them as required canonical state.
6. Re-solve reference sea level from water inventory/target land coverage;
   islands must emerge only where final terrain exceeds sea level.
7. Add derived plate, boundary, bathymetry, and volcano layers for diagnostics
   and optional display. Their deletion must not affect the source.
8. Add bounded metrics for plate area distribution, boundary coverage,
   continental-crust area, shelf area, elevation percentiles, trench/ridge
   separation, volcanic-arc offset, and island connected components.

### Exit gate

- Every spherical cell belongs to exactly one valid plate and crust type; plate
  adjacency is reciprocal and seam/pole fixtures have no discontinuity.
- Every boundary segment has finite relative motion and exactly one valid
  classification. Reversing its pair orientation does not change physical
  classification.
- Collision belts, subduction trench/arc pairs, rifts, ridges, transform zones,
  and hotspots appear in their causally required relative positions in numeric
  fixture assertions.
- Continental shelves are submerged continental crust; islands are connected
  components of the land mask, never separately authored polygons.
- Land coverage, elevation bounds, source budgets, and deterministic golden
  hashes pass across the full fixture matrix.
- A native rendered fixture exposes coherent terrain and bathymetry with no
  antimeridian seam. Diagnostic layer toggles do not mutate canonical data.

## Iteration 3: current climate and runoff

Implementation status (2026-08-15): the pure-Rust current-climate/runoff slice
is implemented in `daena-physical-spike` under ADR 0017. Climate remains a
derived, disposable field; the locked physical-world-v2 source is unchanged by
climate derivation. Native current-world water and geography are implemented
under ADR 0019; historical climate remains deferred.

### Goal

Add the minimum coherent climate field needed to drive spatially varying
runoff and later terrain evolution.

### Required work

1. Implement latitude/altitude temperature, maritime moderation, and broad
   prevailing-wind bands using physical distances and a fixed traversal order.
2. Transport moisture from ocean cells, apply distance decay, calculate
   uphill/orographic precipitation, remove precipitated moisture, and produce
   rain shadows. Specify convergence behavior where wind paths meet or wrap.
3. Calculate local runoff and runoff volume with exact spherical cell area.
   Hydrology presets adjust coherent parameter groups, not a final river count.
4. Add continuous derived temperature, precipitation, and runoff products.
   Climate-class/biome labels, if displayed, are a versioned interpretation and
   not canonical state.
5. Validate global precipitation/runoff volumes, non-negative finite fields,
   latitude behavior, coastal-to-interior drying, orographic uplift response,
   and seam continuity with analytic and golden fixtures.

### Exit gate

- Temperature, precipitation, moisture, and runoff are finite, bounded, and
  deterministic for every fixture.
- Rain falls more on the windward side and a measurable rain shadow appears on
  the leeward side of the controlled ridge fixture.
- Runoff volume uses cell area: a uniform-precipitation fixture matches the
  analytic spherical total within tolerance and is not biased by raster row
  count.
- Wind/moisture transport crosses the antimeridian without a visible or numeric
  seam and terminates within fixed iteration limits.
- Opening, closing, or deleting derived climate output does not alter the
  canonical source.

## Iteration 4: terrain evolution and drainage

Implementation status (2026-08-15): the bounded terrain-evolution and drainage
slice is implemented in `daena-physical-spike` under ADR 0018. Priority-Flood
routing, normalized continuous-direction edges, conserved accumulation,
stream-power incision, tectonic uplift, and hillslope relaxation are derived
before the final sea-level solve. The evolved signed elevation field is the
canonical v2 elevation payload; routing, accumulation, and before/after
products remain disposable and are exposed by the trusted native host for both
temporary and reopened accepted maps.

The Iteration 4 gate is covered by 31 pure-Rust tests, 62 Tauri tests, the
exact v6 source/coastline golden matrix workflow, and the release benchmark
recorded in `docs/maps/physical-map-budgets.md`.

### Goal

Evolve tectonic relief into a mature final elevation field and build an
acyclic, physically scaled drainage graph without yet promising final lakes.

### Required work

1. Implement Priority-Flood or an equivalent depression-resolving routing
   surface while preserving the real elevation field unchanged.
2. Implement D-infinity or a reviewed continuous-direction routing method with
   deterministic tie handling, wrap-aware neighbors, normalized split weights,
   and physical downstream distance.
3. Accumulate runoff volume in a deterministic topological order. Detect cycles
   and invalid sinks as hard errors rather than silently dropping flow.
4. Implement bounded stream-power incision, limited continuing tectonic uplift,
   and hillslope relaxation. Quantize at the ADR-defined boundary and reject
   unstable timesteps/non-finite terrain.
5. Map Young/Mature/Old settings to versioned evolution budgets. The presets
   affect duration and persistence without changing the meaning of authored
   historical time.
6. Freeze the evolved signed field as the canonical final elevation. Recompute
   current climate/runoff against the evolved surface before later hydrology.
7. Add diagnostic drainage, slope, accumulation, and before/after relief
   products plus metrics for mass/relief change, drainage density, grid
   anisotropy, and convergence.

The Iteration 4 `grid_anisotropy_ppm` value is a locked directional-concentration
proxy, not a literal eight-direction signature measure. For every routed source
cell, it takes the largest normalized outgoing edge weight and then averages
those values with integer division:

```text
grid_anisotropy_ppm = floor(
  sum(max(edge.weight_ppm) for each routed source) /
  routed_source_count
)
```

The current gate is `grid_anisotropy_ppm <= 950,000`. A future literal
eight-direction signature metric would be a separate diagnostic and acceptance
criterion.

### Exit gate

- The temporary routing surface is depression-compatible while the real field
  retains controlled natural depressions.
- The drainage graph is acyclic; split weights sum to one; no routed edge moves
  uphill on the routing surface; all accumulated runoff is accounted for.
- Controlled slope/discharge fixtures produce stronger incision where the
  stream-power model requires it, and no step exceeds the locked relief-loss
  bound.
- Young, Mature, and Old fixtures show monotonically increasing erosion metrics
  while retaining tectonic range orientation and finite bounded elevation.
- Results are deterministic across supported targets, within time/memory
  budgets, and keep the locked directional-concentration proxy at or below
  `950,000 ppm`.

## Iteration 5: lakes, rivers, coastlines, and current-world completion

Implementation status (2026-08-15): the current-world hydrology slice is
implemented in `daena-physical-spike` under ADR 0019, with land ice added as
hydrology derivation version 2. It derives a bounded raw-field sink/spill
hierarchy, bounded basin water balances, land-ice cells locked from the global
water pool, a conserved ocean-level fixed-point solve against frozen inland
storage and remaining liquid inventory, primary river channels with Strahler
order, watershed, lake-entry/junction/spill-outlet river geometry, island,
lake, and ice geometry, final water-aware coastline, and
hillshade/bathymetry/slope renderer arrays. These products remain disposable;
the accepted v2 source is unchanged and the native host re-derives them after
restart.
Accepted physical maps also persist a separate empty GeoJSON authored-overlay
asset and locked physical layer definitions, including `ice`. Authored vector
edits target that overlay asset only; the signed `.pworld` source cannot be
replaced or edited.

### Goal

Complete the current physical world with conserved water, depression-aware
lakes, topologically valid rivers, and final derived coastline/bathymetry.

### Required work

1. Build the bounded Iteration 5 depression hierarchy: assign each raw-field
   land cell by downhill tracing to a deterministic local sink, then select
   the lowest valid spill edge for each sink basin. Record minima, spill
   points/elevations, volume-to-spill, parents, and downstream destinations.
   This is not a full nested Fill-Spill-Merge tree; nested sub-depressions that
   are not represented by separate downhill sinks are not emitted as separate
   lakes.
2. Solve basin water balances from inflow, direct precipitation, evaporation,
   and outflow. Use a fixed `28%` evaporation fraction of direct precipitation.
   Support dry, endorheic, overflowing, and merged basins; `merged` labels
   carried excess inflow into a parent basin rather than a topological union.
3. Resolve ocean level with a bounded fixed-point solve against inland storage
   computed once from the initial sea level. Only ocean level is re-solved;
   inland basin storage is not recomputed during the loop. Preserve total water
   within the declared tolerance and fail on non-convergence.
4. Extract river channels from normalized accumulated discharge. Enforce
   destinations, junctions, lake entry/exit, spill-point outlets, mouths, and
   no ordinary bifurcation. Calculate Strahler order.
5. Vectorize and simplify rivers within drainage corridors. Extract current
   coastlines and lake boundaries from calculated water levels using wrapped
   per-cell-edge boundary extraction. Derive bathymetric contour segments from
   fixed depth thresholds with the same seam/pole-safe cell-edge method. This
   is deterministic and topology-preserving for the represented grid boundary,
   but is not marching-squares isoline contouring.
6. Derive land/ocean polygons, islands, exposed shelf, bathymetric contours,
   hillshade, slopes, watersheds, rivers, lakes, and land ice as bounded
   renderer data.
7. Add the physical-layer UI with useful defaults and clear loading/error
   states. Physical layers remain immutable; authored Daena layers render
   above them and continue to use shared anchors/navigation.

### Exit gate

- Water balance satisfies `total = ocean + land ice + inland water` within the
  locked tolerance, and the ocean-level fixed-point loop converges within its
  maximum for every valid fixture without recomputing inland storage.
- Every lake is level, occupies a valid depression, and has outlet behavior
  consistent with its spill state. Dry and endorheic basins do not acquire
  invented outlets.
- Every river terminates at ocean, lake, or a legitimate endorheic basin; no
  flow cycle, unexplained split, uphill segment, disconnected outlet, or mouth
  beyond the coastline remains.
- Coastline, lake, and river cell-edge geometry is valid, seam-safe,
  deterministic, and within vector/resource budgets.
- The accepted current world survives restart and clean rebuild with identical
  canonical source and derived-layer hashes.
- In the native app, physical and authored layers render in the correct order;
  interacting with authored overlays never mutates the physical source.

## Iteration 6: historical climate and geographic playback

Status: implemented. See ADR 0020. The current implementation
persists deterministic forcing parameters, applies a global temperature offset
to climate, derives cell land ice and bounded thermal expansion from immutable
final terrain, exposes cache-keyed epoch products, and adds the accepted-map
integer-years control. The response carries an explicit physical-offset
chronology mapping; it does not implicitly invent shared Timeline date
components. Native epoch and layer changes keep the current globe camera.

### Goal

Make meaningful geographic history from climate-driven water redistribution
while keeping tectonic structure and final terrain fixed.

### Required work

1. Generate and persist versioned low-frequency climate-forcing parameters.
   Derive global temperature from physical time offset without claiming Earth
   orbital simulation.
2. Implement bounded land-ice storage from the climate field (freezing land
   cells lock water from the global pool; thaw returns it) and ocean thermal
   expansion. Floating sea ice is excluded from the sea-level reservoir.
   Keep the ADR 0020 logistic volume as a lag diagnostic, not as the inventory
   subtraction used by hydrology.
3. At an epoch, solve land ice, available ocean water, inland storage, sea
   level, climate, hydrology, coastline, rivers, and lakes from the immutable
   final terrain.
4. Add derivation cache keys containing source hash, derivation version, and
   normalized epoch. Stale/missing cache entries recompute; cache invalidation
   never rewrites the source.
5. Add a time control with debounced/cancellable derivation and truthful
   phase/counter progress. Preserve integer physical offsets and integrate with
   shared Daena chronology only through an explicit mapping contract.
6. Validate monotonic water responses, hysteresis from ice lag, land-bridge and
   shelf exposure, island connectivity changes, and stable terrain/tectonic
   hashes across epochs.

### Exit gate

- Cooling increases land-ice storage and decreases available ocean water;
  warming reverses it; thermal expansion has the configured sign and bound.
- Total water remains conserved at every fixture epoch, and all iterative
  solves converge within locked limits.
- Changing epoch changes only derived climate/water/geography. Canonical final
  elevation, plates, boundaries, and volcanic centers remain byte-identical.
- Repeating an epoch returns the same derived hashes; deleting the cache and
  recomputing returns equivalent output.
- The native time control cancels superseded work, never displays an older
  result after a newer request, and remains correct across map switch/restart.
- If linked to Timeline, authored date precision is preserved and no absent
  month/day/time is invented.

## Iteration 7: hazards and optional materialized natural history

Status: hazard foundation, bounded event materialization, and the shared
Timeline/Lore boundary implemented, including canonical chronology validation
and optional-module degradation. See ADR 0021. Broader story projections are
explicitly beyond this phase's acceptance gate.

### Goal

Expose persistent tectonic/volcanic hazards and, only after that field is
stable, allow explicitly generated natural events to become durable shared
history.

The current slice derives versioned relative/generated earthquake and volcanic
hazard fields from accepted boundaries and volcanic centers. It exposes the
strongest bounded samples as read-only styled layers, preserves the canonical
physical source, and records the hazard derivation version in reopened/cache
provenance. Hazard values and rates are model outputs, not real-world
predictions. An explicit bounded request can now sample and accept durable
natural events without changing the source or derived hazard cache.

### Required work

1. Derive earthquake hazard from boundary classification, relative speed,
   background rate, and geodesic distance decay. Validate expected qualitative
   ordering without hardcoding event locations.
2. Derive volcanic centers, origin, activity class, and long-term eruption rate
   from arcs, hotspots, rifts, and spreading systems.
3. Add derived, styled earthquake and volcanic hazard layers with legends that
   describe relative/generated hazard rather than real-world prediction.
4. Define a reviewed, versioned event-materialization request. Bound the time
   interval to +/-100,000 years and the event count to 128; use the explicitly
   named `hazardSeed`, independent from the geography seed. Implemented in the
   physical event contract at version 1.
5. Sample earthquakes with a Poisson occurrence model and bounded
   Gutenberg-Richter-style magnitudes. Sample eruptions from persistent rates
   with the separate `persistent-rate-v1` model. Aftershock sequences remain
   deferred unless separately approved.
6. Commit accepted events as normal revisioned Daena entities/relationships in
   one idempotent core mutation. Store source, generator, hazard, request, and
   model provenance plus canonical `maps.locations` pins. Retries with the
   same request ID replay the accepted identities; reusing that ID with a
   different input conflicts. Never place events only in a derived cache or
   inside opaque physical source bytes.
7. Integrate Timeline/Lore through public shared contracts and degrade without
   losing events when either module is disabled. Implemented for the shared
   `maps.physicalChronology` contract: Timeline displays relative events
   without fabricating Gregorian dates, while Lore continues to consume the
   normal entity/relationship graph.

### Exit gate

- Hazard values are finite, deterministic, seam-safe, and respond to controlled
  boundary/volcanic fixtures with the specified qualitative ordering.
- Hazard layer deletion/rebuild leaves the physical source unchanged.
- Sampling and validation happen before the single commit boundary, and any
  failed request leaves no durable event. The commit is atomic and
  receipt-backed: retries are idempotent and a request-ID/input mismatch
  conflicts.
- Materialized events survive restart, module disable/re-enable, clean
  checkpoint rebuild, and later derivation-version changes without being
  regenerated or silently moved.
- Map links preserve one identity per event through normal relationships and
  canonical `maps.locations` pins with readable labels. Timeline/Lore consume
  the same public entity/field contracts when enabled. Timeline reads the
  explicitly shared `maps.physicalChronology` field and keeps relative offsets
  relative; it continues to read that field when Maps navigation is disabled,
  and no event is owned by or deleted with either optional module. The
  `listShared` bridge filters results to fields explicitly declared
  `shared: true` by the owning manifest.

## Product-spec reconciliation after ADRs 0014-0024

This section compares the original native physical-map product specification
with this plan and the decisions recorded in ADRs 0014 through 0024. It is
normative for corrective work and Iteration 8. An ADR's `Implemented` status
means that its bounded slice exists; it does not mean that a feasibility
surrogate is equivalent to the final product model or that every product exit
gate has passed.

Agents must preserve the distinction between:

- an invariant that already matches the product;
- a deliberate implementation or architecture choice that still satisfies the
  product requirement;
- a meaningful deviation that needs an explicit compatibility or authority
  decision; and
- a gross simplification that is useful as scaffolding but must not be
  presented as completion of the corresponding physical model.

### Invariants that must not regress

The plan and ADRs preserve the defining product invariants:

1. Physical causes precede visible geography. Plates, crust, tectonic
   deformation, climate, runoff, erosion, water storage, and drainage remain
   upstream of mountains, coastlines, islands, lakes, rivers, and hazards.
2. The accepted world has one immutable numeric signed elevation/bathymetry
   field. Land, ocean, and islands are classifications relative to sea level,
   not authored base polygons.
3. Large physical calculations and validation run in deterministic Rust.
   Frontend code configures jobs and renders derived products; it does not
   become an alternative canonical generator.
4. One request presents one world. Bounded deterministic retries repair named
   hard-invalid states and must never become hidden candidate scoring.
5. Canonical state and materialized history survive restart and clean
   checkpoint recovery. Derived climate, hydrology, contours, render arrays,
   and hazard samples remain disposable.
6. Longitude wraps, spherical cell area and geodesic distance are used where
   physical quantities require them, and the pole policy is explicit.
7. Accepted terrain does not continue tectonic or erosional evolution through
   historical time. Historical geography changes through climate, land ice,
   thermal expansion, inland water, and sea level.
8. Authored overlays and shared Timeline/Lore records remain ordinary Daena
   data above the physical source. Editing an overlay cannot rewrite the
   `.pworld` source.
9. Materialized natural events are durable shared records with stable
   provenance. They are not cache entries and are not regenerated when a
   hazard derivation changes.

These are product boundaries, not iteration-specific conveniences. A proposed
optimization or schema change that weakens one requires product-level review,
not only a generator-version increment.

### Meaningful deviations requiring an explicit decision

#### Canonical state is split between source bytes and descriptor settings

The product specification requires the generator version, settings, total
water inventory, climate-history parameters, and physical causes to be
persisted as canonical physical state. The plan also says that the single
canonical source asset stores everything needed to define the world. The
implemented v2 layout documented by ADR 0016 does not include all of that
state: reference water and historical forcing have been carried in generation
descriptor settings by ADRs 0015 and 0020.

A descriptor plus its source asset can be a valid composite canonical record,
but documentation, validation, hashing, export, and recovery must all treat it
as one indivisible authority. The two possible contracts are retained for
traceability; the production resolution is recorded immediately below.

- move all physical replay inputs into a new source format and keep the
  descriptor as an indexed summary; or
- formally define canonical physical state as the validated descriptor/source
  pair, include normalized physical settings in the canonical identity hash,
  and reject any mismatched or incomplete pair atomically.

Agents must not silently copy missing values from current defaults. Tests must
change one descriptor-carried water or forcing value, prove that the canonical
identity and derived cache key change, and prove that clean checkpoint recovery
preserves the exact pair.

### Production resolution

The production contract is the second option above: canonical physical state is
the validated descriptor/source pair. This is an authority decision, not a
permission to treat either half as optional.

- The source asset remains the immutable `physical-world-v2` terrain and cause
  record. Its bytes are not reinterpreted in place.
- The descriptor is the other half of that record. The identity manifest takes
  provider/source-format and generator identity from the descriptor, takes
  grid, radius, terrain, plate/crust, boundary, center, target-land, and
  reference-sea-level values from decoded source bytes, and takes normalized
  presets, reference water inventory, and historical forcing from mandatory
  descriptor generation metadata. Map names, presentation settings, asset IDs,
  derivation versions, and disposable cache metadata are excluded. Derivation
  versions belong in cache keys because changing one must not create a
  different accepted physical world.
- Every value duplicated in source bytes and descriptor metadata has one
  declared owner and an exact equality rule. For `physical-world-v2`, decoded
  source values for schema/adapter version, grid, radius, seed, retry index,
  target land fraction, and sea level are authoritative; acceptance and reopen
  reject a descriptor that disagrees. Descriptor-only reference water,
  normalized presets, and historical forcing are mandatory and may never be
  supplied from contemporary defaults.
- Define a versioned `PhysicalIdentityManifestV1` in Rust. Encode it with fixed
  field order, explicit integer widths, little-endian integers, length-prefixed
  UTF-8 strings, and no floating-point or map values. Compute:

  ```text
  SHA-256(
    "daena-physical-identity-v1\0"
    + u32_le(manifest byte length)
    + manifest bytes
    + u64_le(source byte length)
    + source bytes
  )
  ```

  Do not hash general descriptor JSON, concatenate unframed values, or
  implement normalization independently in TypeScript.
- Acceptance validates and commits the pair atomically. Reopen, checkpoint
  export/import, and clean rebuild validate the same pair before publishing it
  to callers. Missing, malformed, mismatched, or stale halves are rejected.
  Derived cache keys use the composite identity plus only the derivation
  versions and normalized request inputs relevant to that product.
- Accepted physical settings are immutable. Changing reference water,
  historical forcing, or another identity field creates a new temporary world
  and accepted map; it is not an in-place descriptor edit. Recovery retains the
  last complete accepted pair and never repairs it from current defaults.

The v1 disposition is a separate release blocker. Inventory all portable
checkpoints and supported pre-release project data. If any accepted v1 source
can exist, keep a strict read-only v1 adapter that opens and re-derives only
v1-supported products, or provide an explicit user-approved migration to a new
map identity while retaining the original bytes. If no accepted v1 data can
exist, record release and fixture evidence in a superseding ADR and keep a
typed unsupported-version diagnostic. Do not infer v2 tectonics that v1 never
stored.

The following production upgrades are release prerequisites. Until each gate
passes, the implementation must keep its existing model-category label; a
green feasibility fixture does not complete the feature.

1. **Resolution and level of detail.** Define author-visible resolution tiers
   from minimum resolvable feature sizes, with explicit source, derived, render,
   CPU, memory, and cancellation budgets. The supported tier must resolve the
   controlled trench/arc, rift shoulders, shelf/slope, drainage divide,
   tributaries, lake/sill, strait, and small-island fixtures. A larger raster
   is not accepted as the sole remedy; the chosen sampling/LOD strategy must
   retain seam, pole, and spherical-area correctness.
2. **Tectonics.** Replace nearest-site-visible plate geometry with deterministic
   irregularized boundaries and causal crust/relief construction. Craton
   attraction, unrelated-group repulsion, detached terranes, continental
   grouping, shelf continuity, and directional hotspot chains must be measured
   from the cause fields and not inferred from a rendered diagnostic. The
   morphology and boundary cross-section gates in this section become required
   production fixtures.
3. **Drainage and erosion.** Use a bounded, versioned stream-power evaluation
   with declared units and calibrated `erodibility`, discharge exponent, and
   slope exponent. Use aspect/facet-based D-infinity routing, or document an
   equivalent continuous-direction solver with the same rotational behavior.
   Keep Priority-Flood as a temporary routing surface only; retain real
   depressions and prove rotational invariance, drainage-angle distribution,
   basin-shape behavior, and multi-resolution convergence.
4. **Nested water topology.** Replace downhill-trace single-sink labeling with
   a nested depression hierarchy containing sub-basins, spill saddles, parent
   merges, and ocean destinations. `merged` is reserved for a topological merge;
   forwarding excess to a parent is labeled `overflow` or `spill`.
5. **Coupled water balance.** Solve inland storage, lake area/level, overflow,
   river discharge, and ocean level in one bounded fixed-point iteration. A
   sea-level change must recompute all affected connected basins rather than
   reuse storage from the initial solve. Use a state-dependent evaporation term
   based at minimum on lake area and derived climate, with compatible units for
   precipitation, inflow, evaporation, storage, and outlet discharge. Prove
   convergence, conservation, and initial-guess independence for nested,
   chained, endorheic, coastal-capture, and merge/split fixtures.
6. **Contour and polygon geometry.** Replace cell-edge presentation segments
   with seam-safe interpolated contour extraction on the spherical grid. Lock
   the ambiguous-cell rule, pole and antimeridian representation, ring winding,
   hole ownership, and bounded simplification. Validate closure,
   self-intersection, connectivity, straits, island/lake preservation, and
   drainage outlets before MapLibre receives the result.
7. **Historical forcing.** Replace the triangle wave with a versioned smooth
   sum of bounded low-frequency components whose amplitudes, periods, and
   phases are persisted. Keep climate-driven cell land ice as the inventory
   lock. Any additional lag or equilibrium volume is a diagnostic or climate
   constraint, not a pre-hydrology scalar subtraction. Thermal expansion stays
   derived from inventory and temperature change with declared units. Require
   timestep-refinement, continuity, extrema, and long-interval conservation
   gates.
8. **Hazard semantics.** Make volcanic origin, activity class, and long-term
   rate either canonical center properties or explicitly versioned derivations
   from all canonical causes. The display cap may sample output but must not
   change the hazard field or event distribution. Name the Poisson time unit,
   account for all materially relevant sources before sampling, preserve the
   independent hazard seed and accepted-event provenance, and label outputs as
   fictional generated hazards.

### Executable production-upgrade instructions

The numbered requirements above are outcomes. Agents must execute the following
work packets in order. Each packet ends at its own gate; do not combine all
model changes into one unreviewable generator-version bump.

#### Packet 0: promote the pure model and lock identity

1. Promote `crates/daena-physical-spike` to the production physical-model crate
   before adding more algorithms. The production crate must retain no Tauri,
   SQLite, Svelte, MapLibre, project-store, or plugin-host dependency. Keep
   Tauri command adaptation in `src-tauri`, acceptance and identity validation
   in `daena-core`, and all numeric generation/derivation in the pure crate.
2. Implement `PhysicalIdentityManifestV1` and its sole encoder in
   `crates/daena-core/src/maps/physical.rs`. Export the resulting identity to
   host/client responses as an opaque lowercase SHA-256 string. TypeScript may
   compare or transmit it but may not recreate it.
3. Parse source bytes before descriptor validation. Cross-check every duplicate
   field and reject the pair with a stable field-specific diagnostic. Validate
   mandatory descriptor-only values without defaults. Use this one validator
   from generation acceptance, plugin transfer commit, reopen, checkpoint
   export/import, and clean rebuild.
4. Add identity fixtures for each individual manifest field, source-byte
   changes, reordered irrelevant JSON keys, changed names/presentation fields,
   malformed lengths, missing forcing/water values, duplicate-field mismatch,
   cache deletion, and checkpoint reconstruction. Relevant physical changes
   must change the identity; irrelevant presentation changes must not.
5. Resolve v1 data before changing source format again. A migration may decode
   and preserve v1, but it may not manufacture absent plate, crust, boundary,
   or volcanic state and call the result equivalent.

Exit only when all acceptance paths call the same pair validator, exact identity
hashes match on supported targets, and no accepted identity field has an
in-place mutation route.

#### Packet 1: choose physical resolution from feature requirements

Status: production default and maximum are `384 x 192` (ADR 0024). Feature
fixtures and the four/eight-sample gates in `resolution.rs` are unchanged.
`256 x 128` and `512 x 256` remain preview candidates; `1024 x 512` and
`2048 x 1024` remain preview-only.

1. Define fixture features in metres before choosing dimensions: trench
   half-width and arc offset, collision-belt width, rift floor and shoulders,
   shelf and slope width, minimum retained strait and island width, lake sill
   width, drainage-divide width, and minimum displayed tributary catchment.
   Store these values in a reviewed fixture specification rather than in UI
   pixels.
2. Run the same seeds at `256 x 128`, `512 x 256`, `1024 x 512`, and
   `2048 x 1024`. For each latitude band, calculate physical cell width and
   require at least four samples across every minimum retained feature and at
   least eight across a feature whose internal shape is evaluated. A tier that
   cannot satisfy this rule is preview-only.
3. Select exactly one production default and one bounded maximum from measured
   results. Record source bytes, peak live bytes by stage, derived bytes, wall
   time, cancellation latency, coastline load time, and MapLibre frame/update
   time on each supported reference target. Do not retain the ADR 0014
   `128 x 64` maximum as a production maximum merely because it is fast.
4. Generate canonical terrain once at the selected physical resolution.
   Lower-resolution previews, raster pyramids, simplified vectors, and viewport
   products are derived LODs keyed by the composite identity; they are not
   independently generated worlds and never feed back into hydrology.
5. Reuse stage buffers and process local kernels in deterministic row bands or
   tiles. Halo width must be declared per algorithm. Global reductions use a
   fixed tile order and wide accumulators. Parallel execution may change timing
   but not bytes, reduction order, or cancellation semantics.
6. If the selected dimensions, metadata, or random-access requirements exceed
   the strict v2 layout/budget, define `physical-world-v3`. Use independently
   bounded, checksummed sections for the field and cause arrays; validate all
   section offsets and lengths before allocation. Do not loosen v2 limits or
   reinterpret its header.

The gate requires cross-resolution feature metrics, exact supported-target
hashes, native rendered seam/pole fixtures, and measured budgets. Visual
preference alone cannot select the tier.

#### Packet 2: replace the tectonic scaffold

Implement these steps in `tectonics.rs`, keeping each intermediate field
temporary unless the source schema explicitly persists it:

1. Generate approximately even plate sites, then assign ownership with a
   spherical multi-source cost field. Irregularize boundaries by adding a
   named-seed, low-frequency correlated cost perturbation before ownership is
   finalized; never perturb an extracted boundary afterward. Preserve complete
   ownership, reciprocal adjacency, longitude wrap, and pole connectivity.
2. Build explicit shared boundary segments from ownership transitions. Give
   each segment a stable orientation-independent key. Evaluate both plates'
   Euler-pole velocities at the segment midpoint, project relative velocity
   into boundary-normal and tangent components, and classify with versioned
   thresholds and deterministic tie rules.
3. Grow continental crust from grouped cratons with a priority expansion whose
   cost terms are separately measurable: geodesic distance, correlated
   lithology variation, plate-crossing cost, same-group attraction,
   other-group repulsion, and occupied-crust cost. Generate detached terranes
   through their own named seed and bounded area budget. Continental crust must
   be classified independently of sea level.
4. Calculate relief as named signed fields, not one opaque noise sum:
   isostatic/crust baseline, collision uplift, trench subsidence, inland
   volcanic arc, oceanic arc, ridge/age bathymetry, rift floor, rift shoulders,
   transform minor relief, hotspot uplift, and restrained detail. Use geodesic
   distance to the appropriate boundary side and preserve the sign and offset
   of every contribution in controlled cross-sections.
5. For hotspots, derive the chain direction from the owning plate's synthetic
   surface velocity. Place age-ordered seamount centers backward along that
   trajectory with monotonically decaying activity/relief. Whether a center is
   an island remains solely `elevation > sea level`.
6. Persist only the final fields and cause metadata required to reproduce
   hazards and derive geography. Add a generator/source version when persisted
   records change; a diagnostic-layer version is insufficient.

Required tests include ownership completeness, adjacency reciprocity, pair
reversal invariance, no seam/pole discontinuity, non-Voronoi boundary
morphology, craton-group connectedness and terrane bounds, submerged
continental shelves, every tectonic cross-section, hotspot age/direction, and
cause-field-to-final-relief accounting.

#### Packet 3: implement continuous drainage and physical erosion

Implement routing and erosion in `evolution.rs` with explicit SI units:

1. Keep Priority-Flood output separate from real elevation. Record fill depth
   and flood order. Use ocean cells and legitimate basin outlets as seeds;
   deterministic index order breaks equal-priority ties.
2. Compute surface gradient on the local tangent plane using physical
   east-west and north-south distances. Evaluate the triangular facets around
   each cell as D-infinity does. Route to the steepest downslope facet and split
   between its two bounding neighbors by angle. Use a reviewed polar stencil;
   never use a zero-width longitude distance at a pole.
3. Quantize each split to integer weights that sum exactly to one million ppm.
   Send the remainder to the deterministically larger unquantized weight.
   Plateau routing follows flood order and the resulting graph must be acyclic.
4. Accumulate local runoff volume in reverse topological order with a wide
   integer or reviewed fixed-point accumulator. Conservation is exact up to the
   declared final-unit rounding; no percent-of-world tolerance may hide lost
   flow.
5. Evaluate stream-power incision as:

   ```text
   incision_rate_m_per_year =
     K * discharge_m3_per_second^m * slope^n
   ```

   Store `K`, `m`, `n`, uplift rate, diffusivity, timestep, and iteration count
   in one versioned evolution preset. Convert annual runoff to discharge before
   applying the equation. Use a deterministic reviewed power implementation
   and quantize only at declared stage boundaries.
6. Choose timestep from stability bounds. Limit incision to a declared
   fraction of local removable relief per step; apply hillslope diffusion under
   its explicit stability limit; then apply continuing tectonic uplift. Reject
   non-finite values, negative discharge, unstable steps, and out-of-range
   terrain instead of clamping an invalid world into acceptance.
7. Recompute routing, accumulation, and climate/runoff at the documented
   cadence during evolution. A preset may reduce cadence only after fixtures
   prove that the approximation does not change drainage topology beyond its
   locked tolerance.

Rotate analytic planes, cones, ridges, and synthetic watersheds through several
longitudes and latitudes. Gate on outlet identity, drainage area, angle
distribution, basin compactness, and incision volume rather than the old
average-largest-weight proxy. Compare each production tier with its next finer
fixture and lock convergence thresholds before replacing generator version 6.

#### Packet 4: implement nested basins and a coupled water solve

Replace the current raw-sink model in `hydrology.rs` with a depression tree and
one explicit steady-state solver:

1. Build a depression hierarchy from real final elevation using
   Priority-Flood/Fill-Spill-Merge labeling. Every node records its minimum,
   cell membership or bounded membership representation, spill saddle, spill
   elevation, parent, children, ocean destination, and cumulative
   volume-versus-level curve. Preserve equal-saddle tie rules.
2. Construct each volume curve from exact spherical cell areas and elevation
   intervals. Lake area and volume at level `h` are evaluated from that curve;
   a lake may not occupy terrain above `h`. A parent merge occurs only when
   children reach the shared saddle.
3. Use one outer state iteration for each current or historical epoch:

   ```text
   start from bracketed sea level and prior/empty lake state
   -> classify ocean-connected cells
   -> derive climate, precipitation, evaporation inputs, and runoff
   -> route runoff and solve basin nodes bottom-up
   -> compute lake levels, areas, storage, overflow, and outlet discharge
   -> compute land ice and thermal expansion for the epoch
   -> solve ocean level from remaining ocean water by bounded bisection
   -> rebuild any connectivity changed by the new ocean level
   -> repeat until sea level, inland storage, and basin states all converge
   ```

4. Calculate precipitation and evaporation as volumes per timestep:
   `depth * exact spherical water-surface area`. The production evaporation
   model must use lake area and derived temperature plus a declared bounded
   humidity/wind or effective aridity term. Calibrate its coefficients as an
   empirical model; do not retain a constant fraction of direct precipitation.
5. Use under-relaxation only as a versioned solver parameter. Detect two-state
   oscillation and non-convergence. The water residual tolerance must be
   derived from integer/field quantization and solver tolerance, not a loose
   percentage selected to pass fixtures.
6. After convergence, derive river channels from final discharge. Preserve one
   downstream channel except an explicitly modeled future delta. Lake entries,
   spill outlets, junctions, endorheic terminals, and ocean mouths must share
   node identities with the solved basin/drainage graph.

Run the solver from low, high, and prior-epoch initial guesses. Require the same
final state for nested bowls, chained overflow, endorheic drying, coastal
capture, saddle merge, lake disappearance, and historical merge/split
fixtures. Verify reservoir conservation at every iteration and final topology
after cache deletion/rebuild.

#### Packet 5: produce topology-preserving vectors

Implement final contouring as a separate derived module; do not mix renderer
geometry into `hydrology.rs`:

1. Add a ghost longitude column copied from column zero and contour each scalar
   threshold with interpolated marching squares. Use the asymptotic decider for
   ambiguous `0101`/`1010` cells. Use marching triangles or an equivalent
   declared cap rule for polar cells.
2. Interpolate crossings from the underlying scalar values, not cell centers
   or a post-hoc jitter. Quantize coordinates only after interpolation. Canonical
   terrain and water levels remain unchanged.
3. Join segments by stable edge identity, assemble closed rings, unwrap
   longitude while assembling, then split/wrap antimeridian output according to
   the locked GeoJSON policy. Assign winding and holes from spherical
   containment, not planar bounding boxes near the seam or poles.
4. Derive land, ocean, island, lake, shelf, and bathymetric products from the
   same threshold topology so adjacent products cannot disagree. Snap a river
   mouth or lake outlet only to its analytically intersected contour within its
   final drainage cell; otherwise fail validation.
5. Simplify in a local geodesic metric with protected vertices for saddles,
   junctions, entries, exits, mouths, narrow straits, and minimum islands.
   Reject a simplification that changes component count, hole ownership, or
   protected connectivity.

Tests must include every marching-squares case, exact-threshold vertices,
ambiguous saddles, antimeridian rings and holes, polar caps, nested islands and
lakes, narrow straits, river mouths, self-intersection rejection, and
round-trip rendering in the packaged native host.

#### Packet 6: replace historical forcing and ice placeholders

Hydrology derivation version 2 already locks per-cell land ice from the global
pool and melts it back above 0 °C. Packet 6 remains about the historical
forcing *shape* and a coupled lag model, not about reintroducing a scalar that
subtracts ice before climate.

Implement the physical history model in `history.rs`:

1. Persist at least three independently derived forcing components. For
   integer physical time `t`, evaluate:

   ```text
   forcing(t) = sum(amplitude_i * cos(
     2 * pi * (t + phase_i) / period_i
   ))
   temperature(t) = reference_temperature + sensitivity * forcing(t)
   ```

   Bound amplitudes, periods, phases, and total temperature. Use the project's
   deterministic numeric policy; platform-default transcendental drift may not
   change quantized products or supported-target hashes.
2. Keep cell land ice as the inventory lock. If a lagged equilibrium volume is
   still required, use a smooth bounded response as a diagnostic or as an
   additional climate constraint, initially:

   ```text
   equilibrium_ice(T) =
     maximum_land_ice / (1 + exp((T - midpoint) / transition_width))
   ```

   Integrate first-order lag with a fixed physical timestep or the analytic
   exponential step. Persist all parameters needed to replay it from the
   reference epoch and bound the integration interval/work. Do not replace
   climate-driven cell ice with this scalar, and do not add glacier flow or
   ice–land mechanical interaction.
3. Compute thermal expansion with declared SI units:
   `delta_volume = alpha * ocean_volume * delta_temperature`. Use it inside the
   coupled water iteration because ocean volume and sea level are not
   independent.
4. Re-run climate, drainage, nested lakes, rivers, coastlines, and connectivity
   at each requested epoch. Plates, crust, volcanic centers, and final terrain
   hashes must remain unchanged.
5. Increment the historical derivation version and cache key, not the source
   format, unless persisted forcing parameters or identity metadata require a
   new canonical manifest/source contract.

Gate continuity of forcing and its first derivative, bounded extrema,
timestep refinement, ice lag and hysteresis, thermal-expansion sign, water
conservation, epoch-zero replay, land bridges, shelf exposure, lake
merge/split, cancellation, stale-response suppression, and cache rebuild.

#### Packet 7: give hazards and events stable rate semantics

Implement the full field in `hazards.rs` before limiting renderer features:

1. For each boundary segment, calculate an annual generated earthquake rate
   contribution from its class, relative speed, length/represented area, and
   geodesic decay. Evaluate all sources, or use a deterministic spatial index
   with a cutoff whose omitted maximum contribution is below a locked error
   bound. Never select the strongest display sources before field evaluation.
2. In the next source format that changes volcanic records, persist a stable
   center ID, location, tectonic origin, activity class, and long-term generated
   eruptions-per-model-year rate. For v2, derive those values with one versioned
   function from existing immutable center/cause data and include that function
   version in hazard/event provenance.
3. Keep field resolution and display sampling separate. Select bounded display
   features only after the complete field and metrics exist. Changing
   `maxFeatures`, viewport, style, or legend must leave field hashes and event
   samples unchanged.
4. Define every event rate as events per physical model year. For interval
   length `years`, use `lambda = annual_rate * years` in the Poisson model.
   Sample location from normalized full-field rate mass, then sample earthquake
   magnitude from the bounded Gutenberg-Richter model or eruption properties
   from the named volcanic model.
5. Preserve `hazardSeed` as an explicit materialization input independent from
   geography. Persist source composite identity, hazard and event model
   versions, annual local rate, interval, seed, sampled cell/center, and
   `prediction: false` on every accepted event.

Gate analytic single-source decay, source superposition, spatial-index error,
boundary reversal, seam/pole behavior, rate-mass conservation, render-cap
independence, Poisson mean over a fixed ensemble, magnitude-frequency ordering,
event idempotency, and survival across hazard-version/cache changes.

#### Packet 8: replace surrogate status with release evidence

1. Extend `check:maps:physical` to run source/identity drift checks, pure-model
   fixtures, property tests, core acceptance/recovery tests, Tauri command
   tests, and frontend contracts. Keep release benchmarks and packaged-native
   rendered checks as explicit additional gates where CI cannot exercise them.
2. Maintain a checked-in fixture manifest containing source/generator/
   derivation versions, input settings, exact canonical identity and source
   hashes, invariant metrics, expected topology, and the reason for any golden
   update. A changed hash without an intentional version/decision is a failure.
3. Run exact canonical fixtures on macOS arm64, Linux x64, and Windows x64.
   Derived floating products either quantize to exact cross-target bytes under
   the numeric policy or are not accepted as deterministic products.
4. Fuzz strict source sections, identity manifests, descriptor/source
   mismatches, depression trees, contour assembly, GeoJSON limits, and cache
   decoders. Add fault injection around temporary output, asset installation,
   atomic acceptance, receipt replay, checkpoint export, and derived rebuild.
5. Record product status per packet as `implemented`, `verified on target`, and
   `product-complete`, with links to evidence. Remove a scaffold/heuristic label
   only when that packet's complete gate passes.

The implementation order is consequently: canonical pair identity and
transactional validation; supported resolution/LOD and budgets; tectonic and
drainage upgrades; nested coupled water; contour/polygon validation; smooth
historical forcing; and hazard calibration. A change to accepted terrain or
its interpretation requires a new source format or adapter. A change confined
to disposable products requires a derivation-version increment and a
delete/rebuild proof. No step may silently upgrade an accepted world.

### Gross simplifications that remain corrective work

The following choices are acceptable for bounded feasibility slices, but they
do not yet fulfill the corresponding product behavior.

#### Feasibility resolution is below feature resolution

The `64 x 32` default and `128 x 64` maximum from ADR 0014 place cells hundreds
of kilometres apart on an Earth-sized world. At that scale, drainage valleys,
river corridors, lake spill points, continental shelves, volcanic-arc offsets,
straits, and mountain-belt widths are mostly cell classifications rather than
resolved geography. Increasing the same raster blindly is not the remedy.

Before release, define an author-visible resolution and level-of-detail
strategy from minimum resolvable physical features. Benchmark generation,
source size, derived size, and rendering at that scale. Controlled fixtures
must resolve, in separate cells or interpolated geometry, a trench/arc pair,
rift shoulders, a shelf/slope transition, a drainage divide, tributary
junctions, a lake and sill, a strait, and a small island. Feasibility-grid
hashes may remain unit fixtures but cannot be the sole product-quality gate.

#### Tectonic structure is still a scaffold

Fibonacci-style plate sites are an acceptable approximately even seed
distribution, but the ADRs do not demonstrate the required deterministic
boundary irregularization or prove that final plate geometry has lost the
visible nearest-site/Voronoi signature. The documented craton growth includes
geodesic distance and correlated variation but does not yet establish related
craton attraction, unrelated-group repulsion, detached terranes, or coherent
continental grouping across plate boundaries. Persistent hotspots are centers;
the required directional seamount/island-chain behavior is not demonstrated.

Add numeric morphology gates for boundary straightness and junction regularity,
continental connectedness and fragmentation, shelf continuity, and hotspot
chain alignment. Add controlled cross-sections that distinguish continental
collision, oceanic-continent subduction, oceanic-oceanic convergence, rifts,
ridges, and transforms. A diagnostic layer looking plausible is supporting
evidence, not proof of the cause chain.

#### Erosion uses a surrogate rather than the specified stream-power model

ADR 0018 correctly identifies its logarithmic bounded incision rule as a v1
surrogate. It does not implement the specified
`erodibility * discharge^m * slope^n` model near the documented calibration
starting points. Likewise, the four-neighbor drop/distance blend is not
automatically equivalent to aspect-based D-infinity routing, and the
`grid_anisotropy_ppm` average-largest-weight metric with a `950,000 ppm` limit
is too weak to establish absence of grid-aligned drainage.

Replace the surrogate with a numerically bounded, versioned stream-power
evaluation, or obtain a product decision that deliberately changes the model
after comparative fixtures show equal or better geomorphology. Implement
aspect/facet-based D-infinity or document and validate an equivalent
continuous-direction method. Add rotational tests, drainage-angle histograms,
basin-shape metrics, and multi-resolution convergence checks. Preserve
Priority-Flood's temporary routing surface and the real depressions.

#### Basin topology and water coupling are intentionally incomplete

ADR 0019's one-sink-per-downhill-trace hierarchy is not the required nested
Fill-Spill-Merge-equivalent topology. It can omit nested sub-depressions.
`merged` currently means forwarding excess water rather than topological basin
merge. More importantly, inland storage is frozen from the initial sea level
while only ocean level is iterated. The product specification requires lake
geometry/storage and sea level to be solved together until stable.

Implement or adopt a bounded nested depression hierarchy that represents
sub-basins, spill saddles, parent merges, and ocean destinations. During the
water fixed point, recompute affected lake area, level, storage, and overflow
when sea level or connected-basin state changes. Prove convergence and
conservation from more than one initial guess. Fixtures must cover nested
basins, chained overflow, an endorheic basin, a coastal lake captured by rising
sea level, and a basin merge/split across historical epochs.

The fixed `28%` of direct precipitation evaporation rule is also only a
placeholder. Replace it with a bounded water-balance term that responds at
least to lake area and derived climate, or explicitly narrow the product model
and label the limitation in the UI. River inflow, precipitation, evaporation,
storage, and outlet discharge must use compatible physical units.

#### Cell-edge vectors are not final contour geometry

ADR 0019 explicitly uses per-cell-edge boundaries rather than interpolated
marching-squares-equivalent isolines. At the feasibility resolution this
produces block geometry and does not by itself fulfill the intended coastline,
lake-boundary, bathymetric-contour, and valid land/ocean polygon contracts.

Add seam-safe interpolated contour extraction with an explicit ambiguous-cell
policy, spherical/pole handling, ring assembly, winding and hole rules, and
bounded simplification. Coastlines must remain the contour of
`elevation = sea level`; smoothing may not invent or remove a strait, island,
lake outlet, or drainage corridor. Validate polygons for closure,
self-intersection, holes, antimeridian representation, and connectivity before
feeding MapLibre.

#### Historical climate is a non-smooth single-wave heuristic

ADR 0020's bounded triangle wave is deterministic and causally connected to
climate and therefore to cell land ice, but it is not the product
specification's smooth low-frequency sum of several long-period oscillations.
Its corners introduce abrupt forcing derivative changes. Per-cell freeze/thaw
now locks water from the inventory; the remaining gap is the specified smooth
multi-component forcing and any explicit lag integration around that climate
field, not a return to subtracting a logistic scalar before hydrology.

Persist a versioned multi-component smooth forcing model with amplitudes,
periods, and phases. Keep cell freeze/thaw as the inventory lock. Calculate
thermal expansion from inventory and temperature change with declared units.
Add timestep-refinement, phase-boundary continuity, extrema, and long-interval
conservation tests. The UI must continue to call this generated physical
chronology, not Earth orbital cycles or a prediction.

#### Hazard rates need stable physical meaning

ADR 0021 exposes normalized generated rates based on a bounded set of strongest
sources. This is a valid read-only relative-hazard preview, but truncating
sources and deriving volcanic activity/rates outside the canonical center
records can make event rates depend on derivation limits rather than the whole
physical system.

Define whether volcanic origin, activity class, and long-term rate are
canonical center properties or deterministic versioned derivations from
canonical causes. Sum or otherwise account for all materially relevant sources
before display sampling; the display feature cap must not change the hazard
field or event distribution. Calibrate and name the time unit used by Poisson
sampling, test rate stability when the render cap changes, and preserve the
independent `hazardSeed` and all accepted-event provenance. Generated rates
must remain clearly labelled as fictional model outputs.

### Required corrective sequence for agents

Agents should close the reconciliation in this order:

1. Resolve composite canonical authority and v1 durable-data compatibility
   before changing another source or generator version.
2. Establish target feature resolution, product-quality fixtures, budgets, and
   supported-target evidence before tuning physical coefficients.
3. Upgrade tectonic morphology, erosion, and continuous-direction drainage
   before treating downstream lake and river shapes as stable.
4. Replace the bounded raw-basin and frozen-inland-water approximations with a
   coupled nested basin/water solve.
5. Replace cell-edge presentation geometry with topology-preserving
   interpolated contours and polygons.
6. Upgrade historical forcing smoothness and any remaining lag diagnostics
   without changing accepted terrain or replacing cell land ice.
7. Run full deterministic, recovery, packaged-native, and supported-target
   gates; then update iteration statuses and user documentation with the exact
   remaining approximations.

For each correction, write the model classification, equations and units,
canonical/derived boundary, deterministic traversal and quantization policy,
failure bounds, migration/version effect, fixture metrics, native-rendered
evidence, and user-facing wording before implementation. If a correction
changes accepted canonical bytes or their interpretation, create a new source
format or adapter as required; never reinterpret `physical-world-v2` in place.
If it changes only a disposable result, increment the derivation version and
prove cache deletion/rebuild equivalence.

Users and reviewers should expect labels such as `feasibility scaffold`,
`procedural heuristic`, `relative generated hazard`, and `physical-year
offset` until the associated corrective gate passes. Agents must not convert
an acknowledged approximation into an implied physical claim merely by
removing that label from the UI or documentation.

## Iteration 8: performance, resilience, and release hardening

### Goal

Prove that the complete system is bounded, recoverable, maintainable, and ready
for supported desktop releases.

### Required work

1. Profile every pipeline stage and derived layer at default and maximum
   settings. Optimize measured bottlenecks without changing deterministic
   results inside generator/derivation versions.
2. Add bounded parallelism only where reduction order and output hashes remain
   deterministic. Verify cancellation and project-close latency under load.
3. Fuzz the source decoder, temporary handle, descriptor/provenance validation,
   cache decoder, contour input, and vector-output limits.
4. Stress repeated generate/cancel/reroll/accept, rapid epoch scrubbing, map
   switching, project switching, cache deletion, renderer failure, low-memory
   behavior, and app restart during generation/acceptance.
5. Inject failures around temporary-file creation, runtime-asset installation,
   SQLite mutation, receipt write, projection refresh, and checkpoint export.
   Recovery must select the complete old or new state.
6. Verify offline packaged behavior, CSP, no remote map resources/telemetry,
   accessibility, keyboard/focus behavior, progress announcements, reduced
   motion, display scaling, and cleanup on macOS, Windows, and Linux.
7. Document the source and generator versioning procedure, golden-fixture
   review process, model-category labels, resource-budget update policy, cache
   invalidation policy, and reproducible benchmark commands.
8. Run full relevant repository gates and separate any unrelated third-party
   diagnostic from application failures. Do not waive focused physical-map or
   rendered-native failures because a broad check has unrelated noise.

### Exit gate

- All focused, contract, recovery, fuzz, and repository checks pass from the
  final worktree, followed by clean `git diff --check` output.
- Default and maximum workloads meet every locked CPU, memory, source/derived
  size, render, and cancellation budget on reference systems.
- Crash injection proves no mixed entity/asset/source state and no loss of a
  committed accepted world.
- Packaged supported targets operate offline and pass generation, progress,
  cancellation, acceptance, reopen, clean rebuild, epoch playback, renderer
  teardown, accessibility, and display-scale checks.
- A new derivation version can rebuild caches without touching canonical data,
  and a new generator version can coexist with existing source assets without
  reinterpretation.
- The implementation documentation identifies every remaining approximation,
  empirical model, heuristic, limitation, and explicitly deferred feature.

## Explicitly deferred

The following are not hidden requirements for the iterations above:

- moving continents or replaying plate tectonics through geological time;
- mantle convection or full plate-evolution simulation;
- detailed sediment transport, groundwater, glacier flow, or ocean circulation;
- full atmospheric circulation or local gravitational sea-level effects;
- user sculpting, painting, or replacing accepted physical terrain;
- automatic regeneration or upgrading of accepted worlds;
- delta bifurcation and detailed channel migration;
- aftershock sequences;
- civilization, population, political, culture, settlement, or road simulation;
- automatic creation of semantic entities from physical features;
- 3D terrain authoring;
- vector tiles or unbounded high-resolution generation;
- collaboration, cloud compute, publishing, or remote runtime resources; and
- a public third-party physical-generator SDK.

These may be planned later only if they preserve the central rule:

> Generate physical causes, persist the immutable numeric physical truth, and
> derive visible geography through bounded, reproducible Rust computations.
