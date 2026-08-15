# Deterministic Atlas Map Rendering implementation guide

## Status and authority

This document defines the implementation path for turning an accepted Daena
physical world into detailed, high-resolution static maps. The feature is called
**Atlas Rendering** in this guide.

Atlas Rendering is a derived publishing pipeline. It reads the accepted physical
source, derives the requested historical epoch, adds bounded and deterministic
cartographic detail, composes selected Daena layers, and writes a static output.
It does not create a second world model, increase the canonical physical grid,
or make exported pixels authoritative.

This guide is subordinate to:

- [`ARCHITECTURE.md`](./ARCHITECTURE.md) for core, shell, module, and plugin
  boundaries;
- [`STORAGE.md`](./STORAGE.md) for SQLite runtime authority, content-addressed
  assets, checkpoints, recovery, and disposable derived state;
- [`MAP_INTEGRATION_PLAN.md`](./MAP_INTEGRATION_PLAN.md) for shared map
  identity, layer, anchor, hierarchy, and navigation contracts;
- [`NATIVE_MAP_GENERATOR.md`](./NATIVE_MAP_GENERATOR.md) for the accepted
  physical source, historical derivation, deterministic numeric policy, and
  physical invariants; and
- [`adr/0014-native-physical-world-iteration-0.md`](./adr/0014-native-physical-world-iteration-0.md)
  for the current physical provider and source boundary.

If this guide conflicts with those authorities, an implementation agent must
stop and reconcile the conflict before changing a stored or public contract. Use
the next available ADR number for any new locked contract; do not assume an ADR
number from this document.

## Product outcome

From an accepted physical map, an author can open **Render Atlas Map** and
choose:

- the physical year or an explicitly bound authored calendar year;
- a versioned map style;
- visible physical, authored, and semantic layers;
- whole-world or, in later iterations, bounded regional extent and projection;
- output pixel dimensions and optional print DPI metadata;
- output format and format-specific options; and
- a deterministic detail variant, normally left at `0`.

The app produces a detailed static map suitable for viewing, print layout, or
external publishing. Repeating the same render from the same project snapshot
produces the same geographic detail and, for deterministic encoders, the same
bytes. Rendering at another resolution or in another style keeps geographic
micro-detail in the same world locations. Changing the historical year changes
only values that the historical model or time-filtered layers say should change;
mountains, paper-independent terrain texture, and other stable detail must not
jump randomly between years.

The feature is complete when:

- no export path depends on the low-resolution interactive canvas or a MapLibre
  screenshot;
- high-resolution detail is derived from validated physical data and cannot
  alter canonical source bytes;
- stochastic detail is keyed, versioned, resolution-independent, tile-seam safe,
  and reproducible;
- the selected epoch, styles, layer content, labels, and output options are
  captured from one bounded immutable snapshot;
- cancellation, failure, or application exit leaves no corrupt destination or
  durable project mutation;
- render presets survive restart and clean checkpoint reconstruction while
  derived caches remain disposable;
- the pipeline works fully offline with bundled fonts, styles, and encoders; and
- measured CPU, memory, temporary-disk, output-size, and duration budgets are
  enforced before work begins.

## Current checkout baseline (2026-08-15)

Agents must verify these facts in the live checkout before implementing because
versions and file locations may advance:

- `daena-physical` is a normal `daena.maps:map` provider. The current provider
  tuple is adapter version `2` and source format `physical-world-v2`.
- `crates/daena-core/src/maps.rs` currently declares physical generator version
  `12`; the physical source container reports source version `2`.
- Production physical sources are currently bounded to `384 x 192` samples in
  `crates/daena-physical-spike/src/lib.rs`.
- The source contains numeric physical truth and persisted causes. Current and
  historical climate, hydrology, geography, and hazard products are derived.
- `project_physical_derived_epoch` already validates the descriptor/source pair,
  derives a requested integer physical offset year, returns GeoJSON plus climate
  and hydrology products, reports progress, supersedes stale requests, and uses
  a derived cache.
- `PhysicalWorldView.svelte` renders an interactive MapLibre globe. Its
  `paintPhysicalSurface` background is a browser canvas derived from the source
  grid. It is intentionally suitable for responsive interaction, not print or
  atlas export.
- Locked physical layers and authored map layers already share the
  `daena.maps:layers` field. Authored vector features remain separate from the
  immutable physical base.
- Timeline currently understands the explicit `physical-offset-years`
  chronology. It does not fabricate Gregorian dates for relative physical
  events.
- There is no high-resolution CPU renderer, atlas render job, atlas preset,
  bundled atlas style schema, export provenance manifest, or static-map format
  pipeline yet.

The current interactive renderer is therefore an input and preview reference,
not the implementation to upscale. Capturing its canvas at a larger browser size
would magnify the same coarse raster, depend on GPU/browser behavior, omit
reproducible layout guarantees, and fail for outputs larger than a safe WebGL
surface.

## Terms

**Canonical physical source** : The accepted immutable `daena-physical` source
asset and its validated generation metadata.

**Physical identity** : The core-produced identity of the validated
descriptor/source pair. Atlas code consumes it as opaque input and never
reimplements its normalization.

**Epoch products** : Disposable current or historical climate, water, ice,
geography, and hazard products for one physical offset year.

**Atlas detail model** : A disposable, versioned, deterministic refinement of
physical data for cartographic rendering. It may add bounded surface texture and
later derived minor features, but is never canonical geography.

**Style** : A declarative, bundled, versioned description of palette, strokes,
symbols, labels, relief, and decorations. Styles do not own world geometry.

**Render preset** : A portable project-owned recipe containing user choices. It
contains no destination filesystem path and no generated image bytes.

**Render snapshot** : A bounded immutable package of validated source data,
epoch products, layer content, assets, labels, style, and revisions captured for
one job.

**Render artifact** : A temporary encoded output plus provenance. It is not
project content until a separately designed and explicitly authorized import
operation registers it as an asset.

## Non-negotiable architecture decisions

### Atlas output is derived, not a new map provider

Atlas Rendering does not add `daena-atlas` to the map descriptor provider union.
The source map keeps its existing entity ID and provider. A render request
points to that map and asks a provider adapter for a validated provider-neutral
atlas snapshot.

The first adapter supports `daena-physical`. Later providers may opt in through
the same Rust capability contract, but the frontend must not infer support from
entity type, MIME, filename, or a hardcoded provider list. The host returns
capabilities such as:

