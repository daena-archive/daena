# ADR 0046: Atlas production detail algorithm 5, drainage 5, renderer 10

- Status: Accepted
- Date: 2026-08-16
- Scope: replace algorithm `4` / drainage `4` / renderer `9` so inland
  terrain is synthesized at geographic amplitude, not as a bilinear
  Physical Map wash. Renderer becomes `10`.

## Context

ADR 0045 reconstructed coastlines, but continent-scale land still read as
smooth green and tan blobs. Ridge and valley synthesis were `420` / `320`
mm, inland residuals were tens of metres after per-cell mean removal, and
hillshade used a large up-vector that flattened slope. Atlas is required to
amplify the Physical Map into geography (ridges, valleys, roughness), not
merely recolor it.

## Decisions

### 1. Released versions

| Contract | Locked value |
| -------- | ------------ |
| Detail algorithm | `5` |
| Derived drainage | `5` |
| Renderer | `10` |
| Seed policy | `1` |
| `AtlasRenderRequest` / Studio `algorithmVersion` | `5` only |

Seed prefix `daena-atlas-detail-v5\0`. Cache keys `atlas-cache-residual-v5`
and `atlas-cache-drainage-v5`. Identities use `v5`.

### 2. Visible landform synthesis

Hierarchical octaves use `36`–`720` m of control-shaped residual (plains
tens of metres; mountain influence can add up to `540` m). Divide-tree
orometry raises ridges by `920` m and cuts valleys by `680` m, fading over
`24` lattice steps, with at most `512` features and `24` peaks per system.
Interior mean residual outside the coastal envelope stays ~0, so macro
continent elevation is preserved while intra-cell grain is not.

### 3. Erosion and renderer

Post-fill erosion steps are `18` m (`8` m during hierarchical octaves).
Renderer `10` exaggerates slope (`nz = 72` m) and maps low-to-high
hypsometry across `800` m so ridge and valley millimetres read as both
shade and color. Coastline reconstruction from ADR 0045 is unchanged in
kind.

Golden `64 x 32` tiles remain the hash authority.

| Tile | Device scale | Locked PNG SHA-256 |
| ---- | ------------ | ------------------ |
| `z=0 / x=0 / y=0` | `1` | `sha256:0d56dfc587e3891d1b0d312f5e22399fe5a381e879acec1a9aa5a29bd77f3720` |
| `z=8 / x=120 / y=90` | `1` | `sha256:beddcae9b5091dad604abaec076b95142019556b96f8081c656b627ef773a5ea` |

## Consequences

PNG hashes, Studio tiles, and provenance versions change. A later
algorithm is a new version, not a revival of `4`.
