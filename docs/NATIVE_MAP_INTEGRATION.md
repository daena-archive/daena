# Daena Native Vector Maps implementation plan

## Status and authority

This document is the implementation authority for Daena-owned vector maps. It
is subordinate to:

- [`ARCHITECTURE.md`](./ARCHITECTURE.md), which defines entity, module, and host
  boundaries;
- [`STORAGE.md`](./STORAGE.md), which defines SQLite-authoritative runtime
  storage and portable checkpoints; and
- [`MAP_INTEGRATION_PLAN.md`](./MAP_INTEGRATION_PLAN.md), which defines the
  provider-neutral Maps domain, location references, navigation, and map
  hierarchy.

Native Vector Maps extend the existing bundled `daena.maps` module. They do not
create another map identity system, storage root, plugin, or host surface.

The initial deliverable is complete when an author can generate candidates or
import an image (skipping generation), accept the result as a normal map entity,
draw and edit Daena-owned vector layers, link vector features to entities, close
and reopen the project, and reconstruct the same canonical map after deleting a
clean `.daena/` directory.

## Scope and non-goals

Daena supports independent map providers:

1. `azgaar-fmg` owns FMG generation, source format, editing, and rendering.
2. `daena-vector` owns GeoJSON geography, vector editing, and optional imported
   image backgrounds.

The provider-neutral map entity, location, navigation, hierarchy, and
checkpoint contracts remain shared. Provider source representations do not.
An FMG source is not converted to GeoJSON. An imported image is a background
on a Native Vector Map; it is not a second map identity or paint-layer provider.

The first native-vector slice includes:

- deterministic landmass candidates;
- explicit candidate acceptance;
- **Import image**, which skips generation and stores the image as `previewAssetId`;
- an offline MapLibre renderer;
- point, line, polygon, and freehand editing through Terra Draw;
- vector layer creation, rename, style, visibility, ordering, locking, and
  deletion;
- revision-aware save and recovery;
- provider-feature linking through the existing Maps location model; and
- deterministic portable checkpoint and rebuild behavior.

It does not include countries, cultures, settlements, history, climate,
hydrology, elevation, vector tiles, topology-aware shared borders, live
collaboration, Maputnik, arbitrary MapLibre styles, or conversion to or from
other providers.

## Existing foundation

The implementation must extend these existing contracts rather than duplicate
them:

- maps are `daena.maps:map` entities;
- `maps:map` stores the versioned provider descriptor;
- `maps:layers` stores ordered layer definitions;
- provider source bytes are `maps` namespace assets owned by the map entity;
- `maps:locations` on any entity stores provider-neutral map links;
- map and location projections in SQLite are disposable;
- mutations carry request IDs and expected revisions;
- runtime writes commit to SQLite first and the checkpoint worker exports
  deterministic portable files; and
- the trusted Maps host surface already dispatches FMG and Native Vector editors.

The primary implementation locations are:

```text
crates/daena-core/src/maps.rs
crates/daena-core/src/maps/vector.rs                 # new
crates/daena-core/src/project.rs
crates/daena-plugin-api/src/{rpc.rs,catalog.rs}
packages/plugin-sdk/src/maps.ts
packages/modules/maps/manifest.json
src/lib/maps/native-vector/                          # new
src/routes/+page.svelte
src-tauri/src/lib.rs
scripts/maps-native-vector.test.mjs                  # new
docs/maps/native-vector-fixtures/                    # new
```

## Fixed architecture decisions

### One map entity and one source asset

A Native Vector Map is a normal map entity with this additive provider tuple in
the existing descriptor:

```json
{
  "schemaVersion": 1,
  "provider": {
    "id": "daena-vector",
    "adapterVersion": 1,
    "sourceFormat": "geojson"
  },
  "sourceAssetId": "018f89ec-25fc-7816-8b47-6f80905f2868",
  "previewAssetId": null,
  "defaultView": {
    "center": [0.5, 0.5],
    "zoom": 1
  }
}
```

