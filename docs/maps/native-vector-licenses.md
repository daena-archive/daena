# Native Vector Maps licenses

Phase 0 records the four frontend dependencies added with
`deno install --node-modules-dir=auto`. Exact resolved versions are in
`deno.lock`. Do not move this slice to MapLibre 6 until
`terra-draw-maplibre-gl-adapter` documents that major and the Phase 0 fixtures
pass.

| Package | Resolved version | License | Notice |
| --- | --- | --- | --- |
| `maplibre-gl` | 5.24.0 | BSD-3-Clause | [`maplibre-gl-LICENSE.txt`](./native-vector-licenses/maplibre-gl-LICENSE.txt) |
| `terra-draw` | 1.32.3 | MIT | [`terra-draw-LICENSE.txt`](./native-vector-licenses/terra-draw-LICENSE.txt) |
| `terra-draw-maplibre-gl-adapter` | 1.4.1 | MIT | [`terra-draw-maplibre-gl-adapter-LICENSE.txt`](./native-vector-licenses/terra-draw-maplibre-gl-adapter-LICENSE.txt) |
| `d3-contour` | 4.0.2 | ISC | [`d3-contour-LICENSE.txt`](./native-vector-licenses/d3-contour-LICENSE.txt) |

The published `terra-draw` packages do not ship a `LICENSE` file. The MIT
notices below follow the license declared in those packages and the upstream
repository copyright.
