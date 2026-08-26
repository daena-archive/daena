# Maps, Physical Worlds, and Atlas

## Purpose and authority

This document defines Daena's durable product and architecture contracts for
maps, physical worlds, and Atlas. It is the single focused authority for:

- map identity, sources, layers, features, styles, and coordinates;
- interactive map authoring and revision-aware persistence;
- generated physical worlds and authored content above them;
- Atlas Studio and deterministic static Atlas rendering; and
- map integration with Daena entities, search, Timeline, plugins, and storage.

It is subordinate to [`ARCHITECTURE.md`](./ARCHITECTURE.md) for project-wide
boundaries and [`STORAGE.md`](./STORAGE.md) for runtime authority, checkpoints,
recovery, and asset publication. Exact public field shapes are defined by the
plugin SDK and the Maps schema. Future physical-world product work belongs in
[`PHYSICAL_WORLD_ROADMAP.md`](./PHYSICAL_WORLD_ROADMAP.md).

The consolidated [`ADR history`](./adr/README.md) may lock narrower
compatibility, determinism, security, or numerical decisions, but it must not
create a second product model.

## Product model

Every map is a normal Daena entity. A map owns or references:

- a provider-neutral descriptor;
- an ordered set of vector, raster, physical, and semantic layers;
- one canonical authored GeoJSON source where author-created geometry lives;
- provider-specific source assets when the map represents an imported image or
  generated physical world;
- stable links between map features or normalized map anchors and Daena
  entities; and
- optional Atlas presets and physical-calendar bindings.

The relationships are:

```text
map entity
  +-- descriptor and default view
  +-- ordered layer definitions
  +-- authored GeoJSON and raster assets
  +-- entity links and normalized anchors
  +-- optional physical source
  |     +-- disposable epoch and presentation products
  +-- optional Atlas presets
        +-- disposable Studio tiles and render caches
```

The map entity is the shared identity. Providers do not create parallel map
tables or isolated ownership systems.

## Core principles

### Daena owns durable state

Daena owns map identity, geometry, layers, styles, labels, links, history, and
persistence. Renderer objects, interaction state, viewport tiles, browser
canvases, render jobs, and caches are runtime data and are never serialized as
project truth.

Only renderer-independent values cross the persistence boundary:

- canonical GeoJSON geometry with stable Daena feature IDs;
- coordinate-space and view values;
- layer and style definitions;
- asset identities and metadata;
- normalized anchors; and
- relationships to shared Daena entities.

### Runtime and portable authority remain distinct

SQLite and project-owned runtime assets are authoritative while a project is
open. Portable files are deterministic checkpoints sufficient to rebuild a
clean project. Map edits, physical acceptance, presets, and links follow the
same revision, request-ID, recovery, and checkpoint rules as other project
content.

Derived physical products, Atlas detail, search projections, previews, and
tiles are disposable. Deleting them must not delete or alter canonical map
content.

### One interactive map engine

OpenLayers is the in-window 2D rendering and geometry-interaction engine. It
renders Daena state; it does not own that state. There is no renderer-specific
project format or alternate persisted renderer model.

Atlas uses OpenLayers for interactive navigation of locally rendered tiles.
Static Atlas export is produced from captured Daena data, never by taking a
screenshot of the interactive viewport.

### Offline by default

Maps, physical derivation, Atlas Studio, and Atlas export operate without public
tiles, fonts, sprites, styles, telemetry, or other network resources. All
required assets are project-owned, generated locally, or bundled and licensed
with the application.

## Map descriptors and sources

A descriptor identifies the map provider and declares the coordinate space,
source assets, raster backgrounds, default view, and map-level settings. The
descriptor must contain plain validated data and must not contain renderer
objects, filesystem paths, temporary URLs, or cache identifiers.

Daena supports three principal map source models.

### Authored vector maps

An authored vector map stores canonical GeoJSON as its editable source. It may
begin blank or from imported GeoJSON. Import normalizes untrusted input into the
Daena feature contract; the imported file is not treated as a live external
authority after acceptance.

Authored vector maps may contain points, multi-points, lines, multi-lines,
polygons, and multi-polygons. Geometry collections remain unsupported until
their editing and semantic ownership have a defined product contract.

### Image-backed maps

An image-backed map preserves the original validated image as a Daena asset and
uses it as a raster background for authored GeoJSON. The image is not converted
into canonical vector geography.