The source asset:

- is owned by the map entity in the `maps` namespace;
- uses MIME type `application/geo+json`;
- uses a `.geojson` filename;
- is stored in the existing content-addressed runtime asset store; and
- is exported by the existing checkpoint system below `assets/maps/`.

There is no portable `maps/<id>/` directory and no direct file writer. SQLite
is live authority. The exported GeoJSON is canonical portable content, not a
second live database.

One source asset contains both base geography and user-authored vector
features. This keeps replacement revision-aware and prevents a map with many
layers from creating an unbounded set of assets. Layer metadata stays in the
`maps:layers` field.

### Canonical GeoJSON profile

The source is one RFC 7946 `FeatureCollection`. Daena accepts only this strict
profile:

- no `crs`, `bbox`, foreign top-level members, or `GeometryCollection`;
- geometry types are `Point`, `LineString`, `Polygon`, and `MultiPolygon`;
- every committed feature has a UUID string `id`, serialized as lowercase
  hyphenated text;
- every feature has a properties object containing only the keys defined
  below;
- coordinates are finite `[longitude, latitude]` pairs;
- longitude is in `[-180, 180]`;
- latitude is in `[-85.05112878, 85.05112878]` for adapter version 1;
- polygon rings are closed, contain at least four positions, and use RFC 7946
  right-hand winding: exterior counter-clockwise and holes clockwise;
- adjacent duplicate positions are removed;
- coordinates are rounded to six decimal places; and
- features are serialized in ascending `id` order with deterministic JSON
  object-key ordering and one trailing newline.

The byte-canonicalization algorithm is part of adapter version 1:

1. Decode UTF-8 and parse JSON while rejecting duplicate object keys at every
   depth, unknown members, non-finite values, and numbers written outside the
   accepted coordinate/property positions.
2. Convert each coordinate to signed integer microdegrees by multiplying by
   `1_000_000` and rounding halfway cases away from zero. Convert negative zero
   to zero.
3. Remove adjacent duplicate positions after rounding. Re-close polygon rings,
   then reject lines with fewer than two distinct positions and rings with
   fewer than three distinct positions, zero signed area, or self-intersection.
4. Apply exterior counter-clockwise and hole clockwise winding. Rotate every
   ring so its lexicographically smallest `[longitude, latitude]` position is
   first; if that position occurs more than once, choose the lexicographically
   smallest complete cyclic sequence.
5. Sort holes by their canonical coordinate sequence. Sort `MultiPolygon`
   members by descending absolute exterior area, breaking ties by canonical
   coordinate sequence. Preserve `LineString` direction because it can carry
   route meaning.
6. Sort features by UUID text. Serialize objects in this exact key order:
   collection `type, features`; feature `type, id, properties, geometry`;
   properties `daenaLayerId, kind, name`; geometry `type, coordinates`.
7. Emit compact UTF-8 JSON, decimal coordinates with at most six fractional
   digits and no exponent notation, and exactly one final `\n`.

Candidate acceptance and editor save normalize through this algorithm before
storage. A clean checkpoint rebuild requires source bytes already to be
byte-canonical and rejects a noncanonical external edit with a path-specific
diagnostic. General GeoJSON import and schema mapping remain deferred.

Feature properties are:

```json
{
  "daenaLayerId": "base",
  "kind": "land",
  "name": null
}
```

`daenaLayerId` is either the reserved string `base` or a UUID naming a vector
layer. `kind` is one of `land`, `lake`, `region`, `route`, `marker`, or
`custom`. `name` is `null` or a non-empty string of at most 256 Unicode scalar
values. Arbitrary MapLibre expressions, URLs, HTML, and provider-private
properties are not canonical data.

Base features use `daenaLayerId: "base"`. They are read-only after candidate
acceptance in the first release, use `kind: "land"` or `kind: "lake"`, and have
`Polygon` or `MultiPolygon` geometry. Every other feature must reference
exactly one existing `kind: "vector"` layer. `region` uses polygonal geometry,
`route` uses `LineString`, `marker` uses `Point`, and `custom` may use any
supported geometry type.

