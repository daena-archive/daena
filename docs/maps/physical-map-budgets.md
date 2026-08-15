# Physical map budget record

This record is produced by `npm run check:maps:physical-benchmark`. It is a
release-mode process measurement on the reference development host, not a
cross-platform claim.

## 2026-08-15 — macOS arm64

| Preset | Grid | Wall time | Generator time | Peak RSS | Source | Derived GeoJSON | Features |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Default | 64 × 32 | 432 ms | 80.7 ms | not captured* | 25,633 B | 2,296,179 B | 8,655 |
| Maximum | 128 × 64 | 1,164 ms | 825.1 ms | not captured* | 79,918 B | 8,358,988 B | 29,264 |

The measured maximum remains below the locked 2 s generation budget, the
128 MiB working-memory ceiling, 16 MiB source budget, and 16 MiB derived-output
budget. Peak RSS is process-level and includes the release binary, not an
estimate of open-map memory. The current sandbox did not expose RSS for this
run (`/usr/bin/time` cannot read the macOS resident-size counter here); the
previous Iteration 4 release record measured 21.00 MiB at the maximum grid.
The cancellation fixture asserts observation below 100 ms at the maximum
grid.

The benchmark uses `/usr/bin/time` when it exposes RSS and otherwise samples
the cargo process tree. If neither mechanism exposes RSS, it records
`peakResidentBytes: null`; such runs do not pass the memory gate.

Cross-target support is governed by
`docs/maps/physical-map-golden-targets.md`: the locked CI matrix must pass the
same exact source and derived hashes on every listed runner. The locally
recorded reference target is `darwin/arm64`; other targets are not treated as
supported until their exact matrix jobs pass.
