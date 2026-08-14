# Physical map budget record

This record is produced by `npm run check:maps:physical-benchmark`. It is a
release-mode process measurement on the reference development host, not a
cross-platform claim.

## 2026-08-15 — macOS arm64

| Preset | Grid | Wall time | Generator time | Peak RSS | Source | Derived GeoJSON | Features |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Default | 64 × 32 | 370 ms | 50.7 ms | 6.05 MiB | 25,633 B | 156,857 B | 772 |
| Maximum | 128 × 64 | 710 ms | 401.1 ms | 19.08 MiB | 79,918 B | 617,697 B | 2,984 |

The measured maximum remains below the locked 2 s generation budget, 16 MiB
source budget, and 16 MiB derived-output budget. Peak RSS is process-level and
includes the release binary; it is not an estimate of open-map memory. The
cancellation fixture asserts observation below 100 ms at the maximum grid.

The benchmark intentionally records `peakResidentBytes: null` when the host
does not expose RSS through its time utility. Such runs do not pass the memory
gate and must not be silently converted into an estimate.

Cross-target support is governed by
`docs/maps/physical-map-golden-targets.md`: the locked CI matrix must pass the
same exact source and derived hashes on every listed runner. The locally
recorded reference target is `darwin/arm64`; other targets are not treated as
supported until their exact matrix jobs pass.