Rust is the canonical parser, validator, normalizer, and serializer. The
frontend may validate early for feedback, but frontend output never bypasses
Rust validation.

### Coordinates and shared map anchors

Canonical vector geometry uses GeoJSON longitude/latitude ordering. These
coordinates describe a fictional world, not Earth, and do not imply real-world
geodesic distance.

The existing Maps anchor contract remains normalized to `[0, 1]`. The native
adapter converts without changing stored geometry:

```text
x = (longitude + 180) / 360
y = (90 - latitude) / 180

longitude = x * 360 - 180
latitude  = 90 - y * 180
```

`defaultView.center` also remains normalized for descriptor compatibility.
MapLibre receives the converted longitude/latitude value.

For `daena-vector`, core validation restricts the normalized center's `y` to
`[0.027493729, 0.972506271]`, corresponding to MapLibre's Web Mercator latitude
limit. The renderer does not silently clamp invalid persisted values.

Adapter version 1 rejects a line or ring segment that crosses the antimeridian
and rejects a ring whose longitude span exceeds 180 degrees. Generated land is
limited to longitude `[-170, 170]` and latitude `[-75, 75]`. Antimeridian,
polar, and globe-aware editing require a later adapter version.

### Layer definitions

Add this variant to the existing `maps:layers` schema and the Rust/TypeScript
`MapLayerDefinition` union:

```json
{
  "id": "018f89ec-25fc-7816-8b47-6f80905f2868",
  "kind": "vector",
  "name": "Countries",
  "order": 10,
  "defaultVisible": true,
  "locked": false,
  "selector": {},
  "style": {
    "fill": "#8f6fd1",
    "fillOpacity": 0.35,
    "stroke": "#5e4893",
    "strokeWidth": 1.5,
    "pointRadius": 5
  }
}
```

Rules:

- IDs are UUIDs and unique within the map.
- Ordering is deterministic by `(order, id)`.
- `selector` is empty; geometry membership comes from `daenaLayerId`.
- Colors match `^#[0-9a-fA-F]{6}$`; alpha is represented by opacity fields.
- opacities are finite in `[0, 1]`.
- `strokeWidth` is finite in `[0, 32]`.
- `pointRadius` is finite in `[1, 64]`.
- At most 64 vector layers exist on one map.
- Existing semantic and raster layer variants retain their current meaning.

A Daena vector layer is not a MapLibre style layer. The renderer may create
fill, line, point, label, hover, and selection style layers for one Daena layer.
Those renderer objects are disposable and are never persisted.

### Data ownership and editor state

The ownership flow is:

```text
generator worker -> temporary candidate -> acceptance mutation
                                           |
                                           v
                              canonical source asset + fields
                                           |
                         +-----------------+-----------------+
                         v                                   v
                  Terra Draw draft                    MapLibre sources
                         |
                         v
                 validated asset replacement
```

MapLibre sources and Terra Draw's feature store are projections of the
canonical source asset. Neither is a durable authority.

An editing session retains:

- source asset ID and observed asset revision;
- canonical bytes loaded at session start;
- current draft FeatureCollection;
- active layer and Terra Draw mode;
- an in-memory undo/redo stack; and
- dirty, validation, conflict, and recovery state.

The editor becomes dirty only after a user-visible geometry mutation. Panning,
zooming, selection, visibility changes, and opening a tool do not dirty source
geometry. Layer metadata mutations use their own field revision.

## Resource budgets

Rust enforces these adapter-version-1 limits before commit:

- source asset: 16 MiB;
- features: 20,000;
- total positions: 200,000;
- positions in one feature: 20,000;
- polygon rings in one feature: 256;
- vector layers: 64;
- feature properties after UTF-8 serialization: 2 KiB per feature.