```ts
interface AtlasRenderCapabilities {
  supported: boolean;
  timeModes: Array<"physical-offset-year" | "calendar-year">;
  projections: string[];
  formats: string[];
  maxWidthPx: number;
  maxHeightPx: number;
  maxPixelCount: number;
  supportsAuthoredLayers: boolean;
  supportsSemanticLayers: boolean;
}
```

An unsupported provider produces a typed `atlas.provider.unsupported` result,
not a disabled button based on frontend name checks.

### The Rust CPU pipeline owns export rendering

The export renderer must be a pure Rust crate, provisionally
`crates/daena-atlas`, with no Tauri, SQLite, DOM, MapLibre, plugin-host, or
ambient filesystem dependency. It receives validated immutable inputs and an
explicit progress/cancellation sink, then writes through a bounded output
abstraction.

Rust owns:

- request validation and normalization;
- deterministic seed domains and detail synthesis;
- physical interpolation, coastline refinement, relief, contours, and later
  refined drainage;
- projection, clipping, antialiasing, compositing, labels, and decorations;
- raster/vector encoding and embedded provenance; and
- exact resource accounting.

`daena-core` owns authorized snapshot creation, portable preset persistence,
revision checks, asset reads, and content-generation metadata. `src-tauri` owns
job lifecycle, host save dialogs, application-controlled temporary files,
progress events, destination confirmation, and final installation.

The Svelte UI owns choices, validation feedback, a bounded preview, job
progress, cancellation, and explicit save. Frontend checks are advisory and
never replace Rust validation.

### Render from one immutable snapshot

A render must not repeatedly query live project state while drawing. Core
preparation performs this sequence:

1. Open a consistent read transaction.
2. Resolve the map descriptor, validated physical source identity, requested
   epoch, layer definitions, time-filtered semantic content, authored geometry,
   label text, required asset records, their revisions, and the current content
   generation.
3. Enforce snapshot counts and byte budgets.
4. Read and hash required content-addressed runtime asset bytes through core
   authority. Do not read portable paths as live input.
5. Build an immutable `AtlasRenderSnapshotV1` and release the database
   transaction before CPU-heavy derivation or rendering begins.

If the project changes after capture, the job may finish truthfully from the
captured generation. The UI reports that a newer project generation exists and
offers **Render again**. It must not mix old geometry with new labels or
silently restart.

### Geographic detail and style randomness are separate

Randomness is allowed only through declared deterministic domains. Geographic
detail is derived from:

```text
physical identity
+ atlas detail algorithm version
+ user detail variant
+ named detail domain
+ world-space address or canonical feature ID
```

It deliberately excludes output format, output dimensions, style ID, tile
execution order, thread count, and historical year. These exclusions keep a
ridge, inlet irregularity, or optional derived tributary in the same world
location across formats, resolutions, styles, tiles, and epochs.

Style-only effects use separate domains. Antique paper grain is interpolated
on a 0.1° world lattice from style id/version and variant. It must never
influence coastline, relief, drainage, label identity, or any other
geographic result. Per-pixel hash grain is forbidden: it reads as static and
breaks tile seams.

Do not use `Math.random`, process-random hash keys, iteration-order-dependent
PRNG streams, platform fonts, GPU shader randomness, wall-clock time, or random
UUIDs as visual seeds. A job ID is operational identity only.

### Resolution does not define geography

Output dimensions control sampling and printable size, not which world exists.
Detail levels define world-space minimum wavelengths and feature thresholds. A
low-resolution render antialiases detail away; a high-resolution render reveals
more samples of the same detail field.

This requires world-space noise, canonical stable feature IDs, deterministic
sort order, and tile halos. An implementation that generates independent noise
per output pixel or per tile is invalid even when repeated renders at one size
look stable.

### Export never mutates the accepted world

Rendering, caching, previewing, cancelling, and saving an external file do not
advance project content generation. Saving or editing a render preset is a
normal revisioned project mutation and does advance it. Registering an output as
a Daena asset, if later requested, is a separate explicit import operation; it
must not happen automatically after export.

Atlas detail caches and job artifacts are disposable. They do not appear in the
checkpoint manifest or Git. Deleting them cannot delete presets, map sources,
authored layers, entity links, or exported user files.

## User experience contract

### Entry and layout

An accepted map that reports atlas capability exposes **Render Atlas Map** in
the map workspace. The action opens an in-app panel, not a second application
webview. The panel contains:

1. a preview using the same CPU renderer at a bounded preview size;
2. time selection;
3. style and detail choices;
4. an ordered layer list with visibility controls;
5. extent/projection controls when supported;
6. width, height, aspect-lock, DPI, format, and format options;
7. an estimated memory, temporary-disk, and output-size summary; and
8. **Render**, **Cancel render**, **Save**, and **Close** actions.

Changing a control schedules a debounced preview job. A newer preview cancels or
supersedes the older one. Export uses an explicit button and never starts merely
because a control changed.

### Time selection

The physical model currently uses signed integer offsets from the accepted
source reference epoch. The initial UI must label this honestly:

- `0`: **Reference epoch**;
- negative: **N years before reference**; and
- positive: **N years after reference**.

Do not label these offsets as Gregorian or authored calendar years.

To offer a literal authored **Year** control, add an optional versioned map
binding through a normal `maps` field, provisionally:

```json
{
  "schemaVersion": 1,
  "calendarId": "<shared calendar id>",
  "calendarReferenceYear": 1200,
  "physicalOffsetAtReference": 0
}
```

The shared calendar service, not JavaScript `Date`, computes the signed number
of year transitions between the reference and selected authored year. Years `1`,
`42`, negative years, and calendars without a year zero remain literal under
that calendar's rules. Never add `1900`, create a missing month/day, or silently
assume Gregorian chronology.

The normalized render request always contains the resolved physical offset and
records the authored selection and binding revision when one was used. If the
binding or calendar is unavailable, the UI falls back to the explicit physical
offset control without changing stored values.

### Layer selection

The layer list starts from the current `daena.maps:layers` order and visibility.
It may include:

- derived physical roles such as relief, ocean, land, ice, lakes, rivers,
  coastlines, contours, hazards, or graticules;
- authored vector and raster layers owned by the map;
- semantic layers resolved from entities, relationships, locations, and validity
  intervals; and
- atlas-only decorations such as labels, title, legend, scale bar, and frame.

