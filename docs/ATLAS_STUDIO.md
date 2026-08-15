# Atlas Studio

## Status and Authority

This document defines the proposed **interactive Atlas Studio**. It is not a
replacement for the existing Atlas Rendering implementation or its accepted
contracts. Atlas Studio must reuse that implementation as its terrain,
cartography, snapshot, and export foundation.

This document is subordinate to:

- [`ARCHITECTURE.md`](./ARCHITECTURE.md) for core, shell, module, and plugin
  boundaries;
- [`STORAGE.md`](./STORAGE.md) for canonical project files and disposable local
  state;
- [`MAP_INTEGRATION_PLAN.md`](./MAP_INTEGRATION_PLAN.md) for shared map,
  layer, anchor, hierarchy, and navigation contracts;
- [`NATIVE_MAP_GENERATOR.md`](./NATIVE_MAP_GENERATOR.md) and its accepted ADRs
  for physical-world identity, epochs, numeric policy, and invariants;
- [`ATLAS_MAP_RENDERING.md`](./ATLAS_MAP_RENDERING.md) for static rendering,
  snapshot, preset, provenance, job, cache, and save contracts; and
- accepted Atlas Rendering ADRs `0031`, `0032`, `0033`, `0034`, and `0036` for
  the contracts already implemented in this checkout.

If this guide conflicts with an authority above, an implementation agent must
stop and reconcile the conflict before changing stored data, public commands,
rendered output, or deterministic algorithms. Add a new ADR for a newly locked
contract; do not silently reinterpret an accepted Atlas version.

## Vision

Atlas becomes a **sister studio to the Physical Map**.

The Physical Map remains the canonical planetary model. Its current `384 x 192` grid is a deliberate simulation resolution used to model large-scale physical state at acceptable cost: land and ocean distribution, elevation trends, mountain systems, climate, hydrology, biomes, ice, sea level, and related physical fields.

Atlas derives detailed geography from that model. It adds refined coastlines, peaks, ridges, valleys, drainage, tributaries, local lakes, small islands where permitted, terrain roughness, biome transitions, relief, and surface texture.

Atlas terrain is **derived, deterministic, and disposable**. It is not a second physical simulation and never becomes more authoritative than the Physical Map.

> **The Physical Map models the planet. Atlas turns that model into detailed geography.**

## Product Relationship

```text
Physical Map
canonical planetary-scale model
384 x 192 simulation grid
        |
        v
Atlas terrain synthesis
constrained geographic amplification
        |
        v
Atlas Studio
view detailed world
        |
        +--> optional static export
```

The Physical Map defines **what exists and why**. Atlas defines **what it looks like at geographic scale**.

Atlas Rendering and Atlas Studio have different product roles:

- **Atlas Rendering** produces bounded static artifacts from an immutable
  snapshot.
- **Atlas Studio** interactively explores the same derived scene and uses Atlas
  Rendering when the user requests an export.

The Studio must not develop a second terrain generator, style system, epoch
model, layer resolver, or export path.

### Invariants

- The Physical Map remains canonical.
- Atlas must preserve major continents, oceans, mountain systems, climate zones, ice, and other simulation-relevant facts.
- Atlas may refine those features into smaller geographic structures but must not contradict them.
- Atlas detail is reproducible from the Physical Map identity, Atlas algorithm version, and detail variant.
- Geography exists in world space; changing zoom, export resolution, style, or tile order must not move generated features.
- Editing the underlying world happens in the Physical Map, not Atlas.
- Atlas caches may be deleted and regenerated without data loss.

## Verified Checkout Baseline

The following facts were verified in the checkout on 2026-08-15. Agents must
re-check them in the live worktree before starting an iteration because version
numbers and file locations may advance.

- The accepted physical source remains a `384 x 192` canonical simulation grid.
- `crates/daena-atlas` already provides a pure Rust renderer with request schema
  `1`, detail algorithm `1`, derived drainage `1`, renderer `5`, seed policy
  `1`, and provenance schema `1`.
- The current renderer supports whole-world and regional equirectangular views,
  regional Web Mercator, deterministic relief and antique styles, a political
  style, authored/semantic overlays, labels, atlas-only minor tributaries, and
  PNG, self-contained SVG, and single-page PDF output. JPEG is not currently an
  approved format.