The frontend cancels a freehand operation as soon as its raw Terra Draw output
would exceed 8,192 positions and reports `vector.limit.exceeded`. It simplifies
accepted freehand output to at most 2,048 positions with a zoom-derived
tolerance before sending it to Rust. Rust enforces the canonical per-feature
and total-position limits on what it receives; it does not claim to validate
discarded pre-simplification points. Simplification must preserve ring closure
and must not silently repair self-intersections. A self-intersecting,
degenerate, over-budget, non-finite, or out-of-range geometry is rejected with
a typed diagnostic and remains an unsaved draft.

These are hard safety limits, not performance targets. Phase 0 records baseline
render, parse, canonicalization, and save measurements. Changing a persisted
limit or geometry rule requires an adapter-version compatibility decision.

## Deterministic landmass generator

Generation runs in a bundled Web Worker so six candidates cannot block the
Svelte UI. It performs no network, filesystem, Tauri, or project mutation.

Version 3 has these inputs:

```ts
type NativeGeneratorSettings = {
  generatorVersion: 3;
  seed: number; // uint32
  landPercent: number; // integer 15..70
  continentCount: number; // integer 1..8
  coastlineRoughness: "low" | "medium" | "high";
  islandFrequency: "none" | "low" | "medium" | "high";
};
```

The algorithm is fixed for reproducibility:

1. Derive each of six candidate seeds with `mix32(seed ^
Math.imul(index + 1, 0x9e3779b9))`.
2. Use these reference integer functions verbatim. Mulberry32 is the only PRNG;
   never use `Math.random`.

   ```ts
   function mix32(value: number) {
     let x = value >>> 0;
     x ^= x >>> 16;
     x = Math.imul(x, 0x85ebca6b);
     x ^= x >>> 13;
     x = Math.imul(x, 0xc2b2ae35);
     return (x ^ (x >>> 16)) >>> 0;
   }

   function next(state: number): [number, number] {
     state = (state + 0x6d2b79f5) >>> 0;
     let t = state;
     t = Math.imul(t ^ (t >>> 15), t | 1);
     t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
     return [((t ^ (t >>> 14)) >>> 0) / 0x1_0000_0000, state];
   }
   ```

3. Evaluate a `512 x 256` row-major scalar field. Cell `(column, row)` samples
   normalized coordinates `x = (column + .5) / 512` and
   `y = (row + .5) / 256`; `cellIndex = row * 512 + column`.
4. Place `continentCount` continent cores with a deterministic 12-sample
   best-candidate pass. The pass favors ocean margins and distance from the
   previously accepted cores, preventing several requested continents from
   collapsing onto the same center. Each core has an arbitrary normalized
   axis, a count-adjusted radius, and an elliptical aspect.
5. Build each continent independently from the core plus eight tapered lobes
   along a gently wandering spine. Four subtractive shelf kernels alternate
   sides to form connected bays and gulfs. Evaluate the oriented kernels as
   signed elliptical distance fields. Sample each continent on a deterministic
   `128 x 64` grid and subtract its own land-rank threshold before combining
   the groups. This normalization prevents one broad core from consuming the
   complete global land budget. Pairwise chains of small, overlapping water
   kernels maintain meandering ocean passages between otherwise touching
   continent groups. A narrow seed-warped pole-to-pole passage also keeps every
   generated adapter-v1 ring within the 180-degree longitude-span contract.
6. Domain-warp every sample with two low-frequency value-noise octaves. The x
   amplitudes are `.09` and `.035`; the y amplitudes are `.07` and `.03`.
   Add five coastline octaves at frequencies `1, 2, 4, 8, 16`. The octave
   weights remain low `[1, .35, .12, .04, .01]`, medium
   `[1, .5, .25, .125, .0625]`, and high `[1, .65, .42, .27, .18]`; normalized
   strengths are `.18`, `.28`, and `.38`. Lattice values and smoothstep
   interpolation continue to use the fixed `mix32` contract.
