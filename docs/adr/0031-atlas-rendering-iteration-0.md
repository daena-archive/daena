# ADR 0031: Atlas Rendering iteration-0 contracts

- Status: Accepted for the iteration-0 feasibility spike
- Date: 2026-08-15
- Scope: contract decisions for the pure CPU atlas spike; production host
  commands, presets, styles, fonts, and project persistence remain deferred to
  later iterations in `docs/ATLAS_MAP_RENDERING.md`

## Context

Daena already has an accepted `daena-physical` provider, adapter version `2`,
source format `physical-world-v2`, and generator version `11`. Historical,
climate, hydrology, and hazard products are derived from that immutable source.
The interactive MapLibre globe is a preview, not an export renderer.

Atlas Rendering must produce high-resolution static maps without creating a
second world model, increasing the canonical grid, or treating exported pixels
as authority. Iteration 0 proves the renderer ownership, seed policy, tiling,
and PNG path on the existing physical golden fixture.

## Decisions

### 1. Renderer ownership

Export rendering lives in a pure crate, `crates/daena-atlas` (`daena-atlas`).
It may depend on `daena-physical` for source decode and epoch derivation. It
must not depend on Tauri, SQLite, `daena-core`, plugin-host, DOM, MapLibre, or
ambient filesystem discovery. The spike CLI may write a caller-supplied output
path; that path is operational, not project content.

`daena-core` remains the sole constructor of production physical identity.
Atlas code consumes identity as opaque bytes and does not reimplement
`PhysicalIdentityManifestV1`. The spike substitutes the UTF-8 bytes of
`sha256:` plus the lowercase hex digest of the source bytes. That stand-in is
not the production identity and must not be copied into core.

Atlas Rendering does not add a `daena-atlas` map provider.

### 2. Request, detail, and provenance versions

| Contract                         | Locked value |
| -------------------------------- | ------------ |
| Atlas request schema             | `1`          |
| Detail algorithm version         | `1`          |
| Seed-policy version              | `1`          |
| Renderer version                 | `1`          |
| Provenance schema                | `1`          |
| Initial format                   | `png`        |
| Initial projection               | whole-world equirectangular |
| Default detail level             | `detailed`   |
| Default detail variant           | `0`          |

Unknown request fields are rejected. JPEG, SVG, PDF, regional extents, labels,
authored layers, and persisted presets are out of scope for this spike.

### 3. Integer and quantization policy

Locked boundaries use integer arithmetic:

- longitude/latitude addresses are microdegrees;
- elevation and residuals are millimetres;
- bilinear weights are parts-per-million in `0..=1_000_000`;
- resource counts use checked multiplication before allocation.

Floating-point is not used to generate geographic residuals or to choose land
versus water. Display hillshade uses integer dot products.

### 4. Seed PRF

Geographic detail uses named domains. The domain key is:

```text
SHA-256(
  "daena-atlas-detail-v1\0"
  + u32le(identity byte length) + identity bytes
  + u32le(detail algorithm version)
  + u32le(detail variant)
  + u32le(domain byte length) + domain bytes
)
```

A lattice sample mixes the first two little-endian `u64` words of that key
with wrapped longitude lattice index, clamped latitude lattice index, and
octave through splitmix64. Samples are independent; there is no sequential
stream across the map.

The first named domain is `continental-relief`. Output format, output
dimensions, style, tile index, thread count, and historical year are excluded
from geographic seeds. Style-only randomness is not implemented in iteration 0.

### 5. World-space lattice and coastal clamp

Detail level selects a world-space lattice denser than the canonical grid:

| Level      | Lattice factor per source cell |
| ---------- | -----------------------------: |
| `standard` |                              4 |
| `detailed` |                              8 |
| `print`    |                             16 |

Longitude lattice indexes wrap. Residual values are constant in longitude at
each pole row (evaluated at lattice index `0`). Residual amplitude is taken
from canonical elevation magnitude, then the mean residual in each canonical
cell is removed. The residual field does not depend on epoch sea level.

After adding the residual, cells whose signed coastal distance exceeds `350/1000`
of a source cell cannot change land/water sign relative to the epoch sea
level. Distance is computed on the epoch land/water mask. Topology-changing
derived islets and tributaries are forbidden in algorithm version `1`.

### 6. Tiles, halo, and threads

Iteration 0 evaluates `512 x 512` output tiles. Point sampling and world-space
hillshade need no halo; the locked halo is `0` until strokes or labels require
one. Tile composition is order-independent. Parallel workers are not used;
forward, reverse, and shuffled tile order must match, which also covers the
future one-versus-N thread requirement until a worker pool exists.

### 7. PNG encoder

The spike encodes 8-bit RGBA with the `png` crate `0.17`,
`Compression::Fast`, `FilterType::NoFilter`, no interlacing, and uncompressed
`tEXt` provenance. Those settings match the existing core PNG writer.

The locked visual guarantee is decoded pixel identity plus those encoder
settings. Encoded file bytes are asserted on this development target; if a
supported CI target diverges, the ADR will narrow the byte-level guarantee
before product data exists. JPEG is not approved.

### 8. Fonts, styles, and network

Iteration 0 bundles no fonts and fetches no URLs. Colouring uses a locked
built-in relief palette inside the renderer (`daena-atlas-relief-spike`).
Declarative JSON styles, antique paper grain, and shaping libraries are
iteration 1. License documentation records that the spike introduces no
third-party font or style files.

### 9. Cache location

Atlas caches, when later implemented, live under the core-owned disposable
path `.daena/cache/atlas/` and are excluded from checkpoints and Git. Iteration
0 does not write that cache. Missing or corrupt cache entries will be misses,
never project errors.

### 10. Budgets (proposal until measured)

| Limit              | Proposal                         |
| ------------------ | -------------------------------- |
| Preview            | `2048 x 1024`, 2_097_152 pixels  |
| Hard maximum       | 33_554_432 pixels                |
| Named exports      | `4096 x 2048`, `8192 x 4096`     |
| Tile size          | `512 x 512`                      |

Measured release-build duration, peak RSS, temporary bytes, and encoded sizes
are recorded in `docs/maps/atlas/budgets.md` from the spike CLI. On the first
Darwin host the proposed 33,554,432-pixel maximum completed in 5.5 s at
553 MiB peak RSS. Iteration 0 still allocates a full-frame RGBA buffer for
encoding. Those measurements may reduce the proposal before iteration 1 if
another supported target exceeds a comfortable desktop envelope.

## Consequences

- Iteration 0 can render the physical golden fixture to PNG without mutating a
  project, database, or checkpoint.
- Geographic queries at a longitude/latitude are independent of output size
  and tile order.
- Host commands, Svelte UI, presets, and `daena-core` snapshot capture wait
  for iteration 1.
- Changing the PRF, lattice factor, envelope, palette, or encoder settings is
  a reviewed renderer/detail version change.