Image maps use pixel-native coordinates by default. Replacing, calibrating, or
reordering a background must not silently reinterpret authored geometry.

### Physical worlds

A physical map owns an accepted physical source plus a separate authored
GeoJSON source. The physical source contains the generated world's canonical
terrain and causes. Coastlines, climate, water, ice, rivers, hazards, and other
view products are derived from that source for the requested epoch.

The accepted physical source is immutable. Rerolling creates another temporary
world and explicit acceptance creates a new map; neither action mutates an
accepted world. Authors place countries, settlements, routes, borders,
annotations, raster overlays, and entity links in ordinary authored layers
above the physical base.

## Coordinates and anchors

Authored maps declare their coordinate meaning explicitly:

- image coordinates use a stored pixel extent and top-left origin;
- fictional-world coordinates use a stored extent, origin, and named units;
- geographic coordinates use the supported geographic projection and wrapping
  policy; and
- physical worlds use their declared global geographic extent.

Coordinates must be finite and extents must have positive area. Measurement
uses the declared units. Pixel maps display pixels until calibrated; arbitrary
world units do not silently become metres; geographic maps may use geodesic
measurement.

Entity links use renderer-independent normalized anchors. An anchor remains
portable even when a viewport, renderer, background, or derived Atlas product
changes. Feature links additionally use stable Daena feature IDs.

## Layers, features, styles, and labels

Layers are first-class Daena values with stable IDs and a canonical total
order. A layer declares its kind, name, visibility, lock state, opacity, blend
mode, and applicable style or raster asset. Array position alone is not layer
authority.

Each runtime vector layer has independent rendering, selection, visibility,
locking, hit testing, and snapping behavior. Raster bytes remain project-owned
assets. Semantic layers resolve shared project records rather than duplicating
them into GeoJSON.

Each authored feature has:

- a stable ID;
- renderer-independent geometry;
- an owning layer ID;
- a semantic type and optional name;
- optional style and label overrides;
- bounded safe custom properties; and
- optional links to shared Daena entities.

Daena styles cover fill, stroke, opacity, width, dash, point or icon
presentation, labels, and visibility rules. Styles and labels must remain plain
data, must use safe Daena-owned resources, and must not embed executable HTML or
remote URLs.

## Interactive authoring

The authoring surface supports:

- drawing points, lines, polygons, rectangles, freehand geometry, and labels;
- single and multiple selection, box selection, vertex editing, translation,
  duplication, deletion, and layer reassignment;
- snapping, tracing, fit-to-selection, and geometry measurement;
- layer creation, duplication, rename, reorder, visibility, locking, opacity,
  styling, and deletion;
- feature metadata, style, label, custom-property, and entity-link editing;
- map-local search and navigation to a selected feature; and
- geometry operations that validate and normalize their results before they
  enter authored state.

Hidden and locked layers remain non-editable at interaction, command, and core
validation boundaries. Locked layers may be eligible snapping references only
through an explicit setting.

### Commands, history, and saving

Every authoring action is represented by a Daena command containing sufficient
plain data to apply and invert the change. Layer, feature, style, background,
link, coordinate, and default-view edits share one history. A continuous pointer
gesture or inspector scrub becomes one coalesced history entry.

Dirty state is derived from the complete current document relative to the
loaded baseline. Undo and redo restore exact Daena state; they never store
OpenLayers objects.

Saving is one revision-aware mutation over the complete map edit: descriptor,
layers, authored GeoJSON, links, and view changes are validated before any part
is committed. A stale revision changes no canonical data. The complete local
draft remains available for retry or recovery export after a conflict.

## Detach for editing

Generated physical geometry is not directly editable because it is a derived
view of the physical source. When an author wants to reshape generated
coastlines, rivers, lakes, borders, or other derived geometry, Daena uses an
explicit **Detach for editing** operation.

Detachment:

1. captures the chosen physical layer or bounded selection at the displayed
   epoch;
2. converts it to canonical authored GeoJSON with new stable Daena feature and
   layer IDs;
3. places the copy in an ordinary editable authored layer;
4. suppresses or hides the corresponding derived presentation so the detached
   copy is not drawn twice; and
5. records the change through normal command, revision, checkpoint, and recovery
   rules.

Detachment never rewrites the physical source. Detached geometry is a snapshot:
it no longer follows physical re-derivation or epoch changes. The UI must state
that consequence before completing the operation. The original physical map
remains recoverable, and an author may instead create a separate editable vector
map from a complete physical snapshot.