7. Generate `0`, `2`, `4`, or `7` archipelagos for none, low, medium, or high.
   Every archipelago is anchored just beyond a selected continental shelf and
   contains four to eight small oriented kernels along a curved, jittered arc.
   This produces island chains and offshore islets instead of uniformly
   scattering unrelated ellipses through the ocean.
8. Add `cellIndex * 2^-40` to break exact scalar ties. Sort a
   copy of the values numerically and choose the midpoint between the adjacent
   values around rank `floor((1 - landPercent / 100) * valueCount)`. This is
   the single contour threshold.
9. Extract polygons using `d3-contour` with smoothing enabled. Convert contour
   coordinates linearly to the generation extent. Rotate each unclosed grid
   ring to its lexicographically smallest point, then simplify cyclically:
   repeatedly remove the vertex with the smallest squared perpendicular
   distance to its two neighbors while that distance is below `.35`, `.06`, or
   `.015` grid cells squared for low, medium, or high roughness. Equal-distance
   ties remove the lowest original vertex index; never remove below three
   vertices; then close the ring.
10. Run the canonical ring cleanup and winding rules, drop polygons below four
    grid cells of absolute area, and sort polygons by descending absolute area,
    breaking ties by canonical coordinate sequence. Emit one candidate
    `Feature` with `Polygon` geometry per surviving exterior and its contained
    holes; do not emit `MultiPolygon` candidates.

Candidate previews use one deterministic SVG string per candidate, not
MapLibre or Canvas. Serialize compact SVG with
`viewBox="0 0 340 150"`, one sorted `path` using `fill-rule="evenodd"`, and no
text, external references, transforms, stylesheets, or metadata. Convert a
position to `x = longitude + 170`, `y = 75 - latitude`; use `M`, `L`, and `Z`
commands with the canonical six-decimal number formatter. The full fixed root
is `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 340 150"><path
fill="#c9a96e" fill-rule="evenodd" d="..."/></svg>` without a trailing newline.
Golden thumbnail hashes cover these UTF-8 bytes. Candidates have no Daena
entity, asset, feature, or layer IDs and disappear when the dialog closes.

The candidate upload profile is narrower than committed GeoJSON: it is a
`FeatureCollection` of polygonal features with empty properties and no IDs.
The acceptance mutation rejects every other shape, assigns `base`, `land`, and
UUID values itself, and then runs the normal canonical serializer. Normal edit
and replacement paths never accept missing IDs or properties.

The committed descriptor records generator provenance in a new optional
`generation` member:

```json
{
  "id": "daena-landmass",
  "version": 3,
  "seed": 831429,
  "settings": {
    "landPercent": 30,
    "continentCount": 3,
    "coastlineRoughness": "medium",
    "islandFrequency": "medium"
  }
}
```

Add `generation` as a strict optional member of the `daena-vector` descriptor
variant only. It is provenance, not an authority. Accepting a candidate causes
Rust to normalize its polygons, assign feature UUIDs, and persist the canonical
source. Regeneration after acceptance is out of scope and must never replace
the source implicitly.

Golden fixtures pin settings, candidate seeds, normalized geometry hashes, and
thumbnail hashes. An intentional generator change increments
`generatorVersion`; old committed maps require no migration because their
GeoJSON is already canonical.

## Rendering and editing

Add `maplibre-gl` major 5, the current compatible `terra-draw` and
`terra-draw-maplibre-gl-adapter` releases, and `d3-contour` through npm. Terra
Draw's published MapLibre adapter currently documents MapLibre majors 4 and 5;
do not move this slice to MapLibre 6 until the adapter's official compatibility
and the Phase 0 fixtures pass. Keep exact resolved versions in the lockfile and
retain license notices.

`NativeVectorMapEditor.svelte` is a trusted host-surface implementation. It is
selected when the descriptor provider is `daena-vector`. FMG continues to use
its isolated child webview. Authors can **Import image** from the generator
instead of accepting a landmass candidate. Import reuses image safety budgets
(`IMAGE_MAX_ENCODED_BYTES`, pixel, and decoded-memory caps), stores an empty
canonical GeoJSON source, and places the image on `previewAssetId`. MapLibre
renders that preview as a local image overlay under authored vector layers.