The request carries stable layer IDs and explicit visibility. It never copies
entity names into a persisted preset. Snapshot preparation resolves current
names and content. If a saved preset references a removed layer, the UI shows it
as unavailable and excludes it only after telling the user; it must not bind
that setting to a different layer.

Disabling the Maps module removes the action and contribution UI. Durable map
and preset data remains available for recovery and checkpoint export.

### Resolution and print size

The UI treats pixel dimensions as the detail-bearing setting. DPI is metadata
and a print-size calculator:

```text
print width in inches = width pixels / DPI
```

Changing DPI alone must not rerender or claim additional detail. Provide named
pixel presets plus validated custom dimensions. Preserve projection aspect ratio
by default, with an explicit unlock only for projections/extents that permit it.

The first production budget should be locked by the feasibility spike. A safe
starting proposal is:

- preview: at most `2048 x 1024` and 2.1 million pixels;
- normal export presets: `4096 x 2048` and `8192 x 4096`;
- first-release hard maximum: 33,554,432 pixels;
- one active export job per application and one preview per map; and
- tiled rendering with a measured working-set ceiling rather than a full-image
  RGBA allocation assumption.

These are proposal values, not permission to skip measurement. The ADR must
record measured release-build time, peak RSS, temporary bytes, and encoded sizes
on supported desktop targets before locking them.

### Formats

Format availability comes from runtime capabilities. The intended roadmap is:

| Format | Initial role                                                  | Determinism and safety rule                                                                      |
| ------ | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| PNG    | Required first format; lossless atlas and transparency        | Pinned CPU encoder; exact byte golden where supported                                            |
| JPEG   | First usable iteration if the feasibility spike stays bounded | Explicit quality and matte color; no alpha; pinned encoder                                       |
| WebP   | Optional later compact raster                                 | Add only with an offline, pinned, cross-target deterministic encoder                             |
| SVG    | Later vector/hybrid publishing                                | Embed bounded raster relief and bundled font outlines or references; no scripts or external URLs |
| PDF    | Later print output                                            | Fixed page box, embedded fonts, bounded images, no external resources or active content          |

The UI must never promise an unavailable format. Format-specific controls appear
only when that format is selected. An extension mismatch is corrected before the
save dialog completes or rejected with a direct diagnostic.

### Save behavior

Rendering first creates an application-controlled temporary artifact. **Save**
opens a host-owned save dialog. The destination path is never supplied by a
sandboxed module or plugin.

Installation must:

1. obtain explicit overwrite confirmation from the host when needed;
2. copy to a validated sibling partial file without following a symlink;
3. flush and close the partial file;
4. atomically replace the confirmed target where the platform supports it;
5. stop on lock, permission, or replacement failure; and
6. delete only the validated temporary/partial file on cancellation or error.

Do not retry with a broader path, stronger deletion primitive, or silent
fallback directory. The UI retains the completed temporary artifact long enough
for a retry, subject to a bounded expiry policy.

## Stored and runtime contracts

### Persisted render preset

Add a new optional `maps` field on `daena.maps:map`, provisionally
`atlasPresets`, while keeping schema version `1` backward-compatible. Do not
change the physical descriptor or existing layer schema merely to store UI
choices.

```json
{
  "schemaVersion": 1,
  "presets": [
    {
      "id": "018f-atlas-preset",
      "name": "Political atlas, 1200",
      "time": {
        "kind": "physical-offset-year",
        "offsetYears": -800
      },
      "detail": {
        "algorithmVersion": 1,
        "level": "detailed",
        "variant": 0
      },
      "style": {
        "id": "daena-atlas-political",
        "version": 1
      },
      "activeLayerIds": ["relief", "ocean", "rivers", "political", "labels"],
      "viewport": {
        "kind": "world",
        "projection": "equirectangular"
      },
      "output": {
        "widthPx": 8192,
        "heightPx": 4096,
        "dpi": 300,
        "format": "png"
      }
    }
  ]
}
```

Preset IDs are stable. Preset updates use request IDs and expected field
revisions. Names are user-authored. Destination path, job ID, cache key,
generated bytes, and live entity names never enter this field.

The exact schema must be generated/validated through the normal Rust contract
path and added to plugin SDK declarations if public modules can read it. Do not
hand-maintain divergent TypeScript and Rust unions.

### Normalized render request

UI input is normalized and validated in Rust before snapshot creation:

```json
{
  "schemaVersion": 1,
  "mapEntityId": "<map UUID>",
  "time": {
    "kind": "physical-offset-year",
    "offsetYears": -800,
    "authoredSelection": null
  },
  "detail": {
    "algorithmVersion": 1,
    "level": "detailed",
    "variant": 0
  },
  "style": {
    "id": "daena-atlas-relief",
    "version": 1
  },
  "activeLayerIds": ["relief", "ocean", "lakes", "rivers", "labels"],
  "layerOverrides": {},
  "viewport": {
    "kind": "world",
    "projection": "equirectangular",
    "centralMeridianMicrodegrees": 0
  },
  "output": {
    "widthPx": 8192,
    "heightPx": 4096,
    "dpi": 300,
    "format": "png",
    "background": "opaque",
    "jpegQuality": null
  },
  "decorations": {
    "title": null,
    "legend": true,
    "scaleBar": false,
    "graticule": true,
    "frame": true
  }
}
```

Unknown fields are rejected for the locked request version. Normalize enum case,
numeric ranges, layer ordering, and default values exactly once in Rust. The
normalized request is canonically encoded and hashed for cache and provenance
use.

### Render snapshot

`AtlasRenderSnapshotV1` is runtime-only and should contain, at minimum:

- normalized request and request hash;
- project ID only if required internally, never an ambient project handle;
- captured database epoch, content generation, and relevant revisions;
- validated physical source, opaque physical identity, and source hash;
- normalized epoch and historical derivation provenance;
- provider-neutral physical fields/products needed by the renderer;
- ordered resolved layers with stable source IDs and content hashes;
- bounded authored vector/raster bytes;
- resolved semantic geometry and literal display labels;
- validated bundled style bytes, style hash, and bundled font hashes; and
- exact resource estimates and enforced limits.

Avoid serializing the snapshot through JSON when it contains large numeric
arrays or image bytes. Keep it as typed Rust data and move ownership into the
blocking render job.

### Provenance manifest

Every output has an `AtlasRenderProvenanceV1`. Embed a compact form in PNG text
chunks, JPEG metadata, SVG metadata, or PDF metadata. Offer an adjacent JSON
sidecar when the user enables **Write render details**.

Record:

- atlas request schema and renderer versions;
- normalized request hash;
- physical identity and canonical source hash;
- requested and normalized physical offset;
- authored calendar selection and binding revision when used;
- historical, climate, hydrology, hazard, and atlas-detail derivation versions;
- detail variant and named seed-policy version, not every expanded seed;
- style ID, version, content hash, and bundled font hashes;
- ordered active layer IDs and captured layer/content hashes;
- database epoch and content generation as opaque snapshot provenance;
- projection, extent, width, height, DPI, format, encoder version, and options;
- renderer target/version where needed to explain an unsupported byte-level
  guarantee; and
- encoded output SHA-256 and render completion time as metadata only.

Do not embed absolute project paths, temporary paths, user account names, or
machine identifiers. Timestamps and output hashes do not participate in
geographic generation or the pre-encode render cache key.

## Deterministic atlas-detail model

### Required invariants

Atlas detail must satisfy all of the following:

1. **Macro preservation.** Downsampling refined elevation to the canonical grid
   reproduces canonical cell means within a locked integer tolerance.
2. **Topology preservation in the first release.** Detail may roughen a coast
   inside a bounded coastal envelope, but must not create or remove continents,
   islands, lakes, or river identities.
3. **Physical sign preservation.** Cells outside the coastal envelope cannot
   cross the epoch sea level because of decorative noise.
4. **Hydrology preservation.** Existing lake, river, basin, and mouth IDs remain
   stable. Refined relief cannot redirect a canonical river to another basin.
5. **World-space stability.** Detail is invariant under output dimensions,
   format, tile order, worker count, and style.
6. **Epoch stability.** The underlying residual terrain is the same for every
   year. Changed water/ice masks reveal or cover that same terrain.
7. **Seam safety.** Longitude wrap, projection cut, tile edges, and poles show
   no discontinuity not present in the source.
8. **Bounded evaluation.** Noise octaves, contour levels, refinement grids,
   label candidates, recursion, and retry counts have explicit limits.

Later topology-changing derived detail, such as new minor islets or tributary
IDs, requires a separate derivation version, explicit UI wording, fixtures, and
a design for stable selection/reference. It must not slip into palette or noise
tuning.

### Refinement stages

The initial deterministic detail algorithm should use this causal order:

1. Decode and validate the physical source through the existing core path.
2. Derive the requested historical epoch before any atlas refinement.
3. Construct seam-wrapped continuous samplers for elevation, bathymetry,
   climate, runoff, ice, and other required coarse fields. Use fixed-point or
   explicitly quantized arithmetic at locked boundaries.
4. Compute a signed coastal distance field from the epoch land/water mask.
5. Build a band-limited elevation residual in spherical/world coordinates. Named
   domains may include `continental-relief`, `mountain-ridges`,
   `erosion-texture`, `coastal-detail`, and `seafloor-detail`.
6. Modulate amplitude by canonical relief, slope, crust, tectonic boundary
   distance/type, climate, and drainage. Noise never invents its own plate or
   biome classification.
7. Remove the residual mean per canonical cell or conservative support region so
   refinement cannot drift macro elevation.
8. Clamp residuals outside the coastal envelope to preserve the epoch land/water
   sign. Inside the envelope, perturb the coastline only within a locked
   fraction of a source cell and preserve connected-component topology.
9. Recompute display-only high-resolution slope, normals, hillshade, local
   occlusion, hypsometric tint, and contours from the refined field.
10. Snap/taper canonical rivers and lake outlines against the refined relief
    without changing their stable identity or basin destination.
11. Emit a provider-neutral detail model or deterministic tile evaluator for the
    projection/composition stage.

Do not use unconstrained image super-resolution or a generative model. Such a
model cannot prove topology, source conservation, cross-platform determinism,
offline availability, or stable detail across future renders.

### Seed domains and addressing

Use a keyed hash/PRF with a locked byte encoding. Each lookup includes:

```text
"daena-atlas-detail-v1\0"
physical identity length + physical identity bytes
detail algorithm version
detail variant
domain length + domain bytes
world-space lattice coordinate or canonical feature ID
octave/scale ID
```

World-space lattice coordinates are derived from spherical longitude/latitude or
another explicitly documented planet coordinate system, not output pixels.
Longitude addressing wraps. Pole sampling has one explicit convergence policy.
Large integer coordinates use checked arithmetic.

Do not consume one sequential random stream across the map. A new draw in one
mountain tile must not alter a coastline or another tile. Add a new named domain
when a subsystem needs randomness; changing an existing domain's meaning
requires a new detail algorithm version.

### Level of detail

Define a small versioned set of detail levels in world units, for example:

- `standard`: regional relief and atlas-scale contours;
- `detailed`: smaller ridges, coastal irregularity, denser contours; and
- `print`: the smallest approved wavelength and highest label density.

The ADR locks their world-space wavelength/amplitude ranges and compute budgets.
Do not define them as “four noise octaves per 4K output.” Output size only
changes sample density and antialiasing.

### Tiled evaluation

Render in fixed-size output tiles with a style-dependent halo large enough for
filters, strokes, symbols, and labels. A proposed starting tile size is
`512 x
512`, but measurement decides the locked value.

Every tile is evaluated from global world coordinates and global feature IDs.
Never seed by tile number alone. Crop the halo only after all operations that
need neighbors. Test that:

- a full-frame render equals the tiled render;
- forward, reverse, and randomized tile execution produce the same bytes;
- thread counts `1` and `N` produce the same bytes; and
- tiles on both sides of the antimeridian and projection cut join exactly.

## Cartographic composition

### Projection and extent

Iteration 1 supports whole-world equirectangular output only. It matches the
canonical global coordinate coverage, has a clear `2:1` aspect ratio, and avoids
pretending that one projection fits every atlas use.

Later projection support is added through versioned projection IDs and bounded
parameters, not arbitrary projection scripts. Candidate later modes are:

- regional Web Mercator within its latitude limit;
- equal-earth or another equal-area world projection;
- orthographic globe views; and
- explicit longitude/latitude regional extents.

Each projection defines valid domains, cut handling, central meridian, inverse
mapping, clipping, scale-bar validity, and label-placement coordinates. Invalid
or self-crossing regions are rejected before a job starts.

### Composition order

The renderer uses one explicit deterministic order:

