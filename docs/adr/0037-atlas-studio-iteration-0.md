# ADR 0037: Atlas Studio iteration-0 tile contracts

- Status: Accepted for the iteration-0 feasibility spike
- Date: 2026-08-15
- Scope: runtime-only Studio session/tile schemas, Web Mercator XYZ
  addressing, north-origin row orientation, tile size and device-scale
  limits, halo/metatile policy, protocol shape, and resource proposals.
  Tauri session registry, custom protocol serving, MapLibre UI, and
  persisted presets remain deferred to later iterations in
  `docs/ATLAS_STUDIO.md`.

## Context

Atlas Rendering already produces deterministic static PNG/SVG/PDF from an
immutable physical snapshot. Atlas Studio must explore the same derived
scene interactively without becoming a second terrain generator or
reinterpreting `AtlasRenderRequest` as a pan-and-zoom contract.

Export addressing is south-increasing (row 0 is south), including regional
Web Mercator. MapLibre XYZ tiles are north-origin (row 0 is north). The
Studio adapter must perform that transform without flipping or versioning
the export renderer.

## Decisions

### 1. Separate runtime contracts

Studio does not persist session or tile records in project files, presets,
provenance, or checkpoints. Locked names and versions:

| Contract                         | Locked value |
| -------------------------------- | ------------ |
| Studio session schema            | `1` (`AtlasStudioSessionRequestV1`, host/core wrapper; iteration 1) |
| Studio scene subset (iteration 0)| `1` (`AtlasStudioSceneRequestV1`) |
| Studio tile schema               | `1` (`AtlasStudioTileRequestV1`) |
| Tile scheme                      | Web Mercator XYZ |
| Logical tile size                | `256` |
| Device scale                     | `1` or `2` |
| Halo                             | `0` |
| Metatile                         | none |
| Maximum zoom                     | `8` |
| Interactive projection           | `web-mercator` |
| Export projection                | unchanged: whole-world equirectangular and regional Web Mercator |

Unknown versions, styles, layers, projections, zooms, tile coordinates,
tile sizes, and device scales are rejected. Session ID, request ID, worker
index, and pan history never enter geographic seeds or tile bytes.

`AtlasStudioSessionRequestV1` remains the iteration-1 host/core wrapper
(map entity, epoch, detail, style, layers, captured generation). Iteration
0 does not define that struct in code. The implemented type is
`AtlasStudioSceneRequestV1`, the pure geographic subset in `daena-atlas`
(offset, algorithm, level, variant, style). Map entity IDs, content
generation, and opaque session tokens wait for iteration 1.

### 2. XYZ bounds and antimeridian ownership

Validate `z/x/y` with checked integer arithmetic. At zoom `z`, `n = 2^z`
and both `x` and `y` must satisfy `0 <= value < n`.

Longitude wraps exactly. Web Mercator latitude clamps to
`WEB_MERCATOR_MAX_LAT_MICRO` (`±85.051129°`). Poles and equirectangular
whole-world extents are not Studio tiles.

A tile's east edge is exclusive. Longitude `-180°` / `+180°` belongs to
tile `x = 0`. Tile `x = n - 1` meets that edge from the west and does not
own the wrapped meridian.

Pixel centers are computed in global Web Mercator space with central
meridian `0`, never from a tile-local seed or a tile-local PRF. Output
size and device scale change sampling density only.

### 3. Row orientation

The Studio tile adapter writes north-origin rows for XYZ. It samples
world-space longitude/latitude for each XYZ pixel center and does not
modify `render_rgba` or export provenance. Existing south-increasing
export fixtures remain the visual contract for static output.

A locked orientation fixture compares a Studio tile to the same geographic
Web Mercator export after a dedicated vertical flip of export rows. That
flip lives only in Studio tests/adapters, never in the export encoder.

### 4. Scene reuse

Static export and Studio tiles share epoch derivation, elevation residual,
epoch drainage, and style evaluation. Iteration 0 Studio tiles evaluate
the per-pixel relief path (`ocean`, `relief`, `ice`, `lakes`) only.
Vector overlays, labels, the print `frame`, and tributary strokes stay on
the export path until a halo/metatile iteration. Halo remains `0` because
point-sampled hillshade already uses a world-space lattice, not tile
neighbors.

Renderer, detail, drainage, and seed-policy versions stay at the accepted
export values (`5` / `1` / `1` / `1`). This ADR does not bump them.

### 5. Protocol shape (locked, not implemented)

Iteration 1 must serve tiles as bounded GET-like reads of encoded PNG
bytes after validating an opaque session token and `z/x/y`. The URL must
not contain project paths, cache paths, or renderer internals. Responses
are `image/png` with explicit MIME, `X-Content-Type-Options: nosniff`,
and no SVG/HTML injection. Writes, path traversal, and guessed tokens are
denied with `atlas.studio.*` codes.

Iteration 0 has no Tauri protocol and no SQLite.

### 6. Sessions, queues, and cache (proposal until measured)

| Limit | Proposal |
| ----- | -------- |
| Idle expiry | 15 minutes (iteration 1) |
| Sessions per app | 4 (iteration 1) |
| Sessions per project/map | 1 live session |
| Tile workers | 1 in this spike; later bounded pool |
| Visible prefetch ring | 1 tile (iteration 1) |
| Residual/drainage cache | existing `.daena/cache/atlas/` rules |
| Tile artifact cache | not written in iteration 0 |

Iteration-0 warm-cache evidence is residual and drainage reuse: the same
tile PNG after a miss then a hit, with cache-directory bytes recorded in
`docs/maps/atlas/budgets.md`. A tile-PNG artifact key remains deferred.

Cache keys for a future tile artifact must include physical identity,
source hash, forcing fingerprint, epoch, detail/drainage/renderer/tile
schema versions, style hash, ordered layer IDs, projection, `z/x/y`,
logical tile size, and device scale. They exclude paths, session ID,
request ID, worker number, and wall-clock time.

### 7. Failure codes

Studio validation uses `atlas.studio.request.invalid`,
`atlas.studio.tile.invalid`, `atlas.studio.resource-limit`, and
`atlas.studio.cancelled`. Diagnostics must not leak paths, SQL, or source
payloads.

### 8. Deterministic guarantee

Repeated, shuffled, and parallel requests for the same normalized tile
inputs produce identical PNG bytes. Adjacent tiles match when concatenated
along ordinary edges and across the antimeridian on the per-pixel relief
path. Geography is invariant across zoom and device scale for a shared
world-space sample.

## Consequences

- Iteration 0 can render XYZ tiles from the physical golden source without
  mutating a project, database, checkpoint, or export fixture.
- MapLibre, Tauri byte delivery, and capability-gated UI wait for
  iteration 1.
- Adding overlay strokes or labels requires a new halo/metatile decision
  and a renderer or tile-schema version if tile bytes change.