- `crates/daena-core/src/maps/atlas.rs` owns provider-neutral capability
  resolution, immutable snapshot capture, calendar binding, preset validation,
  overlay resolution, and the `.daena/cache/atlas/` location.
- `src-tauri/src/atlas_jobs.rs` owns bounded preview/export jobs, cancellation,
  application-controlled temporary artifacts, and native save behavior.
- `src/lib/maps/atlas/AtlasRenderPanel.svelte` is an export panel. It is not yet
  the interactive pan-and-zoom Studio described here.
- `scripts/maps-atlas.test.mjs` and `npm run check:maps:atlas` cover the current
  Atlas Rendering contract and are the minimum regression boundary for Studio
  work.
- The current `daena-atlas` dependency set does not include `noise-rs`, `geo`,
  or `image`. Adding one is a reviewed dependency and algorithm decision, not a
  prerequisite for opening the Studio.

The historical baseline section in `ATLAS_MAP_RENDERING.md` predates the
accepted implementation ADRs. For current-state decisions, live code and the
accepted ADRs take precedence over statements in that section saying the
renderer, jobs, presets, styles, or formats do not exist yet.

## User Experience

Atlas is available alongside the Physical Map when the accepted map reports
Atlas capability through the provider-neutral core contract.

The user can:

- open and view the detailed generated terrain;
- pan and zoom across the world;
- select a supported physical epoch;
- control presentation layers such as relief, terrain texture, water, biome coloring, contours, authored layers, labels, and decorations;
- regenerate disposable derived data when necessary;
- optionally export the whole map or supported extent as a static image.

Opening Atlas does not require an export. Viewing the generated world is a first-class feature.

## Terrain Generation Pipeline

Atlas must **amplify**, not merely upscale, the Physical Map. Interpolation may smooth the coarse grid, but terrain synthesis must add coherent sub-grid geography.

### 1. Physical constraints

Read the accepted elevation, land/water, tectonic or ridge influence, climate, precipitation, biome, ice, and hydrology products for the selected epoch. Convert them into continuous world-space control fields.

### 2. Hierarchical terrain amplification

Refine terrain through controlled subdivision while preserving large-scale landforms. The primary architectural reference is _Real-Time Hyper-Amplification of Planets_.

### 3. Mountain synthesis

Generate coherent peaks, saddles, ridges, secondary ridges, valleys, and foothills constrained by Physical Map mountain systems. Use orometry and Divide Tree techniques as the main reference rather than generic mountain noise.

### 4. Drainage refinement

Resolve artificial depressions where appropriate, calculate flow and catchment structure, and derive tributaries and valleys. Existing lakes and basins required by the Physical Map must remain valid.

### 5. Multi-scale erosion

Apply erosion and deposition during progressive refinement so new detail develops coherent valleys, slopes, drainage basins, and deposition patterns instead of appearing as interpolated noise.

### 6. Procedural detail

Use deterministic noise for local roughness, coastline variation, terrain breakup, elevation residuals, and texture modulation. Noise must be conditioned by slope, terrain type, hydrology, biome, and other physical fields. It must not independently define major geography.

### 7. Rendering

Render terrain, relief, water, biome surfaces, contours, authored layers, labels, and style effects. Large outputs must use tiled or chunked processing with bounded memory usage.

## Candidate Libraries for Later Terrain Versions

These libraries are valid candidates, but they are not current Atlas
dependencies and are not automatically required. An agent may add one only when
an accepted iteration needs it and after recording why the existing bounded,
deterministic implementation is insufficient. The dependency review must cover
license, offline operation, supported desktop targets, maintenance, lockfile
impact, deterministic behavior, numeric behavior, decoded-memory limits, and
whether using it changes a versioned output contract.

### `noise-rs`

Consider for deterministic procedural detail fields and composable terrain or
texture noise. Do not use its sequential generators, default floating-point
behavior, or output directly until world-space addressing, quantization, seams,
and cross-target reproducibility have been proven against the accepted seed
policy.