Imported GeoJSON does not need a physical detachment step. Once accepted into
an authored map, its canonical Daena geometry participates in ordinary editing
and saving.

## Physical-world contract

### Canonical world

The physical source represents one deterministic accepted world. Its core
physical truth is a signed elevation and bathymetry field plus the persisted
causes and settings required to reproduce and validate that world.

Physical generation follows a causal order: planetary geometry and seed,
tectonics and crust, terrain, climate and runoff, erosion and drainage, water
and ice, coastlines and hydrology, and hazards. Visible classifications must not
become an independent replacement for their upstream physical causes.

Generation presents one visible result. Bounded retries may repair named hard
invalid states, but hidden candidate scoring is not part of the product.
Cancellation or failure creates no durable map. Acceptance atomically commits
one complete validated world.

### Epochs and history

Physical time is expressed as a signed offset from the world's reference epoch.
Negative and positive offsets must not be mislabeled as Gregorian dates. An
optional validated calendar binding may translate authored calendar years into
physical offsets without JavaScript date coercion or an assumed year zero.

Epoch changes may derive different climate, water, ice, coastlines, rivers,
lakes, biomes, and hazard products. Stable terrain and other epoch-independent
causes remain attached to the same physical identity.

Hazard fields are derived probability or rate products. A user may explicitly
materialize selected natural events into durable Timeline and Lore records.
Materialized events are authored history and are not regenerated merely because
a hazard derivation changes.

### Physical overlays

Physical layers remain read-only until explicitly detached. Authored vector,
raster, semantic, and relationship-backed layers render above them and may have
their own validity intervals. Editing an overlay cannot rewrite terrain,
climate, hydrology, or another physical product.

## Atlas

### Relationship to Maps and the physical world

Atlas consumes a validated map snapshot and creates detailed cartographic
presentation. It does not create another map provider or another physical
authority.

```text
accepted map and selected epoch
        -> validated immutable snapshot
        -> deterministic Atlas geography and composition
        -> interactive Studio tiles or static export
```

The physical map defines planetary-scale truth. Atlas may refine terrain,
coastlines, relief, drainage, labels, and presentation at geographic scale, but
it must preserve canonical continents, oceans, mountain systems, watersheds,
river mouths, lakes, ice, and other physical constraints.

Atlas geography is addressed in world space. Zoom level, output dimensions,
format, style, tile order, worker count, and cache state must not move a ridge,
tributary, coastline detail, label anchor, or other geographic result.

### Atlas Studio

Atlas Studio is the interactive detailed-map workspace. When a map reports the
capability, the author can:

- pan and zoom through locally rendered Atlas tiles;
- select a supported epoch, style, detail level, and layer set;
- view physical, authored, semantic, and labeled content together;
- inspect bounded feature information;
- refresh a stale captured project generation;
- regenerate disposable Atlas caches; and
- open static export with the current geographic view and presentation choices.

A Studio session represents one captured project generation. Project changes
make it stale; they do not silently mix new labels or layers into existing
tiles. Viewport, hover, selection, panel, session, and tile state are local UI
state unless the user invokes a separately defined save action.

Physical and Atlas-derived features remain read-only in Studio. Authored map
features use the shared Maps authoring and persistence contracts rather than a
Studio-specific mutation model.

### Static Atlas rendering

Static rendering uses the same validated world, epoch, layers, styles, labels,
and deterministic detail as Studio. It captures one bounded immutable snapshot
before expensive work and never queries live project state while composing the
artifact.

The author may choose a supported extent, projection, style, layer set,
dimensions, print metadata, format, and deterministic detail variant. Available
formats and limits come from provider capabilities. Supported outputs include
PNG, self-contained SVG, and single-page PDF; an encoder or format must not be
advertised until it is available and validated.

Export creates an application-owned temporary artifact. Saving uses an explicit
host-owned destination choice and safe replacement behavior. Rendering and
saving an external file do not mutate the project. Registering an export as a
Daena asset is a separate explicit import action.

### Presets, styles, and provenance

Atlas render presets are portable, revisioned map-owned recipes. They may store
the map-relative render choices needed to reproduce a request, but never a
destination filesystem path, session token, temporary file, or generated image
bytes.

Styles are declarative, versioned, offline resources. They control palette,
strokes, symbols, labels, relief, and decorations without changing geographic
identity. Missing or unavailable styles are reported explicitly rather than
silently rebound to another style.

