# ADR 0024: Production 256×128 envelope and non-arbitrary byte ceilings

- Status: Accepted
- Date: 2026-08-15
- Scope: production grid, source/derived byte ceilings, generation time and memory budgets
- Supersedes: ADR 0023 production default/maximum and the 16 MiB derived/source host ceilings

## Decision

The production physical grid is `256 x 128`. It is both the default and the
bounded maximum. Feature fixtures and the four/eight-sample gates from
`crates/daena-physical-spike/src/resolution.rs` are unchanged; this tier already
clears those gates.

The former 16 MiB host ceiling is not a `physical-world-v2` layout constraint.
v2 encoding is unchanged. Host storage, transfer, and derived-product ceilings
are independent bounds. Generation wall time and working memory stay at the
previous 2 s / 128 MiB envelope.

| Bound | Value |
| --- | ---: |
| Canonical source | 128 MiB |
| Derived GeoJSON | 256 MiB |
| Generation wall time | 2 s |
| Working memory | 128 MiB |
| Host asset transfer | same as source ceiling |

`512 x 256`, `1024 x 512`, and `2048 x 1024` remain preview-only. Derived cell
layers keep deterministic stride-based LOD so diagnostic and raster-like
polygons do not emit one feature per source cell.

The measured release-mode fixture uses seed `831429`, retry index `0`, radius
`6,371,000 m`, and target land fraction `300,000 ppm`:

| Measure | 256 x 128 result |
| --- | ---: |
| Canonical v2 source bytes | 271,165 |
| Derived GeoJSON bytes | 12,931,757 |
| Derived feature count | 47,755 |
| Generation wall time | 844 ms |

## Consequences

Accepted physical maps generate at `256 x 128`. Source bytes remain the
strict v2 layout. Derived GeoJSON stays disposable and rebuildable. A later
incompatible source layout still requires `physical-world-v3`; this decision
does not reinterpret v2 fields.
