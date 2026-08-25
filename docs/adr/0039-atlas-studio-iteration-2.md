# ADR 0039: Atlas Studio iteration-2 composition

- Status: Accepted for the complete interactive composition slice
- Date: 2026-08-15
- Scope: epoch and style switching, ordered layer controls,
  authored/semantic overlays, deterministic labels, bounded inspection,
  visible-tile priority with a 1-tile prefetch ring, stale-generation
  notice, and current-view regional export through existing Atlas
  Rendering jobs. Terrain algorithm version `2` remains deferred.

## Context

ADR 0038 shipped a relief-only OpenLayers viewport at physical offset `0`.
Iteration 2 in `docs/ATLAS_STUDIO.md` must complete interactive
composition without a second terrain generator, without persisting
Studio sessions, and without changing renderer version `5` or static
export fixture hashes.

## Decisions

### 1. Session inputs

`AtlasStudioSessionRequestV1` (schema `1`) now accepts:

| Field | Iteration-2 rule |
| ----- | ---------------- |
| `styleId` | any bundled style from capabilities |
| `offsetYears` | physical offset in the accepted historical range |
| `timeKind` | `physical-offset-year` or `calendar-year` |
| `authoredYear` | required for calendar-year; resolved in core |
| `activeLayerIds` | any subset of reported layers, including authored/semantic UUIDs |

Calendar years `1`, `42`, negatives, and no-year-zero calendars are
converted by `physical_offset_for_authored_year` in `daena-core`.
Svelte must not use JavaScript `Date`. Changing style, epoch, or layers
opens a new immutable session and cancels the previous one for that
map. Pan and zoom do not recapture.

The print `frame` layer is omitted from Studio tiles even when selected.
It remains available on static export.

### 2. Overlays, labels, and halo

Relief-only tiles (`ocean`, `relief`, `ice`, `lakes`, no overlay
features) keep halo `0` and the iteration-0 per-pixel path so those PNG
bytes stay unchanged.

When rivers, coastlines, contours, tectonics, watersheds, graticule,
labels, or captured overlays are active, Studio uses halo `16` logical
pixels, composites through the existing overlay path, then crops.
Labels are placed in global XYZ pixel space at the tile zoom and device
scale, sorted by stable feature ID, with a 256-label cap. Adjacent tiles
draw the intersecting portion of the same boxes; they do not independently
re-collide. Atlas-only tributary labels stay `derived-tributary` and are
not mutable.

### 3. Inspection

`project_atlas_studio_inspect` hit-tests the captured overlay set plus
derived tributaries against a click in microdegrees. It returns at most
32 features with stable IDs, layer IDs, kind, label, and a `derived`
flag. It does not query live project state or rebind identities.

### 4. Priority, prefetch, and stale sessions

The protocol accepts `priority=visible` (default) or `priority=prefetch`.
Visible requests wait on the single tile worker. Prefetch uses
`try_lock` and is `503` / `atlas.studio.resource-limit` when the worker
is busy or more than 8 requests are already waiting. Sessions keep an
LRU of at most 64 encoded tile PNGs / 32 MiB. The UI prefetches a
1-tile ring after idle. `atlas.studio.stale` is shown in-Studio with
**Refresh Atlas**; outstanding tiles may finish from the captured
generation.

### 5. Current-view export and presets

**Export** converts the OpenLayers geographic bounds plus the current
session style, epoch, and layers into an existing regional Web Mercator
`AtlasRenderRequest` (`unlockAspect: true`) and opens `AtlasRenderPanel`.
Browser pixels and session tokens never enter the job or a preset.
Existing `atlasPresets` may be applied to Studio controls; applying a
preset does not write session state.

## Failure codes

No new codes. Invalid calendar years use existing core validation.
Prefetch saturation reuses `atlas.studio.resource-limit`.

## Consequences

- Studio composition matches Atlas Rendering for the same captured
  inputs, projection, extent, style, epoch, and layers.
- Static export hashes and renderer version `5` stay unchanged.
- Packaged Tauri inspection remains required for protocol prefetch,
  inspection, and current-view export.
