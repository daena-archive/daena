# ADR 0044: Atlas production detail algorithm 3 and drainage 3

- Status: Accepted
- Date: 2026-08-15
- Scope: replace released detail algorithm `2` and derived drainage `2`
  with full controlled planetary hyper-amplification, divide-tree
  orometry that synthesizes elevation, and multi-scale erosion during
  hierarchical subdivision. Renderer version becomes `8`. Algorithm `2`
  and drainage `2` are removed from the production path.

## Context

ADR 0043 released bounded original implementations of the hyper-
amplification, orometry, and multi-scale erosion papers. `docs/ATLAS_STUDIO.md`
recorded remaining gaps: hierarchical control-field transfer, erosion
during subdivision with deposition landforms, and a divide tree over
every Physical Map mountain system used to synthesize elevation. Closing
those gaps requires a new version, conservation metrics, and this ADR.
Research implementations are not copied. No new crate is added. Seed
policy version stays `1` with prefix `daena-atlas-detail-v3\0`.

## Decisions

### 1. Released versions

| Contract | Locked value |
| -------- | ------------ |
| Detail algorithm | `3` |
| Derived drainage | `3` |
| Renderer | `8` |
| Seed policy | `1` |
| `AtlasRenderRequest` / Studio `algorithmVersion` | `3` only |

`request.normalize` and Studio session normalize reject algorithm `1`
and `2`. Tributary IDs are `atlas:tributary:v3:{lattice-index}`. Valley
IDs are `atlas:valley:v3:{lattice-index}`. Orometry IDs are
`atlas:orometry:v3:{kind}:{lattice-index}`. Deposition IDs are
`atlas:deposition:v3:{fan|floodplain}:{lattice-index}`.

### 2. Controlled planetary hyper-amplification

Version `3` subdivides hierarchically (`f/4`, then `f/2`, then `f`).
Each coarser residual is bilinearly upsampled before the next octave is
added. Amplitude and residual sign are transferred from accepted control
fields: elevation, crust, mountain influence, climate class, runoff,
precipitation, ice thickness, lake mask, and hydrosphere (ocean vs
land). Residual millimetres are therefore epoch-dependent. Residual
cache keys are `atlas-cache-residual-v3` and include offset years and
the historical forcing fingerprint. Mean residual in each canonical
cell is still removed. Polar rows stay constant in longitude. The
coastal sign clamp (`COASTAL_ENVELOPE_PPM = 350_000`) still applies.

### 3. Orometry / divide tree synthesis

Every connected component of positive mountain influence is a mountain
system. A divide tree is extracted on the version-`3` lattice across
all systems (not one `16 x 12` window), capped at 256 features and 12
peaks per system. Features include peaks, saddles, primary ridges,
secondary ridges, valleys, and foothills. The tree synthesizes elevation
by raising toward ridges and lowering toward valleys before the final
mean-removal pass. Cached residuals already contain that synthesis;
`from_cached_detail` only re-extracts identities.

### 4. Multi-scale erosion amplification

Fluvial (stream-power-style), thermal, and deposition processes run
during hierarchical subdivision at hops `1` with a `40` mm step, then
again on the finest lattice after Priority-Flood-style fill at hops
`4`, `2`, and `1` with an `80` mm step. Flux is slope-, accumulation-,
runoff-, and mountain-limited. Low-slope deposition is recorded as
first-class fan and floodplain identities. Mean-change conservation,
coastal sign restore, polar lock, and peak local-maxima re-enforcement
(`80` mm) remain. Drainage cache keys are `atlas-cache-drainage-v3`.

Relief-only golden tiles on the `64 x 32` physical source (renderer `8`,
algorithm `3`, drainage `3`).

| Tile | Device scale | Locked PNG SHA-256 |
| ---- | ------------ | ------------------ |
| `z=0 / x=0 / y=0` | `1` | `sha256:35061d4eae8215954c495fb6f6a8b7898873d6e5ebcb44621baafb65ad4c1eee` |
| `z=8 / x=120 / y=90` | `1` | `sha256:51ee1d895222db24a2dd1a586a3b6f49c5873f077e90228226efe548fdb7923f` |

### 5. Budgets

The in-process lattice cap remains `96 MiB`. Production `384 x 192` at
`standard`, `detailed`, and `print` stays inside that cap. Cancellation
remains every 4096 cells. Golden `64 x 32` fixtures remain the hash
authority. Macro elevation, land sign, component topology, lake/basin,
watershed, and mouth conservation stay the invariants from ADRs
0040–0043.

## Consequences

- Whole-world PNG hashes, Studio golden tiles, and provenance
  `detail_algorithm_version` / `derived_drainage_version` /
  `renderer_version` change.
- Epoch changes may change residual millimetres because climate, ice,
  and hydrosphere participate in amplification.
- Enabling a later algorithm requires a new version, not a revival of
  algorithm `2`.
