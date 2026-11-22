# Atlas redesign: layered maps on the world engine

Status: proposal (2026-11-21). Decisions taken with the user:

- Deterministic detail to zoom 12 (about 40 m per pixel at the equator).
- Hybrid layering: Rust renders raster terrain and thematic tiles, and
  OpenLayers draws vector layers.
- The old Atlas detail code is replaced, not retuned.
- The physical map view moves onto the same tiles in a later phase.

Authorities that still apply: ADR 0005 (Atlas is a deterministic derived
consumer, world-space geography, isolated PRF domains, captured sessions) and
the Atlas section of [`MAPS.md`](../maps/MAPS.md). This plan changes how Atlas
makes geography, not who owns it.

## What is wrong today

Rendered from seed 831429 at `n = 512` on 2026-11-21:

- **Whole world, relief style.** The coasts are blocky, there are no rivers
  and almost no relief, the ice shows as flat white rectangles, and the polar
  rows streak.
- **A 40°×30° region.** It is a blurred upscale. Lakes are staircase blobs and
  faint rectangular block seams show.
- **Studio zoom 6 and 8, standard and detailed.** Both are featureless beige
  blur with block seams. Zoom stops at 8.
- **Biome style, detailed level.** The biomes are pixel squares, with a hard
  horizontal band where the coarse climate grid steps. The rivers are one-pixel
  stair lines, and the bathymetry has dark streak artefacts offshore.

Causes:

1. **Atlas reads a 384×192 view (about 104 km per cell).** That is the size of
   the retired generator; the engine has 20 km cells at `n = 512` and 10 km
   at `n = 1024`. E5b kept the small view because the old refine stage could
   not afford more. Atlas discards five to ten times the real detail before
   it starts.
2. **The detail pipeline fakes what a 100 km world lacked.** `amplify`,
   `refine`, `erosion`, `drainage`, `detail` and `chunk`, about 7,000 lines,
   synthesize ridges and valleys, fill pits, and run a second erosion and
   drainage model over that blurred copy. The engine already has real
   erosion, rivers and lakes. The finest lattice is about 6.5 km, so past
   zoom 5 the tiles have nothing to show.
3. **Masks are thresholded from coarse cells.** Water, ice and biome are
   per-cell booleans or classes drawn as hard-edged pixels. Rivers are drawn
   on the lattice instead of as lines.
4. **Relief shading is clamped** to at least 78 % brightness, so mountains
   barely read.
5. **Everything is one baked raster.** Toggling rivers or contours means a
   new tile render, and lines can never be sharper than the tile pixels.

## Goals

- **Use the engine's full resolution.** Atlas samples the canonical cube
  cells directly, so at zoom 0–6 it shows the engine's own geography with
  nothing invented.
- **Add deterministic detail at zoom 7–12.** Detail is a fixed function of
  world position: zooming in only adds finer features and never moves
  coarser ones. Tiles, export size, threads and cache state cannot change a
  result, as ADR 0005 already requires.
- **Keep the canonical facts.** A canonical land cell stays land at its
  centre, a canonical river stays in its valley, and lakes keep the engine's
  water level. Detail stays inside each cell's relief envelope.
- **Layers.** One base raster: relief, biome, a climate variable or a hazard.
  Optional raster overlays, such as hillshade and ice. Vector layers:
  coastline, lakes, rivers, ice edge, contours, graticule and labels. Authored
  layers stay on top. Each can be toggled without re-rendering the others.
- **Same look in Studio and export.** PNG export composites the same layers;
  SVG and PDF draw the vector layers as vectors.
- **Budgets.**
  - A cold tile takes at most 50 ms at any zoom (release, 12 threads). Prepare
    is one world derivation.
  - The resident Atlas session is about 300 MB at `n = 1024`. The prepare
    peak is one world derivation, below today's 1.64 GB epoch-prepare peak.

## Design

### 1. Atlas surface (prepare)

Prepare derives the world for the chosen epoch once, the same way the tool
surface does. It keeps a compact `AtlasSurface` of canonical per-cell arrays
and then drops the world:

- elevation;
- the epoch sea level and the lake id with its level;
- ice cover and thickness;
- discharge and the receiver;
- biome;
- the continuous climate fields (annual and seasonal temperature,
  precipitation, aridity, storm fields);