1. background/matte;
2. ocean and bathymetry;
3. refined land tint, relief, and ice;
4. physical polygons and contours;
5. lakes and rivers;
6. authored raster layers;
7. authored vector layers;
8. semantic areas, paths, and points;
9. labels and symbols;
10. graticule, scale bar, legend, title, and frame; and
11. format conversion and metadata.

Within a stage, sort by persisted layer order, then stable layer ID, then stable
feature/entity ID. Never depend on SQL row order, hash-map order, GeoJSON input
order unless that order is itself a stored contract, or worker completion order.

### Physical layers

Map existing physical layer roles to atlas renderer roles in Rust. Keep the
mapping versioned and validated. Interactive MapLibre styles are not a print
style specification and should not be parsed as one.

The first release should include at least:

- relief/hypsometric land;
- ocean/bathymetry;
- coastlines;
- land ice;
- lakes;
- canonical rivers;
- elevation and bathymetric contours;
- optional tectonic/hazard diagnostic overlays; and
- graticule and frame.

Diagnostics remain off by default and retain explanatory legends. Hazard
rendering must say that values are relative generated rates, not real-world
predictions.

### Authored and semantic layers

Authored vector geometry is projected from its canonical normalized/world
coordinates and clipped without rewriting it. Raster assets use validated
dimensions, MIME, ownership, revision, and decoded-memory limits. Alpha and
resampling behavior are locked by renderer version.

Semantic layer resolution reuses the shared map projection rules. It reads
stable entity/location/relationship IDs, validity intervals, and current display
names from the captured snapshot. Atlas code must not invent a separate pin,
border, route, or entity-link model.

Provider-feature selectors must already be resolvable into provider-neutral
geometry by the source adapter. An unresolved selector is reported in the
preview/export summary and omitted or rendered with an explicit fallback
according to the existing anchor contract. It is never silently rebound.

### Labels

Labels are data plus deterministic layout, not pixels captured from the
interactive UI. The label pipeline must:

- use bundled, licensed, hashed font files only;
- shape text with a pinned offline library;
- preserve Unicode and authored display text;
- generate candidates from stable feature/entity IDs;
- apply style-defined priorities and zoom/scale thresholds;
- score candidates using fixed/quantized arithmetic;
- resolve collisions in stable priority and ID order;
- use deterministic line-breaking and abbreviation rules; and
- record omitted-label counts in diagnostics.

Style changes may alter label placement because label rules are stylistic.
Changing output resolution may admit more labels, but labels already present at
both resolutions should retain stable anchors when space permits.

### Styles

Bundle styles as validated declarative JSON under a dedicated resource path,
with a JSON Schema and content hash. A style can define:

- physical palettes and hypsometric stops;
- hillshade direction/strength and contour intervals;
- per-role stroke, fill, casing, symbol, and opacity rules;
- label families, priorities, sizes, halos, and abbreviations;
- graticule, title, legend, scale bar, and frame rules;
- optional bounded paper/ink texture domains; and
- default background/alpha behavior.

It cannot contain JavaScript, shaders, filesystem paths, remote URLs, CSS, HTML,
arbitrary fonts, executable expressions, or unbounded filter graphs.

Ship at least two materially distinct styles in the first usable iteration so
style selection is a real feature:

- `daena-atlas-relief`: clean physical/topographic atlas;
- `daena-atlas-biome`: climate-class cover (ice, tundra, arid, grassland, forest);
- `daena-atlas-temperature`: land temperature ramp from the epoch climate field;
- `daena-atlas-precipitation`: land rainfall ramp from the epoch climate field;
- `daena-atlas-bathymetry`: ocean-led hypsometry with muted land;
- `daena-atlas-hydrology`: muted land with rivers, lakes, and watersheds;
- `daena-atlas-antique`: restrained antique-paper cartography; and
- `daena-atlas-political`: authored/semantic territories.

A political style belongs in the iteration that composes authored/semantic
territories. User or plugin style packs are deferred until the schema,
licensing, resource budgets, and trust boundary are proven. If added, they are
declarative assets validated by Rust; plugins never receive renderer or
filesystem access.

## Job, cache, and failure contract

### Host commands

Use typed main-shell commands similar to:

```text
project_atlas_capabilities(mapEntityId)
project_atlas_preview_begin(request, requestId)
project_atlas_render_begin(request, requestId)
project_atlas_job_status(jobId)
project_atlas_job_cancel(jobId)
project_atlas_artifact_save(jobId)
project_atlas_artifact_discard(jobId)
```

The exact names are not locked here. Generate TypeScript declarations from the
Rust contract when the repository's normal tooling supports it.

`begin` returns promptly after validation/snapshot capture and schedules CPU
work away from the async/UI thread. Progress events contain job ID, request ID,
phase, completed, total, and a monotonic sequence. Stale preview progress is
ignored by request ID.

### State machine

```text
validating -> snapshotting -> deriving-epoch -> refining-detail
-> rendering -> encoding -> ready-to-save -> saving -> saved
```

Terminal alternatives are `cancelled` and `failed`. Cancellation is cooperative
and checked at bounded intervals in every large loop, label stage, tile stage,
and encoder. A cancelled or failed job cannot transition to `ready-to-save`.

Jobs have bounded lifetimes. Project close, database replacement, and app exit
cancel relevant work and remove only application-owned temporary artifacts.
Saving a completed artifact may finish from its captured snapshot even if the
project later changes, but the UI shows the captured generation.

### Caches

Use separate cache keys for:

1. epoch products;
2. atlas geographic detail;
3. resolved layer/label snapshot products where safe; and
4. final pre-encode or encoded artifacts.

The geographic detail key includes physical identity, normalized epoch only for
epoch-dependent masks/products, detail version, detail level, variant, and
required derivation versions. Stable terrain residuals should be stored under an
epoch-independent subkey so year changes reuse them.

Final artifact keys include the normalized request hash and captured content
hashes. They exclude destination path and job ID.

Put disk caches only under a core-owned, explicitly documented `.daena` local
cache path after reconciling it with `STORAGE.md`. Enforce total bytes, entry
bytes, age, count, and LRU behavior. Validate cache headers and checksums before
use. A missing, malformed, old-version, or partially written cache entry is a
miss, never a project error. Cache writes use application-owned staging and
atomic installation.

### Typed failures

At minimum distinguish:

- `atlas.provider.unsupported`;
- `atlas.request.invalid`;
- `atlas.time.unavailable`;
- `atlas.style.unavailable` or `atlas.style.invalid`;
- `atlas.layer.unresolved` and `atlas.layer.over-budget`;
- `atlas.asset.invalid`;
- `atlas.resource-limit`;
- `atlas.snapshot.changed` when a required precondition changes during capture;
- `atlas.render.cancelled`;
- `atlas.render.failed`;
- `atlas.encoder.failed`; and
- `atlas.save.failed`.

