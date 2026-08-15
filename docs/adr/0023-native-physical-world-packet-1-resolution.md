# ADR 0023: Native physical-world Packet 1 resolution envelope

- Status: Accepted for the measured Packet 1 envelope
- Date: 2026-08-15
- Scope: production-resolution selection and derived LOD boundary

## Decision

Feature fixtures are specified in metres, independently of UI pixels. The
reviewed fixture specification is owned by
`crates/daena-physical-spike/src/resolution.rs` and covers trench half-width
and arc offset, collision belt, rift floor and shoulders, shelf and slope,
retained strait and island, lake sill, drainage divide, displayed tributary
catchment, and an internal-shape width.

The conservative cell dimension is the larger of the equatorial longitudinal
cell and meridional cell. A candidate must provide at least four samples
across every minimum retained feature and at least eight across the internal
shape fixture. Resolution tiers are measured as:

| Candidate | Maximum cell width | Minimum samples | Internal samples | Status |
| --- | ---: | ---: | ---: | --- |
| 256 x 128 | 156,368 m | 5 | 9 | production default and maximum |
| 512 x 256 | 78,184 m | 9 | 17 | preview-only pending budget evidence |
| 1024 x 512 | 39,092 m | 17 | 34 | preview-only pending budget evidence |
| 2048 x 1024 | 19,546 m | 34 | 67 | preview-only; outside selected envelope |

The selected production envelope is exactly `256 x 128`. It is both the
default and the bounded maximum because it is the largest candidate with
completed generation evidence in the current algorithm. The former ADR 0014
`128 x 64` maximum is not retained as the production maximum; it fails the
four-sample feature gate.

The measured release-mode fixture uses seed `831429`, retry index `0`, radius
`6,371,000 m`, and target land fraction `300,000 ppm`:

| Measure | 256 x 128 result |
| --- | ---: |
| Canonical v2 source bytes | 271,165 |
| Derived GeoJSON bytes | 18,417,309 |
| Derived feature count | 54,311 |
| Generation wall time | 12,091 ms |
| Source SHA-256 | `sha256:6f1d5a8a69e4de6095ed5860fea34e054d96fd30c7cfa2ce832208876c18129` |
| Derived SHA-256 | `sha256:a2ffe56e607ef95b0f822d70ada95723e6d9cce5a3e911dadf207be3f077cf8f` |

The existing small `64 x 32` and `128 x 64` fixtures remain unit-test
fixtures, not production defaults. The CLI exposes the selected production
tier through its default and `--max` paths, and exposes the four-candidate
feature matrix for repeatable review.

## Consequences

Canonical terrain is generated once at `256 x 128`. The source remains the
strict `physical-world-v2` layout and is well below its 16 MiB source limit.
The larger candidates are not accepted as physical maps. They may be used for
future preview/LOD experiments, but their derived products must not feed back
into hydrology or mutate canonical source bytes.

Diagnostic cell layers use deterministic stride-based LOD for grids above the
small fixture size. This keeps high-resolution diagnostics derived and
disposable; it does not change canonical terrain or identity.

The 12-second generation time and 18 MiB derived payload are recorded evidence,
not a claim that Packet 1 satisfies every future interactive performance
budget. A later packet must reduce the generation and transport costs before a
larger production tier can be selected. If a future selected tier exceeds v2's
strict layout or source budget, it requires a separately versioned v3 source;
v2 is never loosened or reinterpreted.