MapLibre configuration:

- use a local style object with only a background and Daena-owned GeoJSON
  sources;
- request no tiles, glyphs, sprites, telemetry, or remote URLs;
- bundle MapLibre CSS;
- use MapLibre 5's CSP bundle and self-hosted CSP worker, set its URL explicitly,
  and keep `worker-src 'self'`;
- require WebGL2 and show a typed `renderer-unavailable` diagnostic if context
  creation fails;
- call `map.remove()` and terminate workers/listeners on component teardown;
- derive one GeoJSON source for base geography and one for all authored
  features, with generated filters per Daena layer; and
- update data with `setData` only after canonical load or a valid local edit.

Terra Draw configuration:

- initialize only after MapLibre emits `style.load`;
- register select, point, line-string, polygon, and freehand modes;
- expose delete through selection, not as an independent canonical store;
- load only the active editable vector layer into Terra Draw;
- exclude the active layer from MapLibre's authored source while Terra Draw is
  rendering it, so features are never drawn twice;
- stop and clear Terra Draw before switching active layers;
- preserve feature IDs on edits and assign UUIDs to newly completed features;
- stamp `daenaLayerId`, `kind`, and `name` in the editor adapter rather than
  accepting arbitrary Terra properties; and
- unsubscribe, stop, and dispose on map switch or component teardown.

Base geography renders below all authored layers and is not loaded into Terra
Draw. Selection and hover are disposable MapLibre style layers. Map text labels
are deferred because the offline style deliberately has no glyph source;
feature names remain available to selection and inspector UI.

The toolbar-to-feature mapping is fixed: point creates `Point/marker`, line
creates `LineString/route`, and polygon creates `Polygon/region`. Freehand is a
filled-region tool and must produce `Polygon/region`; if the installed Terra
Draw mode emits a line string, the adapter closes it after at least three
distinct positions before simplification. `custom` is preserved when editing
an existing feature but is not assigned by the first-release toolbar.

## Mutations and concurrency

### Candidate acceptance

Add `maps.vector.create.begin` and `maps.vector.create.commit` RPC methods,
mirroring the bounded image-import transfer flow.

`begin` accepts the map name, generator provenance, declared byte size, and
request ID, then returns a session-bound upload handle. `commit` accepts the
handle and content hash. Rust:

1. verifies handle ownership, expiry, size, and hash;
2. parses the candidate FeatureCollection profile;
3. normalizes geometry and assigns base feature UUIDs;
4. creates the map entity, source asset, descriptor, and empty
   `maps:layers` field in one SQLite mutation transaction;
5. records the idempotency result;
6. refreshes disposable map projections; and
7. wakes the checkpoint worker.

Cancellation or validation failure removes staging data and creates no entity,
asset, or portable generation.

### Geometry save

Add `maps.vector.replace.begin` and `maps.vector.replace.commit` on top of the
existing bounded transfer manager. `begin` requires the source asset's observed
revision. `commit` receives `uploadContentHash`, verifies it against the exact
staged bytes, canonicalizes those bytes, computes a separate stored content
hash, durably installs the canonical bytes, and returns the updated asset
record. The asset record always describes the canonical bytes, never the upload
bytes. Idempotent retry returns the same asset result. A stale revision returns
the existing typed conflict and changes neither canonical bytes nor metadata.

On conflict the editor keeps its draft and offers:

- reload canonical source;
- export the draft through the existing Maps recovery-copy flow; or
- continue editing without pretending the draft was saved.

No force-overwrite action belongs in the first slice.

### Layer mutations

Extend the existing `maps.layer.create`, `maps.layer.update`, and
`maps.layer.delete` contracts instead of adding a second layer API:

- create accepts an optional `kind`; `daena-vector` defaults to `vector`;
- update adds an optional `style` object containing all five required keys
  `fill`, `fillOpacity`, `stroke`, `strokeWidth`, and `pointRadius`; when
  present it replaces the complete vector style rather than merging it.
  `style` is rejected for non-vector layers. `locked` remains an independent
  optional field. Both use the observed layers field revision; and
- delete detects `kind: "vector"`, removes all features for that layer from the
  canonical source, updates the layers field, and advances both revisions in
  one core mutation.

Vector-layer deletion requires a host confirmation that states the feature
count. Its payload requires both `expectedRevision` for `maps:layers` and
`expectedSourceRevision` for the source asset, plus the non-negative integer
`expectedFeatureCount` displayed by the confirmation. Its result contains the
updated layers field, updated source asset, and `deletedFeatureCount`. Either
stale revision, a feature count unequal to `expectedFeatureCount`, or an
asset-install failure leaves both source and layers unchanged. The core
mutation uses the existing durable runtime-asset installation pattern so a
crash recovers to the complete old or complete new state, never mixed metadata
and bytes.

### Entity links

Extend provider-feature anchor validation to accept:

```json
{
  "kind": "provider-feature",
  "provider": "daena-vector",
  "featureKind": "geojson-feature",
  "featureId": "018f89ec-25fc-7816-8b47-6f80905f2868",
  "fallbackPoint": [0.5, 0.5]
}
```

The fallback point is the point itself, line midpoint, or polygon
point-on-surface converted to normalized coordinates. Reconciliation marks a
link unresolved if its feature ID no longer exists; it never retargets by name,
position, array index, or geometry similarity.

## Error model

Core and RPC paths return stable typed codes with human-readable detail:

```text
vector.source.invalid
vector.source.unsupported-version
vector.geometry.invalid
vector.geometry.antimeridian
vector.limit.exceeded
vector.layer.missing
vector.layer.in-use
vector.generator.invalid-settings
vector.renderer.unavailable
asset.revision-conflict
transfer.invalid
transfer.expired
```

Validation errors identify a feature ID and JSON path when available but do not
echo an entire source asset. Frontend errors do not include runtime asset paths
or internal SQLite details.

## Delivery phases

### Phase 0: dependency and renderer spike

- Add the four frontend dependencies through npm and record licenses.
- Render a local GeoJSON fixture in the real Tauri host surface with no network
  requests.
- Prove the self-hosted MapLibre worker under the packaged CSP.
- Prove Terra Draw create, edit, delete, layer switch, and teardown.
- Record WebGL2 failure behavior and resource measurements at the stated
  budgets.
- Add a short ADR covering the single-GeoJSON-asset profile, coordinate/anchor
  conversion, and trusted host-surface placement.

**Exit gate:** A packaged development build edits and tears down a local
fixture offline. Repeated map open/close leaves no MapLibre instance, Terra
Draw listener, object URL, or worker owned by the closed editor.

### Phase 1: Rust contract and canonical storage

- Add vector provider, descriptor, layer, source-profile, and anchor types.
- Add the reviewed Rust GeoJSON/geometry dependencies needed by the strict
  parser and validity checks through Cargo; do not hand-roll an incomplete JSON
  parser.
- Implement strict parsing, normalization, quotas, canonical serialization,
  and stable diagnostics in `maps/vector.rs`.
- Implement candidate-acceptance and provider-aware layer mutations.
- Extend the Rust-first RPC catalog, regenerate JSON Schemas and SDK types, and
  update the Maps manifest capabilities only if catalog authorization requires
  it.
- Rebuild vector feature IDs and bounds into the disposable Maps projection.

**Exit gate:** Valid fixtures round-trip byte-identically. Invalid geometry and
limits fail before mutation. Candidate acceptance is atomic and idempotent.
Deleting `.daena/` after a clean checkpoint reconstructs the same entity,
asset, layers, feature links, and projection.

### Phase 2: deterministic generation

- Implement the worker, PRNG, scalar field, contours, simplification, and
  candidate previews.