Diagnostics identify the failed stage and safe corrective action. They do not
expose absolute internal paths, SQL, plugin session secrets, or large source
payloads.

## Security and resource boundaries

- Rendering is offline. Reject any style, SVG, font, image, or layer that would
  require a network fetch.
- Treat SVG and imported rasters as untrusted input. Reuse the existing asset
  validation/sanitization authority and apply decoded pixel, nesting, path,
  filter, and decompression limits before snapshot completion.
- Do not raw-inject SVG/HTML into a webview for preview. Preview displays bytes
  produced by the trusted renderer through an application-controlled URL or
  bounded binary channel.
- Do not transfer a high-resolution RGBA frame or encoded artifact as a JSON
  number array/base64 string through ordinary Tauri or plugin RPC.
- Plugins receive no atlas renderer handle, job temp path, save path, font
  directory, database handle, or arbitrary style execution.
- Validate dimensions with checked multiplication before allocation. Account for
  tile buffers, halos, masks, label indexes, decoded assets, encoder state, and
  cached intermediates, not only `width * height * 4`.
- Enforce maximum vector features, coordinates, labels, glyphs, raster layers,
  decoded pixels, contour levels, output bytes, temporary bytes, and duration.
- One project cannot starve the UI with unlimited preview churn. Use a bounded
  queue and supersede obsolete previews.
- Never hold a project writer lock or SQLite transaction during derivation,
  rendering, encoding, dialog interaction, or destination I/O.

## Proposed code boundaries

Agents must confirm the live repository shape before creating files. The
intended ownership is:

```text
crates/daena-atlas/
  src/request.rs          normalized request and budgets
  src/detail.rs           stable physical refinement
  src/projection.rs       projection and clipping
  src/style.rs            declarative style validation
  src/labels.rs           shaping, candidates, collision
  src/render.rs           tiled composition
  src/encode.rs           PNG/JPEG and later formats
  src/provenance.rs       canonical manifest and embedding

crates/daena-core/src/maps/atlas.rs
  capability resolution
  authorized snapshot capture
  preset validation and persistence
  asset/layer/semantic resolution

src-tauri/src/atlas_jobs.rs
  bounded job registry
  progress/cancellation
  temp artifacts and host save flow

src/lib/maps/atlas/
  AtlasRenderPanel.svelte
  request/preset UI models
  bounded preview/artifact display

docs/maps/atlas/
  schemas, style fixtures, render fixtures, budgets, and licenses
```

If the current Tauri adapter is not yet split into modules, an iteration may
keep a small adapter in `src-tauri/src/lib.rs`; do not combine the pure renderer
or full job registry into that already broad file merely to avoid one module.

## Implementation iterations

Each iteration is a vertical slice. Do not begin the next iteration until all
exit-gate evidence for the current one is recorded. A passing unit test is not
proof of native save-dialog behavior, large-output memory, cancellation, restart
persistence, or checkpoint recovery.

### Iteration 0: contract and deterministic rendering spike

#### Prerequisites

- Re-read the authority documents and current physical provider/derivation
  constants.
- Inspect the worktree and preserve unrelated changes.
- Index/refresh the code graph if needed and trace the current physical epoch,
  map layer, asset, and checkpoint paths.

#### Required work

1. Write the next ADR locking request/preset/provenance versioning, renderer
   ownership, initial formats, CPU rendering library choices, font/shaping
   stack, integer/quantization policy, seed PRF, tile/halo policy, and cache
   location.
2. Add a pure Rust spike that consumes the existing physical golden fixture and
   emits a whole-world equirectangular PNG without Tauri or project mutation.
3. Implement one stable world-space detail domain and prove that its samples do
   not depend on dimensions, tile order, or thread count.
4. Render at `2048 x 1024`, `4096 x 2048`, and the proposed maximum. Record
   release-build duration, peak RSS, temporary bytes, output bytes, and hash on
   every supported CI/desktop target available.
5. Compare fixed CPU raster/encoder libraries for cross-target output. If exact
   bytes differ, identify the exact library/stage and narrow the supported
   guarantee in the ADR before product data exists.
6. Add visual fixtures for coast, pole, antimeridian, mountains, flat terrain,
   lakes, rivers, and ice. Do not tune indefinitely after named metrics pass.
7. Record bundled font/style licenses and prove no runtime URL is needed.

#### Exit gate

- The same geographic sample queries are exact across dimensions, tiles,
  execution order, and supported targets.
- Tiled and untiled fixture pixels are identical.
- Downsample conservation, coastline envelope, topology, hydrology identity,
  pole, and seam invariants pass.
- The maximum proposal stays within recorded CPU/RSS/temp/output budgets or the
  proposal is reduced before proceeding.
- The PNG decodes to the requested dimensions, contains bounded provenance, has
  no external references, and matches the locked deterministic guarantee.
- No database, project file, or portable checkpoint is changed by the spike.

### Iteration 1: first usable atlas export

#### Product slice

An author can render an accepted physical map at a chosen relative physical year
using either the relief or antique style, choose physical layers, choose bounded
pixel dimensions, export PNG and—only if Iteration 0 approved it—JPEG, cancel
safely, and save through the native host.

#### Required work

1. Add provider-neutral capability reporting and the `daena-physical` snapshot
   adapter. Keep provider checks in Rust provider dispatch.
2. Add request normalization, resource estimation, immutable snapshot capture,
   and the pure renderer boundary.
3. Reuse the existing historical derivation contract for the selected physical
   offset. Do not create another historical model or frontend calculation.
4. Implement relief, bathymetry/ocean, coast, ice, lake, river, contour,
   graticule, and frame roles with explicit composition order.
5. Bundle and validate the relief and antique styles plus fonts/licenses.
6. Add the bounded Tauri job registry, progress, supersession, cancellation,
   temp artifact lifecycle, and host-owned save flow.
7. Add the in-app panel and CPU-rendered preview. Do not use a MapLibre
   screenshot for preview or export.
8. Embed provenance and expose a compact render summary after completion.
9. Add a focused `check:maps:atlas` command covering Rust tests, contract drift,
   fixture hashes, style validation, licenses, and frontend diagnostics.

#### Exit gate

