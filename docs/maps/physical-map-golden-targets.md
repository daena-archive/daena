# Physical map golden target policy

The project chooses the cross-target determinism option from ADR 0016: lock a
supported target matrix and require exact canonical hashes in CI. The fixture
does not compare tolerances or metrics; `scripts/maps-physical.test.mjs`
asserts the complete source and derived GeoJSON SHA-256 values.

The locked matrix is:

| Runner | Target |
| --- | --- |
| `macos-14` | macOS arm64 (`darwin/arm64`) |
| `ubuntu-24.04` | Linux x64 (`linux/x64`) |
| `windows-2025` | Windows x64 (`win32/x64`) |

The gate is [`.github/workflows/maps-physical-golden.yml`](../../.github/workflows/maps-physical-golden.yml).
A platform is not supported until its matrix job passes the exact v6 source
and coastline hashes. Approximate matches do not pass. New targets require a
deliberate matrix change and a reviewed fixture result; no target silently
inherits support from another platform. The matrix pins Rust `1.97.1` so a
toolchain upgrade is an explicit golden-fixture decision rather than silent
numeric drift.
