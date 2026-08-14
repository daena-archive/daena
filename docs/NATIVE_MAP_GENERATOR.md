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
ocean, coastlines, islands, lakes, rivers, climate, relief, and hazards derive
from that field and its persisted physical causes. Authored countries,
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
    "version": 4,
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
trusted shell and must run CPU work away from the async/UI thread.

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
- rivers, lakes, watersheds, and drainage basins; and
- earthquake and volcanic hazard fields.

MapLibre consumes derived GeoJSON and bounded derived raster products. These
representations may be cached using the canonical source hash, derivation
version, and epoch. They are never authoritative and must be safely deletable.
No physical layer may exist only as a PNG or JPEG.

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
  -> water inventory, land ice, inland water, and sea level
  -> depression hierarchy and drainage
  -> rivers, lakes, coastline, and derived climate
  -> physical validation
  -> temporary result
  -> explicit acceptance
```

Historical derivation starts from the accepted final terrain and persistent
physical parameters. It never moves plates or changes terrain:

```text
climate forcing at t
  -> temperature
  -> land ice and thermal expansion
  -> available ocean water
  -> sea level
  -> land/ocean mask
  -> climate and hydrology
  -> coastline, rivers, and lakes
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

Terrain evolution uses a bounded stream-power-style model:

```text
incision = erodibility * discharge^m * slope^n
new terrain = old terrain + limited active uplift - incision
              + hillslope relaxation
```

Initial calibration begins near `m = 0.5` and `n = 1.0`, but these are
versioned empirical parameters, not literal constants of nature. `Young`,
`Mature`, and `Old` settings select coherent iteration counts, relaxation, and
tectonic persistence; they are not literal geological ages.

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
persistent inland water. Lake volume and ocean level may require a bounded
fixed-point loop: estimate sea level, derive lakes, update inland storage, and
resolve sea level until the declared tolerance is met.

Flow routing uses D-infinity or another reviewed continuous-direction method;
basic D8 is not acceptable because it produces visible grid alignment.
Accumulation weights sum to one, use physical runoff volume, and form an
acyclic downstream graph.

The final depression hierarchy records basin minima, spill points/elevations,
volume-to-spill, parents, and downstream basin/ocean destinations. A basin's
water balance includes inflow, direct precipitation, evaporation, and outlet
discharge. Outcomes include dry, endorheic, overflowing, and merged lakes.

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

Temperature drives a bounded equilibrium land-ice volume. Ice approaches that
equilibrium with a configured response lag. Only land ice changes sea level.
Thermal expansion applies a bounded effective coefficient to ocean volume.
Every epoch then resolves sea level, lake storage, climate, and hydrology from
the immutable final terrain.

The first historical contract uses an integer physical offset from a persisted
reference epoch. Integration with Timeline must use Daena's shared calendar and
date-precision contract; it may not invent missing month/day/time components or
hardcode JavaScript timestamp semantics.

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
  budgets, and show no eight-direction grid signature above the locked metric.

## Iteration 5: lakes, rivers, coastlines, and current-world completion

### Goal

Complete the current physical world with conserved water, depression-aware
lakes, topologically valid rivers, and final derived coastline/bathymetry.

### Required work

1. Build a Fill-Spill-Merge or equivalent depression hierarchy containing
   minima, spill points/elevations, volume-to-spill, parents, and downstream
   destinations.
2. Solve basin water balances from inflow, direct precipitation, evaporation,
   and outflow. Support dry, endorheic, overflowing, and merged basins.
3. Couple inland water and ocean volume with a bounded fixed-point solve.
   Preserve total water within the declared tolerance and fail on
   non-convergence.
4. Extract river channels from normalized accumulated discharge. Enforce
   destinations, junctions, lake entry/exit, spill-point outlets, mouths, and
   no ordinary bifurcation. Calculate Strahler order.
5. Vectorize and simplify rivers within drainage corridors. Extract current
   coastlines and lake polygons from calculated water levels using wrapped,
   topology-preserving contouring.