- A packaged desktop render at reference, negative, and positive offset years
  succeeds offline and shows the correct epoch-specific sea, ice, climate, and
  hydrology response.
- Repeating one request from the same captured project generation produces the
  locked deterministic result after app restart.
- Geographic detail stays aligned across both styles, PNG/JPEG where enabled,
  `4096 x 2048`, and `8192 x 4096`.
- Layer toggles change only their declared composition stages.
- Cancelling during snapshot, refinement, tile rendering, encoding, and saving
  leaves no durable mutation, corrupt destination, or leaked temp artifact.
- A concurrent project mutation cannot mix generations; the completed result
  reports its captured generation and the UI reports that newer data exists.
- Native save, overwrite confirmation, locked-file failure, and retry are
  exercised directly in Tauri, not inferred from browser tests.

### Iteration 2: authored atlas, calendar year, and portable presets

#### Product slice

An author can bind the physical reference epoch to a project calendar, enter a
literal authored year, compose authored/semantic layers and deterministic
labels, use a political style, and save named render presets that survive clean
checkpoint reconstruction.

#### Required work

1. Add and validate the optional physical/calendar binding through the shared
   calendar contract. Preserve literal fictional years and calendars without a
   year zero.
2. Add revisioned `atlasPresets` persistence using the existing map entity and
   `maps` namespace. Keep schema version `1` backward-compatible.
3. Resolve authored vector/raster layers and semantic layers into the immutable
   snapshot with strict counts, bytes, validity intervals, ownership, and
   revision checks.
4. Implement bundled font shaping, deterministic label candidates/collision,
   stable symbols, legend entries, title, and political style.
5. Report missing layers, unresolved anchors, omitted labels, unavailable
   fonts/styles, and filtered time content before final render.
6. Include binding revision and all captured layer/content hashes in provenance
   and artifact cache keys.
7. Verify disabled-module behavior: contributions disappear, while project data
   and presets remain portable and recoverable.

#### Exit gate

- Authored years `1`, `42`, negative years, and the project's no-year-zero rule
  map to the expected physical offsets without JavaScript date coercion.
- Labels and semantic layers are stable under restart, tile order, and thread
  count; higher resolution can add labels but does not randomly move common
  candidates.
- Layer edits during a render do not leak into the captured result. A rerender
  captures the new revision.
- Presets round-trip, reject stale revisions/request-ID mismatches, survive map
  reopen, survive deleting `.daena/` after a clean checkpoint, and do not
  contain output paths or generated bytes.
- Disabling Maps removes its UI/service contributions without deleting map,
  preset, chronology, or shared semantic data.

### Iteration 3: regional and print atlas output

#### Product slice

An author can render a validated region with an approved projection and produce
print-ready SVG/PDF or another explicitly accepted format while retaining the
same world detail and provenance.

#### Required work

1. Add one projection at a time with forward/inverse fixtures, valid-domain
   validation, cuts, clipping, central meridian, scale, and regional extent.
2. Add current-viewport and explicit-coordinate extent capture without storing
   browser pixel coordinates.
3. Add SVG/PDF through pinned offline encoders. Embed bounded raster relief,
   fonts, metadata, and vector geometry; prohibit scripts/external resources.
4. Add multi-page or tiled print sheets only after single-page output is
   correct. Page overlap, crop marks, legend repetition, and page numbering are
   explicit settings.
5. Add format-aware visual and structural validation, including PDF/SVG parser
   reopening and resource enumeration.

#### Exit gate

- Projection round trips stay within locked error bounds; cuts, poles, and
  antimeridian fixtures have no cracks or duplicated geometry.
- The same region/style/year/layers has aligned geographic detail across raster,
  SVG, and PDF output.
- SVG/PDF contain no active content, remote URL, unembedded required font, or
  out-of-budget resource.
- Page geometry and DPI/physical size are verified by reopening the output, not
  inferred from encoder success.

### Iteration 4: advanced derived detail and performance hardening

#### Product slice

Atlas output may include explicitly labeled derived minor tributaries or other
approved geographic micro-features, and repeated large renders become faster
through safe persistent derived caches.

#### Required work

1. Version and implement topology-affecting detail separately from visual
   refinement. Give every derived feature a stable ID and provenance.
2. Refine drainage on a canonical world-space grid constrained by canonical
   basins, mouths, runoff, lakes, and river identities. Do not let a tributary
   cross a watershed merely because local noise descends that way.
3. Decide whether derived minor features are atlas-only or can be promoted to
   canonical authored geometry. Promotion, if supported, is an explicit
   revisioned mutation and never automatic.
4. Add the core-owned disk cache with checksum/version validation, atomic
   writes, per-project/global quotas, LRU eviction, and cleanup on project
   lifecycle events.
5. Measure cold/warm renders, preview churn, cancellation latency, concurrent
   project use, output limits, low-disk behavior, and crash recovery.

#### Exit gate

- Derived minor features keep stable IDs/positions across year, resolution,
  style, format, tile order, restart, and cache deletion/rebuild.
- Canonical downsample, basin, mouth, topology, and water invariants still pass.
- Deleting every atlas cache changes no source/preset bytes and reproduces the
  same output from a clean project.
- Corrupt/truncated/wrong-version cache entries are misses and cannot crash or
  alter output.
- Warm-cache speedup is measured, and cache size/eviction stays within locked
  limits.

### Iteration 5: declarative style extensibility and release hardening

This iteration is optional until the built-in product is proven.

#### Required work

1. Decide whether style packs are core-bundled only, project assets, or plugin
   contributions. Record trust, license, portability, and compatibility rules in
   an ADR.
2. If plugins contribute styles, accept only declarative validated resources
   through a narrow broker contract. Do not expose renderer execution, temp
   paths, save dialogs, fonts, database, or filesystem.
3. Add style compatibility ranges, deterministic migrations or explicit
   incompatibility, resource quotas, disable/uninstall behavior, and missing
   style fallback in saved presets.
4. Run the full supported-target matrix, accessibility review, long-running
   cancellation/cleanup tests, malicious asset/style corpus, and packaged
   offline verification.

#### Exit gate

- A disabled/uninstalled style contributor disappears from choices without
  deleting presets; affected presets show an unavailable style and require an
  explicit replacement.
- Malicious styles/assets cannot execute code, read paths, fetch network
  resources, escape budgets, or produce unbounded work.
- Every supported target passes the declared deterministic guarantee and
  packaged native render/save/reopen checks.

## Verification matrix

### Pure detail and math tests

