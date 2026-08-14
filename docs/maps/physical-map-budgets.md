# Physical map budget record

This record is produced by `npm run check:maps:physical-benchmark`. It is a
release-mode process measurement on the reference development host, not a
cross-platform claim.

## 2026-08-15 — macOS arm64

| Preset | Grid | Wall time | Generator time | Peak RSS | Source | Derived GeoJSON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Default | 64 × 32 | 380 ms | 30.6 ms | 6.17 MiB | 25,633 B | 204,226 B |
| Maximum | 128 × 64 | 690 ms | 386.7 ms | 19.06 MiB | 79,918 B | 786,649 B |

The measured maximum remains below the locked 2 s generation budget, 16 MiB
source budget, and 16 MiB derived-output budget. Peak RSS is process-level and
includes the release binary; it is not an estimate of open-map memory. The
cancellation fixture asserts observation below 100 ms at the maximum grid.

The benchmark intentionally records `peakResidentBytes: null` when the host
does not expose RSS through its time utility. Such runs do not pass the memory
gate and must not be silently converted into an estimate.

The source/derived golden fixture currently passes on `darwin/arm64`, the
recorded reference target. Other targets must run the same fixture and add a
recorded hash before they are treated as supported; the double-precision
tectonic internals are not declared cross-target deterministic yet.