6. Derive land/ocean polygons, islands, exposed shelf, bathymetric contours,
   hillshade, slopes, watersheds, rivers, and lakes as bounded renderer data.
7. Add the physical-layer UI with useful defaults and clear loading/error
   states. Physical layers remain immutable; authored Daena layers render
   above them and continue to use shared anchors/navigation.

### Exit gate

- Water balance satisfies `total = ocean + land ice + inland water` within the
  locked tolerance, and the fixed-point loop converges within its maximum for
  every valid fixture.
- Every lake is level, occupies a valid depression, and has outlet behavior
  consistent with its spill state. Dry and endorheic basins do not acquire
  invented outlets.
- Every river terminates at ocean, lake, or a legitimate endorheic basin; no
  flow cycle, unexplained split, uphill segment, disconnected outlet, or mouth
  beyond the coastline remains.
- Coastline, lake, and river geometry is valid, seam-safe, deterministic, and
  within vector/resource budgets.
- The accepted current world survives restart and clean rebuild with identical
  canonical source and derived-layer hashes.
- In the native app, physical and authored layers render in the correct order;
  interacting with authored overlays never mutates the physical source.

## Iteration 6: historical climate and geographic playback

### Goal

Make meaningful geographic history from climate-driven water redistribution
while keeping tectonic structure and final terrain fixed.

### Required work

1. Generate and persist versioned low-frequency climate-forcing parameters.
   Derive global temperature from physical time offset without claiming Earth
   orbital simulation.
2. Implement bounded equilibrium land-ice storage, response lag, and ocean
   thermal expansion. Floating sea ice is excluded from the sea-level reservoir.
3. At an epoch, solve land ice, available ocean water, inland storage, sea
   level, climate, hydrology, coastline, rivers, and lakes from the immutable
   final terrain.
4. Add derivation cache keys containing source hash, derivation version, and
   normalized epoch. Stale/missing cache entries recompute; cache invalidation
   never rewrites the source.
5. Add a time control with debounced/cancellable derivation and truthful
   progress. Preserve integer physical offsets and integrate with shared Daena
   chronology only through an explicit mapping contract.
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

### Goal

Expose persistent tectonic/volcanic hazards and, only after that field is
stable, allow explicitly generated natural events to become durable shared
history.

### Required work

1. Derive earthquake hazard from boundary classification, relative speed,
   background rate, and geodesic distance decay. Validate expected qualitative
   ordering without hardcoding event locations.
2. Derive volcanic centers, origin, activity class, and long-term eruption rate
   from arcs, hotspots, rifts, and spreading systems.
3. Add derived, styled earthquake and volcanic hazard layers with legends that
   describe relative/generated hazard rather than real-world prediction.
4. Define a reviewed, versioned event-materialization request. Bound the time
   interval and event count; use a named hazard seed independent from geography.
5. Sample earthquakes with a Poisson occurrence model and bounded
   Gutenberg-Richter-style magnitudes. Sample eruptions from persistent rates.
   Aftershock sequences remain deferred unless separately approved.
6. Commit accepted events as normal revisioned Daena entities/relationships in
   one idempotent core mutation. Store their generation provenance. Never place
   them only in a derived cache or inside opaque physical source bytes.
7. Integrate Timeline/Lore through public shared contracts and degrade without
   losing events when either module is disabled.

### Exit gate

- Hazard values are finite, deterministic, seam-safe, and respond to controlled
  boundary/volcanic fixtures with the specified qualitative ordering.
- Hazard layer deletion/rebuild leaves the physical source unchanged.
- Cancelled or failed event materialization creates no durable event. Retried
  acceptance is idempotent and a request-ID/input mismatch conflicts.
- Materialized events survive restart, module disable/re-enable, clean
  checkpoint rebuild, and later derivation-version changes without being
  regenerated or silently moved.
- Links to map, Timeline, Lore, and affected shared entities preserve one
  identity per event and use readable labels in the UI.

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
