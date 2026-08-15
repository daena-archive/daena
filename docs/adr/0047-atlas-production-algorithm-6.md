# ADR 0047: Atlas production detail algorithm 6, drainage 6, renderer 11

- Status: Accepted
- Date: 2026-08-16
- Scope: replace algorithm `5` / drainage `5` / renderer `10` so zoomed-in
  terrain is divide-tree ridge and valley *paths*, not isotropic noise pits.

## Context

ADR 0046 raised residual and orometry amplitudes so continents read at world
scale. At Studio zoom the same pipeline produced a leopard-print of isolated
dark blobs: hierarchical octaves were independent hashes per lattice cell,
valleys were local minima of that noise, and orometry flooded isotropically
from those points. Atlas is required to amplify the Physical Map into
geography (ridges, valleys, drainage), not to threshold noise.

## Decisions

### 1. Released versions

| Contract | Locked value |
| -------- | ------------ |
| Detail algorithm | `6` |
| Derived drainage | `6` |
| Renderer | `11` |
| Seed policy | `1` |
| `AtlasRenderRequest` / Studio `algorithmVersion` | `6` only |

Seed prefix `daena-atlas-detail-v6\0`. Cache keys `atlas-cache-residual-v6`
and `atlas-cache-drainage-v6`. Identities use `v6`.

### 2. Path orometry instead of pit blobs

Hierarchical octaves sample bilinear interpolated noise (steps `8` / `4` /
`2`) at `8`–`96` m so grain is spatially coherent. Unprotected pits are
Priority-Flood filled up to `720` m before the divide tree runs.

The divide tree still walks mountain-influence cells high-to-low, but saddles
record the steepest-ascent paths to both peaks. Those polylines, plus
steepest-descent from saddles and high-accumulation drainage, are the ridge
and valley *seeds*. Synthesis is a distance field from those paths with
`8`-cell falloff (`780` m ridges, `520` m mountain valleys, `96` m plains
valleys). Local-minima valley features are not extracted.

Peak spacing is at least `6` lattice steps, with at most `768` features and
`48` peaks per system.

### 3. Renderer

Renderer `11` keeps reconstructed-shore painting from renderer `10`. Hillshade
uses a quarter-lattice slope sample and a high up-vector so relief follows
baked ridge/valley paths instead of a global noise hatch. World-space
landform grain is not part of this algorithm: it painted the same smudge on
every land cell and is not orometry.

Golden `64 x 32` tiles remain the hash authority.

| Tile | Device scale | Locked PNG SHA-256 |
| ---- | ------------ | ------------------ |
| `z=0 / x=0 / y=0` | `1` | `sha256:882d34d1bc3a72d227ae2f87e2f697d8d9facbb1a3873b01172ebb27e020de50` |
| `z=8 / x=120 / y=90` | `1` | `sha256:7bdbc334f10c15b638c4a6fa86953b1250b1ad6326675a7037ebeea8786c50de` |

## Consequences

PNG hashes, Studio tiles, and provenance versions change. A later
algorithm is a new version, not a revival of `5`.
