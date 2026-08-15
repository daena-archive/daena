# ADR 0045: Atlas production detail algorithm 4, drainage 4, renderer 9

- Status: Accepted
- Date: 2026-08-16
- Scope: replace released detail algorithm `3` and derived drainage `3`
  with coastline reconstruction that actually moves the sea-level contour,
  inland residual amplitudes that shade, and a renderer that paints land
  from refined elevation instead of the physical-cell ocean mask.
  Renderer version becomes `9`. Algorithm `3` and drainage `3` are
  removed from the production path.

## Context

ADR 0044 shipped hierarchical amplification, orometry, and multi-scale
erosion, but Studio still showed the Physical Map's `384 x 192` squares.
Three defects caused that:

1. `pixel_rgba` classified land from `nearest_cell` ocean, so the shore
   was the Voronoi edge of the simulation grid.
2. Residual amplitude was capped at `1_200` mm. Ocean and land differ by
   kilometres, so noise could not cross sea level.
3. Per-cell mean residual removal forbade the systematic coastal ramp
   needed to place the zero contour on a displaced land-fraction isoline.
   Hillshade also clamped almost all slopes to a flat value.

`docs/ATLAS_STUDIO.md` requires Atlas to amplify the Physical Map into
believable geography, not upscale the control grid.

## Decisions

### 1. Released versions

| Contract | Locked value |
| -------- | ------------ |
| Detail algorithm | `4` |
| Derived drainage | `4` |
| Renderer | `9` |
| Seed policy | `1` |
| `AtlasRenderRequest` / Studio `algorithmVersion` | `4` only |

Seed prefix is `daena-atlas-detail-v4\0`. Residual cache keys are
`atlas-cache-residual-v4`. Drainage cache keys are
`atlas-cache-drainage-v4`. Tributary, valley, orometry, and deposition
IDs use `v4`.

### 2. Coastline reconstruction

After hierarchical octaves, orometry, and interior mean-removal, Atlas
reconstructs a coastal ramp from the bilinear land mask (`0` / `1_000_000`)
plus three interpolated noise octaves (`coastline-synthesis` domain,
displacement `380_000` ppm of a physical cell). Where the displaced
land fraction is `500_000`, elevation is forced through sea level with a
`72_000` mm ramp. Lake cells are skipped. Polar rows stay constant.
The coastal sign clamp (`COASTAL_ENVELOPE_PPM = 500_000`) still forbids
sign changes outside the envelope. Interior (non-envelope) cells still
have near-zero mean residual.

### 3. Renderer 9

Land vs ocean is `refined_at >= sea_level`. Canonical lakes and ice stay
nearest-cell physical facts. Physical-grid `coastline_segments` are not
stroked; the `coastlines` layer inks a sea-level band on the reconstructed
shore instead. Hillshade uses a slope-relative light so tens-of-metres
of residual and orometry are visible.

Inland landform amplitude is `2_400`–`96_000` mm (mean-removed per
canonical cell). Ocean interiors stay millimetre-scale.

### 4. Conservation

Unchanged outside the coastal envelope: land sign, interior component
topology, lakes, basins, watersheds, and river mouths. Envelope cells
may change sign and mean elevation so bays and capes can exist.
Golden `64 x 32` fixtures remain the hash authority.

Relief-only golden tiles on the `64 x 32` physical source (renderer `9`,
algorithm `4`, drainage `4`).

| Tile | Device scale | Locked PNG SHA-256 |
| ---- | ------------ | ------------------ |
| `z=0 / x=0 / y=0` | `1` | `sha256:a42f94e0fadfa0405351f7a41bb4f563bf0e91b2b4e9d7fc7fe96b009500cae4` |
| `z=8 / x=120 / y=90` | `1` | `sha256:3aa9eaec65aafe86446f77cfa0020c6a9a188042af77543b2149aec8722412fc` |

## Consequences

- Whole-world PNG hashes, Studio golden tiles, and provenance versions
  change.
- Enabling a later algorithm requires a new version, not a revival of
  algorithm `3`.
