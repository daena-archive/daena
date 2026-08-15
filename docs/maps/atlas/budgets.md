# Atlas rendering budgets

Proposal values from ADR 0031. Measurements below are from a release
`atlas-map` build on Darwin 25.6 (2026-08-15) consuming the physical golden
`64 x 32` source (`sha256:6e9a13df19859f2f0d6978526abf60d20354c23e3ba6c5acd22360e510f429c2`).
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

## Renderer version 2 (iteration 1 overlays)

Same host and golden source, relief style with default physical layers. Repeat
of `2048 x 1024` matched. Encoded bytes differ from renderer 1 because rivers,
coasts, contours, graticule, and frame are now composited.

| Target | Width | Height | Duration ms | Peak RSS | PNG bytes | SHA-256 |
| ------ | ----: | -----: | ----------: | -------: | --------: | ------- |
| Darwin 25.6 | 2048 | 1024 | 321 | 42_156_032 | 8_391_109 | `sha256:a956c3f5bf559cac827c87b1116439d68c2defe76a621f10bdc31f59a03dc626` |
| Darwin 25.6 | 4096 | 2048 | 1_227 | 143_638_528 | 33_559_877 | `sha256:9af533757d8bed624ed1741ff7e43bfc525013c90a559179e42fe205d12dbd8c` |
| Darwin 25.6 | 8192 | 4096 | 4_925 | 553_910_272 | 134_232_901 | `sha256:27a379189d8793bf6a489361c5652766837d3fa6d6631b21e0d36d00929508bb` |

## Renderer version 5 (iteration 4 derived drainage and cache)

Same host and golden source. Derived drainage version `1` emits 3 atlas-only
tributaries on this fixture. Geographic residual version remains `1`. Encoded
bytes differ from renderer 4 because tributaries are composited on the rivers
layer.

| Target | Width | Height | Duration ms | PNG bytes | SHA-256 |
| ------ | ----: | -----: | ----------: | --------: | ------- |
| Darwin 25.6 | 2048 | 1024 | 358 | 8_391_268 | `sha256:3dc3611aedbea11867da311c4ee8f47b9b045d5a66f1c05e29df288591108f14` |
| Darwin 25.6 | 4096 | 2048 | 1_345 | 33_560_036 | `sha256:3f542b3f1c7fc51e7c8353b6c816ae597f485c32588ca6aa195daceecf75eb9c` |
| Darwin 25.6 | 8192 | 4096 | 5_239 | 134_233_060 | `sha256:4c8d4826e856dd46910120839114a25c60f4117eeca698d01904ac834660f1ed` |

Artifact cache on `256 x 128`: cold miss then warm hit, identical PNG. Quota is
512 MiB / 64 entries / 160 MiB per entry under `.daena/cache/atlas/`.

## Atlas Studio iteration 0 (ADR 0037)

Same host and golden source. Studio tiles are Web Mercator XYZ, 256 px,
device scale 1, per-pixel relief layers only. Scene preparation is shared
with export; each CLI invocation below includes one prepare plus the named
tile work. Repeating `z=8 / x=120 / y=90` produced the same PNG bytes.
The `2048 x 1024` export hash above is unchanged after the scene extract
(`sha256:3dc3611aedbea11867da311c4ee8f47b9b045d5a66f1c05e29df288591108f14`).

Iteration 0 writes **no tile artifact cache**. Warm reuse is the existing
residual and drainage entries under an explicit cache directory. Those
entries are 525_016 bytes on this fixture (one residual blob, one drainage
blob, plus `index.json`). They stay inside the accepted 512 MiB / 64
entries / 160 MiB-per-entry quota. A later tile-PNG cache is an iteration-1
decision.

| Target | Work | Duration ms | Peak RSS | PNG bytes | Cache |
| ------ | ---- | ----------: | -------: | --------: | ----- |
| Darwin 25.6 | cold tile `z=0` | 39 | — | 263_357 | off |
| Darwin 25.6 | cold tile `z=4` | 40 | — | 257_100 | off |
| Darwin 25.6 | cold tile `z=8` | 40 | — | 263_189 | off |
| Darwin 25.6 | prepare + `z=8` tile + 3×3 burst (9 tiles) | 152 (burst 126) | 6.7 MiB | 263_189 | off |
| Darwin 25.6 | `z=8` residual/drainage miss | 40 | 6.5 MiB | 263_189 | 525_016 bytes |
| Darwin 25.6 | `z=8` residual/drainage hit | 31 | 6.6 MiB | 263_189 | same 525_016 bytes |

Maximum zoom `8` stays inside an interactive envelope on this host. Overlay
halo and Tauri protocol serving remain iteration 1.
