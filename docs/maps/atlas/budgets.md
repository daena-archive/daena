# Atlas rendering budgets

Proposal values from ADR 0031. Measurements below are from a release
`atlas-map` build on Darwin 25.6 (2026-08-15) consuming the physical golden
`64 x 32` source (`sha256:f520abeaf54426178f6c208879341991fe611cd676073d060a844a27a89d7a2e`).
Temporary bytes equal the encoded PNG; the spike writes no `.daena` cache.

| Item | Proposal | Measured status |
| ---- | -------- | --------------- |
| Preview | at most `2048 x 1024` (2,097,152 pixels) | 367 ms, 41 MiB peak RSS, 8.0 MiB PNG |
| Named export | `4096 x 2048` | 1.38 s, 137 MiB peak RSS, 32.0 MiB PNG |
| Named export / hard maximum | `8192 x 4096` (33,554,432 pixels) | 5.45 s, 527 MiB peak RSS, 128.0 MiB PNG |
| Tile size | `512 x 512` | used; composition is order-independent |
| Halo | `0` | sufficient for point sampling |

Iteration 0 still allocates a full-frame RGBA buffer for PNG encoding, so peak
RSS tracks `width * height * 4` plus the encoded output rather than one tile.
That stays inside a comfortable desktop envelope at the proposed maximum; a
streaming encoder can wait until labels/strokes force a larger halo.

## Release-build spike CLI

Host includes `cargo run --release` in the sampled process tree.

| Target | Width | Height | Duration ms | Peak RSS | Temp bytes | PNG bytes | SHA-256 |
| ------ | ----: | -----: | ----------: | -------: | ---------: | --------: | ------- |
| Darwin 25.6 | 2048 | 1024 | 366 | 41_009_152 | 8_390_923 | 8_390_923 | `sha256:2828c951bcd178d858e56b868ba421ff4878096625153a02db46ef8b75ae17f1` |
| Darwin 25.6 | 4096 | 2048 | 1_379 | 143_294_464 | 33_559_691 | 33_559_691 | `sha256:b4beb840e7cae1de055433e85a6d00b3e7b11d88607aacfec054c0eead81eace` |
| Darwin 25.6 | 8192 | 4096 | 5_454 | 552_697_856 | 134_232_715 | 134_232_715 | `sha256:a3690768e75633a8995405ae23f0308cea78608a8a591f561473886851e91f8c` |

Repeating the `2048 x 1024` request from the same source produced the same
PNG bytes. Other CI/desktop targets should record a row before treating encoded
file bytes as a cross-target guarantee; decoded pixels plus encoder settings
remain the locked visual contract.