- Add accessible controls, seed copy/paste, regenerate, cancel, and explicit
  acceptance.
- Add golden generator fixtures and a drift check.
- Wire acceptance to the Phase 1 upload/commit path.

**Exit gate:** The same settings produce the same six fixture hashes on all
supported platforms. Cancel creates no durable record. Acceptance persists
exactly one map and reopening never invokes the generator.

### Phase 3: editor and layer UX

- Add `NativeVectorMapEditor.svelte`, MapLibre style/source management, and the
  Terra Draw adapter.
- Add vector layer create, rename, style, order, visibility, lock, delete, and
  active-layer controls.
- Add dirty state, save, validation diagnostics, undo/redo, keyboard access,
  and revision-conflict recovery.
- Dispatch `daena-vector` from the existing Maps workspace and add it to both
  create-map menus.

**Exit gate:** Point, line, polygon, and freehand features survive save,
restart, map switch, layer reorder, and clean checkpoint rebuild. A stale
revision never overwrites another edit. Deleting a populated layer is atomic.

### Phase 4: linking and hardening

- Capture and resolve native feature anchors through the existing Maps
  selection bridge.
- Reconcile links after source replacement and projection rebuild.
- Exercise large valid sources, every quota, malformed JSON, duplicate IDs,
  invalid winding, self-intersections, antimeridian geometry, cancellation,
  project close, and renderer failure.
- Add packaged macOS, Windows, and Linux checks for resize, display scaling,
  focus, keyboard behavior, CSP, offline operation, and WebGL lifecycle.

**Exit gate:** Bidirectional map/entity navigation survives feature rename,
restart, module disable/re-enable, and projection deletion. Feature deletion
produces an unresolved link with its fallback point, never a silent retarget.
All supported packaged desktop targets pass the lifecycle and offline checks.

Phases are sequential. Phase 0 may add only disposable fixtures and dependency
plumbing; public RPC and stored-data changes begin in Phase 1.

## Verification requirements

Focused Rust tests must cover:

- provider tuple and MIME/ownership validation;
- every GeoJSON shape, coordinate, winding, ID, property, and quota rule;
- canonical byte stability;
- candidate create cancel/fail/retry/success;
- stale asset and layers revisions;
- atomic populated-layer deletion under crash injection;
- feature-link resolution and unresolved behavior; and
- checkpoint export, external import, and clean rebuild.

Frontend and contract tests must cover:

- generator golden hashes and cancellation;
- MapLibre source/style generation without remote URLs;
- Terra Draw ID and layer stamping;
- layer switching and teardown;
- undo/redo budget eviction;
- save, validation, and conflict state machines;
- generated Rust/JSON Schema/TypeScript contract drift; and
- create-menu and provider dispatch behavior.

Run focused tests during each phase, then the repository gates:

```sh
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
rtk cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings
rtk npm run check
rtk npm run build
rtk npm run check:plugin-contract
rtk npm run check:maps:native-vector
```

Unit and build checks do not prove WebGL rendering, CSP worker loading, native
webview lifecycle, or checkpoint recovery. Those boundaries require the
packaged Tauri checks named in the phase gates.

## Explicitly deferred

- editing accepted base geography;
- antimeridian and polar geometry;
- globe projections;
- vector tiles and level-of-detail generation;
- imported arbitrary GeoJSON and schema mapping;
- multiple source assets per native map;
- topology constraints or mutually exclusive polygons;
- polygon split, merge, cut, snapping, and shared-border editing;
- automatic semantic generation;
- advanced style expressions and Maputnik;
- rendered map text labels and bundled glyph management;
- distance, routing, climate, elevation, and simulation;
- historical source versions; and
- a third-party native-vector provider SDK.

Future work must preserve the central rule:

> Generate candidates without durable effects, commit through the Rust
> authority boundary, edit a revisioned canonical GeoJSON asset, and treat
> MapLibre and Terra Draw as disposable views of Daena-owned data.
