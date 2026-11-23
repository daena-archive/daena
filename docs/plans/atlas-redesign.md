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

## Status

Phase 1 is done, plus an imagery style that was not in the original plan:

- **1 (`14f04b3`):** cube-sphere stencils, lake levels on the reference datum,
  the terrain module, relief tiles, and the `atlas-terrain` review bin.
- **1b (`3dd4f26`):** the imagery style.
  - Land colour comes from biome vegetation targets, nudged by climate.
  - Lighting is one sun plus sky light.
  - Water is coloured by depth.
  - Detail is eroded, with downhill gullies.
- **1c:** ice and lowlands.
  - Bright colours roll off instead of clipping.
  - Ice has snow and melt zones, and its surface is smoothed.
  - Ice edges follow terrain.
  - Sea ice is pack ice with a broken edge.
  - Lowlands get micro-relief and soft patches.
  - Vegetation targets are smoothed over land neighbours.

Lessons from 1b and 1c:

- Linear features made by thresholding noise look artificial, and were
  removed. These include drainage veins, glacier flow lines, crevasses and
  hard tree clumps. Real drainage and flow structure has to come from the
  engine's flow routing, which is phase 2.
- A domain warp must move each octave by less than its own wavelength, or it
  folds space into swirls.
- The biome map is per cell. At zoom 5 a cell is only a few pixels, so
  per-cell targets must be smoothed before they are interpolated.

Imagery uses almost the whole tile budget: about 49 ms of 50 at zoom 12,
n=512. Phase 3 needs headroom before it adds vector layers.

Known artifacts, deferred:

- **Ice-sheet edges** at zoom 5–8 are sharp cut-outs. The ice flag is per
  cell.
- **Thin horizontal bars of ice** at zoom 5, in both styles. The cause is not
  found yet.
- **One-cell lakes** along rivers draw as staircases at zoom 5.
- **A faint crease along cube-face edges** shows on smooth ice and at zoom 2.
- **The tundra review site** is a 3 km plateau at −21 °C, so it stays snow.

**Phase 3a (Studio base map).** Imagery (new, the default) and Elevation are
served by the terrain renderer up to zoom 12. A Studio session is either
`Terrain` or `Legacy`:

- **Terrain sessions** build `AtlasTerrain` from the physical tools' shared
  `ToolSurface` LRU. They cache at most two terrains, one per (map, identity,
  generation, epoch), and imagery and relief share one. At n=1024 a terrain
  owns about 246 MiB. Terrain sessions never run the legacy prepare, which
  peaks at about 1.6 GB.
- **Inspect and region proposal** read the terrain's `ToolSurface`. The
  inspect readout shows the detailed elevation and the cell's climate. It
  has no winds, seasonal climate, storms, tributaries or orometry.
- **Vectors** are the phase 3b tiles: coastline, lakes, rivers and contours,
  plus the existing graticule. Imagery turns rivers on. Elevation also turns
  on coastline, lakes and contours. The graticule stays off. Toggles do not
  reopen the session.
- **Not on terrain styles yet:**
  - labels;
  - authored lines and points drawn by the server (the frontend still draws
    authored overlays);
  - tectonics, hazards, wind and debug layers. Their toggles are hidden on
    terrain sessions.
- **Still legacy:**
  - thematic styles, capped at zoom 8;
  - export, which renders "Imagery" with the relief palette.
  - Capabilities still report `studio_max_zoom` 8. The view reads the
    session's `max_zoom` instead.

**Phase 3b (vector tiles).** Terrain sessions serve
`/{z}/{x}/{y}.{layer}.json` for `coastline`, `lakes`, `rivers` and `contours`.
Each tile is a GeoJSON `FeatureCollection` in lon/lat. Coast, lakes and
contours are marching squares on a lattice of one node every two scale-1
pixels, with a one-node halo, sampled the same way as the raster. Rivers are
smoothed canonical centre-lines from `ToolSurface`, not yet carved into
`h(p)` (phase 2), so at high zoom they can cross small detailed ridges.
Rivers end on the centre of the first ocean or lake cell, so they reach the
water. Vector tiles have their own render gate and queue, 4 renders and 128
waiting, so they never crowd out the raster tiles. Neither queue retries a
rejected tile yet.
Legacy sessions reject the vector URL. On a release n=512 world, z12 median
tile time was 2.1 ms for the coastline, 0.7 ms for lakes, 0.004 ms for rivers
and 5.4 ms for contours (3.3 ms at the highest land site), all under the
15 ms budget. `TerrainVectors::new`
was 2.2 ms at n=512 (2.6 MiB) and 3.7 ms at n=1024 (4.5 MiB).

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
