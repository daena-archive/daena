# World engine: emergent physical worlds at real-world scale

Status: accepted direction, phase A in progress. The engine is a new crate,
`crates/daena-worldgen`; the current `crates/daena-physical` pipeline keeps
working until the engine is integrated (phase D).

Decisions (2026-11-08):

- New crate, integrated in phase D.
- Default resolution `n = 512` (~20 km cells); `n = 1024` (~10 km) is the
  high setting.
- Phase D is a hard cut: Maps is in beta, so worlds saved in the current
  source format are not kept readable.
- Hazards (tsunamis, tornadoes, cyclones and the rest) are deferred until
  terrain, climate and water are proven.
- Atlas keeps solving the coarse-to-detail transform for now; it is revisited
  once the canonical cells are ~10–20 km instead of ~100 km.

## Why a new engine

The current generator runs every process on one equirectangular grid at one
resolution, and Atlas paints detail on top of it afterwards (ridge and valley
synthesis, amplified coastlines). Measured on an M4 Pro (12 cores), release
build, seed 831429:

| Grid | Cell at equator | Cells | Time | Peak RSS |
| --- | --- | --- | --- | --- |
| 384×192 (production) | 104 km | 74k | 1.7 s | – |
| 512×256 | 78 km | 131k | 3.9 s | – |
| 1024×512 | 39 km | 524k | 19.8 s | 950 MB |

Cost grows faster than the cell count (7× cells, 11× time), polar rows are
point-adjacent to a whole row of cells, and the climate model iterates at the
full terrain resolution. Tuning that design buys percent, not orders of
magnitude. At 104 km per cell a strait, an island, a rift lake or a valley
glacier cannot exist in the canonical world at all; Atlas can only fake them.

The new engine changes three things at once:

1. **Each process runs at the scale its physics needs.** Plates are hundreds
   of kilometres wide, the atmosphere organizes over ~150 km, rivers and
   coastlines need the finest grid we can afford. Climate never runs on the
   terrain grid.
2. **Terrain emerges instead of being painted.** Tectonics provides rock
   uplift; rivers whose strength comes from climate carve it (stream-power
   law). Ridges, dendritic valleys, plateaus, concave river profiles and rift
   basins come out of the uplift–erosion balance. Coastlines are a contour of
   that eroded landscape after sea level rises over it, so rias, islands and
   straits are drowned hills and saddles, not noise thresholds.
3. **Every pass is linear or near-linear in the cell count**, on a grid with
   uniform cells and no poles, solved coarse-to-fine so the finest level only
   adds the detail coarser levels could not resolve.

Diverse worlds (frozen, hothouse, desert, archipelago, water world, pangea)
are then a matter of physical parameters — water inventory, insolation,
greenhouse, rotation, tectonic activity — not separate painters.

## Targets

| Level `n` (cells per cube edge) | Cell (Earth radius) | Cells | Target time | Target RSS |
| --- | --- | --- | --- | --- |
| 256 | 39 km | 393k | < 0.5 s | < 64 MB |
| 512 | 19.5 km | 1.57M | < 1.5 s | < 200 MB |
| 1024 | 9.8 km | 6.29M | < 6 s | < 700 MB |
| 2048 (stretch) | 4.9 km | 25.2M | < 30 s | < 2.5 GB |

`n = 1024` is 85× the cells of today's production grid. These are engine
targets; the product budgets in ADR 0004 change only in phase D, with the
measurements from this engine as the evidence.

## Phase A results (2026-11-08)

`crates/daena-worldgen`: cube-sphere grid, tectonics, coarse-to-fine
uplift–erosion with depression routing, sea level, previews and the
`worldgen` CLI. Same machine and seed 7 as the baseline, engine only (no
rendering):

| Level `n` | Cell | Time | Peak memory |
| --- | --- | --- | --- |
| 512 (default) | 19.5 km | 0.86 s | 204 MB |
| 1024 (high) | 9.8 km | 2.16 s | 787 MB |

Results are bit-identical for 1 and 12 threads. At `n = 512` the highest peak
is 7.1 km and median land is 350 m. Known gaps: `n = 1024` memory is above
the 700 MB target (per-root basin records and buffer growth), shelves and
coasts are tuned by one continental-crust margin constant, and rifts follow
plate boundaries that are still smoother than Earth's.

## Grid: equiangular cube sphere

- Six faces, `n × n` cells each. Cell id = `face * n * n + j * n + i`.
- A cell centre is `normalize(normal + tan(a)·u + tan(b)·v)` with
  `a = ((i + 0.5) / n − 0.5) · π/2` (same for `b` with `j`). Faces are
  `+X, −X, +Y, −Y, +Z, −Z`, each with a fixed right-handed `(u, v)` basis.
