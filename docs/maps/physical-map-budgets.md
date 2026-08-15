# Physical map budget record

This record is produced by `npm run check:maps:physical-benchmark`. It is a
release-mode process measurement on the reference development host, not a
cross-platform claim.

Locked host ceilings after ADR 0024:

| Bound | Value |
| --- | ---: |
| Production grid | 384 × 192 |
| Canonical source | 128 MiB |
| Derived GeoJSON | 256 MiB |
| Generation wall time | 8 s |
| Working memory | 128 MiB |

The 16 MiB ceiling is not retained. It was a host convenience bound, not a
v2 layout limit.

## 2026-08-15 — macOS arm64

| Preset | Grid | Wall time | Generator time | Peak RSS | Source | Derived GeoJSON | Features |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Default / maximum | 384 × 192 | 3,829 ms | 3,828.6 ms | not captured* | 577,980 B | 18,611,371 B | 62,743 |

The measured production grid remains below the locked 8 s generation budget,
the 128 MiB working-memory ceiling, 128 MiB source budget, and 256 MiB
derived-output budget. Peak RSS is process-level and includes the release
binary, not an estimate of open-map memory. The current sandbox did not expose
RSS for this run (`/usr/bin/time` cannot read the macOS resident-size counter
here).

The benchmark uses `/usr/bin/time` when it exposes RSS and otherwise samples
the cargo process tree. If neither mechanism exposes RSS, it records
`peakResidentBytes: null`; such runs do not pass the memory gate.

Cross-target support is governed by
`docs/maps/physical-map-golden-targets.md`: the locked CI matrix must pass the
same exact source and derived hashes on every listed runner. The locally
recorded reference target is `darwin/arm64`; other targets are not treated as
supported until their exact matrix jobs pass.
