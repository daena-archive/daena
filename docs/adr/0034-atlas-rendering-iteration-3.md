# ADR 0034: Atlas Rendering iteration-3 regional and print output

- Status: Accepted
- Date: 2026-08-15
- Scope: regional extent, Web Mercator, single-page SVG/PDF, print page box

## Decision

Iteration 3 keeps the existing south-increasing pixel addressing (row 0 is
south) so world-space detail stays aligned with renderer versions 1–3.

The request carries a geographic `extent` in longitude/latitude microdegrees
and a projection ID. Browser or CSS pixel coordinates are never stored.
Whole-world equirectangular remains `west = -180°`, `east = +180°` exclusive,
`south = -90°`, `north = +90°`. An empty or inverted latitude range is
rejected. Longitude may cross the antimeridian; a zero-width longitude span is
rejected. `east = +180°` is stored unwrapped so full-world is distinguishable
from an empty interval at `-180°`.

Projection IDs added this iteration:

- `equirectangular` (default): whole-world or regional.
- `web-mercator`: regional only, latitudes inside `±85.051129°`
  (`WEB_MERCATOR_MAX_LAT_MICRO = 85_051_129`). Poles and whole-world extents
  are rejected. Forward/inverse uses IEEE-754 `f64` rounded to microdegrees;
  locked fixtures must round-trip within 1 microdegree.

Equal-earth, orthographic, and multi-page/tiled sheets remain deferred.
Aspect ratio is preserved unless `unlockAspect` is true: width and height must
match the projected span within one pixel.

Formats are `png`, `svg`, and `pdf`. JPEG remains unapproved. Preview jobs
always encode PNG for in-app display. Export writes the requested format.

SVG is a self-contained XML wrapper around the same raster PNG (base64 `data:`
image, not a remote URL) plus provenance in `<desc>`. Scripts, event handlers,
and `http(s)` URLs are prohibited.

PDF is a single-page PDF 1.4 file: MediaBox from `width_px * 72 / dpi` by
`height_px * 72 / dpi` points, Flate-compressed DeviceRGB image, no JavaScript,
no `/URI` actions, no external streams. Provenance is stored in the Info
dictionary. Page geometry is checked by reopening the bytes, not by trusting
the encoder return.

Renderer version is `4`. Geographic detail algorithm version remains `1`.
Encoder IDs are `png-0.17-fast-nofilter`, `svg-1-embedded-png`, and
`pdf-1.4-flate-rgb`.