- Cell sizes vary by at most ~1.3× across a face. No poles, no singular rows.
- Eight neighbours in a fixed order in the cell's own face frame:
  `E(+i), NE, N(+j), NW, W(−i), SW, S(−j), SE`. Interior neighbours are index
  arithmetic. Border cells use a precomputed table built by extrapolating the
  face coordinates past the edge and projecting onto the face the point falls
  on. At the eight cube corners one diagonal does not exist and is `NONE`.
- All faces are congruent under rotation, so per-face tables (cell area,
  distance to each of the 8 neighbours for interior cells, local position)
  are computed once for one face and reused for all six.
- Exact cell area from the spherical-rectangle formula
  `Ω(x, y) = atan(x·y / sqrt(1 + x² + y²))` with `x = tan a`, `y = tan b`;
  areas sum to `4πR²`.
- Levels nest exactly: child `(i, j)` of level `2n` lies inside parent
  `(i/2, j/2)` of level `n` on the same face. Upsampling is bilinear in face
  coordinates (weights 0.75/0.25), downsampling is an area-weighted mean of
  the four children.
- Point sampling (`lat, lon → value`) picks the face by the largest
  component, then interpolates bilinearly in face coordinates.

## Pipeline

```text
settings + seed
  -> tectonics (plates, crust, boundaries, hotspots) on a tectonic level (n ≤ 256)
  -> climate pass 1 on the climate level (n = 64) from the tectonic base surface
  -> landscape evolution, coarse to fine: n = 32, 64, ..., N
       each level: upsample + add detail at this scale, then K iterations of
       flow routing -> depression routing -> drainage -> implicit stream power
       -> threshold hillslopes -> smoothing
       climate pass 2 at n = 256 (or the final level if smaller)
  -> sea level: solve for the water inventory, shift so sea level = 0
  -> climate pass 3 on the final surface, downscaled to N
  -> hydrology: oceans, lakes by water balance, rivers, glaciers, sea ice
  -> biomes (hazards deferred)
  -> outputs: cube-sphere fields, lat/lon sampling, previews
```

Phases A–C below build the engine; phase D integrates it.

### Tectonics (phase A)

Evaluated on a tectonic level `n_t = min(N, 256)` and sampled to finer levels,
which add detail noise above `n_t`'s resolution.

- **Plates.** `P` sites on the sphere with a weighted, domain-warped Voronoi
  partition (weights give a few large plates and many small ones; the warp
  gives irregular boundaries). Each plate has an Euler pole and angular speed,
  so velocity at a point is `ω × p`.
- **Continental crust is independent of plates**, as on Earth: a field made
  of continent blobs (count and sizes from the layout: pangea, few, many) with
  warped fBm, thresholded so its area matches the continental fraction implied
  by the target land fraction.
- **Boundaries.** Cells with a 4-neighbour on another plate. Relative velocity
  against the boundary normal gives convergence and shear; classification is
  convergent, divergent or transform, then by the crust on each side.
- **Distance fields** by multi-source Dijkstra on the tectonic level: to the
  nearest boundary (with that boundary's type, rates and the side the cell is
  on), to the nearest divergent oceanic boundary (sea-floor age), and to the
  continent edge (margins).
- **Base surface** (what the world looks like before erosion):
  - oceanic crust deepens with age, `−2600 m` at ridges to about `−5500 m`
    on old floor (`∝ sqrt(distance / spreading rate)`);
  - continental margins slope from the continent edge down to abyssal depth
    over 100–200 km, and the continental surface rises from ~0 m at its edge
    to a few hundred metres inland;
  - trenches (−7 to −10 km) on the subducting side of ocean–continent and
    ocean–ocean convergence; island arcs ~150 km behind ocean–ocean trenches;
  - hotspots with trails opposite to plate motion: seamount and island chains
    that sink with age on oceanic crust, broad domes on continents.
- **Rock uplift `U`** (m/yr) drives erosion:
  - collision orogens (continent–continent): wide, strong, with a plateau
    component;
  - Andean arcs (ocean under continent): a belt 150–250 km inland of the
    trench;
  - continental rifts: subsidence along the axis, uplifted shoulders, which
    makes rift basins and later rift lakes;
  - background epeirogenic uplift over continental crust with low-frequency
    noise, so interiors keep freeboard instead of eroding to sea level;
  - old orogen belts as low ridged-noise uplift, for worn ranges inside
    continents.
- **Rock erodibility** `K_rock` from lithology noise: resistant shields,
  soft basins.

### Landscape evolution (phase A)

Per level, on the cells of the erosion domain (base surface above the
erosion base level of 0 m). Everything else is a fixed base-level boundary
whose bathymetry comes from tectonics.