- the hazard fields;
- the river polylines.

This is about 40–50 B per cell, or about 300 MB at `n = 1024`. It can share
the tool surface where the fields overlap.

The structure detail does not depend on the epoch, so a new epoch rebuilds
only its epoch fields. That keeps ADR 0005's structure/epoch split.

### 2. Sampling canonical fields

A world point is mapped to its cube face, then to fractional cell
coordinates. Fields are sampled with a C1 bicubic, using ghost cells across
face edges (the engine already has cross-face neighbours) so face seams don't
show:

- Continuous fields (elevation, temperature, precipitation and the rest) are
  interpolated.
- Classes (biome, lake id) are taken from the nearest cell, and the boundary
  between two classes is moved by a small world-space warp. Edges follow the
  terrain and noise instead of the pixel grid.

Polar distortion and the antimeridian disappear because sampling is done on
the sphere, not on an equirectangular view.

### 3. Deterministic elevation detail

Detailed elevation at point `p` is:

    h(p) = canonical(p) + Σ_k  a_k(p) · n_k(p)

where:

- **`n_k`** is a band-limited 3D gradient noise evaluated at the unit-sphere
  position, scaled by `R / λ_k`, with wavelength `λ_k = cell_size / 2^k`. A
  PRF keyed by physical identity, domain and octave makes it the same in
  every tile. 3D noise on the sphere has no face seams and no poles.
- **The amplitude `a_k`** comes from local canonical relief and terrain class:
  - mountains get ridged noise aligned with the slope direction;
  - plains are low and smooth, plateaus flat-topped with steep edges;
  - the continental shelf is gentle, and trenches stay smooth.

  Local relief caps the amplitude so that detail never exceeds the relief
  the canonical cell implies.