- exact named seed-domain vectors;
- world-space sample invariance across resolutions;
- longitude wrap and pole policy;
- conservative downsample to canonical elevation;
- coast-envelope and land/water sign preservation;
- connected-component and island/lake identity preservation;
- basin, river mouth, and canonical river identity preservation;
- stable residual terrain across historical epochs;
- bounded finite values and checked overflow;
- projection forward/inverse and cut fixtures; and
- deterministic contour levels and geometry ordering.

### Renderer tests

- tile versus full-frame identity;
- forward/reverse/random tile order;
- one versus many worker threads;
- halo crop equality and no edge seams;
- stable layer composition and alpha;
- bundled font hashes, shaping, line breaks, and label collision order;
- style schema rejection of code, URLs, paths, unknown fields, and excess
  resources;
- PNG/JPEG decode, dimensions, color model, alpha/matte, DPI, metadata, and
  output hash;
- later SVG/PDF parse, page box, embedded resources, and active-content
  rejection; and
- output with no locale, timezone, wall-clock, or OS-font dependence.

### Core and storage tests

- authorization and provider capability reporting;
- consistent snapshot generation and content hashes;
- layer/asset counts, MIME, ownership, revision, and byte budgets;
- preset create/update/delete idempotency and stale-revision conflict;
- render-only operations do not advance content generation;
- preset mutations do advance it;
- restart/reopen and clean-checkpoint rebuild;
- deleting `.daena` only when storage says the checkpoint is clean;
- corrupt/missing disposable caches rebuild without project mutation; and
- external portable edits follow explicit import, never live render-time
  reconciliation.

### Job and native boundary tests

- progress sequence and stale preview suppression;
- cancellation in every phase within a locked latency;
- project close, database replacement, and app exit cleanup;
- no database lock held during CPU render or dialog;
- no large JSON/base64 binary transfer;
- native save, overwrite, permission error, locked target, low disk, retry, and
  cancellation;
- no partial destination after injected encoder/copy/install failure; and
- temp-artifact expiry deletes only application-owned validated paths.

### Rendered acceptance fixtures

Maintain a small reviewed corpus rather than one attractive screenshot:

- world seam centered and moved away from the edge;
- both poles;
- shallow shelf/coast with islands;
- tall tectonic range and broad plateau;
- flat lowland with canonical rivers/lakes;
- ice advance/retreat at multiple physical years;
- authored political border, route, point symbol, raster overlay, and Unicode
  label;
- crowded labels and deterministic omissions;
- both initial styles at preview, 4K, and 8K; and
- transparent PNG plus JPEG matte where enabled.

Visual goldens require an explicit renderer/style version update and a short
reason. Never refresh them merely because an algorithm changed.

## Instructions for implementation agents

Before editing an iteration:

1. Run `rtk git status --short` and inspect staged, unstaged, and untracked
   changes. Preserve unrelated user work; do not stage, commit, or push unless
   explicitly requested.
2. Read this guide and the authority documents named by the active iteration.
3. Prefer the codebase knowledge graph for symbol discovery and call tracing.
   Use `rg` for literals, configuration, docs, fixtures, and when graph results
   are insufficient.
4. Verify live provider, generator, source, historical, hydrology, hazard,
   layer, calendar, SDK, and storage versions. Do not copy the baseline numbers
   blindly.
5. Write a short vertical-slice plan naming the production path, stored/public
   contracts, tests, native/rendered boundary, and exit gate.
6. If the iteration adds a dependency, verify its license, offline operation,
   supported targets, deterministic behavior, lockfile impact, decoded-memory
   behavior, and maintenance state before adding it.

While implementing:

- Keep the pure renderer independent of Tauri/core/plugin infrastructure.
- Keep stored schemas at version `1` for backward-compatible additions unless an
  incompatible persisted change genuinely requires a new version.
- Derive UI contributions from reported provider/module capabilities; do not
  hardcode entity types or provider names in Svelte visibility rules.
- Generate large data outside database locks and pass typed Rust ownership, not
  JSON arrays, across internal boundaries.
- Use named deterministic seed domains and stable ordering everywhere.
- Treat detail/style/golden version changes as reviewed contract changes.
- Preserve immutable physical layers and authored layers separately; never
  flatten them back into the physical source.
- Keep output paths out of project data and keep exported bytes out of the
  checkpoint unless the user explicitly invokes a later asset-import flow.
- Add cancellation and budget checks with the first implementation of every
  large loop, not as cleanup work.
- Exercise the packaged Tauri boundary for preview, save, overwrite,
  cancellation, and restart. Browser automation cannot prove native behavior.

Do not:

- increase canonical physical resolution merely to make exports larger;
- persist high-resolution elevation, hillshade, PNG tiles, or GeoJSON as a
  second physical authority;
- use MapLibre/browser screenshots as final output;
- run generative image super-resolution over the map;
- let style choice or output resolution change geographic seeds;
- create new islands, lakes, rivers, borders, or settlements as incidental noise
  in the first release;
- use platform fonts, locale-sensitive formatting, remote resources, or GPU
  rendering in the deterministic export path;
- read portable asset paths as live authority;
- let plugins pass destination paths or execute renderer code;
- hold a SQLite transaction while rendering or saving; or
- claim completion from compilation/unit tests without rendered, native,
  restart, recovery, cancellation, and resource evidence named by the gate.

After the final edit in an iteration:

1. Run focused atlas/detail/core/frontend tests with explicit Cargo manifests
   and `--locked --offline` where the repository guidance requires them.
2. Run the relevant broader application checks and separate unrelated baseline
   failures from defects introduced by the iteration.
3. Render and inspect the required fixtures at their actual output sizes.
4. Exercise native persistence/restart/save/recovery boundaries named in the
   gate.
5. Run `rtk git diff --check` and `rtk git status --short`.
6. Report exact evidence, resource measurements, changed contracts, deferred
   work, and any unrelated worktree state. Stop at the iteration exit gate.

## Deliberately deferred questions

These are not required to begin Iterations 0–2:

- cloud rendering or render farms;
- collaboration around render jobs;
- animation/video export;
- arbitrary user-authored projection code;
- neural/generative terrain enhancement;
- automatic import of exported images into the project;
- automatic promotion of atlas-only tributaries or micro-features into canonical
  geography;
- unrestricted third-party renderer plugins; and
- a general desktop-publishing/page-layout application.

They must not be smuggled into the initial cache, plugin, or file-output
contracts.