1. **Upsample** the parent elevation. Add roughness at this level's scale
   (amplitude proportional to cell size and local uplift) so new channels can
   form. New domain cells start just above base level.
2. **Receivers**: steepest descent among the 8 neighbours (slope by true
   distance). Ties break by the fixed direction order.
3. **Donors and stack**: CSR donor lists and a topological order from roots
   (boundary cells and pits) outward, built once per iteration. Each cell is
   labelled with its basin (its root).
4. **Depression routing** (Cordonnier, Bovy & Braun 2019): pit basins are
   joined to the ocean by a minimum spanning tree over basin-to-basin passes
   (pass height = `max(h_a, h_b)` over adjacent cells). Each open pit basin is
   re-routed by reversing the receiver path from its pass cell back to its pit
   ("carving"). Cells below the pass height are lake cells for this
   iteration; the lake surface is their receivers' base level. Only cells in
   pit basins are scanned, so the cost is proportional to the depression
   area. From phase B, a basin whose inflow cannot balance evaporation over
   its potential lake stays closed (its pit is a root), which keeps endorheic
   basins in arid worlds.
5. **Drainage**: effective area accumulated in reverse stack order,
   `A_eff = Σ cell_area × runoff_weight` (runoff weight is 1 in phase A and
   comes from climate in phase B).
6. **Implicit stream power** (Braun & Willett 2013, `n = 1`) in stack order:
   `h_i ← (h_i + U_i·dt + F·h_r) / (1 + F)`, `F = K_i·dt·A_eff^m / d_ir`.
   Unconditionally stable, so `dt` can be large and each level approaches
   steady state in tens of iterations. Lake cells only uplift.
7. **Threshold hillslopes**: in stack order, `h_i ≤ h_r + S_c·d_ir`.
8. **Smoothing**: a small explicit diffusion step for regularization.

Default schedule (tunable): 30 iterations at `n ≤ 64`, 20 at 128, 12 at 256,
6 at 512, 4 at 1024, 3 at 2048, with `dt` shrinking at finer levels.
Physical units throughout (metres, years, m²), so the same constants mean the
same thing at every level.

Sea level is solved after the last level from the target land fraction
(phase C: from a water inventory), then all elevations shift so sea level is
0 m. Sea rising over the eroded margins drowns valleys and lowlands: shelves,
rias, islands and straits come from the landscape itself. Ocean is water at or
below sea level connected to the open ocean; enclosed depressions below sea
level stay land until hydrology decides whether they hold a lake.

### Climate (phase B)

On a climate level `n_c = 64` (~150 km, 24.6k cells), from the elevation
downsampled to it, then downscaled to the terrain grid with a lapse rate and
windward/leeward corrections.

- **Insolation** by latitude and month from luminosity, orbital distance and
  axial tilt (daily-mean insolation formula with solar declination).
- **Temperature**: a seasonal energy-balance model (Budyko–Sellers outgoing
  radiation `A + B·T`, diffusive meridional transport on the cube-sphere
  graph), stepped over 12 months until periodic. Ocean has a mixed-layer heat
  capacity, land a small one, so continents get strong seasons. Albedo by
  surface (ocean, land, snow and ice below a threshold temperature), which
  gives ice–albedo feedback: cold presets can run away toward a snowball.
  Greenhouse strength shifts `A`. Lapse rate −6.5 K/km on downscaling.
- **Circulation**: Hadley cell edge from rotation rate (Held–Hou scaling,
  ~30° for Earth, wider for slow rotators), giving trade winds, westerlies and
  polar easterlies; the ITCZ follows the monthly thermal equator, which pulls
  monsoons over large continents.
- **Moisture**: semi-Lagrangian advection of vapour along monthly winds,
  evaporation from the ocean by Clausius–Clapeyron, recycling over land,
  condensation where vapour exceeds saturation (including orographic cooling
  on windward slopes; descending lee air dries, giving rain shadows),
  convection in the ITCZ and suppression under the subtropical highs.