- **Mean preservation.** Each octave is zero-mean at the cell scale (wavelet
  noise, or subtracting the octave's low-pass), so the average over a
  canonical cell still matches the engine.
- **The canonical land/ocean split.** Near the shore the residual is clamped
  so that a canonical land centre stays above sea level and an ocean centre
  stays below.
- **Zoom.** A tile at zoom `z` evaluates the octaves whose wavelength is at
  least 2 pixels at `z`. Coarser octaves are identical at every zoom, so
  zooming adds features but never moves them.

### 4. Water that agrees with terrain

Water is the hard part, and it is prototyped first in phase 2:

- **Engine rivers.** They are canonical. Their polylines get a deterministic
  meander displacement, scaled by discharge and slope and bounded so the
  river stays in its canonical cells. The channel and a valley profile are
  carved into `h(p)` along the polyline, so the river always runs in a
  valley at every zoom.
- **Tributaries below engine resolution.** Each canonical cell gets a
  deterministic dendritic stream tree that drains toward the cell's receiver
  edge. Its branches are refined by zoom, the same way as the noise octaves.
  These streams are carved too. They are presentation-grade detail: they
  appear from zoom 8 and carry no hydrology claims.
- **Coastlines and lake shores.** They are contours of `h(p)`: the coast at
  sea level, and lake shores at the engine's lake level inside the lake's
  cells. Coves, headlands and islets come from the elevation detail, with no
  separate coastline algorithm.
- **Ice.** The ice edge is a contour of the interpolated cover field, and
  thickness adds a smooth surface over the ice.

### 5. Layers and tiles

| Layer                                       | Kind                      | Source                                                   |
| ------------------------------------------- | ------------------------- | -------------------------------------------------------- |
| Relief                                      | raster base               | hypsometric and bathymetric tint of `h(p)`               |
| Biome                                       | raster base               | warped class boundaries plus a small terrain tint        |
| Temperature, precipitation, aridity, storms | raster base               | interpolated fields, versioned colour ramps              |
| Earthquake and volcanic hazard              | raster base or overlay    | interpolated hazard fields                               |
| Hillshade                                   | raster overlay (multiply) | multi-directional shade of `h(p)`, no brightness clamp   |
| Ice                                         | raster overlay            | cover and thickness                                      |
| Coastline, lakes                            | vector                    | contours of `h(p)`                                       |
| Rivers, streams                             | vector                    | carved polylines, width scaled by discharge              |
| Contours                                    | vector                    | `h(p)` at styled intervals                               |
| Ice edge                                    | vector                    | contour of the cover field                               |
| Volcanoes, graticule                        | vector                    | canonical points; lines                                  |
| Labels                                      | vector                    | stable feature ids: landmass rank, lakes, rivers, ranges |
| Authored layers                             | vector                    | unchanged Maps authoring model                           |

Tile format:

- Web Mercator XYZ, 256 px, zoom 0–12, as today. Raster tiles are PNG.
- Vector tiles are compact per-tile GeoJSON at first, with MVT only if size
  demands it. The Studio protocol gains `z/x/y.<layer>.json` next to `.png`,
  under the same token and path checks.
- Contours are extracted with marching squares on a lattice aligned to the
  tile grid, with a halo, so edges match across tile seams.
- Vector geometry is simplified to the tile's pixel scale.

The OpenLayers Studio view composes the layers. A style picks the base raster
and its ramp, which overlays are on, and the vector styles.

### 6. Export

- **Static export** evaluates the same layers at the requested extent and
  size.
- **PNG** composites the rasters and rasterizes the vectors.
- **SVG and PDF** embed the raster base and write the vector layers as paths.
- **Zoom-equivalent scale.** Export picks the zoom whose pixel size matches
  the output scale, so a print and the Studio view at that zoom show the same
  detail.

### 7. What stays and what goes

These stay:

- `request`, `style` (with new style schema fields for layers), `encode`,
  `cache`, `studio` (tile protocol extended), `provenance`, `labels`,
  `projection`;
- the frontend Studio, render panel and overlay panel;
- presets, staleness, captured sessions, provenance and safe export.

These are deleted:

- `amplify`, `refine`, `erosion`, `drainage`, `detail`, `chunk`;
- the 384×192 `AtlasInputs` path and the `control` lattice;
- `STUDIO_CHUNK_*`;
- detail levels as separate lattices. Detail now follows zoom.

`ATLAS_DETAIL_ALGORITHM_VERSION` and the renderer version are bumped. Old
caches miss, and old presets that named a removed detail level resolve to the
zoom-driven detail.

## Phases

Each phase is reviewed with rendered images, a tile-seam check and timings.

1. **Surface and terrain.** The `AtlasSurface`, the cube-face sampler, the
   elevation detail octaves, and the relief and hillshade rasters.
   - Gate: a comparison sheet at zoom 2, 5, 8, 10 and 12 for three seeds,
     with the 512 and 1024 versions of the same place.
   - Gate: tiles are seam-free, the same at every thread count, and stable
     across zoom.
   - Gate: tiles meet the 50 ms budget.
2. **Water.** River meanders and carving, per-cell tributary trees, coastline
   and lake contours, and ice. Gate: rivers stay in valleys at every zoom, and
   canonical land/ocean and lakes are preserved, checked on every canonical
   cell centre.
3. **Layers in Studio.** The vector tile endpoint, the layer model, the style
   schema, and OpenLayers composition with toggles. Labels move onto stable
   feature ids.
4. **Thematic layers.** Biome with warped boundaries, and the climate and
   hazard rasters with their legends.
5. **Export parity.** PNG, SVG and PDF from the same layers, plus provenance.
6. **Clean-up and budgets.** Delete the old code, measure, set the
   constants, and update ADR 0005 and `MAPS.md`.
7. **Later.** The physical map view switches to the Atlas tiles.

## Open questions

- **Atlas constraints.** `LockPath`, `ForceLake`, `FreezeErosion` and
  `PinShoreline` are stored by `daena-core` but no UI creates them.
  Recommendation:
  - retire `FreezeErosion`, because there is no Atlas erosion any more;
  - keep `PinShoreline` and `LockPath`, as zones where detail is suppressed
    or a path is carved;
  - retire `ForceLake`, because lakes are canonical.
- **Tributary density.** The density should follow the climate's runoff,
  which needs a calibration target (drainage density against Earth values).
  The realism checks can supply that.
- **Zoom 12 at `n = 512`.** It means 9 octaves below a 20 km cell. Phase 1
  should show whether that still looks natural or needs more terrain-class
  variety.
