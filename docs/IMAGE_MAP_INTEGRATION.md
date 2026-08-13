# Daena Image Map Integration

## Status and authority

This document defines the first implementation of image-backed maps. It is
subordinate to [`ARCHITECTURE.md`](./ARCHITECTURE.md),
[`STORAGE.md`](./STORAGE.md), [`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md),
and the provider-neutral contracts in
[`MAP_INTEGRATION_PLAN.md`](./MAP_INTEGRATION_PLAN.md).

Image Maps are a second Maps provider alongside Azgaar FMG. They reuse the
existing `daena.maps:map` entity, asset, location, navigation, projection,
plugin, and checkpoint contracts. They do not introduce a parallel map table,
filesystem authority, entity-link model, or privileged frontend storage path.

The first release is intentionally an annotation tool, not an image editor or
GIS. It imports a PNG, JPEG, or SVG; supports pan and zoom; paints on independent
raster layers; and uses Daena's existing normalized anchors for entity links.

## Validated current foundation

The current codebase already has:

- map entities of type `daena.maps:map`;
- a version-1 `daena.maps:map` descriptor with provider, source asset, preview
  asset, and default viewport;
- version-1 `daena.maps:locations` fields with point, provider-feature, path,
  and area anchors in normalized `[0, 1]` coordinates;
- version-1 `daena.maps:layers` fields for provider-neutral semantic layers;
- derived map/location projections and reverse entity-to-map lookup;
- the `daena.maps/navigation@1` service and Maps host-surface bridge;
- revision-aware native assets, bounded binary read/replace transfers, and a
  Maps-specific first-source upload path; and
- SQLite-authoritative runtime storage with deterministic portable checkpoint
  export and content-addressed runtime asset bytes.

The current implementation is still FMG-specific in several places. Rust
validation accepts only provider `azgaar-fmg` and source format `fmg-map`.
Provider-feature anchors also intentionally accept only FMG selectors. The
Maps-specific create upload always writes `map.map` as
`application/x-fmg-map`, and the frontend adapter is built around the FMG
editor lifecycle. Those paths must be generalized; Image Maps must not be
bolted onto them with filename or MIME special cases in Svelte.

## Corrected architectural decisions

### Runtime and portable storage

SQLite and `.daena/assets/` are the live authority. Portable JSON and native
assets are generated checkpoints for Git, inspection, backup, interchange, and
rebuild. Therefore an Image Map save must use core asset and field mutations,
advance the runtime content generation, and let the checkpoint worker export
the result. The UI must never write `maps/...` or any other project path
directly.

Portable assets use the existing asset layout and records, normally below
`assets/maps/`. Asset records, rather than map metadata, own the portable path,
filename, MIME type, byte size, content hash, and revision. A clean checkpoint
must reconstruct the same map descriptor, layers, locations, and asset bytes
after `.daena/` is removed.

### One map model, multiple providers

An Image Map remains a normal `daena.maps:map` entity. Its `map` descriptor uses
the existing schema version and a new provider ID:

```json
{
  "schemaVersion": 1,
  "provider": {
    "id": "daena-image",
    "adapterVersion": 1,
    "sourceFormat": "png"
  },
  "sourceAssetId": "018f89ec-25fc-7816-8b47-6f80905f2868",
  "previewAssetId": null,
  "defaultView": {
    "center": [0.5, 0.5],
    "zoom": 1
  }
}
```

`sourceFormat` is one of `png`, `jpeg`, or `svg`. `sourceAssetId` identifies the
unchanged imported source. The descriptor schema remains version 1; validation
is broadened by provider instead of introducing a second descriptor shape.
Existing FMG descriptors remain valid without migration.

The provider adapter contract should become genuinely provider-neutral. FMG
continues to implement editable opaque provider state. The Image Map adapter
implements view transforms, layer composition, hit testing, selection capture,
focus, dirty state, and cleanup. Host navigation continues to pass stable map
and link IDs rather than renderer details.

### Base image and SVG safety

The imported base asset is immutable after import in iteration 1. Replacing the
base is a later explicit operation because changed dimensions require a clear
policy for raster layers and anchors.

PNG and JPEG imports must validate both the declared MIME type and decoded
content. SVG is untrusted active content: Daena must reject scripts, event
handlers, external references, foreign objects, and other unsupported active
features before display. Preserve the original SVG bytes as the source asset,
but render only a validated/sanitized representation under the plugin webview's
deny-by-default CSP. Do not inject imported SVG markup into the DOM.

Every source has an intrinsic logical width and height obtained from decoded
image dimensions or a valid SVG `viewBox`/size. Reject missing, zero, non-finite,
or over-budget dimensions. Width and height are adapter runtime facts; anchors
remain normalized and do not need a new coordinate system.

### Raster annotation layers

The existing `daena.maps:layers` field remains the single layer collection.
It already represents semantic layers, so raster support extends each layer
definition with a discriminator and raster-only properties rather than
creating a competing layer store.

```json
{
  "schemaVersion": 1,
  "layers": [
    {
      "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
      "name": "Countries",
      "order": 0,
      "defaultVisible": true,
      "style": {},
      "selector": {},
      "kind": "raster",
      "rasterAssetId": "018f8a03-bc44-70e2-a910-f4d2ef8d93df",
      "opacity": 1,
      "locked": false
    }
  ]
}
```

For compatibility, a layer without `kind` remains an existing semantic layer.
A raster layer requires `kind: "raster"`, a PNG asset owned by the same map
entity in the `maps` namespace, finite `opacity` in `[0, 1]`, and `locked`.
`style` and `selector` remain present because they are required by the current
version-1 contract; they are empty for raster layers. Layer IDs and `order` are
stable metadata. List order is derived by numeric `order` with ID as a stable
tie-breaker.

Each raster layer is a transparent PNG with exactly the base image's logical
pixel dimensions. Painting and erasing modify only the active layer asset.
The base and other layer assets are never flattened or rewritten. Visibility,
opacity, lock state, name, and order are durable map metadata; temporary tool,
selection, and viewport state remains local UI state unless the user explicitly
updates `defaultView`.

Iteration 1 uses whole-layer PNGs. It does not introduce tiles or a custom
stroke format. The implementation must enforce measured encoded-byte,
dimension, decoded-memory, and layer-count budgets before allocation. Existing
64 MiB transfer limits are a transport ceiling, not a sufficient image-memory
policy.

### Locations and entity links

Image Maps use the existing location model. A pin is not a second durable
record; it is the rendered form of a `daena.maps:locations` entry on the linked
entity, normally with:

```json
{
  "kind": "point",
  "point": [0.72, 0.41]
}
```

Paths and areas already use the same normalized map space and remain valid for
future tools. Provider-feature anchors stay FMG-specific and are not generated
for Image Maps. Unlinked visual notes belong on a raster layer; an unlinked pin
record is out of scope until the product defines who owns it.

This preserves the existing bidirectional behavior:

- entity to map uses reverse location lookup and the navigation service; and
- map to entity resolves the selected location reference to the shared entity.

No code may hardcode linkable entity types such as Place or Event. Entity
selection uses enabled manifests and the shared entity APIs.

### Mutation boundaries

General asset replacement already supports revision-checked layer saves. Image
Map import and layer creation still need domain operations because the current
Maps create upload hardcodes FMG and a layer must not become visible in metadata
before its asset exists.

Add narrow broker/core operations that:

1. import the validated base asset and create/link its image-provider descriptor
   as one logical mutation;
2. create a transparent layer asset and add its layer definition atomically;
3. delete a raster layer definition and its owned asset atomically; and
4. update layer metadata with the observed field revision.

Layer painting saves replace only the existing raster asset through the normal
binary replace flow and its observed asset revision. Request IDs make retries
idempotent. A stale field or asset revision returns a conflict; it never silently
overwrites. If the current core cannot commit the entity, field, and asset rows
in one SQLite transaction, extend the core transaction boundary rather than
coordinating multiple successful RPC calls in the frontend.

## First iteration user experience

The Maps creation chooser adds **Import image** next to FMG. A host-owned file
dialog selects one PNG, JPEG, or SVG and passes it through trusted validation
and the brokered import operation. The plugin never receives ambient filesystem
access.

View mode provides:

- smooth pan and bounded zoom;
- fit/reset to map;
- ordered layer visibility and opacity controls;
- rendered entity pins; and
- existing focus and entity navigation behavior.

Edit mode adds:

- create, rename, reorder, delete, show/hide, lock, and opacity controls;
- one active raster layer;
- brush, eraser, color, and brush size;
- bounded in-memory undo/redo for the active unsaved editing session; and
- explicit save plus clear dirty/conflict/error state.

Switching maps, leaving the workspace, disabling Maps, closing the project, or
closing the app uses the existing dirty-session lifecycle. The user must be able
to save, discard, or remain in the editor. A no-edit close writes nothing.

## First iteration boundaries

- changing or destructively editing the base image;
- filters, selections, transforms, text effects, or blending modes;
- vector drawing or editable SVG layers;
- anonymous durable pins separate from entity locations;
- polygon authoring UI, semantic region editing, or GIS features;
- scale, distance, travel time, projections, or pathfinding;
- tiling, mipmaps, GPU-specific persisted data, or cached composites;
- automatic conversion between Image Maps and FMG; and
- world-history animation or timeline-driven raster versions.

Future renderers may add tiles, GPU composition, previews, semantic geometry,
or temporal variants, but those remain derived/runtime concerns. The canonical
contract stays base asset + independent layer assets + versioned metadata +
normalized shared locations.

These features are deferred from the first iteration, not rejected from the
product. The following sections define the foreseeable direction they should
take when implemented.

## Foreseeable later capabilities

### Semantic and vector layers

Raster painting is intentionally opaque to Daena. A later semantic layer can
store normalized geometry that Daena can query and associate with shared
entities. Raster and semantic content should coexist:

```text
Terrain             -> raster layer
Old Borders         -> raster layer
Countries           -> semantic areas
Cities              -> point locations
Trade Routes        -> semantic paths
```

Semantic geometry should build on the existing normalized `point`, `path`, and
`area` anchors. Later schema additions may add circles, multipolygons, and
stable feature records when a concrete authoring workflow needs them. They
must preserve stable IDs, deterministic serialization, explicit entity links,
and core validation. Renderer hit-test indexes and simplified geometry are
derived projections, never the only copy of authored geometry.

This enables:

- clickable and hoverable regions;
- entity-linked countries, forests, cultures, religions, and climate zones;
- dynamic styling and per-feature visibility;
- area and intersection queries; and
- provider-neutral region display on both Image Maps and providers that expose
  compatible geometry.

Freehand raster layers remain first-class. Semantic layers add understanding;
they do not replace the faster, looser painting workflow.

### Paths, movement, and routes

Normalized paths can later represent character journeys, migrations, military
campaigns, trade routes, exploration, and other movement. A route should be a
stable authored map feature or location reference linked to ordinary Daena
entities and events. It must not duplicate the character, organization, or
event record inside Maps.

Timeline integration may filter route segments or show progress at a selected
world date. Missing temporal bounds remain unbounded, and authored date
precision is preserved. Animation and interpolation are renderer behavior;
the durable model stores explicit geometry and temporal facts.

### Historical maps and layers

World history is distinct from Git history:

- Git/checkpoints record changes to the author's project; and
- temporal map data represents changes inside the fictional world.

Later iterations may attach validity intervals to semantic layers, individual
features, or explicit map variants. Political borders in years 300, 350, and
400 may be separate layer states or separate map entities connected through
typed variant relationships. The implementation must define deterministic
selection for overlapping intervals and must never infer missing dates.

Raster history should initially use explicit layer or map variants rather than
delta images or animation-specific storage. Semantic geometry may later vary by
date if real workflows justify the added model.

### Scale, distance, and travel estimates

A later map-scale contract may relate normalized map space to authored world
units. It can support point distance, route length, and eventually estimated
travel duration. Scale metadata must state its unit and assumptions; it cannot
pretend a decorative or distorted map is geographically accurate.

Travel estimates should be a separate service over route geometry, scale,
movement profiles, and optional terrain information. Image Maps without scale
remain fully usable. FMG and Image Maps may share the service contract without
sharing provider-specific geography.

### Plugin-provided and generated layers

A plugin may later provide population, political control, climate, language,
religion, economy, historical borders, or character-position overlays. These
are derived layers delivered through versioned brokered services or events.
Plugins receive no database, filesystem, renderer, or ambient Tauri access.

The common presentation contract should remain small: stable layer ID, name,
kind, order, visibility, opacity, style, and provider-neutral render data. A
generated layer declares its source plugin and service version. Daena persists
only user-owned configuration that must survive; generated frames and caches
remain disposable. Disabled or unavailable plugins produce a clear diagnostic
without deleting configuration or shared entity links.

### External editor workflow

Advanced editing belongs in tools such as Krita or Photoshop. A future
workflow may export a base or raster layer, let the user edit it externally,
and explicitly replace the corresponding asset after hash, dimensions, MIME,
ownership, and revision checks.

Daena must not treat arbitrary portable-file changes as automatic live writes.
Whole-project external changes use the explicit checkpoint import contract.
A focused **Replace layer from file** action may be added as a trusted,
host-owned import operation. If the asset changed after export, replacement
reports a conflict and preserves both versions until the user chooses.

### Base replacement and resizing

Replacing an immutable base is foreseeable but requires an explicit policy.
When dimensions or aspect ratio change, normalized entity locations remain
stable, but raster layer pixels may no longer align. A future replace-base flow
must preview the change and require one of these explicit outcomes:

- reject dimension changes;
- preserve the old base and create a new map variant; or
- resample raster layers with a recorded, reviewable operation.

It must never silently stretch, crop, or discard layers. Keeping the original
base asset or recovery copy is required until the replacement checkpoint is
complete.

### Advanced rendering and large maps

Representative project measurements may justify lazy decoding, lower-resolution
previews, mipmaps, tiled storage, viewport-based loading, cached composites,
or GPU composition. These are incremental renderer and derived-cache choices.

If whole-layer PNGs eventually become an actual storage bottleneck, a later
portable format may add deterministic tiles through an explicit schema change.
Iteration 1 must not pre-build that complexity, but code should keep viewport,
composition, persistence, and editing boundaries separate so the renderer can
evolve without changing entity links or map identity.

### Additional import and provider workflows

Future formats may include WebP, TIFF, PDF page import, or images exported by
other map generators. Each requires a reviewed MIME/decoder safety policy,
resource budgets, portable representation, and truthful editing capability.
Importing a static export does not imply Daena can round-trip the originating
tool's editable model.

Image Maps can also serve as the safe static fallback for a generator whose
editable source cannot be bundled. Provider-specific editable state remains a
separate provider concern; automatic conversion between FMG and Image Maps is
not a roadmap requirement.

## Implementation plan for Agents

Agents should implement one phase at a time, stop at its exit gate, and preserve
unrelated worktree changes. Keep schema version 1 and regenerate contracts from
their Rust/schema authorities; do not hand-edit generated outputs. Do not add
compatibility readers, dual writers, or frontend filesystem access.

### Phase 1: Lock the image-provider domain contract

- Generalize the Rust map descriptor validator and `maps-domain-v1.json` to
  accept the existing FMG tuple or `daena-image` with `png|jpeg|svg`.
- Extend version-1 layer validation/types with optional `kind`; validate raster
  asset ownership, MIME, opacity, lock state, uniqueness, and deterministic
  ordering while treating missing `kind` as semantic.
- Update the generated TypeScript contracts, SDK/test-host fixtures, and focused
  Rust/TypeScript contract tests.

**Exit gate:** Existing FMG fixtures remain byte-compatible; valid image
descriptors and raster layers round-trip; unknown provider tuples, dangling or
cross-entity assets, invalid MIME/opacity, and malformed layer shapes fail in
Rust with focused tests.

### Phase 2: Add atomic import and layer mutations

- Add decoded-image/SVG safety validation and explicit resource budgets in the
  trusted host/core boundary.
- Generalize the hardcoded first-source upload into typed image-map import, and
  add atomic create/delete/update layer operations with revision and request-ID
  handling.
- Reuse the existing bounded transfer protocol for bytes and normal asset
  replacement for painted-layer saves.
- Include base and layer assets in checkpoint export, import validation, and
  clean-rebuild coverage.

**Exit gate:** Import, layer create/save/delete, conflict, retry, cancellation,
and interrupted export resolve to a complete old or new runtime state. After a
flush and deletion of `.daena/`, the portable checkpoint rebuilds identical map
metadata and asset hashes with no orphan layer references. Invalid or
over-budget dimensions and unsafe SVG content fail before display or large
allocation.

### Phase 3: Implement the Image Map adapter and editor

- Add a provider-neutral adapter selection boundary and implement
  `daena-image` without importing FMG internals.
- Render the safe base representation, independent PNG layers, and existing
  semantic location overlays under one normalized viewport transform.
- Implement view mode, edit mode, layer controls, brush/eraser tools, bounded
  undo/redo, dirty state, explicit save, revision conflicts, and teardown.
- Add the import choice and reuse the existing Maps collection, navigation,
  entity picker, and host-surface lifecycle.

**Exit gate:** In the native Tauri app, PNG, JPEG, and safe SVG maps import,
pan/zoom, edit, save, reopen, and focus linked entities correctly. Unsafe SVGs
are rejected. Restart, map switching, module disable/re-enable, and project
reopen preserve saved state and never leak an editor webview or silently discard
dirty work.

### Phase 4: Validate practical limits and accessibility

- Measure representative large images and multiple layers; set documented
  encoded-size, pixel-count, decoded-memory, layer-count, and undo-memory limits.
- Avoid decoding hidden layers when it materially reduces memory, without
  changing persisted data.
- Verify keyboard-accessible layer controls, focus behavior, labels, contrast,
  and reduced-motion behavior.

**Exit gate:** Over-budget inputs fail before large allocations with actionable
diagnostics; supported inputs remain responsive within recorded budgets; and
the complete rendered workflow passes accessibility and lifecycle checks in a
packaged desktop build.

### Phase 5: Add semantic regions and paths

- Define stable authored semantic-feature records using normalized point, path,
  and area geometry, with entity links and optional validity.
- Add selection, drawing, editing, styling, and hit testing without changing
  raster-layer assets.
- Extend disposable projections for bounding boxes, reverse links, and spatial
  queries.

**Exit gate:** Linked regions and routes survive rename, restart, checkpoint
rebuild, and renderer-cache deletion; raster and semantic layers compose in one
viewport; malformed geometry fails in Rust.

### Phase 6: Add temporal layers and movement

- Consume Daena's shared date context to filter semantic features and routes.
- Add explicit historical layer/map variants and deterministic interval
  selection.
- Render event and character movement from shared entities and relationships,
  without copying their authoritative records into Maps.

**Exit gate:** Changing the world date deterministically changes eligible map
content while Git history, map assets, and missing date precision remain
untouched; disabling Timeline leaves maps usable.

### Phase 7: Add scale and travel services

- Define optional map scale and unit metadata with clear accuracy limitations.
- Provide distance and route-length calculations before adding travel-time
  estimates.
- Add movement profiles and terrain inputs only through versioned,
  provider-neutral services.

**Exit gate:** Scaled maps produce deterministic, unit-tested measurements;
unscaled maps make no estimates; decorative/distorted maps clearly communicate
their limitations.

### Phase 8: Add dynamic plugin layers

- Define a bounded, versioned render-data service for generated layers.
- Persist user-owned layer configuration while keeping generated frames and
  caches disposable.
- Handle plugin disable, failure, stale output, and budget violations without
  deleting shared data.

**Exit gate:** A sandboxed test plugin can provide and refresh a layer using only
granted broker capabilities; disabling it leaves the project valid and a clean
checkpoint rebuild restores all durable configuration.

### Phase 9: Add external replacement workflows

- Add trusted export and explicit replace-from-file actions for raster layers.
- Add base replacement only with dimension/aspect-ratio preview and an explicit
  reject, variant, or resample choice.
- Reuse asset revisions, conflicts, request IDs, recovery copies, and checkpoint
  barriers.

**Exit gate:** External edits either install completely or leave the prior asset
intact; stale replacements never overwrite silently; resizing never silently
misaligns or discards annotations.

### Phase 10: Evolve rendering when measurements require it

- Introduce previews, lazy decoding, tiling, mipmaps, or GPU composition only
  for demonstrated limits.
- Keep caches derived and preserve provider, entity, location, and semantic
  contracts.
- If portable tiling becomes necessary, specify and validate it as an explicit
  format change with complete rebuild coverage.

**Exit gate:** Large-map improvements meet recorded memory and interaction
budgets without changing logical map behavior or making renderer caches
authoritative.

## Verification checklist

- Rust validation is authoritative; TypeScript mirrors it for early feedback.
- FMG creation, save, navigation, selectors, and semantic layers still work.
- Base bytes never change during layer editing.
- Saving one layer changes only that asset and required metadata/generation.
- Stale asset and field revisions produce conflicts rather than last-write-wins.
- Pins survive entity rename, app restart, and projection rebuild.
- Portable checkpoint paths are deterministic and contain no temporary editor
  state, undo history, decoded buffers, or renderer caches.
- A clean checkpoint rebuild restores descriptors, layers, locations, asset
  ownership, hashes, and map projections.
- The packaged app works offline and imports no SVG script or network behavior.

## Enduring design principle

> An Image Map is an immutable source image with independently stored
> annotations and Daena-aware spatial references layered on top.

Later iterations may teach Daena more about the represented geography, time,
movement, and scale. That understanding must extend the same map entity, asset,
layer, and location foundations rather than making the first iteration carry
their complexity or creating a second map system.