- **Water balance**: potential evaporation from temperature, actual
  evaporation by the Budyko curve (Fu's equation), runoff = P − AET.
- **Ice**: monthly snow accumulation against positive-degree-day melt gives a
  surface mass balance; ice is routed downhill along the drainage tree like
  water, so glaciers extend below the equilibrium line and ice sheets cover
  areas of positive balance. Sea ice from monthly ocean temperature.
- **Coupling back to erosion**: runoff sets the drainage weight (arid land
  erodes slowly and keeps high rough plateaus); glaciated cells erode faster
  and may cut below sea level at the coast (fjords) and overdeepen basins
  (lake districts on shields).

### Hydrology, cryosphere and biomes (phase B)

- **Lakes by water balance**: a priority flood from the ocean finds every
  depression and its spill level. Runoff is accumulated over the filled
  surface; at each depression's outlet the inflow is compared with net
  evaporation over the lake. A full lake spills and its outflow continues; a
  partial lake settles at the level where inflow equals evaporation over its
  area (from the depression's hypsometry), with no outflow (saline). Tiny
  inflow gives a playa or salt flat. Rift and glacial basins become lakes in
  wet climates and salt pans in dry ones.
- **Rivers**: discharge per cell; river cells above a discharge threshold,
  traced from heads to mouths with identities by mouth.
- **Biomes**: from annual temperature, precipitation, aridity (P/PET) and
  seasonality: ice sheet, glacier, tundra, alpine, boreal forest, temperate
  forest, temperate rainforest, grassland, cold and hot desert, savanna,
  tropical seasonal and rain forest, wetland, mangrove, salt flat. Ocean
  classes: sea ice, reef, shelf, deep ocean, trench.

### Presets (phase C) and hazards (deferred)

Hazards are deferred (see Decisions). When they return, they are long-term
rate or potential fields (ADR 0004), not simulated events:

- earthquakes from boundary distance, type and relative speed;
- volcanism at arcs, rifts, ridges and hotspots;
- tsunami exposure: ocean-path distance from subduction trenches to low
  coasts;
- tropical cyclones: genesis where the warmest-month sea surface exceeds
  26.5 °C between ~5° and the Hadley edge, tracks carried by trade winds and
  recurving into the westerlies, decaying over land;
- tornadoes and severe convection: warm, moist, low-relief continental
  interiors in mid-latitudes with a strong meridional temperature contrast,
  stronger in the lee of ranges (dryline);
- floods (high discharge on low slopes), landslides (slope, rain, shaking),
  avalanches (slope and snow).

Presets set physical parameters only:

| Preset | Land | Climate | Tectonics |
| --- | --- | --- | --- |
| Earthlike | 29% | Earth forcing | ~14 plates |
| Frozen | 32% | weak star / low greenhouse, ice to mid-latitudes, sea ice | Earthlike |
| Hothouse (tropical) | 25% | strong greenhouse, no ice, wet, high sea | Earthlike |
| Desert | 60% | low water inventory, dry interiors, endorheic basins | Earthlike |
| Archipelago | 10% | warm, wet | many small plates, strong arcs and hotspots |
| Water world | 2% | warm | volcanic islands only |
| Pangea | 30% | one supercontinent, monsoons, interior desert | few large plates |
| Shattered | 35% | Earthlike | many plates, high activity: straits and islands |

Every preset is a starting point; each parameter stays editable.

### Integration (phase D, after review)

- A new physical source version that stores the cube-sphere elevation and
  causes natively (≈ 25 MB of `i32` elevation at `n = 1024`, within the
  128 MB source bound), with the old version rejected per ADR 0004.
- `daena-physical` generation calls the engine; climate, hydrology, hazards,
  Find Place, routing and landmass selection read engine fields through
  lat/lon sampling instead of the equirectangular grid.
- Later, once the canonical cells are fine enough: Atlas may run the same
  landscape-evolution solver on regional patches, with the global solution as
  boundary condition, instead of synthesizing ridges and valleys.
- ADR 0004 and `MAPS.md` budgets are updated with the measured envelopes.

## Determinism

- All randomness comes from `splitmix64`-derived hashes of the seed and a
  per-domain tag. No `HashMap` iteration order reaches any output; edges and
  sets that feed results are sorted.
- Parallel passes are per-cell maps. Floating-point reductions run in a fixed
  order (sequential, or fixed chunks combined in order), never through
  rayon's dynamic splitting. The same seed and settings produce bit-identical
  fields for any thread count.

## References

- J. Braun, S. Willett (2013), *A very efficient O(n), implicit and parallel
  method to solve the stream power equation governing fluvial incision and
  landscape evolution*, Geomorphology.
- G. Cordonnier, B. Bovy, J. Braun (2019), *A versatile, linear complexity
  algorithm for flow routing in topographies with depressions*, Earth Surface
  Dynamics.
- G. Cordonnier et al. (2016), *Large scale terrain generation from tectonic
  uplift and fluvial erosion*, Computer Graphics Forum.
- R. Barnes, C. Lehman, D. Mulla (2014), *Priority-flood: an optimal
  depression-filling and watershed-labeling algorithm*.
- R. Barnes, K. Callaghan, A. Wickert (2020), *Computing water flow through
  complex landscapes, part 3: fill–spill–merge*.
- G. North (1975), *Theory of energy-balance climate models*.
- I. Held, A. Hou (1980), *Nonlinear axially symmetric circulations in a
  nearly inviscid atmosphere*.
- B. Fu (1981), the Budyko-curve form for actual evaporation.
