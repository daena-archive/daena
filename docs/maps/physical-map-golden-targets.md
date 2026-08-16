# Physical map target policy

The project still follows the cross-target determinism option from ADR 0016:
supported runners must generate valid v13 sources on the locked matrix. Tests
check structure, land-fraction tolerance, byte sizes against the CLI summary,
and the 10 s generation budget. They do not lock canonical SHA-256 values.

The locked matrix is:

| Runner | Target |
| --- | --- |
| `macos-14` | macOS arm64 (`darwin/arm64`) |
| `ubuntu-24.04` | Linux x64 (`linux/x64`) |
| `windows-2025` | Windows x64 (`win32/x64`) |

`scripts/maps-physical.test.mjs` is the focused gate. New targets require a
deliberate matrix change. The matrix pins Rust `1.97.1` so a toolchain upgrade
is an explicit decision rather than silent numeric drift.