Every export carries bounded provenance sufficient to identify its captured
map generation, physical identity where applicable, epoch, style, layer set,
detail contract, and output settings. Provenance must not expose local paths or
secrets.

## Daena ecosystem integration

### Entities, links, and search

Map features and normalized anchors may link to shared Daena entities. Deleting
a linked entity preserves geometry and surfaces an unresolved link that can be
repaired. Map-local and project search may index feature names, semantic types,
layer names, safe text properties, and linked entity names. Opening a result
reveals the map, layer, feature, and inspector context.

### Timeline and calendars

Timeline may provide validity intervals for authored or semantic map content,
bind an authored calendar to a physical reference epoch, and receive explicitly
materialized natural events. Maps must preserve literal fictional chronology
and must not substitute host-language date behavior.

### Lore, Language, and other modules

Lore and other modules use the shared entity-link and normalized-anchor
contracts. Language territory, political regions, routes, and similar domain
features remain authored or semantic layers over stable feature IDs rather than
special renderer objects.

### Plugins

Maps and Atlas capabilities are exposed through versioned, provider-neutral
services. Plugins receive only authorized plain data and narrow operations.
They do not receive renderer execution, database handles, arbitrary local URLs,
cache paths, destination paths, or ambient filesystem and network access.

Disabling Maps removes its UI and service contributions without deleting map
entities, fields, sources, layers, presets, or links.

## Security and resource boundaries

- Validate MIME and file content before accepting raster or vector input.
- Passive SVG assets must not execute scripts, load remote references, or
  escape decoded-size budgets.
- Reject remote raster, tile, font, style, sprite, and glyph URLs.
- Enforce explicit limits for source bytes, decoded pixels, features,
  coordinates, rings, properties, labels, layers, rasters, tiles, render jobs,
  output dimensions, memory, temporary storage, and cache size.
- Generate and render outside database writer locks.
- Cancellation and failure must leave no partial canonical mutation or corrupt
  destination.
- Local tile and artifact delivery uses opaque bounded authority and exposes no
  project or cache paths.
- Cache cleanup operates only on validated application-owned cache entries and
  never accepts an arbitrary caller-supplied path.

## Recovery and determinism

A clean portable checkpoint must reconstruct:

- map entities, descriptors, layers, authored GeoJSON, raster assets, and links;
- accepted physical sources and the settings required to validate them;
- physical-calendar bindings, materialized events, and Atlas presets; and
- all stable IDs and revisions represented in portable state.

It need not preserve derived physical products, Atlas terrain caches, Studio
tiles, previews, search projections, or temporary export artifacts. Rebuilding
those products from the same canonical inputs must preserve their declared
deterministic identity.

Determinism is scoped to declared contracts. Geographic results must be stable
for the same canonical inputs and versioned algorithm choices. Encoded bytes
must be stable where the selected encoder contract promises byte identity.
Changes to a versioned physical, Atlas-detail, style, or encoding contract must
be explicit rather than silently reinterpreting existing project data.

## Product verification

Changes to Maps, physical worlds, or Atlas require evidence at the boundaries
they affect:

- strict schema, validation, resource-limit, and canonicalization tests;
- command apply, inverse, coalescing, and dirty-baseline tests;
- atomic save, stale-revision, request replay, restart, and clean-checkpoint
  recovery tests;
- rendered editing checks for drawing, selection, snapping, labels, raster
  alignment, and layer order;
- physical generation and derivation invariants, cancellation, and deterministic
  replay checks;
- Atlas tile seam, order, cache deletion, epoch, style, overlay, and label
  stability checks;
- static output parsing, dimensions, embedded resources, provenance, and safe
  native save checks; and
- packaged desktop lifecycle checks where browser-only tests cannot exercise the
  native renderer, protocol, dialogs, or cleanup behavior.

Passing compilation or unit tests alone does not prove interaction, native
lifecycle, persistence, recovery, deterministic rendering, or output safety.

## Future work

Future planetary configuration, climate, wind, currents, biomes, roads, place
search, natural-event presentation, and related physical-world product work is
maintained in [`PHYSICAL_WORLD_ROADMAP.md`](./PHYSICAL_WORLD_ROADMAP.md). That
roadmap may extend this product, but it must preserve the authority,
authorship, detachment, persistence, offline, and recovery boundaries above.
