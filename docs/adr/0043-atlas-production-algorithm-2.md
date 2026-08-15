# ADR 0043: Atlas production detail algorithm 2 and drainage 2

- Status: Accepted
- Date: 2026-08-15
- Scope: replace released detail algorithm `1` and derived drainage `1`
  with hierarchical amplification, mountain topology, refined drainage,
  and multi-scale erosion. Renderer version becomes `6`. Algorithm `1`
  and drainage `1` are removed from the production path.

## Context

ADRs 0040 and 0041 locked versions `2` as hidden spikes. Those spikes
now replace the released products. Static export hashes, Studio tiles,
and cache keys change. Seed policy version stays `1` with the
`daena-atlas-detail-v2\0` prefix.

## Decisions

### 1. Released versions

| Contract | Locked value |
| -------- | ------------ |
| Detail algorithm | `2` |
| Derived drainage | `2` |
| Renderer | `6` |
| Seed policy | `1` |
| `AtlasRenderRequest` / Studio `algorithmVersion` | `2` only |

`request.normalize` and Studio session normalize reject algorithm `1`.
Capabilities, Studio, and the export panel request algorithm `2`.
Tributary IDs are `atlas:tributary:v2:{lattice-index}`. Valley IDs are
`atlas:valley:v2:{lattice-index}`.

### 2. Pipeline

`prepare_from_source` builds `ControlFields` from the accepted physical
source plus the selected epoch, then `build_amplification_model`, then
`build_refined_hydrology`. The eroded `worked_mm` lattice is converted
to residuals for `AtlasDetailModel::refined_at`. Atlas-only tributaries
come from the refined product, not the former `minor-tributaries`
domain. Residual cache keys use `atlas-cache-residual-v2`; drainage
cache keys use `atlas-cache-drainage-v2` and store tributaries plus
`worked_mm`. Old version-`1` entries are misses.

Relief-only golden tiles on the `64 x 32` physical source (renderer `6`,
algorithm `2`, drainage `2`):

| Tile | Device scale | Locked PNG SHA-256 |
| ---- | ------------ | ------------------ |
| `z=0 / x=0 / y=0` | `1` | `sha256:6ed0b97af9cbd5ff5da06b5846f68a6af1cbe22a2fe6e6932c0c2f54156ef39a` |
| `z=8 / x=120 / y=90` | `1` | `sha256:dd1381438ea6c0059d62f9a8322ea649a0a27f6321a3e9a93b83d64e4b20908f` |

### 3. Budgets

The in-process lattice cap is `96 MiB` per elevation lattice so the
production `384 x 192` grid at `detailed` (and `print`) can refine.
Cancellation remains every 4096 cells. Golden `64 x 32` fixtures remain
the hash authority; production-grid conservation stays the same
invariants as ADRs 0040–0041.

### 4. Removed paths

`build_detail_model` (`continental-relief`) and
`derive_minor_tributaries` (`minor-tributaries`, `atlas:tributary:v1:*`)
are deleted. There is no dual-version switch.

## Consequences

- Whole-world PNG hashes, Studio golden tiles, and provenance
  `detail_algorithm_version` / `derived_drainage_version` /
  `renderer_version` change.
- Enabling a later algorithm requires a new version, not a revival of
  algorithm `1`.