- [Razaekel/noise-rs](https://github.com/Razaekel/noise-rs)

### GeoRust `geo`

Consider `Point`, `LineString`, `Polygon`, and its geometry algorithms for
coastlines, rivers, contours, clipping, intersections, containment, and
simplification. Define longitude wrapping, antimeridian splitting, pole policy,
coordinate quantization, stable ordering, and invalid-geometry limits before
placing `geo` on a deterministic path.

- [georust/geo](https://github.com/georust/geo)
- [geo documentation](https://docs.rs/geo/)

### `image`

Consider for bounded CPU-side raster buffers, image processing, or additional
static encoders. Do not replace the accepted PNG/SVG/PDF path or widen decoded
image budgets merely to adopt the crate.

- [image-rs/image](https://github.com/image-rs/image)

Atlas should remain a focused Rust terrain/cartography pipeline rather than adopt a general-purpose game engine.

## Research References for Later Terrain Versions

These papers are architectural and algorithmic references, not permission to
copy research code or change Atlas detail algorithm `1`. Before incorporating a
technique, an agent must verify the paper and implementation license, isolate a
small reproducible spike, define conservation and topology metrics, and assign a
new version to every changed deterministic product. Reference implementations
remain test or research inputs unless their license and production suitability
are explicitly accepted.

### Controlled planetary hyper-amplification

Transforms coarse planetary control data into detailed terrain while preserving large-scale landforms and hydrosphere. This is the closest research model to the Physical Map -> Atlas relationship and should guide the overall synthesis architecture.

- [Real-Time Hyper-Amplification of Planets](https://hal.science/hal-02967067)
- [DOI 10.1007/s00371-020-01923-4](https://doi.org/10.1007/s00371-020-01923-4)
- [Reference implementation](https://github.com/Arches-Team/Real-Time-Hyper-Amplification-of-Planets)

### Multi-scale erosion amplification

Increases terrain resolution while repeatedly applying erosion and deposition, allowing newly created detail to develop hydrologically coherent structure.

- [Terrain Amplification using Multi-scale Erosion](https://hal.science/hal-04565030)
- [ACM DOI 10.1145/3658200](https://dl.acm.org/doi/10.1145/3658200)

### Orometry / Divide Tree synthesis

Models mountain structure through explicit relationships between peaks, saddles, ridges, and valleys. Use it as the primary reference for detailed mountain generation.

- [Orometry-based Terrain Analysis and Synthesis](https://hal.science/hal-02326472)
- [ACM DOI 10.1145/3355089.3356535](https://dl.acm.org/doi/10.1145/3355089.3356535)
- [Reference implementation](https://github.com/oargudo/orometry-terrains)

### Priority-Flood

Identifies and resolves closed depressions in digital elevation models to support consistent drainage analysis. Apply selectively so intentional lakes and basins are preserved.

- [Priority-Flood paper](https://doi.org/10.1016/j.cageo.2013.04.024)
- [Reference implementation](https://github.com/r-barnes/Barnes2013-Depressions)

### D-infinity flow routing

Calculates continuous flow directions and contributing area, reducing the strong grid-direction artifacts of D8 routing. Use it as the reference model for refined Atlas drainage and catchment calculation.

- [A New Method for the Determination of Flow Directions and Upslope Areas in Grid Digital Elevation Models](https://digitalcommons.usu.edu/cee_facpub/2507/)
- [DOI 10.1029/96WR03137](https://doi.org/10.1029/96WR03137)

## Determinism and Resolution

Atlas geography is generated in **world space**, not output-pixel space.

A ridge, tributary, inlet, valley, or terrain irregularity must remain in the same location across viewer zoom levels, image dimensions, formats, styles, tile boundaries, and execution order.

Output dimensions only control sampling density. A higher-resolution render reveals more of the same geography; it does not create a different world.

## Caching and Persistence

Atlas may cache expensive derived products such as:

- refined elevation;
- mountain/ridge topology;
- drainage graphs;
- refined coastlines;
- erosion results;
- contour geometry;
- rendered tiles.

These caches are not canonical project state and must not be required for Git history, checkpoints, recovery, or portability. They may be discarded and reconstructed deterministically.

## Export

Static export is optional.

Atlas Studio must call the existing Atlas Rendering path for export. Current
formats are PNG, self-contained SVG, and single-page PDF. JPEG may be added only
through a later accepted encoder decision. Normal whole-world presets may target
`4096 x 2048` and `8192 x 4096`, subject to the enforced release-build budgets.

Export must render directly from Atlas terrain and vector data. It must never upscale or screenshot the `384 x 192` Physical Map canvas.

## Detailed Implementation Guide for Agents

### 1. Product Boundary

Atlas Studio is a capability-gated, read-first map workspace for an accepted
physical map. It provides an interactive viewport, epoch and style selection,
layer visibility, feature inspection, cache regeneration, and an entry to the
existing export workflow.

The first iteration does not edit physical terrain. A later Studio tool may edit
authored map layers only by calling the existing Maps mutations with their
normal revision, request-ID, ownership, and checkpoint behavior. It must never
write elevation, climate, hydrology, biome, ice, tectonic, or Atlas-derived
detail back into the accepted physical source.

Studio visibility must be derived from the enabled Maps contribution and
`project_atlas_capabilities`. Do not hardcode the `daena-physical` provider or a
map entity type in Svelte. Disabling Maps removes the Studio contribution and
actions while preserving canonical maps, fields, presets, layers, and assets.

### 2. Required Runtime Architecture

Use one shared scene pipeline:

```text
accepted physical source + requested epoch + captured project generation
                               |
                               v
              core-authorized Atlas snapshot/session
                               |
                               v
        daena-atlas detail, drainage, styles, layers, and labels
                    |                              |
                    v                              v
          bounded Studio tiles             existing static export
                    |
                    v
           MapLibre interactive viewport
```

Ownership is fixed as follows:

- `crates/daena-atlas` owns pure tile/viewport math, deterministic terrain and
  drainage, style evaluation, label placement, rasterization, encoding, and
  cache payload validation. It does not discover a project root, open SQLite,
  call Tauri, or know about a webview.
- `daena-core` owns capability resolution, authorization, immutable project
  snapshot capture, source identity, current content generation, field and
  asset reads, authored/semantic layer resolution, and the approved local cache
  root.
- `src-tauri` owns Studio session lifetime, bounded worker scheduling,
  cancellation, custom-protocol or application-controlled byte delivery,
  project-close/app-exit cleanup, and protection of local paths and tokens.
- Svelte owns viewport state, controls, accessible loading/error UI, stale
  response suppression, and calls to typed project APIs. It does not derive
  terrain, epochs, tile extents, cache keys, or authorization decisions.

Do not send raster bytes as JSON arrays or base64 through ordinary Tauri
commands. Prefer an application-controlled protocol or URL whose handler returns
bounded encoded bytes after validating an opaque session token and tile
coordinates. The URL must expose neither project paths nor cache paths.

### 3. Studio Session and Tile Contracts

Do not overload the persisted `atlasPresets` schema or silently reinterpret
`AtlasRenderRequest` as an interactive-session contract. Introduce separate,
runtime-only versioned Rust types, with final names locked by an ADR. A minimum
contract has:

```text
AtlasStudioSessionRequestV1
  map entity ID
  physical offset or resolved authored year
  detail algorithm, level, and variant
  style ID and version
  ordered active layer IDs
  projection ID
  captured content generation and relevant revisions

AtlasStudioTileRequestV1
  opaque session ID/token
  tile schema version
  z/x/y
  logical tile size and device scale
  request ID
```

Rust validates and normalizes both contracts. Reject unknown versions, unknown
styles or layers, unsupported projections, out-of-range epochs, excessive zoom,
invalid tile coordinates, excessive device scale, integer overflow, and any
request that exceeds CPU, memory, temporary-byte, or queue budgets.

A session is an immutable view of one captured generation. It may reuse
epoch-independent residual and versioned drainage caches, but it must not query
live project state while drawing tiles. When project content changes, the UI
marks the session stale and offers **Refresh Atlas**. It may finish outstanding
tiles from the captured generation, but it cannot mix new labels or layers into
the old session.

Session and request IDs are operational. They are never persisted in project
files, presets, provenance, or checkpoints. Sessions expire after a bounded idle
period, are capped per application/project/map, and are cancelled on project
close, database replacement, or app exit.

### 4. Geographic Addressing and Tile Semantics

The initial Studio uses one locked tile scheme. Prefer Web Mercator XYZ for the
interactive viewport because MapLibre consumes it directly; keep whole-world
equirectangular for export and for any later explicitly designed globe mode.

The tile adapter must:

1. validate `z/x/y` with checked integer arithmetic;
2. convert XYZ bounds to longitude/latitude or projected fixed-point bounds in
   Rust, never in Svelte;
3. wrap longitude exactly and clamp Web Mercator latitude to `±85.051129°`;
4. evaluate Atlas geography from global world coordinates, never tile-local
   seeds;
5. render a halo large enough for relief filters, strokes, symbols, and labels;
6. crop only after all neighbor-dependent work;
7. define one stable antimeridian ownership rule; and
8. return the same pixels regardless of tile order, worker count, cache state,
   or neighboring requests.

Existing Atlas export versions use south-increasing output rows. XYZ uses a
north-origin `y`. Preserve accepted export bytes: perform the required row or
extent transformation in a dedicated Studio tile adapter and lock it with
fixtures. Do not silently flip the existing export renderer.

Zoom controls sampling density and label admission, not geographic identity.
The chosen detail level and variant remain session settings. Do not seed by zoom,
tile number, pixel coordinates, device scale, viewport dimensions, or request
order. A ridge or tributary visible at multiple zooms must occupy the same world
location.

### 5. Scene Reuse and Cache Keys

Refactor only as needed to let static export and Studio tiles consume the same
validated scene inputs. A useful internal boundary may separate:

- epoch derivation and immutable physical products;
- epoch-independent elevation residual;
- epoch-dependent drainage and masks;
- resolved authored/semantic overlays;
- style and label inputs; and
- projection/raster output.

Do not duplicate these stages for Studio. Refactoring must first prove that
current Atlas fixture hashes and provenance remain unchanged.

A tile cache key must include every input that can change tile bytes, including:

- physical identity and source hash;
- historical forcing fingerprint and normalized epoch where relevant;
- detail, drainage, renderer, tile-schema, style, and font versions/hashes;
- detail level and variant;
- projection, `z/x/y`, logical tile size, and device scale;
- ordered active layer IDs and captured layer/content hashes; and
- label-placement or metatile version where applicable.

It excludes project paths, session ID, request ID, worker number, request order,
wall-clock time, destination path, and viewport pan history. Cache entries live
under the core-owned `.daena/cache/atlas/` boundary, are size/count/age bounded,
refuse symlinks, use checksummed headers and atomic installation, and treat
missing, corrupt, partial, or old-version entries as misses.

### 6. Layers, Labels, and Inspection

Populate controls from `AtlasRenderCapabilities`. Preserve the declared map
layer order for composition and use stable IDs in requests. Never bind a removed
layer setting to another layer. Show an unavailable layer explicitly or omit it
with a diagnostic from snapshot preparation.

Physical layers remain read-only. Authored and semantic layers are resolved by
core through the existing Maps contracts, including validity intervals,
ownership, revisions, anchor resolution, and byte/count limits. Atlas Studio
must not invent a parallel location, border, route, pin, or entity-link model.

Labels require cross-tile coordination. Use deterministic global placement or
bounded metatiles with stable ownership and collision order. A label crossing a
tile edge must render once, without clipping or duplication. Higher zoom may
admit more labels, but labels visible at two zooms retain stable feature anchors.
Use only bundled, licensed, hashed fonts; never platform fonts or runtime URLs.

Feature inspection returns stable source IDs and provider-neutral feature
metadata from a bounded hit-test index. Atlas-only tributaries must remain
visibly identified as derived and cannot be mutated or promoted to canonical
geography by inspection actions.

### 7. UI and Interaction Contract

The first usable Studio contains:

- a full-size pan-and-zoom viewport with keyboard operation and a readable
  loading/error state;
- an honest epoch control using relative physical offsets unless a validated
  authored-calendar binding is available;
- style and detail selection from reported capabilities;
- ordered layer visibility controls;
- cursor coordinates and optional bounded feature inspection;
- **Refresh Atlas**, **Regenerate cache**, and **Export** actions; and
- progress that distinguishes snapshotting, deriving, rendering, cache hits,
  cancellation, and failure.

Changing style, epoch, detail, or layer inputs creates a new immutable session or
new normalized scene key and cancels/supersedes obsolete tile work. Panning and
zooming do not rebuild the physical snapshot. Debounce control changes, bound
prefetch to the visible region plus a small ring, prioritize visible tiles, and
cancel queued work that leaves that set.

**Export** converts the current geographic view and selected scene settings into
the existing Atlas render request and opens/reuses the existing export workflow.
It never screenshots MapLibre. A current-view export records geographic extent,
projection, layers, epoch, style, and detail settings; browser pixel coordinates
and session tokens never enter the request or preset.

Studio viewport, panel-open, hover, and selection state are local UI state and
not canonical project content. A future **Save viewpoint** feature requires a
separate versioned Maps-owned contract and a deliberate product decision.

### 8. Security, Failure, and Resource Rules

- Atlas Studio is fully offline. Reject styles, fonts, SVG, images, and layers
  requiring a network fetch.
- A tile protocol accepts only bounded GET-like reads for a live opaque session.
  It does not expose arbitrary filesystem paths, project IDs as authority,
  renderer internals, or write operations.
- Apply a restrictive CSP and explicit MIME/cache headers. Never inject returned
  SVG/HTML into the webview; interactive tiles should be trusted raster bytes.
- Limit zoom, tiles per session, active sessions, queued work, concurrent jobs,
  tile bytes, decoded pixels, overlay features, coordinates, labels, glyphs,
  contour levels, and total cache bytes.
- Add cancellation checks to every large derivation, label, tile, encode, and
  cache loop when the loop is introduced.
- Never hold a project writer lock or SQLite transaction while deriving,
  rasterizing, waiting for a worker, serving a tile, or saving an export.
- Fail with typed `atlas.studio.*` codes for invalid requests, unsupported
  capability, stale session, expired session, cancellation, resource limit,
  tile failure, and protocol denial. Diagnostics give a safe corrective action
  without leaking paths, SQL, secrets, or source payloads.
- Regenerating the cache deletes only validated Atlas cache entries through a
  dedicated core/Tauri operation. It must not use a caller-supplied path or
  touch canonical files.

### 9. Implementation Iterations and Exit Gates

Each iteration is a bounded vertical slice. Stop at its exit gate and record
evidence before beginning the next one.

#### Iteration 0: tile contract and feasibility spike

Required work:

1. Re-read the authority documents, accepted Atlas ADRs, and live constants.
2. Write the next ADR locking session/tile schemas, projection, XYZ orientation,
   tile size/device-scale limits, halo/metatile policy, protocol shape, queue and
   cache budgets, session expiry, and deterministic guarantee.
3. Add a pure Rust tile spike over the existing golden physical source without
   Tauri, SQLite, or project mutation.
4. Prove static Atlas fixture hashes remain unchanged after any scene refactor.
5. Measure cold/warm tile latency, pan burst behavior, peak RSS, cache bytes, and
   cancellation at the proposed maximum zoom and overlay budget.

Exit gate:

- adjacent tiles match exactly at ordinary edges and the antimeridian;
- the XYZ north/south fixture is correct and existing exports are unchanged;
- repeated, shuffled, parallel, cold-cache, and warm-cache requests produce the
  same bytes;
- the spike meets recorded UI-latency/RSS/cache budgets or the proposal is
  reduced; and
- no database, canonical file, checkpoint, or current Atlas golden is changed.

#### Iteration 1: first usable Studio

Required work:

1. Add provider-neutral Studio capability/session commands in core/Tauri.
2. Add the bounded worker/session registry and application-controlled tile byte
   channel.
3. Add the Studio entry beside Physical Map only when capability and module
   state allow it.
4. Mount the MapLibre viewport with relief style, reference epoch, physical
   layers, loading/progress/error UI, cancellation, and cache regeneration.
5. Keep the existing Atlas export panel available and route **Export** through
   it.

Exit gate:

- an accepted physical map opens Studio, pans, zooms, wraps longitude, and
  reopens after restart without changing canonical content;
- disabling Maps removes Studio and re-enabling it restores access without data
  loss;
- project close, app exit, rapid pan, session expiry, and cache corruption leave
  no leaked jobs or broken UI;
- deleting `.daena/` after a clean checkpoint rebuilds the project and Studio
  tiles without losing maps or presets; and
- the actual packaged Tauri app is exercised because browser-only tests cannot
  prove the protocol, native lifecycle, or desktop rendering boundary.

#### Iteration 2: complete interactive composition

Required work:

1. Add relative/authored epoch switching through the existing calendar binding.
2. Add all reported styles and ordered layer controls.
3. Add authored/semantic overlays, deterministic labels, and bounded feature
   inspection through shared snapshot contracts.
4. Add visible-tile prioritization, bounded prefetch, stale-generation notice,
   and explicit refresh.
5. Export the current geographic view through existing regional Atlas Rendering
   requests and support existing portable presets without adding session state.

Exit gate:

- years `1`, `42`, negative years, and no-year-zero calendars map through core
  without JavaScript date coercion;
- styles change presentation but not geography, and epoch changes preserve the
  stable residual while changing valid epoch products;
- labels and authored/semantic geometry have no tile-edge duplication, random
  movement, stale-generation mixing, or identity rebinding;
- current-view export matches the Studio scene for the same captured inputs; and
- presets, layers, and calendar binding survive restart and clean checkpoint
  reconstruction.

#### Iteration 3: terrain synthesis version spike

Do not replace detail algorithm `1` in place. Introduce a separately selectable
experimental algorithm version and keep version `1` reproducible.

Required work:

1. Build continuous, seam-wrapped, pole-safe control samplers for accepted
   elevation, crust/tectonic influence, climate, runoff, biome, ice, hydrology,
   and sea level.
2. Spike controlled hierarchical amplification in world space and prove macro
   elevation, land sign, coastline envelope, component topology, and canonical
   drainage identity conservation.
3. Add mountain topology in one bounded region using explicit peaks, saddles,
   ridges, valleys, and foothills conditioned by canonical mountain systems.
4. Compare the result against named fixtures and measurable structure metrics,
   not visual preference alone.
5. Review any new library/research-code license and lock numeric/seed behavior in
   an ADR before exposing the version in capabilities.

Exit gate:

- the new version is opt-in and version `1` fixture hashes remain unchanged;
- downsampling and topology/hydrology conservation metrics pass;
- world-space samples match across export sizes, Studio zooms, tiles, worker
  counts, epochs, and styles; and
- runtime and memory stay within a measured interactive budget or the new
  version remains experimental and hidden.

#### Iteration 4: refined drainage and multi-scale erosion

Required work:

1. Preserve intentional canonical lakes/basins, then apply a bounded
   Priority-Flood-style depression policy only to artificial refined pits.
2. Add a versioned continuous-flow model based on the D-infinity reference or a
   justified alternative, constrained to canonical watersheds and mouths.
3. Generate stable atlas-only tributary/valley identities without replacing
   canonical river IDs.
4. Apply bounded multi-scale erosion/deposition during refinement, followed by
   conservation correction and topology validation.
5. Add cancellation, iteration, allocation, and cache limits before enabling the
   new product in Studio.

Exit gate:

- intentional basins survive and artificial pits follow the locked policy;
- no refined flow crosses a canonical watershed or changes a canonical mouth;
- derived IDs and pixels survive cache deletion and restart;
- erosion does not violate macro elevation, coastline, lake, basin, or mountain
  topology tolerances; and
- the enabled interactive path remains responsive within recorded budgets.

#### Iteration 5: release hardening

Required work:

1. Finish accessibility, keyboard navigation, diagnostics, cache controls, and
   user-facing provenance/derived-feature explanations.
2. Exercise supported desktop targets, display scales, GPU/webview combinations,
   offline packaging, and app upgrade/restart behavior.
3. Lock resource budgets and golden fixtures for Studio tiles and current-view
   export alignment.
4. Remove obsolete experimental paths only after current contracts and user
   data remain covered.

Exit gate:

- the complete verification matrix below passes on supported packaged targets;
- no current Atlas static output or storage contract regresses;
- all licenses and bundled resources are present offline; and
- documentation, ADRs, capability reporting, commands, and UI copy describe the
  same released behavior.

### 10. Verification Matrix

Pure Rust tests must cover:

- request/version/range/overflow rejection;
- XYZ bounds, longitude wrap, Web Mercator latitude clamp, poles, and row
  orientation;
- world-space detail invariance across zoom, extent, tile size, device scale,
  tile order, worker count, style, format, and epoch;
- adjacent-edge, halo, metatile, label-ownership, and antimeridian seams;
- macro elevation, physical sign, topology, lake/basin, watershed, and mouth
  conservation;
- cache key completeness, corruption-as-miss, atomic writes, eviction, and
  deletion invariance; and
- cancellation and resource limits at every large stage.

Core/Tauri tests must cover:

- provider-neutral capability gating and disabled-module behavior;
- one immutable generation per session and stale-generation reporting;
- authorization and denial of guessed/expired/cross-project tokens;
- bounded sessions, queues, workers, temporary bytes, and cleanup on close,
  replacement, exit, and panic/failure paths;
- protocol MIME/CSP/cache headers and refusal of paths, writes, malformed tiles,
  and oversized requests; and
- cache regeneration constrained to `.daena/cache/atlas/`.

Frontend and rendered acceptance must cover:

- first open, loading, empty/error, retry, cache-hit, and stale-project states;
- pan, zoom, antimeridian wrap, resize, device scale, rapid control changes, and
  cancellation/supersession;
- style, epoch, layer, label, inspection, and current-view export behavior;
- keyboard navigation, focus order, accessible names, contrast, and reduced
  motion where relevant; and
- immediate state plus app restart/reopen and clean checkpoint reconstruction.

Minimum commands after a Studio change are:

```text
rtk npm run check:maps:atlas
rtk cargo test --manifest-path crates/daena-atlas/Cargo.toml --locked --offline
rtk cargo test --manifest-path crates/daena-core/Cargo.toml --locked --offline maps::atlas
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --lib atlas
rtk npm run check
rtk git diff --check
rtk git status --short
```

Run the focused commands first and the broader checks required by the touched
boundary. Passing these commands does not replace packaged Tauri inspection,
actual tile/image inspection, restart, cache-deletion, checkpoint-rebuild,
cancellation, or resource measurements.

### 11. Agent Working Instructions

Before editing:

1. Run `rtk git status --short`; inspect staged, unstaged, and untracked work and
   preserve unrelated user changes. Do not stage, commit, or push unless asked.
2. Read this document, the authority documents, accepted Atlas ADRs, and the
   files in the active iteration.
3. Use the codebase knowledge graph first for symbols, call paths, and impact.
   Use `rg` for literals, configuration, docs, fixtures, or insufficient graph
   results.
4. Verify all live Atlas/physical versions, capabilities, budgets, schemas,
   commands, paths, and tests. Do not copy this dated baseline blindly.
5. Write a short vertical-slice plan naming production paths, stored/public
   contracts, security/resource boundaries, tests, rendered/native evidence,
   and the exact exit gate.

While editing:

- Keep changes within one iteration and one accepted contract decision.
- Preserve current Atlas static fixture hashes unless an explicitly approved
  version change says otherwise.
- Keep stored schema version `1` for backward-compatible additions; use a new
  schema version only for a genuinely incompatible persisted contract.
- Keep UI visibility capability-driven and module-state-driven.
- Use typed Rust ownership for large products and stable IDs/order everywhere.
- Add limits, cancellation, diagnostics, cache invalidation, and failure tests
  with the first production path, not as later cleanup.
- Keep generated terrain, tiles, session state, hit-test indexes, and render
  artifacts disposable and outside checkpoints/Git.
- Avoid parallel edits to `crates/daena-atlas/src/lib.rs`,
  `crates/daena-core/src/maps/atlas.rs`, `src-tauri/src/atlas_jobs.rs`, Atlas
  client types, or the Studio component unless agents have explicitly divided
  ownership and agreed on the contract first.

Do not:

- increase or replace the canonical physical grid;
- make Atlas a provider or canonical world model;
- edit physical causes from Studio;
- duplicate epoch, terrain, style, layer, label, or export logic in Svelte;
- generate independent geography per tile, zoom, output size, style, or epoch;
- use screenshots, generative image enhancement, platform fonts, runtime URLs,
  wall-clock seeds, random UUIDs as geographic seeds, or unordered iteration;
- expose cache/temp paths, project handles, or renderer access to plugins;
- persist tiles, high-resolution elevation, hillshade, or generated artifacts as
  canonical project content; or
- claim completion from compilation and unit tests without the rendered,
  native, restart, recovery, cancellation, seam, and resource evidence in the
  active exit gate.

After editing:

1. Run focused tests, then the relevant broader checks.
2. Render and visually inspect the named fixtures and neighboring tiles at
   actual size.
3. Exercise packaged Tauri behavior, restart/reopen, cache deletion, and clean
   checkpoint reconstruction where the iteration requires them.
4. Run `rtk git diff --check` and `rtk git status --short`.
5. Report exact changed contracts, test/fixture evidence, measurements, deferred
   work, and unrelated worktree state. Stop at the iteration exit gate.

## Non-Goals

Atlas is not:

- a replacement for the Physical Map;
- a second canonical simulation;
- a terrain editor in the first iteration;
- a tool for changing planetary causes such as tectonics or climate;
- a raster upscaler;
- unconstrained procedural noise;
- a requirement to permanently store a high-resolution world raster.
