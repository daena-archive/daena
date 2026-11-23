# World engine: emergent physical worlds at real-world scale

Status: phases A–E are built (terrain; climate, water and ice; presets;
hazards; integration, E1–E8). The engine is `crates/daena-worldgen`;
`crates/daena-physical` is its product facade, and the old `daena-physical`
generator was deleted in E8. Open realism work is under "Known realism gaps,
deferred to after E8".

Decisions (2026-11-08):

- New crate, integrated in phase E.
- Default resolution `n = 512` (~20 km cells); `n = 1024` (~10 km) is the
  high setting.
- Integration (phase E) is a hard cut: Maps is in beta, so worlds saved in
  the current source format are not kept readable.
- Hazards (tsunamis, tornadoes, cyclones and the rest) were deferred until
  terrain, climate and water were proven. They are back on the agenda as
  phase D (2026-11-09), before integration: the shipped product already
  derives earthquake, volcanic, storm and flood hazards, and integration must
  keep them.
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

Revised 2026-11-11 for the full pipeline (terrain, climate, water, ice,
biomes). Release build, Earthlike seed 7, engine only, default threads, on an
M4 Pro:

| Level `n` (cells per cube edge) | Cell (Earth radius) | Cells | Target time | Target RSS |
| --- | --- | --- | --- | --- |
| 256 | 39 km | 393k | < 1.0 s | < 100 MB |
| 512 | 19.5 km | 1.57M | < 1.6 s | < 300 MB |
| 1024 | 9.8 km | 6.29M | < 4.5 s | < 1.1 GB |

The first targets (0.5 s / 64 MB, 1.5 s / 200 MB, 6 s / 700 MB, and a
2048 stretch of 30 s / 2.5 GB) were set in phase A, before the engine had a
climate. Tectonics and the three coarse climate passes cost the same at every
`n`, about 0.6 s even after the planned cuts, so `n = 256` cannot reach
0.5 s. The 2048 level is no longer a target: at the planned memory per cell
it would need about 4.5 GB.

Stage budgets at `n = 512`, summing to about 1.45 s (10 % margin):

| Stage | 2026-11-11 | Budget |
| --- | --- | --- |
| Climate, three coarse passes | 1.00 s | 0.45 s |
| Erosion, all levels | 0.71 s | 0.60 s |
| Tectonics | 0.21 s | 0.15 s |
| Downscale, sea level, final routing, hydrology, biomes | 0.28 s | 0.25 s |

The memory targets assume about 180 bytes per cell at the peak, against about
290 today. They are provisional until a memory profile shows what is live at
the peak.

`n = 1024` is 85× the cells of today's production grid. These are engine
targets; the product budgets in ADR 0004 change only in phase E, with the
measurements from this engine as the evidence.

## Performance results (2026-11-13)

Release build, Earthlike seed 7, engine `timings_ms.total`, median of three
runs with the baseline, A and B binaries rotating order:

| `n` | `65423b0` | A | B | B RSS with `MallocLargeCache=0` |
| --- | --- | --- | --- | --- |
| 256 | 1,334 ms / 192 MB | 1,126 / 143 | 1,083 / 144 | 112 MB |
| 512 | 2,017 / 518 | 1,576 / 304 | 1,435 / 312 | 233 MB |
| 1024 | 4,668 / 1,843 | 3,058 / 996 | 2,745 / 1,061 | 711 MB |

A is Q1-Q7 from the performance review: a streamed downscale, parallel
donor and stack builds, parallel grid tables with a tangent table, final
routing and the lake flood beside the final coarse climate, parallel
tectonic distance fields, tectonic field preparation beside the erosion
climate passes, trimmed erosion buffers, and thread-scaled cancellation
chunks. B adds R1, a compact active set for erosion and final routing. R1 is
a speed gain, not a memory gain: A without the allocator cache is already at
717 MB at `n = 1024` and 233 MB at `n = 512`.

Peak RSS includes the macOS malloc cache, which keeps freed large blocks
resident. The live heap is the `MallocLargeCache=0` column. At `n = 1024` the
peak sits in the hazard stage, about 695 MB of live fields plus about 350 MB
of cached erosion and routing buffers; at `n = 512` the live peak is 233 MB
and the rest is cache. `malloc_zone_pressure_relief` does not release that
cache on macOS 27.2 (it returns zero), so the engine does not call it.
Windows retention was not measured in E8 either; the E8 budget table is macOS only.

`timings_ms.total` is engine time from after settings validation to the last
hazard field. The CLI's wall time adds process startup and the serial JSON
report, about 0.5 s at `n = 1024` and 0.13 s at `n = 512`, which the product
does not pay.

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

## Phase B results (2026-11-09)

Recorded when phase B landed. Phase C changed the climate; current numbers
are under phase C and "Remaining work".

Climate, water, ice and biomes, coupled back into erosion:

- **Climate** (`climate.rs`, `planet.rs`): seasonal energy balance on a 64-cell
  climate grid with lagged snow and sea-ice cover, meridional transport on
  32 equal-area bands, zonal mixing within bands, and maritime heat capacity
  from geography. No wind or upwind-temperature feedback in the temperature
  step: those loops were unstable. Winds are diagnosed after spin-up and drive
  moisture only (steering wind 1.6× the surface wind, poleward eddy transport
  in the westerlies, land recycling). Every pass spins up from the same warm
  start (15 °C, no ice), so a bistable cold planet does not inherit its branch
  from the coarse history.
- **Coupling**: climate passes at levels 64 and 256 set runoff-weighted
  erosion, close basins whose evaporation beats their inflow, and give
  glaciers stronger erosion, fjord base levels and overdeepened rock basins.
- **Water and ice** (`hydrology.rs`): priority-flood lakes settled by water
  balance (fresh, saline, salt flat), river discharge and traced river
  segments, glaciers and ice sheets routed along drainage.
- **Biomes** (`biome.rs`): 26 classes from temperature, aridity and
  seasonality.
- **Cross-face sampling** uses a slope limiter, so sharp contrasts next to a
  cube-face edge cannot overshoot (this was the seam in cold worlds).

Seed 7, default Earth-like settings: global mean 14.7 °C, equator 28 °C,
global precipitation about 1000 mm, 23 % of ice-free land is desert, peaks
7.8 km. Engine only: `n = 512` 2.5 s / 536 MB, `n = 1024` 5.4 s / 1.58 GB.
Identical output for 1 and 12 threads.

Known gaps: with this model a −15 °C offset already tips seed 7 into a
snowball (the published state is a two-year mean of a 12 K cycle), so a
"frozen" preset should use a milder offset; memory at `n = 1024` is well above
the phase A target; the dry-belt edges over the ocean are still sharper than
Earth's.

## Phase C results (2026-11-09)

Presets (`preset.rs`, CLI `--preset` and `--list-presets`; flags override a
preset in any order; the JSON report records the preset and the planet
settings). Calibrating them exposed four climate faults, fixed here:

- **Runaway ocean cells.** Meridional transport added one temperature step to
  every cell of a band, so each cell took energy in proportion to its heat
  capacity. In a polar band that is mostly land, the few ocean cells took the
  band's whole convergence and ran past 200 °C (rain then reached 6–196 m a
  year). Transport is now a flux per unit area, solved implicitly with each
  cell's local balance: one tridiagonal solve on the bands per month.
- **Hot polar land.** With honest transport, land under the polar summer sun
  reached 20 °C next to perennial sea ice. Zonal mixing went from 1 to
  3 W/m²/K (land and sea in one band now exchange heat about as strongly as
  they radiate).
- **Striped sea-ice edges.** The albedo jump is worth about 15 K to one cell,
  so with cover following each cell's own temperature every cell was bistable
  and ice edges came out as alternating frozen and open rows. Cover now follows
  the 3×3 neighbourhood temperature. Fronts then move until latitude pins them,
  so a 40-year spin-up can leave 1–2 K (Earth-like) to 5 K (Frozen) at edge
  cells; the published state is the usual two-year mean, and the tests check
  the area-weighted drift (< 0.1 K) instead of the single worst cell.
- **A rain floor.** Each month the moisture column restarted at half
  saturation and ran 48 six-hour sweeps; in the dry belt that start-up vapour
  was still raining out at the end, about 200 mm a year that no evaporation
  setting could remove (a rain supply of 0.15 still gave 370 mm on land).
  Vapour and soil water now carry from month to month after one discarded
  warm-up year, and each month publishes the mean of its sweeps. Without the
  floor, land above 60° was nearly rainless, because the storm-track eddy flux
  stopped at the Ferrel edge; it now rises to the middle of the Ferrel band
  and fades to zero at the pole.

### Phase C, second pass: transport, sea ice, land ice and island arcs

- **Moist transport.** Warm air carries more latent heat, so the atmosphere's
  diffusivity is scaled by `(1 + χ(T)) / (1 + χ(T_Earth))`, with
  `χ = (L/c_p)·RH·dq_s/dT` at each band interface and `T_Earth` the
  calibration climate's zonal profile. Earth is unchanged; warm planets
  flatten their pole-to-equator contrast and cold planets sharpen it.
  Hothouse went from a 36 K to a 27 K contrast (equator 40 → 37 °C, poles
  4 → 10 °C).
- **Ocean heat transport.** A second band exchange on ocean-mean
  temperature, scaled by the sea two bands share, deposits heat on ocean
  only, and is solved with the atmosphere by alternating implicit solves.
  The ocean takes about a fifth of the transport (diffusivity 0.12 against
  0.375 for the atmosphere, which together keep Earth's total).
- **Insulating sea ice.** Ice hides 70 % of the ocean's extra heat capacity
  from the air, so winters over the pack are cold and the ice melts back in
  summer. Earthlike's sea ice went from 53 M km² perennial and 2 M km²
  seasonal to 12 M km² and 20 M km² (Earth: a few and about 25–30).
  Spin-up now converges.
- **Land-ice albedo.** Land whose warmest month stays below about −2 °C at
  its height (regional, over the last twelve months) becomes ice sheet with
  albedo 0.5. A stronger version (0.62, from +2 °C) never settled: ice
  sheets kept advancing for decades.
- **Island arcs and hotspot islands.** Ocean–ocean arcs carry a lower
  platform plus discrete volcanoes on a jittered lattice (about 140 km apart,
  55 km radius, a third of sites empty); sites are kept only in a thin shell
  around the sphere, which avoids seams at lattice planes. Oceanic hotspots
  rise 6.8 km. Archipelago at `n = 128` has 132–133 small islands against
  82–94 before; most arcs show as chains of shallow banks with a few islands.

Seed 7, `n = 512` (global, equator and polar °C; rain mm; desert share of
ice-free land; ice in million km²):

| Preset | T | T eq | T pole | P | P land | Desert | Ice sheet | Perennial / seasonal sea ice |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Earthlike | 15.0 | 28.5 | −10.7 | 1058 | 615 | 29% | 1.1 | 12 / 20 |
| Frozen | 3.3 | 22.4 | −30.8 | 571 | 327 | 35% | 12.4 | 98 / 2 |
| Hothouse | 27.1 | 37.2 | 9.8 | 2023 | 1284 | 14% | 0 | 0 / 0 |
| Desert | 13.1 | 26.0 | −13.3 | 487 | 269 | 61% | 0 | 0 / 3 |
| Archipelago | 22.9 | 33.0 | 2.2 | 1593 | 1260 | 17% | 0 | 0 / 1 |
| Water world | 23.7 | 33.2 | 5.3 | 1619 | 2017 | 17% | 0 | 0 / 0 |
| Pangea | 15.1 | 28.2 | −10.0 | 1040 | 510 | 42% | 0 | 5 / 10 |
| Shattered | 14.9 | 28.8 | −15.6 | 1113 | 660 | 26% | 0.7 | 0 / 3 |

Eight other Earthlike seeds at `n = 256`: 14.9–16.6 °C global (about 1 K
warmer than Earth), poles −9 to −12.5 °C, 6–17 % desert, 670–990 mm of land
rain, 0–27 M km² perennial and 6–23 M km² seasonal sea ice. Every run is
3.2–4.1 s on the development build; output is identical for 1 and 12 threads.

Open gaps are listed under "Remaining work".

### Spin-up and sea-ice creep (2026-11-11)

The climate passes no longer run to an attractor. Some worlds' sea ice keeps
spreading for more than a century: Earthlike seed 5 grows from about 30 M km²
of annual-mean sea ice at year 40 to 50 at year 200 (Earth is about 22), and
Frozen and some Earthlike seeds behave the same. The cause is the cover
feedback. A cell is restored locally by `B + gamma` (about 5 W/m²/K), while a
full ice cover raises albedo by about 0.34, worth about 11 K at polar
insolation over the 6.2 K cover ramp. The 3×3 cover target is therefore
bistable and the feedback has no local heat diffusion to stop it. Land ice is
not the driver: turning its albedo off changes the 200-year temperature by at
most 0.8 K, and sea ice still grows.

Five cover rules were tested: ramp ends −8, −15 and −20 °C without smoothing,
and −8 and −15 °C with albedo from the 3×3 mean of cover. The wider ramps
settle but leave Earthlike at 2.2–4.5 M km² of annual sea ice and little or no
perennial ice; −20 °C warms the Earthlike median by about 1.3 K. Smoothing
the −8 °C rule (R3) keeps Earthlike at 19.9 M km² (20.7 before) and roughly
halves the tail, but does not remove it. R3 is now the cover rule.

The published state is defined by the schedule, not the attractor: the
annual-mean equilibrium start, the published 20-year pass (erosion 10 years,
12-month moisture warm-up), and the RMS/max stop rule above. A saved world
rebuilds the same climate from its surface and settings. The remaining slow
sea-ice drift on some worlds is a known limitation; the proper fix is local
heat diffusion between cells, which is a larger physics change and costs more
per month than this step.

## Remaining work (2026-11-09)

Phases A–C are built. Order to finish the engine, decided 2026-11-11:
phase D parity hazards, then phase E integration, then the new hazards. The
items below are open but do not block that order.

1. **Speed and memory targets** (see "Targets"). Release build, Earthlike
   seed 7, engine only, on 2026-11-11, after the climate speedup (median of
   three runs):

   | Level | Time | Target | Peak memory | Target |
   | --- | --- | --- | --- | --- |
   | `n = 256` | 1.23 s | 1.0 s | 175 MB | 100 MB |
   | `n = 512` | 1.90 s | 1.6 s | 512 MB | 300 MB |
   | `n = 1024` | 4.38 s | 4.5 s | 1.80 GB | 1.1 GB |

   Speed work with identical output stopped here: each further step saved
   tens of milliseconds for hours of work. The targets stay as goals, not
   gates for phases D and E. At `n = 512` the three climate passes take
   0.64 s, erosion 0.71 s and tectonics 0.21 s. Every climate pass stops at
   its year cap; the erosion passes end 10 years still changing by about
   0.1 K RMS a year. The levers left, each a real change to output or
   memory layout, in order of expected gain:
   1. a shorter climate schedule for the erosion passes, checked against the
      climate and terrain bands (an estimated 0.3 s);
   2. a memory profile of the peak, then cuts; phase A left the `n = 1024`
      basin records and buffer growth, and phase B added the full-resolution
      climate and hydrology fields;
   3. tectonics and the fine-grid stages;
   4. fewer erosion iterations on the small levels.
2. **Sea level from a water inventory** (planned for phase C). Sea level is
   still solved from a target land fraction, so presets set land share
   directly instead of it following from how much water the planet has.
3. **Open physics gaps**:
   - rifts follow plate boundaries that are smoother than Earth's and show as
     long thin troughs (phase A);
   - continental shelves are tuned by one crust-margin constant (phase A);
   - continental crust is placed independently of plates, so on one seed
     Archipelago and Water world share their continents, and island arcs
     stay modest at low land fractions (phase A);
   - dry-belt edges over the ocean are sharper than Earth's (phase B);
   - sea ice has no thickness or latent heat, so near the pole it melts back
     as readily as at its edge (phase C);
   - cold worlds keep up to about 4 K of drift at ice fronts after the
     40-year spin-up (Frozen, and two of nine Earthlike seeds); the
     published state is the two-year mean (phase C);
   - Earthlike runs about 1 K warmer than Earth (14.9–16.6 °C), and the
     Hothouse equator averages about 37 °C (phase C).

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
  -> sea level: from the target land fraction (a water inventory is still to
       come), shift so sea level = 0
  -> climate pass 3 on the final surface, downscaled to N
  -> hydrology: oceans, lakes by water balance, rivers, glaciers, sea ice
  -> biomes
  -> hazards (phase D): long-term rate and potential fields
  -> outputs: cube-sphere fields, lat/lon sampling, previews
```

Phases A–D below build the engine; phase E integrates it.

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
    component (a broad 750 km crustal-thickening term and a 720 km base
    plateau behind the suture);
  - Andean arcs (ocean under continent): a belt 150–250 km inland of the
    trench, plus a 450 km plateau term on the overriding continent;
  - continental rifts: subsidence along the axis, uplifted shoulders, which
    makes rift basins and later rift lakes. Along-strike noise breaks the
    subsidence into en-echelon basins about 100–300 km long, and the gaps
    carry a sill uplift so neighbouring basins do not merge into one long
    trough. Oceanic rifts stay continuous and may remain marine;
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

Sea level is solved after the last level from the target land fraction (a
water inventory is planned; see "Remaining work"), then all elevations shift
so sea level is 0 m. Sea rising over the eroded margins drowns valleys and lowlands: shelves,
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
  radiation `A + B·T`, diffusive meridional transport on equal-area bands,
  zonal mixing within a band). Each pass starts from an annual-mean
  energy-balance equilibrium, then steps 12 months per year on a fixed
  schedule: the published pass runs at most 20 years (erosion 10), with a
  12-month moisture warm-up in both, and stops when the area-weighted RMS
  same-month change is below 0.005 K (0.01 for erosion) and the largest
  change below 0.5 K. `converged` still means the largest change below
  0.02 K; a run that stops with flickering cells publishes the two-year
  mean. Transport reaches each cell as the same flux
  per unit area, solved implicitly with the local balance. The atmosphere's
  diffusivity scales with moisture (Clausius–Clapeyron); a second, ocean
  channel moves heat between bands only as far as their shared sea reaches
  and deposits it on ocean. Sea ice insulates the ocean below it. Ocean has a
  mixed-layer heat capacity, land a small one, so continents get strong
  seasons. Albedo by surface (ocean, land, snow and ice below a threshold
  temperature; cover follows the 3×3 neighbourhood temperature, sea-ice
  albedo uses the 3×3 mean of cover, and land whose warmest month stays below
  freezing becomes ice sheet), which gives ice–albedo feedback: cold presets
  can run away toward a snowball. Greenhouse strength shifts `A`. Lapse rate
  −6.5 K/km on downscaling.
- **Circulation**: Hadley cell edge from rotation rate (Held–Hou scaling,
  ~30° for Earth, wider for slow rotators), giving trade winds, westerlies and
  polar easterlies; the ITCZ follows the monthly thermal equator, which pulls
  monsoons over large continents.
- **Moisture**: semi-Lagrangian advection of vapour along monthly winds,
  evaporation from the ocean by Clausius–Clapeyron, recycling over land,
  condensation where vapour exceeds saturation (including orographic cooling
  on windward slopes; descending lee air dries, giving rain shadows),
  convection in the ITCZ and suppression under the subtropical highs.
  Vapour and soil water carry from month to month, and storm-track eddies
  carry vapour poleward from the Hadley edge to the pole.
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
  wet climates and salt pans in dry ones. On the final erosion level, every
  overflowing basin gets a visible outlet unless at least 20 % of its cells
  subsides or at least 10 % were glaciated on that level: the reversed
  depression path is cut through the rim to just below the basin floor, so the
  basin drains and the channel is part of the canonical surface. Actively
  subsiding rift basins are left alone, and arid basins keep their saline lake
  or salt flat. There is no lake count or area cap.
- **Rivers**: discharge per cell; river cells above a discharge threshold,
  traced from heads to mouths with identities by mouth.
- **Biomes**: from annual temperature, precipitation, aridity (P/PET) and
  seasonality: ice sheet, glacier, tundra, alpine, boreal forest, temperate
  forest, temperate rainforest, grassland, cold and hot desert, savanna,
  tropical seasonal and rain forest, wetland, mangrove, salt flat. Ocean
  classes: sea ice, reef, shelf, deep ocean, trench.

### Presets (phase C)

Presets set physical parameters only (`Preset::settings`, CLI `--preset`):

| Preset | Land | Greenhouse | Rain supply | Plates | Layout | Activity |
| --- | --- | --- | --- | --- | --- | --- |
| Earthlike | 29% | 0 | 1.0 | 14 | mixed | 1.0 |
| Frozen | 32% | −8 W/m² | 1.0 | 14 | mixed | 1.0 |
| Hothouse | 25% | +20 W/m² | 1.0 | 14 | mixed | 1.0 |
| Desert | 60% | +6 W/m² | 0.25 | 14 | few | 1.0 |
| Archipelago | 10% | +8 W/m² | 1.0 | 30 | many | 1.5 |
| Water world | 2% | +8 W/m² | 1.0 | 20 | many | 1.2 |
| Pangea | 30% | 0 | 1.0 | 8 | pangea | 1.0 |
| Shattered | 35% | 0 | 1.0 | 40 | many | 1.8 |

Climate presets use the greenhouse term, not the artistic temperature offset,
so that offset stays free for the user. Every preset is a starting point; each
parameter stays editable, and CLI flags override the preset in any order.

### Hazards (phase D)

Built as `src/hazards.rs` and attached to `World.hazards`; `generate` builds it
last, after biomes, under `Stage::Hazards` and the `hazards` timing key. Every
field is a long-term rate or potential derived from the engine's own causes,
not a simulated event. Hazards read the finished world and never feed back.

**Fields.** Final grid unless noted:

- `quake_rate_km2` (`f32`): M ≥ 5 per year per km².
- `quake_hazard`, `volcanic_hazard` (`u8` potentials).
- `volcanic_sources`: stable ids, position, final cell, origin, activity and
  eruptions per year.
- `storms` on the coarse climate grid: genesis and track density per km² and
  potential intensity.
- `drought`, `heat_wave`, `extreme_rain`, `flood` (`u8` potentials).
- `stats`: planet totals, source counts, area means and `hazard_hash`
  (FNV-1a over every field in declaration order, sources in id order).

**Earthquakes.** Each tectonic `BoundaryRecord` splats a line-source Gaussian
on the tectonic grid: `λ · ℓ · exp(−(d/w)²) / (π w²)` per km² inside `3 w`.
`λ` is the kind weight (convergent 1, transform 0.45, divergent 0.12) times
`hypot(convergence, shear) / 50 mm/yr`, clamped to `[0.2, 2.0]` of the reference
speed; `ℓ` is the tectonic cell edge and `d` the true great-circle distance.
`w` is 150 km for ocean–continent and ocean–ocean convergence, 400 km for
continent–continent collision, 60 km for transform and 40 km for divergent.
Both sides of an interface carry records (verified reciprocal for every record),
so each side carries half the rate and the belt integral stays `λ`. The
tectonic density is sampled bilinearly at each final cell, an intraplate floor
of 2 % of the boundary rate mass is added, and hazard is
`1 − exp(−density / D_sat)`. `D_sat` is set so a 50 mm/yr convergent centre
reads 0.9. `A` is calibrated once on Earthlike seed 7, `n = 512`:
0.013880390 per km per year. The M ≥ 5 total is then 1 500 per year; the
convolution is resolution-independent and has no nearest-cell wedges.

**Volcanoes.** A source is a volcanic centre that stands for several volcanoes:
arc sources are the occupied jittered-lattice sites
(`SeedDomain::ArcVolcano`, 140 km spacing) whose `d_arc` bump is at least 0.3.
Activity uses `u = hash_u01(id)` and `s = clamp(convergence/50 mm/yr, 0.4, 1.5)`:
vigorous below `0.06 s`, moderate below `0.36 s`, otherwise dormant. Hotspot
trail indices 0, 1 and 2 are vigorous, moderate and dormant; older points are
extinct and not sources. Rift sites use a separate 140 km lattice
(`SeedDomain::RiftVolcano`); a site is a source when its nearest boundary is
divergent within 50 km. Continental rifts are moderate with probability 0.3 and
otherwise dormant; ocean ridges are dormant. Rates are 1.5, 0.15 and 0.009
eruptions per year. Hazard is the maximum over sources of
`weight · exp(−(d/30 km)²)` cut off at 90 km, with weights 1.0, 0.6 and 0.25,
splatted by BFS on true great-circle distance in id order. Ids come from lattice
coordinates or hotspot indices with a per-origin tag, so they do not depend on
`n`.

**Storms.** Coarse-cell ocean fraction is a block aggregation of the final ocean
mask. For month `m`,
`G_m = ocean_frac · smoothstep(26, 28.5, SST_m) · L(lat)`, where
`L = smoothstep(4°, 8°, |lat|) · (1 − smoothstep(φ_h − 5°, φ_h + 5°, |lat|))`.
`genesis_km2 = C · Σ_m G_m`, with `C` calibrated once on Earthlike seed 7 so
the planet has about 85 storms per year. Each cell and month with `G_m > 0`
traces one track: 250 km steps, at most 20, steered by the zonal part of the
surface wind, `(wind · east) · east`, plus beta drift 2.5 m/s poleward and
1 m/s westward. Intensity starts at `smoothstep(26, 32, SST)`, relaxes 30 % per
ocean step toward local potential intensity and decays by 0.6 per land step.
The track stops below 0.1 on land; over cool water the potential-intensity
target reaches zero and would otherwise end every track before recurvature, so
ocean tracks keep a tracked circulation. A track raises `track_km2` once per
visited cell and `intensity` to its carried value.

**Extremes and floods.** Drought uses
`0.75 · (1 − smoothstep(0.1, 0.65, P/PET)) +
0.25 · (1 − smoothstep(10, 60, p_dry)) · (1 − smoothstep(0.65, 1.5, P/PET))`,
zero on ocean and where the warm month is below 0 °C. Humid land
(`P/PET ≥ 1`) stays at or below 0.2; hyper-arid land with a dry season reads
1. Heat is
`0.7 · smoothstep(22, 38, t_warm) + 0.3 · smoothstep(4, 25, t_warm − t_cold)`,
times 0.25 over ocean. Extreme rain first computes the coarse wettest-month to
annual precipitation ratio, samples it with the same chart stencil `downscale`
uses, multiplies by `p_annual`, then applies
`smoothstep(80, 450, peak) · (0.7 + 0.3 · smoothstep(0.15, 0.35, peak/p_annual))`.
Floods walk `Hydrology.pop_order`: a cell at or above the river discharge
threshold is its own river, otherwise it inherits its receiver's river. HAND is
`elevation_m[cell] − elevation_m[river]`; a cell below that drainage is a
depression floor and is not a river floodplain, so its flood is 0.
`flood = (1 − smoothstep(0, 12 m, HAND)) · smoothstep(1, 4, log10 Q_river) ·
(0.6 + 0.4 · extreme_rain)`, zero on ocean and under lakes.

**Calibration, Earthlike seed 7 at `n = 512`:**

| Quantity | Result | Target |
| --- | ---: | --- |
| M ≥ 5 earthquakes per year | 1 500 | 1 000–2 500 |
| Quake rate within 300 km of a convergent boundary | 68.2 % | ≥ 60 % |
| Eruptions per year | 34.9 | 30–100 |
| Arc share of eruptions | 73.3 % | ≥ 50 % |
| Non-dormant volcanic centres | 95 | 60–300 |
| Tropical storms per year | 84.1 | 60–110 |
| Land-mean drought potential | 0.584 | 0.40–0.65 |
| Track-weighted mean latitude poleward of genesis | 8.3° | ≥ 4° |
| Track passages poleward of `φ_h − 5°` | 17.8 % | ≥ 10 % |

All 8 presets at seed 7, `n = 256`:

| Preset | Quakes/yr | Near conv. | Eruptions/yr | Arc share | Non-dormant | Storms/yr |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| earthlike | 1 500.0 | 68.3 % | 34.9 | 73.3 % | 95 | 85.0 |
| frozen | 1 499.9 | 67.9 % | 34.0 | 70.5 % | 98 | 0.0 |
| hothouse | 1 500.1 | 69.2 % | 33.5 | 74.3 % | 94 | 220.0 |
| desert | 1 499.2 | 62.3 % | 20.1 | 30.4 % | 83 | 68.7 |
| archipelago | 2 732.3 | 73.4 % | 80.3 | 83.3 % | 191 | 222.0 |
| water-world | 2 140.2 | 76.0 % | 65.9 | 86.3 % | 146 | 232.8 |
| pangea | 1 190.9 | 72.4 % | 19.0 | 53.7 % | 66 | 86.3 |
| shattered | 3 612.9 | 72.0 % | 70.6 | 73.9 % | 200 | 90.1 |

**Still to build (phase D2).**

The hazards of the original brief that the product does not have yet:

- tsunami exposure: ocean-path distance from subduction trenches to low
  coasts;
- tornadoes and severe convection: warm, moist, low-relief continental
  interiors in mid-latitudes with a strong meridional temperature contrast,
  stronger in the lee of ranges (dryline);
- landslides (slope, rain, shaking) and avalanches (slope and snow).

Each field is per cell on the cube sphere, bounded, versioned and
deterministic like the rest of the engine, and costs a few percent of the
generation time at most. Tests check the physical ordering each field must
keep (nearer a fast convergent boundary is more seismic; a warmer sea spawns
more storms), as the current hazard tests do.

**Known gaps.**

- No vertical wind shear: storm genesis uses only SST, ocean fraction and
  latitude.
- The storm intensity cutoff applies on land. Over cool ocean the prescribed
  potential-intensity target is zero, so applying the cutoff there would stop
  every track near genesis and leave no recurvature.
- Volcanic activity is a static class from the hash; it does not depend on the
  site's other causes beyond arc convergence.
- Drought and extreme rain inherit the climate's hard-edged dry-belt and ITCZ
  edges. The rectangular blocks in `extreme-rain.png` are the same edges as in
  the annual precipitation layer; they are not a hazard bug. Storm tracks
  inherit the same edges in the surface wind: on Earthlike seed 7 they pile
  into one bright recurving streak near 50–85°E, where the zonal wind changes
  sign across that edge.
- Earthlike is drier than Earth: a third of its land has `P/PET` below 0.2,
  against about a fifth on Earth, so its land-mean drought (0.58) sits above
  an Earth estimate.
- Floods use HAND above the inherited river cell. Lake overflow events are
  still phase E.
- The hazard fields carry no derivation version yet. Phase E adds one with
  the new physical source version.
- Events are not materialized here. Tsunami, tornado, landslide and avalanche
  fields from the original brief remain future work.

### Integration design (phase E)

This section replaces the earlier integration sketch. It is the E1 design;
E2–E8 implement it. It records the fourteen decisions below without reopening
them, and marks what was read in code, what is inferred, and what is an
estimate.

Verification for this section: the engine `World`, `generate_inner`,
`WorldSettings`, `Schedule`, `ErosionParams`, `Stage`, `Progress`,
`evolution.rs`, `sealevel.rs`, `planet.rs`, `preset.rs`, the public climate
types and `coarse_with`/`downscale`/`surface_water`, `hydrology.rs`, the
hazard types and storm tracer, and the public biome ids were read. On the
product side, `daena-physical/src/lib.rs`, `planetary.rs`, `history.rs`,
`events.rs`, `find_place.rs` (query and filters), `routing.rs` (query and
cost inputs), `hydro_claim.rs`, `landmass_claim.rs`, `region.rs`,
`derived_cache.rs`, the source codec, `physical_commands.rs`,
`physical_jobs.rs`, `daena-core/src/maps/physical.rs`,
`crates/daena-atlas/src/lib.rs` (inputs, not internals), and the frontend
physical/Atlas types and controls were read. Atlas style internals and every
Svelte view were not read line by line; exact presentation details that
depend on them are unverified. Every byte count and timing in this section is
an estimate unless it is a source-size calculation.

#### Decisions

1. **Hard cut.** Maps is beta. `DAENAPW1` sources are rejected with
   `physical.source.unsupported-version`; no source is migrated or
   reinterpreted. Events already materialized in projects are authored history
   and are never regenerated or rewritten.
2. **One product-facing crate.** `daena-physical` is the only crate the
   product calls, and the only crate that depends on `daena-worldgen`.
   `daena-worldgen` stays serde-free. Conversions to product types live in
   `daena-physical`. Generator code that nothing calls is deleted in E8.
3. **Source `DAENAPW2`.** It stores the resolved settings that reproduce the
   causes (preset tag, planet, seed, `n`, every `WorldSettings` field), the
   engine version, the final elevation per cube cell as `i32` millimetres
   relative to sea level, a hash of the regenerated tectonics, and the
   generation hashes. Tectonics is regenerated on load (about 0.21 s at the
   tectonic level); a hash mismatch is `physical.source.invalid`. Decode
   followed by derive must reproduce the climate, hydrology, biome and hazard
   generation hashes exactly. Any change to engine output bumps
   `WORLD_ENGINE_VERSION`, and older sources then fail as unsupported. That
   is the ADR 0004 contract; while Maps is in beta it is accepted that an
   engine fix retires saved worlds.
4. **Canonical elevation.** Generation rounds every elevation to whole
   millimetres immediately after the sea-level threshold is applied and before
   the ocean flood, routing and climate. The stored `i32` therefore maps back
   to the exact `f32` bits used downstream. This is an approved output change
   and sets a new baseline; engine golden tests are regenerated in E2.
5. **Derive from the surface.** The engine gains one path that rebuilds every
   post-erosion product from settings, tectonics and stored elevation: ocean
   mask, final routing, final climate pass, downscale, hydrology, biomes and
   hazards. Generation calls the same downstream path after erosion, so the
   two cannot drift. The final climate pass was confirmed to depend only on
   the grid, elevation, ocean mask, planet and options; it does not read the
   erosion domain. The ocean flood needs only `domain = base_m > 0` on the
   final grid, which is rebuilt from settings, tectonics and the same detail
   noise without running erosion. The ocean flood is then a function of
   elevation and that domain.
6. **Resolution.** `n = 512` is the product default and `n = 1024` the high
   setting. The engine still accepts any power-of-two `n` in `32..=2048` for
   tests and the CLI; the product accepts only 512 and 1024 at acceptance.
7. **No fabricated fields.** Every product field is either computed from
   engine data or removed with all of its consumers and UI. The removals are
   listed in "Removal list". The product must never fill a missing field with
   a zero or a legacy placeholder.
8. **Raster access.** Raster consumers (previews, coastlines, contours,
   GeoJSON, Atlas input) read an explicit equirectangular resample of the
   engine fields. Point consumers (Find Place candidates, routes, claims,
   event placement, Atlas inspection, landmass selection) sample or address
   canonical cube cells by latitude/longitude. The consumer table marks each
   one.
9. **Epochs** keep the product's forcing math in `history.rs`. The reference
   water inventory is the ocean volume below the reference sea level,
   computed on the cube grid with exact cell areas. An epoch's sea level is
   the hypsometric solve for that inventory minus land ice plus thermal
   expansion. The engine climate reruns on the epoch surface with the epoch
   temperature offset; hydrology, biomes and hazards are then rederived.
   Target: at most 1.5 s per epoch at `n = 512`. The epoch cache key gains the
   engine and source versions. The earlier branch's water-inventory sea level
   does not exist here and is not used.
10. **Hazards and events** use the phase D1 fields. Unit conversions are
    explicit tested functions. The engine quake rate is M ≥ 5 per km² per
    year; the product samples M ≥ 4 with Gutenberg–Richter `b = 0.9`, so
    `rate(M ≥ 4) = rate(M ≥ 5) · 10^0.9` per km², multiplied by the canonical
    cell area. Volcanic source ids are the engine's `u64` ids; the product
    type is widened rather than truncated. The inline storm tracer becomes one
    reusable function from `(genesis cell, month)` to a track, with identical
    output in E2. Floods read each lake's spill cell from the engine. Bump
    `HAZARD_DERIVATION_VERSION`, `VOLCANIC_SOURCE_DERIVATION_VERSION` and the
    event materialization versions whose meaning changes.
11. **Atlas** consumes an equirectangular view built from the engine. Its
    coarse-to-detail transform, styles and tile cache are not reworked. Golden
    tests that pin old-world values are regenerated, each with a written
    reason. Removing a field that Atlas styles used (humidity, currents) is a
    removal, not a new Atlas algorithm.
12. **Budgets.** `GENERATION_TIME_BUDGET_MS`, `WORKING_MEMORY_BUDGET_BYTES`,
    `MAX_SOURCE_BYTES`, the production resolution constants and the GeoJSON
    limits are set from measured envelopes in E8. ADR 0004's decision history
    and `MAPS.md` change in the same commit. The current 128 MB working-memory
    budget is below the engine's measured `n = 512` peak and must not be
    carried forward unchanged.
13. **Progress and cancellation.** The engine `Stage` maps to
    `ProgressPhase` as listed below. `Progress::cancelled` is checked inside
    the climate spin-up year loop, moisture sweeps, erosion iterations,
    hydrology flood/settle loops and hazard loops, so the command path meets
    `CANCELLATION_LATENCY_BUDGET_MS` (100 ms), with a test.
14. **Storage** follows `STORAGE.md`. The accepted source is in the portable
    checkpoint; derived caches under `.daena/` are disposable; a project
    rebuilds after `.daena/` is deleted. Storage work is verified by canonical
    round trip, checkpoint import, and recovery after deleting `.daena/`.

Additions the code showed:

- The source needs `reference_water_inventory_m3` and a public engine
  inventory solver. Epochs cannot use the product's old equirectangular
  `ocean_volume_curve`.
- The engine needs public generation hashes (tectonics and hazards included),
  a public engine-version constant, and a public tectonic builder.
- The climate summary needs solstice temperature and solstice precipitation
  fields; the product publishes both and the engine currently only publishes
  the annual, warmest and coldest values.
- `Lake` needs `spill_cell` and `minimum_cell`; flood materialization and
  claims use them and cannot reconstruct them from the public fields today.
- `StormClimatology` needs the monthly genesis weights so an event can choose
  a genesis month and call the shared tracer.
- Product claims must key canonical cube cells, not equirect view cells, or
  the same feature changes id when a raster view size changes.

#### Source format `DAENAPW2`

One little-endian record. There is no padding. `f64` values are stored as
their raw IEEE-754 bits (`u64` little-endian), so decoding is bit-exact; the
engine remains serde-free and the codec is manual. `u16`, `u32`, `u64` and
`i32` are little-endian.

Fixed header and settings, with offsets at the default six-row schedule:

| Offset | Field | Type | Encoding |
| ---: | --- | --- | --- |
| 0 | `magic` | `[u8; 8]` | `DAENAPW2` |
| 8 | `format_version` | `u16` | `2` |
| 10 | `header_bytes` | `u16` | `348` at six schedule rows; decoder must read it |
| 12 | `engine_version` | `u32` | `WORLD_ENGINE_VERSION` |
| 16 | `preset` | `u8` | `0` custom, `1` earthlike, `2` frozen, `3` hothouse, `4` desert, `5` archipelago, `6` water-world, `7` pangea, `8` shattered |
| 17 | `continent_layout` | `u8` | `0` mixed, `1` pangea, `2` few, `3` many |
| 18 | `flags` | `u16` | `0`; reserved, must be zero |
| 20 | `resolution` | `u32` | `WorldSettings::resolution`; product accepts 512 or 1024 |
| 24 | `cell_count` | `u32` | must equal `6 · resolution²` |
| 28 | `seed` | `u64` | `WorldSettings::seed` |
| 36 | `radius_m` | `f64` | `WorldSettings::radius_m` raw bits |
| 44 | `land_fraction` | `f64` | `WorldSettings::land_fraction` raw bits |
| 52 | `plate_count` | `u32` | `WorldSettings::plate_count` |
| 56 | `tectonic_activity` | `f64` | `WorldSettings::tectonic_activity` raw bits |
| 64 | `schedule_count` | `u32` | number of `ScheduleStep` rows, normally 6 |
| 68 | `schedule[]` | rows | each row: `max_n: u32`, `iterations: u32`, `dt_yr: f64` (16 bytes) |
| 68 + 16·s | `erosion.m` | `f64` | raw bits |
| 68 + 16·s + 8 | `erosion.k` | `f64` | raw bits |
| 68 + 16·s + 16 | `erosion.s_c` | `f64` | raw bits |
| 68 + 16·s + 24 | `erosion.kappa` | `f64` | raw bits |
| 68 + 16·s + 32 | `erosion.max_elevation_m` | `f64` | raw bits |
| 68 + 16·s + 40 | `planet.star_luminosity` | `f64` | raw bits |
| 68 + 16·s + 48 | `planet.star_mass` | `f64` | raw bits |
| 68 + 16·s + 56 | `planet.orbit_au` | `f64` | raw bits |
| 68 + 16·s + 64 | `planet.eccentricity` | `f64` | raw bits |
| 68 + 16·s + 72 | `planet.perihelion_deg` | `f64` | raw bits |
| 68 + 16·s + 80 | `planet.axial_tilt_deg` | `f64` | raw bits |
| 68 + 16·s + 88 | `planet.rotation_hours` | `f64` | raw bits |
| 68 + 16·s + 96 | `planet.greenhouse_wm2` | `f64` | raw bits |
| 68 + 16·s + 104 | `planet.temperature_offset_c` | `f64` | raw bits; the authored offset, not an epoch offset |
| 68 + 16·s + 112 | `planet.precipitation_scale` | `f64` | raw bits |
| 68 + 16·s + 120 | `river_min_m3s` | `f64` | raw bits |
| 68 + 16·s + 128 | `reference_water_inventory_m3` | `u64` | ocean volume below reference sea level, rounded |
| 68 + 16·s + 136 | `elevation_hash` | `u64` | FNV-1a over every `i32` elevation in cell order |
| 68 + 16·s + 144 | `tectonics_hash` | `u64` | FNV-1a over the regenerated tectonic fields |
| 68 + 16·s + 152 | `climate_hash` | `u64` | `ClimateSummary::hash_bits` |
| 68 + 16·s + 160 | `hydrology_hash` | `u64` | `Hydrology::hash_bits` |
| 68 + 16·s + 168 | `biome_hash` | `u64` | FNV-1a over biome ids |
| 68 + 16·s + 176 | `hazard_hash` | `u64` | FNV-1a over hazard fields and sources |
| `header_bytes` | `elevation[]` | `i32 × cell_count` | metres × 1000, relative to sea level, cell id order |

For `s` schedule rows the header is `252 + 16·s` bytes, so the default six
rows give `header_bytes = 348`. The named offsets are: `erosion.m` 164, `erosion.k` 172, `erosion.s_c` 180,
`erosion.kappa` 188, `erosion.max_elevation_m` 196, `star_luminosity` 204,
`star_mass` 212, `orbit_au` 220, `eccentricity` 228, `perihelion_deg` 236,
`axial_tilt_deg` 244, `rotation_hours` 252, `greenhouse_wm2` 260,
`temperature_offset_c` 268, `precipitation_scale` 276, `river_min_m3s` 284,
`reference_water_inventory_m3` 292, `elevation_hash` 300,
`tectonics_hash` 308, `climate_hash` 316, `hydrology_hash` 324,
`biome_hash` 332, `hazard_hash` 340, `elevation[]` 348.

Cell order is `face · n² + j · n + i`, faces `+X, −X, +Y, −Y, +Z, −Z`.
Every `WorldSettings` field is encoded:

| `WorldSettings` field | Encoding |
| --- | --- |
| `seed` | `u64` at 28 |
| `resolution` | `u32` at 20 |
| `radius_m` | `f64` raw bits at 36 |
| `land_fraction` | `f64` raw bits at 44 |
| `plate_count` | `u32` at 52 |
| `continent_layout` | `u8` at 17 |
| `tectonic_activity` | `f64` raw bits at 56 |
| `schedule.steps[]` | `schedule_count` plus 16-byte rows at 68 |
| `erosion.m`, `k`, `s_c`, `kappa`, `max_elevation_m` | five `f64` raw-bit fields |
| `planet` | ten `f64` raw-bit fields, in `PlanetSettings` declaration order |
| `river_min_m3s` | `f64` raw bits |

`preset` is display/metadata only: generation and decode use the resolved
settings above. It is range-checked and recorded for provenance; it is not
read back through `Preset::settings` and may not match after an advanced
override. Only the resolved fields reproduce the causes.

Size at the default schedule:

| `n` | Cells | Elevation bytes | Header | Total bytes | MiB |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 512 | 1,572,864 | 6,291,456 | 348 | 6,291,804 | 6.000332 |
| 1024 | 6,291,456 | 25,165,824 | 348 | 25,166,172 | 24.000332 |

Both are inside the 128 MB source bound; `n = 1024` leaves about 5.3×
headroom. The product never writes `n = 2048` sources even though the record
would fit.

Decode order:

1. Read the fixed header; reject a wrong magic or `format_version != 2` with
   the unsupported-version error. Reject a wrong `engine_version` the same
   way.
2. Validate `resolution`, `cell_count == 6 · n²`, the schedule, every
   settings range, and the exact record length
   `header_bytes + 4 · cell_count`; trailing bytes are invalid.
3. Decode elevation `i32` values in cell order and check
   `elevation_hash`.
4. Rebuild `Tectonics` with `generate_tectonics(settings, progress)` and
   compare `tectonics_hash`.
5. Recompute `reference_water_inventory_m3` on the cube grid and compare it.
6. Decode stops here; it does not run a full derive only to validate.
   Whenever a derive runs (accept, reload without a cache, cache rebuild),
   its `climate_hash`, `hydrology_hash`, `biome_hash` and `hazard_hash` are
   compared with the stored ones before any product is published or cached.
   Derived caches record the hashes they were built from.
7. Any mismatch is `physical.source.invalid`. Old magic or version is
   `physical.source.unsupported-version`. Nothing is repaired or migrated.

`Tectonics::hash_bits` is FNV-1a over: `n`, resolved layout, crust fraction,
every plate in order (site, axis, omega, subduction rank, Voronoi weight,
crust majority), every boundary record in order, every hotspot and trail point
in order, every per-cell field in declaration order, and
`fragments_reassigned`. No map iteration order reaches it. `hazard_hash` is
`Hazards::hash_bits`, which already covers the fields in declaration order and
sources in id order; it becomes public in E2. `ClimateSummary::hash_bits`
covers the nine stored annual summary fields in declaration order. Solstice
temperature and precipitation are computed on demand, so they are not stored
and add no hash field.

#### Consumer crosswalk

| Consumer | Reads today (verified) | Reads after the cut | Access | Step |
| --- | --- | --- | --- | --- |
| `src-tauri/src/physical_commands.rs` | `GeneratedWorld.field: PhysicalField`, `tectonics: TectonicWorld`, `climate: ClimateField`, `evolution: EvolutionField`, `hydrology: HydrologyField`, `source`, `derived_geojson`, `report`; commands for generate, preview, accept, derived climate/evolution/hydrology, epochs, events, Find Place, routes, regions | engine `World` through `daena_physical::engine::Products`; source v2; no evolution; hazards added; event commands call `DecodedSource::sample_events` and keep the world inside `daena-physical`; epoch commands use derived products | mixed: raster for preview/GeoJSON, point for tools | E3–E6 |
| `src-tauri/src/physical_jobs.rs` | `PhysicalJobResult` carries source, generation JSON, derived GeoJSON, `ClimateField`, `EvolutionField`, `HydrologyField`; progress goes through `ProgressSink` | source, generation JSON, derived GeoJSON, climate, hydrology, hazards; no evolution; `Stage` maps to `ProgressPhase` | mixed | E3, E4 |
| `crates/daena-physical/src/lib.rs` and its generator modules | `GeneratedWorld` of `PhysicalField`, `TectonicWorld`, `ClimateField`, `EvolutionField`, `HydrologyField`; `generate_world_with_evolution`; old source codec; `PhysicalIdentityManifestV1` inputs | source codec v2, engine facade, product adapters; old generator modules deleted in E8 | mixed | E3, E4, E8 |
| `src-tauri/src/atlas_studio.rs` | `hydro_claim::claim_at`, `landmass_claim::claim_at`, `hydro_claim::overflowing_at`, `region::propose_region` over `HydrologyField`; surface sample fields | `hydro_claim::claim_at_canonical`, `landmass_claim::claim_at_canonical`, `overflowing_at_canonical`, `region::propose_region_canonical` over `ToolSurface`; no humidity/current/thickness sample fields | point | E5b, E7 |
| `src-tauri/src/atlas_jobs.rs` | source bytes, identity, generation forcing; no direct physical types | unchanged contract; source v2 and identity v2 | – | E3, E5b |
| `crates/daena-atlas/src` | `decode_source`, `validate_field_report`, `TectonicWorld`, `PhysicalField`, `ClimateField`, `HydrologyField`, `HistoricalWorld`, claim ids, basin/lake fields | engine `DecodedSource` and `AtlasInputs` at an explicit 384×192 view; 26→10 biome adapter; canonical `ToolSurface` for claims; no humidity/current/ice-thickness inputs; same coarse-to-detail transform, styles and tile cache | raster plus point inspect | E5b, E7 |
| `crates/daena-core/src/maps.rs`, `maps/physical.rs` | `SOURCE_MAGIC DAENAPW1`, `SOURCE_VERSION 1`, `TectonicWorld`, `PhysicalIdentityManifestV1`, width/height, retry index, evolution preset, tectonic style | `DAENAPW2`, version 2, `PhysicalIdentityManifestV2` with resolution, engine version, source/elevation/tectonic hashes and reference inventory; old magic/version rejected | – | E3 |
| `crates/daena-core/src/maps/atlas.rs` | `validate_source`, source bytes, generation forcing, `source_sha256`, cache keys | source v2 through `validate_source`; identity and forcing unchanged in meaning; Atlas cache keys include the v2 physical identity (engine version, source format and hashes) | raster | E3, E5b |
| `crates/daena-core/src/maps/calendar.rs` | `history::MAX_HISTORICAL_OFFSET_YEARS`, calendar binding | unchanged; epoch offset range and calendar math stay | point | – |
| `crates/daena-core/src/project/store_map_locations.rs` | `hydro_claim::is_valid_id`, `landmass_claim::is_valid_id`, `hydro_claim::resolve` over view hydrology | `lake:c…`, `river:c…`, `landmass:c…`, `flood:c…` with a canonical cube cell; old view-cell ids are unresolved under the hard cut | point | E5 |
| `src-tauri/src/tests.rs`, `crates/daena-core/src/project/tests.rs` | old generated fixtures, recovery, event and epoch expectations | v2 fixtures at small `n`; accept/reload, delete `.daena/` rebuild, old-source rejection, cancellation latency | mixed | E3, E4, E8 |
| `daena-physical/src/find_place.rs` | `ClimateField` arrays, `HydrologyField` arrays, `HazardField`, full view-grid scan; humidity filter | engine canonical cells; engine climate summary, hydrology, hazards; no humidity criterion; candidates carry canonical cell ids | point | E5 |
| `daena-physical/src/routing.rs` | `HydrologyField` water level/slope/lake/ice, `ClimateField` biome; view-grid Dijkstra | canonical cube cells; elevation-derived slope, engine lake/river/ice, engine biome cost; routes return canonical cell ids and coordinates | point | E5 |
| `daena-physical/src/hydro_claim.rs`, `landmass_claim.rs` | view-grid cell ids, `BasinStatus`, `minimum_cell`, `island_id` | engine lake/landmass components by canonical cell; `c`-marked ids from the lowest cube cell; no `BasinStatus` dependence where the engine has no equivalent | point | E5 |
| `daena-physical/src/region.rs` | view hydrology `island_id`, `watershed_id`, polygons | engine landmass and receiver-root watersheds, polygonized on the requested view | raster for polygons, point for pick | E5 |
| `daena-physical/src/contours.rs` | `PhysicalField` equirect elevation + sea level | equirect engine elevation view + epoch sea level | raster | E4, E5 |
| `daena-physical/src/history.rs`, `derived_cache.rs` | `PhysicalField`, `ClimateField`, `DrainageField`, `HydrologyField`, old ocean volume curve, forcing math | engine `World`/`SurfaceContext`, cube inventory solver, product forcing math, engine-derived products; cache version dir gains engine/source versions | mixed | E4 |
| `src/lib/maps/physical/*` | engine climate/hydrology/hazard products; preset + resolution generation form; 26-class biome legend; hazard rasters and volcanic markers; no humidity/current/evolution/terrain-age/tectonic-style views | unchanged; product shapes now come from the Rust JSON builders and the contract fixture guards them | mixed | E7 |
| `src/lib/maps/atlas/*` | engine-backed scene; sample without humidity/current/ice thickness; divergence in 1e-9 s⁻¹; no humidity style; 26 engine classes grouped into the unchanged ten Atlas biome groups | unchanged; Atlas style label says "Biome groups" and the studio shows divergence as ×10⁻⁶ s⁻¹ | raster plus point inspect | E7 |
| `src/lib/project/types.ts`, `client.ts` | v2 product shapes; hazard products and volcanic coordinates; preset/resolution generation input; no evolution, humidity, currents, retry index or tectonic style | unchanged; `physical-contract.fixture.json` and the typed mirror keep the TS interfaces aligned with the Rust builders | – | E7 |
| `docs/maps/MAPS.md`, ADR 0004 | old source, climate and budget wording | v2 source, engine climate, measured budgets; updated in E8 in the same commit as the constants | – | E8 |

#### Field mapping

The product keeps the existing field names where the meaning is the same, but
the arrays are views of engine fields at the chosen raster size. Continuous
fields are bilinearly sampled; class and id fields use nearest cell. Point
tools read canonical cells directly and do not go through the view.

##### `ClimateField`

| Product field | Engine source and formula | Status |
| --- | --- | --- |
| `grid` | requested equirect view `width × height`; the starting size is 1024×512, with 2048×1024 at `n = 1024` only if E4 measures the budget (open question 8) | kept |
| `derivation_version` | new `CLIMATE_DERIVATION_VERSION` | kept |
| `planetary` | product display configuration reconstructed from source `PlanetSettings`; display-only fields carry no physical effect | kept |
| `temperature_centi_c` | `round(100 · ClimateSummary.t_annual_c)` | kept |
| `temperature_nh_summer_centi_c` | `world.climate_month_at_point(lat, lon, 3).0`, downscaled with the same lapse and orographic correction | kept; E2 addition |
| `temperature_nh_winter_centi_c` | `world.climate_month_at_point(lat, lon, 9).0` | kept; E2 addition |
| `moisture_mm_per_year` | no engine published column-moisture field and no consumer after the views are cut | removed |
| `precipitation_mm_per_year` | `round(ClimateSummary.p_annual_mm)` | kept |
| `runoff_mm_per_year` | `round(ClimateSummary.runoff_mm)` | kept |
| `runoff_volume_m3_per_year` | `runoff_mm / 1000 · view_cell_area_m2`; area-weighted metrics use cube cell areas instead | kept |
| `maritime_factor_ppm` | `round(1e6 · ClimateField.maritime)`, resampled | kept |
| `wind_east_milli`, `wind_north_milli` | annual mean of the twelve `ClimateField.wind` tangent vectors, projected on local east/north, ×1000 | kept |
| `wind_east_nh_summer_milli`, `wind_north_nh_summer_milli` | same projection at month 3 | kept |
| `wind_east_nh_winter_milli`, `wind_north_nh_winter_milli` | same projection at month 9 | kept |
| `wind_divergence_nano_per_s` (was `wind_divergence_ppm`) | spherical finite-difference divergence of the view wind, in 1e-9 s⁻¹ | kept, computed |
| `wind_divergence_nh_summer_nano_per_s`, `wind_divergence_nh_winter_nano_per_s` | same at months 3 and 9 | kept, computed |
| `wind_band`, `wind_band_nh_summer`, `wind_band_nh_winter` | latitude relative to `ClimateField.phi_h_deg` and `phi_f_deg`: `0` Hadley, `1` Ferrel, `2` polar | kept, computed |
| `current_east_milli`, `current_north_milli` | no ocean-circulation model | removed with current layer, current arrows, studio current sample and current metrics |
| `humidity_ppm` | no engine published relative humidity | removed with humidity view, Find Place filters, Atlas humidity style and studio humidity sample |
| `aridity_ppm` | land: `round(1e6 · clamp(1 - P_annual / PET_annual, 0, 1))`; ocean: `0` | kept, computed |
| `precipitation_nh_summer_mm` | `world.climate_month_at_point(lat, lon, 3).1` | kept; E2 addition |
| `precipitation_nh_winter_mm` | `world.climate_month_at_point(lat, lon, 9).1` | kept; E2 addition |
| `biome_class` | engine `Biome` discriminant `0..=25` | kept; legend and classes replaced |
| `storm_suitability_ppm` | presentation index: `round(1e6 · genesis_km2 / max(genesis_km2))`; 0 when the maximum is 0 | kept, computed; physical rate stays in `Hazards.storms` |
| `storm_track_ppm` | presentation index: `round(1e6 · track_km2 / max(track_km2))`; 0 when the maximum is 0 | kept, computed |
| `storm_intensity_ppm` | `round(1e6 · intensity)`, engine intensity is `[0, 1]` | kept |
| `metrics` | see below | partly kept |

Climate metrics:

- Kept and computed from engine fields: area means of annual temperature,
  precipitation, runoff; min/max temperature; wettest cell, driest land cell;
  seasonal range and seasonal min/max from `t_warm_c`/`t_cold_c`; permanent
  and seasonal frozen land from `t_cold_c`/`t_warm_c` and sea ice; mean wind
  speed and easterly/converging shares from the coarse wind; ITCZ from
  `ClimateField.itcz_lat_deg`; land aridity from `1 - P/PET`; dominant land
  biome; storm
  genesis/track/intensity means; expected storms per year from
  `Σ genesis_km2 · coarse_cell_area`; drought/heat/extreme-rain means from
  the engine hazard fields; growing/dry/wet season means from the monthly
  precipitation and temperature fields.
- Removed: `meanCurrentSpeedMilli`, `meanHumidityPpm`.
- `transportIterations` is replaced by the engine spin-up year count; it is
  not a product field with a physical meaning.

##### `HydrologyField`

| Product field | Engine source and formula | Status |
| --- | --- | --- |
| `grid` | requested view | kept |
| `derivation_version` | new `HYDROLOGY_DERIVATION_VERSION` | kept |
| `sea_level_mm` | `0` at the reference surface; epoch `S` on an epoch view | kept |
| `water_level_mm` | lake cell: `round(1000 · lake.level_m)`; ocean: sea level; land: `max(elevation_mm, sea_level_mm)` | kept, computed |
| `lake_level_mm` | lake cell: lake level; otherwise sea level | kept, computed |
| `slope_ppm` | central-difference slope of the engine elevation view, same ppm convention as today | kept, computed |
| `hillshade_ppm` | existing product hillshade formula applied to the engine elevation view | kept, computed |
| `bathymetry_mm` | `max(0, sea_level_mm - elevation_mm)` for every cell, the existing product convention; consumers use the ocean/lake masks to interpret it | kept, computed |
| `watershed_id` | root cell of the engine priority-flood `receiver` tree; each cell maps to its root, nearest-sampled to the view | kept, computed |
| `basin_by_cell` | engine `lake_id` for lake cells; `u32::MAX` for ocean and non-lake land | kept; old dry-basin ids removed |
| `lake_cells` | `lake_id != NONE` | kept |
| `ice_cells` | engine `ice_cover != 0` | kept |
| `ice_thickness_mm` | engine publishes ice class only, not thickness | removed with the studio ice-thickness row and the thickness metric |
| `shelf_cells` | ocean cells where crust is continental, or where crust is absent and depth is at most 200,000 mm; matches today's rule and uses the engine crust field | kept, computed |
| `island_id` | connected land components on the cube grid; ids assigned by lowest canonical cell; `u32::MAX` on ocean | kept; ids change meaning from discovery order to lowest-cell order |
| `basins` | one record per engine `Lake`; see basin table | partly kept |
| `rivers` | one record per engine `RiverSegment`; see river table | kept |
| `river_coordinates` | canonical cell centres along each engine river segment | kept |
| `coastline_segments` | product contour of the engine ocean mask at sea level | kept, computed |
| `lake_polygons` | product polygonization of `lake_id` | kept, computed |
| `watershed_polygons` | product polygonization of the receiver-root watershed | kept, computed |
| `land_polygons` | product polygonization of `ocean == 0` | kept, computed |
| `ocean_polygons` | product polygonization of `ocean == 1` | kept, computed |
| `shelf_polygons` | product polygonization of `shelf_cells` | kept, computed |
| `island_polygons` | product polygonization of `island_id` | kept, computed |
| `ice_polygons` | product polygonization of `ice_cells` | kept, computed |
| `bathymetry_contours` | product contour of `bathymetry_mm` | kept, computed |
| `metrics` | counts and volumes computable from engine data; water-balance solver metrics have no engine equivalent | partly kept |

Basins, one row per `Basin` field:

| Product basin field | Engine source | Status |
| --- | --- | --- |
| `id` | engine lake id | kept |
| `minimum_cell` | E2 `Lake.minimum_cell` | kept |
| `minimum_elevation_mm` | engine elevation at `minimum_cell` | kept, computed |
| `cell_count` | count of cells with that `lake_id` | kept, computed |
| `spill_cell` | E2 `Lake.spill_cell` | kept |
| `spill_elevation_mm` | `round(1000 · Lake.level_m)` for a fresh lake; `None` otherwise | kept |
| `volume_to_spill_m3` | no product consumer after flood fill becomes a full-lake boolean | removed |
| `parent_basin`, `children` | no engine basin tree and no product consumer | removed |
| `destination` | no engine basin tree and no product consumer after claims/events use lake state | removed |
| `water_level_mm` | `round(1000 · Lake.level_m)` | kept |
| `water_volume_m3` | `Lake.volume_m3` | kept |
| `inflow_m3_per_year` | `Lake.inflow_m3` | kept |
| `direct_precipitation_m3_per_year` | no engine equivalent and no consumer | removed |
| `evaporation_m3_per_year` | `Lake.evaporated_m3` | kept |
| `outflow_m3_per_year` | `Lake.outflow_m3` | kept |
| `status` | fresh with outflow: overflowing; fresh without outflow: active; saline: endorheic; salt flat: dry; no merged state | kept, derived |

River fields:

| Product river field | Engine source | Status |
| --- | --- | --- |
| `id` | engine river index | kept |
| `source_cell` | first cell of `RiverSegment.cells` | kept |
| `mouth_cell` | last cell of `RiverSegment.cells` | kept |
| `strahler_order` | computed from the engine receiver donor counts and the river cells | kept, computed |
| `destination` | `RiverEnd::Ocean` → ocean, `Lake` → basin, `Confluence` → junction | kept |
| `spill_outlet` | river ends at a lake whose `Lake.outflow_m3 > 0` | kept, computed |
| `coordinate_count` | `cells.len()` | kept |

Hydrology metrics: keep lake, river, watershed, coastline, land/ocean polygon,
shelf, bathymetry-contour and island counts computed from the view. Remove
`totalWaterM3`, `oceanWaterM3`, `inlandWaterM3`, `landIceM3`,
`balanceErrorM3`, `toleranceM3`, `fixedPointIterations` and `converged`; the
engine hydrology is not a global water-balance fixed point, and epoch volumes
live in `HistoricalMetrics`. Keeping those names with engine numbers would
imply a conservation solver that does not exist.

##### Hazards and volcanic sources

| Product field | Engine source and formula | Status |
| --- | --- | --- |
| `earthquake_hazard_ppm` | `round(quake_hazard_u8 · 1_000_000 / 255)` | kept |
| `volcanic_hazard_ppm` | `round(volcanic_hazard_u8 · 1_000_000 / 255)` | kept |
| `earthquake_annual_rate_nano` | `round(quake_rate_km2 · cell_area_km2 · 10^0.9 · 1e9)`; `10^0.9` converts M ≥ 5 to M ≥ 4 | kept, explicit conversion |
| `volcanic_annual_rate_nano` | sum `VolcanicSource.eruptions_per_yr` at each source cell, `round(rate · 1e9)`; several sources in one cell add | kept, explicit conversion |
| `earthquake_rate_per_million_years_ppm` | `min(1e6, round(annual_rate_nano · 1e6 / 1e9))`, the existing bounded presentation formula | kept |
| `eruption_rate_per_million_years_ppm` | same formula from the eruption rate field | kept |
| `volcanic_sources` | engine `VolcanicSource` mapped one-to-one; `stable_id: u64`, `cell`, origin, activity, `eruptions_per_model_year_nano` | kept; product id type widened |
| `metrics` | source counts, means/maxima and rate masses from the engine fields | kept, computed |

The view fields are presentation products. Event sampling and `expected_lambda`
read canonical cells, so their rate mass is the canonical sum, not the sum of
view pixels. A view-cell annual rate uses the view cell area; it is never used
to sample an event.

`DerivedVolcanicSource` becomes:

| Product field | Engine field |
| --- | --- |
| `stable_id: u64` | `VolcanicSource.id: u64` |
| `cell` | `VolcanicSource.cell` |
| `origin` | `VolcanicOrigin::{SubductionArc, Hotspot, Rift}` |
| `activity_class` | `VolcanicActivity` |
| `eruptions_per_model_year_nano` | `round(eruptions_per_yr · 1e9)` |

##### `sample_events` and `expected_lambda`

| Input today | After the cut |
| --- | --- |
| `world: &TectonicWorld` | engine `World`; canonical grid and source elevation |
| `hazards: Option<&HazardField>` | engine `Hazards` |
| `climate: Option<&ClimateField>` | engine `ClimateSummary` plus `ClimateField` for the storm tracer; product `ClimateField` is view-only |
| `epoch_field: Option<&PhysicalField>` | epoch surface `(reference_elevation_mm, sea_level_mm)`; no equirect view for event logic |
| `hydrology: Option<&HydrologyField>` | engine `Hydrology`; lake spill cells and fresh/closed state |
| `request` | unchanged shape: kind, interval, `maxEvents`, `hazard_seed` |
| `MaterializedEvent.sampled_center_id` | `Option<u64>`: the engine volcanic source id for eruptions; the canonical cube cell (widened) for storms and floods; `None` for earthquakes. JSON/TypeScript carry it as a decimal string because a `u64` can exceed `2^53` |

Event rates:

- Earthquake: `Λ = Σ_cells(rate_M4_per_year_nano) / 1e9 · interval_years`, then
  the existing Poisson draw; magnitude remains Gutenberg–Richter `b = 0.9`
  with `M ≥ 4`.
- Eruption: `Λ = Σ_cells(eruption_rate_nano) / 1e9 · interval_years`; the
  sampled cell is a volcanic source cell; `sampled_center_id` is the `u64`
  source id.
- Storm: genesis cell is drawn from `Σ genesis_km2 · coarse_cell_area`; month
  is drawn from the engine's monthly genesis weights; the track is
  `trace_storm(climate_field, ocean_fraction, genesis, month)`. The event
  `track_cells` are the returned cells; `hazard_ppm` is the carried intensity
  times 1e6. The product's `STORM_NOTABLE_PER_YEAR` policy may scale the rate;
  its value is a product policy, not a physical field (open question).
- Flood: every fresh engine lake with `outflow_m3 > 0` is an overflowing
  basin at the interval midpoint; the event cell is `Lake.spill_cell` and the
  stable center is `Lake.minimum_cell`; fill potential is 1e6 because a
  spilling engine lake is at its spill level.

`expected_lambda` takes the same engine hazards, climate and hydrology plus
the request. Flood lambda is the count of fresh overflowing lakes; storm
lambda is the notable policy applied to the engine genesis census; earthquake
and eruption lambda are the rate masses above.

##### Find Place filters

| Query field | After the cut |
| --- | --- |
| `land_only` | engine ocean mask at the selected epoch |
| `altitude_m_min/max` | engine elevation in metres, converted from millimetres |
| `latitude_milli_deg_min/max` | canonical cell latitude |
| `slope_ppm_min/max` | slope computed from engine elevation and true neighbour distances |
| `temperature_centi_c_min/max` | engine annual temperature |
| `precipitation_mm_min/max` | engine annual precipitation |
| `humidity_ppm_min/max` | removed, with the UI fields and `humidity` soft key |
| `aridity_ppm_min/max` | `1 - P/PET` computed on land |
| `biome_class` | engine `Biome` id |
| `island_id` | engine landmass component id (lowest canonical cell) |
| `ice` | engine `ice_cover != 0` |
| `coast_distance_km_min/max` | distance to the engine ocean mask on the cube grid |
| `freshwater_distance_km_min/max` | distance to engine lake or river cells |
| `earthquake_hazard_ppm_min/max` | engine `quake_hazard` scaled to ppm |
| `volcanic_hazard_ppm_min/max` | engine `volcanic_hazard` scaled to ppm |
| `storm_suitability_ppm_min/max` | engine genesis presentation index |
| `soft`, `max_candidates`, `min_cells` | unchanged; soft keys that named removed fields are removed |
| candidate `cells` | canonical cube cell ids, not view cells |
| candidate `means.islandId` | engine landmass component id |

##### Route inputs

| Input | After the cut |
| --- | --- |
| start/end lat/lon | nearest canonical cube cell |
| ocean mask | engine `ocean` |
| elevation | engine elevation |
| slope | computed from engine elevation and true neighbour distance |
| river cells | engine `discharge_m3s >= river_min_m3s` and not lake |
| lake cells | engine `lake_id != NONE` |
| ice cells | engine `ice_cover != 0` |
| biome cost | engine `Biome` grouped into the existing route-cost families; old 10-class ids are mapped in one adapter table |
| existing authored routes | unchanged; rasterized on the view, then projected to canonical cells |

##### Claim and feature ids

| Claim | Id today | Id after the cut |
| --- | --- | --- |
| Lake | `lake:{equirect_cell}` | `lake:c{canonical_cube_cell}` from `Lake.minimum_cell` |
| River | `river:{equirect_cell}` | `river:c{canonical_cube_cell}` from the first cell of `RiverSegment.cells` |
| Flood | `flood:{equirect_cell}` | `flood:c{canonical_cube_cell}` from `Lake.minimum_cell` |
| Landmass | `landmass:{equirect_cell}` | `landmass:c{canonical_cube_cell}` from the lowest cell of the land component |
| Volcanic source | `u32` hash of product centers | engine `u64` `VolcanicSource.id`, serialized as a decimal string for JSON/TypeScript |
| Orometry | Atlas id | unchanged |

Ids are stable for one accepted source and `n`; they are not stable across
`n`, view size or epoch (the feature itself can move). The `c` marker keeps
old format-only ids such as `lake:1234` out of the validators, so every old
physical id is unresolved and shown as unresolved; there is no mapping from
`DAENAPW1`. Resolving a new id at an epoch looks it up in that epoch's tool
surface; a lake that is dry or merged at that epoch is unresolved, and a
landmass id resolves only while its component's lowest cell is still that
cell. Authored overlays and materialized events keep their geometry and
provenance.

#### Settings mapping

Product `GenerationSettings` after the cut:

| Product field | Engine field | Conversion |
| --- | --- | --- |
| `resolution` | `WorldSettings.resolution` | product accepts 512 or 1024 |
| `radius_metres` | `WorldSettings.radius_m` | metres; must equal `planetary.radius_metres` |
| `target_land_fraction_ppm` | `WorldSettings.land_fraction` | `ppm / 1_000_000`; preset default unless an advanced/test override is supplied |
| top-level `seed` | `WorldSettings.seed` | `u64`; the product UI may keep a 32-bit seed and zero-extend it |
| `preset` | source metadata and `Preset` | selects the engine preset; resolved settings are what the source stores |
| `width`, `height` | removed | replaced by `resolution` |
| `retryIndex` | removed | one deterministic result per settings; seed and preset carry the identity |
| `evolutionPreset` | removed | engine erosion is not a product control |
| `tectonicStyle` | removed | the engine preset controls plate count, layout and activity |

Product `PlanetaryConfiguration` to `PlanetSettings`:

| Product field | Engine field | Conversion and note |
| --- | --- | --- |
| `star_luminosity_ppm` | `star_luminosity` | `/ 1_000_000` |
| `star_mass_ppm` | `star_mass` | `/ 1_000_000`; sets year length |
| `semi_major_axis_milli_au` | `orbit_au` | `/ 1_000_000`; despite its name the field holds micro-AU (`ASTRONOMICAL_UNIT_MILLI = 1_000_000`) |
| `eccentricity_ppm` | `eccentricity` | `/ 1_000_000` |
| `axial_tilt_milli_deg` | `axial_tilt_deg` | `/ 1_000` |
| `rotation_period_seconds` | `rotation_hours` | `/ 3_600` |
| `retained_heat_centi_c` | `temperature_offset_c` | `(value - 1_400) / 100`; the Earth default is zero. Epoch offsets add to this value |
| `radius_metres` | `WorldSettings.radius_m` | duplicate check against `GenerationSettings.radius_metres` |
| `preset`, `version` | product metadata | reconstructed for display; not causes |
| `bond_albedo_ppm` | none | proposed removal from generation and UI; the engine already derives surface albedo from surface type, snow, ice and greenhouse. Adding a second planetary albedo would double count |
| `mean_density_kg_m3` | none | proposed display-only; it already only feeds `surfaceGravityMilliG` and has no climate or terrain effect |
| absent | `perihelion_deg` | engine default `283.0`; the product has no control |
| absent | `greenhouse_wm2` | set by the engine surface preset |
| absent | `precipitation_scale` | set by the engine surface preset |

If the product removes `bond_albedo_ppm` or `mean_density_kg_m3`, the product
`PlanetaryConfiguration` version becomes 2; if they remain display-only, the
version stays 1 and the source ignores them. A new engine setting is not
needed for retained heat: the engine already has `temperature_offset_c`. If
Claude wants bond albedo to affect climate, the
only safe proposal is a new `PlanetSettings::bond_albedo` whose default is
`0.0` (today's output unchanged at the default), plus a calibration change
for Earthlike; that is a separate climate change, not part of E1. Recommended:
keep it out of generation.

Engine settings set by the surface preset (not independent product fields):

| `WorldSettings` field | Source |
| --- | --- |
| `plate_count` | engine preset |
| `continent_layout` | engine preset |
| `tectonic_activity` | engine preset |
| `schedule` | engine `Schedule::default()` |
| `erosion` | engine `ErosionParams::default()` |
| `river_min_m3s` | engine default `100.0` |
| `greenhouse_wm2` | engine preset |
| `precipitation_scale` | engine preset |
| `land_fraction` | engine preset, unless the advanced `targetLandFractionPpm` override is set |

Generation keeps the engine's target land-fraction sea-level solve. The water
inventory is derived from the accepted surface and is used by epochs; it is
not a second generation sea-level mode.

Product presets and controls:

- Replace the product's "Terrain age" control with a **World preset** control
  that selects one of the eight engine `Preset` values. The preset sets land
  fraction, plate count, layout, activity, greenhouse and precipitation scale.
- Keep the seed control. "Reroll" picks a new seed; it no longer increments a
  retry index.
- Keep the planetary preset/advanced controls for star, orbit, tilt, rotation
  and retained heat. They override the engine planet after the surface preset.
  The surface preset owns greenhouse and precipitation; the planetary preset
  does not.
- Add a resolution control: 512 (default) or 1024 (high).
- Remove "Continent layout" and "Terrain age"; the engine preset owns the
  first and erosion owns the second.
- `targetLandFractionPpm` stays in the command contract as an advanced/test
  override; the UI uses the preset value. A hard cut may change the JSON shape,
  but keeping this field avoids a second sea-level mode.

#### Epochs

The product forcing math stays in `history.rs`. The engine supplies the
surface, climate, hydrology, biomes and hazards; `daena-physical` combines
them with the history metrics.

1. Normalize the offset. `T = temperature_offset_centi_c(t, forcing)`;
   `T_lag = lagged_temperature_offset_centi_c(t, forcing)`. The lagged value
   is used only for land ice, not for the climate run. For epoch zero, reuse
   the accepted present products directly; no sea-level solve or climate
   rerun is needed.
2. `I_ref` is `World::reference_water_inventory_m3()`: for every cube cell,
   `area_m2 · max(0, -elevation_mm) / 1000`, rounded to `u64`. This is the
   product's existing inventory definition: all cells below sea level,
   including dry depressions, not only the connected ocean. The source stores
   the same value and decode checks it.
3. Land ice and thermal expansion are changes relative to epoch zero, so
   epoch zero is exactly the present world. The engine has no ice thickness,
   so there is no climate-derived ice volume. The product's current solve
   withholds a volume from its own ice-thickness model, which goes away
   (decided 2026-11-11, open question 13):
   - `I_eq(t) = land_ice_volume(I_ref, T_lag(t), forcing)`, the existing
     `history.rs` logistic;
   - `ΔI_ice = I_eq(t) − I_eq(0)`;
   - `E(t) = thermal_expansion_volume(I_ref, T(t), forcing)`;
   - `ΔE = E(t) − E(0)`.
   - **Defaults for new maps (2026-11-22).** The climate run amplifies the
     offset 2.5–4× through snow and ice albedo, and the old defaults froze
     40–65% of epochs (global mean below 0 °C, sea level −240 to −420 m).
     `default_for_seed` now draws amplitudes of 0.80–1.15, 0.35–0.60 and
     0.20–0.30 °C, land ice of 6–9.5% of the inventory, a 1.1–1.6 °C ice
     transition and 90–150 ppm/°C thermal expansion. A 12-seed sweep at
     n=256 (−200 ka to +20 ka, 5 ka steps) has no frozen epoch, the coldest
     epoch 4–13 °C below the present (typically about 6.5 °C) and the lowest
     sea level −53 to −146 m. Existing maps keep their stored forcing, so
     their epochs are unchanged.
4. Solve the epoch sea level `S` on the cube grid for the target
   `I_epoch = clamp(I_ref − ΔI_ice + ΔE, 0, maximum ocean volume)`:
   `V(S) = Σ area_m2(cell) · max(0, (S − elevation_mm[cell]) / 1000)`. Choose
   the candidate millimetre level nearest the target, with the same
   tie-breaking as the product's current inventory solver. `V(0) = I_ref`,
   so epoch zero gives `S = 0`. This replaces the old equirectangular
   `ocean_volume_curve`. There is one solve and no coupled iteration with
   lake volumes: lakes and dry depressions below sea level are inside the
   hypsometric `V(S)`, as in `I_ref`.
5. Build the surface `h_m = (elevation_mm − S) / 1000` and run the engine
   climate, hydrology, biomes and hazards on it once.
6. The erosion `domain` is unchanged across epochs; it is rebuilt from
   settings and tectonics, not from the epoch sea level.
7. The epoch temperature offset goes in
   `PlanetSettings::temperature_offset_c` as
   `authored_offset_c + (T(t) − T(0)) / 100.0`, so epoch zero adds nothing.
   The lagged `T_lag` is not used in climate.
8. Final routing is regenerated from the epoch surface for consistency, even
   though hydrology has its own priority flood. If E4 measures it as a
   material cost, the epoch path may skip only the returned `receiver`, and
   must keep hydrology and hazards on the same code.
9. Report `HistoricalMetrics` from the history math and the solve:
   - `land_ice_equilibrium_m3 = I_eq(t)`;
   - `land_ice_m3 = ΔI_ice`, signed in meaning, stored as the existing
     field's type with the sign convention recorded in E4;
   - `thermal_expansion_m3 = ΔE`;
   - `effective_ocean_water_m3 = V(S)`;
   - `sea_level_mm = S`;
   - the conservation error `|V(S) − I_epoch|` from millimetre rounding.

   The old fixed-point fields of the product hydrology are removed (see
   "Removal list").
10. The epoch cache key is
    `(physical_identity, source_format, engine_version, forcing, normalized_epoch,
    view_size)`. The cache directory/version string includes
    `WORLD_ENGINE_VERSION`, `SOURCE_VERSION`, `CLIMATE_DERIVATION_VERSION`,
    `HYDROLOGY_DERIVATION_VERSION` and `HAZARD_DERIVATION_VERSION`.
11. Target: at most 1.5 s per epoch at `n = 512`. Estimate: about 0.65 s
    for the climate pass and downscale, about 0.3 s for hydrology, biomes and
    hazards, and about 0.2 s to rebuild the surface and routing, on the
    current M4 Pro: one pass, about 1.15 s. This is an estimate, to be
    measured in E4.

The product view for an epoch keeps the existing convention: `PhysicalField`
holds the reference elevation in `elevations_mm` and sets `sea_level_mm = S`.
All other epoch products are derived from `h_epoch_m`. This preserves the
meaning of every existing consumer that compares elevation with sea level and
avoids shifting the canonical source.

**What an epoch models today, and what is left for later (2026-11-22).**
Epochs are deterministic, not random per epoch. The world seed fixes the
three cycle amplitudes, periods (100–150 ka, 41–50 ka and 23–28 ka, like
Earth's orbital cycles) and phases. The offset at any year is their sum, so
the same year always gives the same climate. An epoch changes only:

- a global temperature offset, run through the full climate model;
- sea level, from land-ice volume (lagged) and thermal expansion.

Not modelled yet, and intended later:

- **Continental drift.** Plates, crust and elevation are the present ones
  at every epoch, back to the 10 Ma playback limit. Only the coastline moves,
  with sea level.
- **Ice sheets as geography.** Land ice is a volume for the sea-level budget.
  The climate's own glaciers grow and shrink, but no ice sheet is built at a
  place with thickness that lowers the sea and loads the crust.
- **Slow climate drivers:** greenhouse changes over millions of years,
  mountain building and ocean gateways.
- **Response calibration.** The climate run amplifies the offset 2.5–4×,
  so a stored offset is not the global change. Existing maps whose stored
  forcing came from the pre-2026-11-22 defaults can still freeze. Either
  solve for the forcing that gives the stated global change, or migrate old
  forcing.

#### Progress and cancellation

`ProgressPhase` mapping:

| Engine `Stage` | Product `ProgressPhase` | Note |
| --- | --- | --- |
| `Tectonics` | `BuildingTectonicStructure` | 0/1, then 1/1 |
| `Level(n)` | `ErodingLandscape` | one report per iteration at level `n` |
| `SeaLevel` | `BuildingTerrain` | the final terrain datum; not a second geometry pass |
| `FinalRouting` | `CalculatingWater` | routing on the finished surface |
| `Climate` | `CalculatingClimate` | coarse spin-up and downscale |
| `Hydrology` | `BuildingRiversAndLakes` | lakes, rivers and ice |
| `Biomes` | `PreparingGeography` | class field |
| `Hazards` | `ValidatingWorld` | hazards are the last pass |

`Progress::report(stage, done, total)` maps to the phase and preserves
`done/total`. The adapter must not emit the old product phases independently;
the engine is the only source of stage progress. `Progress::cancelled` is
checked at every stage boundary and inside the long loops named in decision
13. E3 tests the command path with a cancellation flag set from a progress
callback and asserts a `Cancelled` result within 100 ms. Final routing and the
lake flood run beside the final coarse climate pass; their `timings_ms` spans
overlap, and `total` is wall time rather than their sum.

#### Engine additions for E2

Signatures are the intended public or crate-visible API; exact visibility is
settled in E2.

```rust
pub const WORLD_ENGINE_VERSION: u32 = 1;

pub fn generate_tectonics(
    settings: &WorldSettings,
    progress: &mut dyn Progress,
) -> Result<Tectonics, WorldError>;

pub struct SurfaceContext {
    pub grid: CubeSphere,
    pub domain: Vec<u8>,
    pub base_m: Vec<f32>,
    pub uplift_m_yr: Vec<f32>,
    pub k_rock: Vec<f32>,
    pub plate: Vec<u8>,
    pub crust: Vec<u8>,
}

pub fn prepare_surface(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    progress: &mut dyn Progress,
) -> Result<SurfaceContext, WorldError>;

pub fn derive_after_surface(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    surface: &SurfaceContext,
    elevation_m: &[f32],
    progress: &mut dyn Progress,
) -> Result<World, WorldError>;

pub fn derive_world(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    elevation_mm: &[i32],
    progress: &mut dyn Progress,
) -> Result<World, WorldError>;

pub fn derive_epoch(
    settings: &WorldSettings,
    tectonics: &Tectonics,
    reference_elevation_mm: &[i32],
    epoch_sea_level_mm: i32,
    temperature_offset_c: f64,
    progress: &mut dyn Progress,
) -> Result<World, WorldError>;
```

- `generate_inner` keeps erosion and the sea-level threshold, canonicalizes
  elevation, builds `SurfaceContext` from the evolved fields, and calls
  `derive_after_surface`. `derive_world` builds `SurfaceContext` from settings
  and tectonics, converts `i32` millimetres to `f32`, and calls the same
  function. There is one downstream implementation.
- `derive_epoch` subtracts the epoch sea level from the reference elevation
  and adds the temperature offset to the planet before calling
  `derive_after_surface`.
- A derived `World` has the same substantive fields, but its `timings` and
  `climate_passes` contain only the passes it ran, `stats.levels` is empty,
  and `sea_level_shift_m` is `0` because the source does not store the
  pre-canonicalisation shift. Generation keeps the full diagnostics. Tests
  compare every substantive field and all generation hashes, not those
  diagnostics.

Canonical elevation:

```rust
pub fn elevation_mm_from_m(value_m: f32) -> i32; // round(value_m * 1000)
pub fn elevation_m_from_mm(value_mm: i32) -> f32; // value_mm as f32 / 1000
pub fn canonicalize_elevation_m(elevation: &mut [f32]);
```

Canonicalisation moves a value by at most 0.5 mm from the rounding step plus
half an `f32` ULP. Below 16 km that bound is under 1 mm; the observed maximum
over the E2 world set is 0.9766 mm.

Source data and hashes:

```rust
pub struct GenerationHashes {
    pub elevation_mm: u64, // FNV-1a over the i32 millimetres, not the f32 bits
    pub tectonics: u64,
    pub climate: u64,
    pub hydrology: u64,
    pub biome: u64,
    pub hazards: u64,
}

impl World {
    pub fn source_elevation_mm(&self) -> Vec<i32>;
    pub fn generation_hashes(&self) -> GenerationHashes;
    pub fn reference_water_inventory_m3(&self) -> u64;
    pub fn tectonics_hash(&self) -> u64;
    pub fn hazard_hash(&self) -> u64;
}

impl Tectonics {
    pub fn hash_bits(&self) -> u64;
}
```

`elevation_mm` hashes the `i32` millimetres, not the `f32` bits. `tectonics`
covers every tectonic field, plate, boundary, hotspot and count. `climate`
covers the nine stored summary fields; `ClimateStats` is report-only and is
excluded. `hydrology` covers lake kind, level, area, volume, evaporation,
outflow and inflow, every river, `lake_id`, discharge, ice and the receiver
tree. `Lake::id`, `minimum_cell` and `spill_cell` are excluded because they are
claim metadata derived from the water state and lake geometry; `filled_m` is
excluded as routing scratch, and `pop_order`/`lake_exit` are test-only.
`biome` covers the ids. `hazards` covers the hazard fields, volcanic sources
and all four storm arrays; `HazardStats` is excluded.

```rust
pub struct OceanVolumeSample {
    pub sea_level_mm: i32,
    pub volume_m3: u64,
}

pub fn ocean_volume_curve(
    grid: &CubeSphere,
    elevation_mm: &[i32],
) -> Result<Vec<OceanVolumeSample>, WorldError>;

pub fn ocean_volume_at(
    grid: &CubeSphere,
    elevation_mm: &[i32],
    sea_level_mm: i32,
) -> Result<u64, WorldError>;

pub fn sea_level_for_ocean_volume(
    curve: &[OceanVolumeSample],
    inventory_m3: u64,
) -> Result<i32, WorldError>;
```

Cancellation:

```rust
pub fn check_cancelled(progress: &dyn Progress) -> Result<(), WorldError>;
```

`generate_tectonics` and `prepare_surface` take `progress` and check between
their stages. `coarse_with` and `downscale` gain a cancellation callback; the
spin-up year loop, each moisture month and each of the 48 sweeps call it.
Chunks are sized by measured work: a downscale batch is 160 columns (55-90 ms
at `n = 1024` in release), resample and detail chunks are 4.2M and 1.0M cells, and
tectonic, biome and hazard chunks are sized to stay under the 100 ms budget.
`Hydrology::build` checks before/after the priority flood and inside the
settle/ice loops every fixed chunk. Routing checks between its receiver,
stack, depression, rebuild and drainage stages. Erosion checks between pit
fill, routing, ice, stream power, overdeepening, hillslopes, smoothing and
clamping. `Hazards::build` checks at hazard stage boundaries; its rayon
chunks are not interrupted. `resample`, `upsample` and `downsample` call the
cancellable implementations with a never-cancel callback, so one copy of each
arithmetic remains. A deterministic test cancels at 30 evenly spaced checks in
`generate` and `derive_world`, so the default `cargo test` exercises
cancellation without relying on wall-clock tests.
The gap test measures the largest wall-clock gap between `cancelled()` calls
and runs with `--ignored --test-threads=1` because parallel test load can
deschedule the test thread. Release gaps on a 12-thread machine: at
`n = 512`, about 57 ms for generation (Tectonics) and 41 ms for derive
(Climate); at `n = 1024`, 58 ms for cold generation (Tectonics), 58 ms warm
(Tectonics) and 48 ms for derive (Climate). The cold grid-table gap is gone
because `build_tables` now runs in parallel. `CubeSphere::new` does not hold
the cache lock during that build, so a stolen rayon job cannot block on the
same lock. Chunks scale with
`rayon::current_num_threads()`, and routing checks every 65,536 sequential
graph steps inside `depress` and the donor/stack builds. The gap tests build
in the test profile, which also runs the debug-only routing validation, so
the `n = 1024` test reports its gaps without asserting. The budget is
asserted at `n = 512` and by the end-to-end deadline test, which cancels
inside the final climate stage.

Point sampling:

```rust
pub struct PointSample {
    pub cell: u32,
    pub elevation_m: f32,
    pub ocean: bool,
    pub biome: Biome,
    pub climate: ClimatePoint,
    pub hydrology: HydrologyPoint,
    pub hazards: HazardPoint,
}

impl World {
    pub fn climate_month_at(&self, cell: u32, month: usize) -> (f32, f32);
    pub fn climate_month_at_point(
        &self,
        lat_deg: f64,
        lon_deg: f64,
        month: usize,
    ) -> (f32, f32);
    pub fn sample_point(&self, lat_deg: f64, lon_deg: f64) -> PointSample;
    pub fn coarse_ocean_fraction(&self) -> Vec<f64>;
    pub fn cell_lat_lon(&self, cell: u32) -> (f64, f64);
    pub fn cell_area_m2(&self, cell: u32) -> f64;
}
```

`CubeSphere::lat_lon`, `area_m2`, `cell_at`, `position` and `sample_bilinear`
are already public; the `World` helpers are thin convenience wrappers.

`climate_month_at` uses the same per-cell downscale corrections and returns
`(temperature °C, precipitation mm)`. Monthly values are computed on demand;
`ClimateSummary` keeps only its existing annual fields. `climate_month_at_point`
uses the cross-face chart stencil and calls `climate_month_at` for its terms.
The equirect view's four monthly fields use the point method; no
full-resolution monthly fields live in `ClimateSummary`.

`ClimatePoint` carries annual, warmest, coldest, solstice temperatures,
precipitation, PET, runoff, ice balance, sea-ice months, wind and aridity
components. `HydrologyPoint` carries lake id, lake level, discharge, ice class
and ocean mask. `HazardPoint` carries the four hazard potentials and the quake
rate. Continuous values are bilinear on the cube grid; ids and classes use the
containing cell.

Equirectangular view:

```rust
pub struct EquirectView {
    pub width: u32,
    pub height: u32,
    pub elevation_m: Vec<f32>,
    pub ocean: Vec<u8>,
    pub climate: ClimateSummary,
    pub t_nh_summer_c: Vec<f32>,
    pub t_nh_winter_c: Vec<f32>,
    pub p_nh_summer_mm: Vec<f32>,
    pub p_nh_winter_mm: Vec<f32>,
    pub biome: Vec<u8>,
    pub lake_id: Vec<u32>,
    pub discharge_m3s: Vec<f32>,
    pub ice_cover: Vec<u8>,
    pub hazards: HazardView,
    pub storms: StormClimatology,
}

pub struct HazardView {
    pub quake_rate_km2: Vec<f32>,
    pub quake_hazard: Vec<u8>,
    pub volcanic_hazard: Vec<u8>,
    pub drought: Vec<u8>,
    pub heat_wave: Vec<u8>,
    pub extreme_rain: Vec<u8>,
    pub flood: Vec<u8>,
}

impl World {
    pub fn equirect_view(&self, width: u32, height: u32) -> Result<EquirectView, WorldError>;
}

pub fn resample_equirect_f32(
    grid: &CubeSphere,
    field: &[f32],
    width: u32,
    height: u32,
) -> Vec<f32>;
pub fn resample_equirect_nearest_u8(
    grid: &CubeSphere,
    field: &[u8],
    width: u32,
    height: u32,
) -> Vec<u8>;
pub fn resample_equirect_nearest_u32(
    grid: &CubeSphere,
    field: &[u32],
    width: u32,
    height: u32,
) -> Vec<u32>;
```

The view uses cell centres: longitude
`-180 + 360·(col + 0.5)/width`, latitude
`-90 + 180·(row + 0.5)/height`. This is not the current product convention;
`daena-physical/src/lib.rs` `latitude_micro`/`longitude_micro` currently use
edge-origin row/column values. E4 moves those two helpers to centres together
with the view. Class and id fields use nearest sampling; continuous fields use
bilinear sampling, with the engine's cross-face chart stencil at seams.

Storm tracer:

```rust
pub struct StormStep {
    pub cell: u32,
    pub intensity: f64,
}

pub struct StormTrack {
    pub genesis_cell: u32,
    pub month: u32,
    pub steps: Vec<StormStep>,
}

pub fn trace_storm(
    climate_field: &ClimateField,
    ocean_fraction: &[f64],
    genesis_cell: u32,
    month: u32,
) -> Result<StormTrack, WorldError>;
```

The ocean fraction stays `f64` because the hazard pass computes it in `f64`
and the tracer must match the pre-E2 loop bit for bit. `StormStep.intensity`
is `f64` for the same reason; product consumers may cast it.

`Hazards::storms` calls `trace_storm` and must produce the same cells and
intensities as the current inline loop. `StormClimatology` gains
`genesis_monthly_km2: Vec<f32>` in month-major order, so event sampling can
choose a month from the engine's own monthly genesis weights. The new field
is included in `Hazards::hash_bits` in declaration order.

Lake spill cells and claim metadata:

```rust
impl Lake {
    pub spill_cell: u32;
    pub minimum_cell: u32;
}
```

The values come from the priority-flood depression and the lowest lake cell.
No product code reconstructs them from geometry.

Canonical millimetre elevation changes three lake counts relative to the
engine before E2 (575bf6f): Earthlike at `n = 512` 326 to 325, Frozen seed 3
at `n = 512` 300 to 299, and Hothouse seed 11 at `n = 1024` 1,415 to 1,430.
Total lake area changes by -0.004 %, and dry depressions are unchanged.

Epoch planet override:

```rust
impl PlanetSettings {
    // Adds to the authored temperature offset; it does not replace it.
    pub fn with_temperature_offset(self, offset_c: f64) -> Self;
}
```

Anything else E3–E6 needs:

- `Tectonics::hash_bits`, `World::generation_hashes`, `World::hazard_hash`
  and public `Hazards::hash_bits`.
- `World::source_elevation_mm` and the cube inventory solver above.
- `Biome::ALL` or an equivalent ordered list for the 26-class legend.
- `StormClimatology::genesis_monthly_km2`.
- A public `World` constructor path (`derive_world`) that does not require a
  product type.
- No engine land-ice volume. Epochs use the `history.rs` land-ice change
  relative to epoch zero (see "Epochs"), so the engine needs no thickness
  state.
- Product claims/regions use canonical cells, so the adapter exposes the
  canonical `cell` for a lat/lon point and the canonical cell centre for a
  claim; it does not expose view indices as ids.

#### Removal list

Every item is removed in E3–E8 as part of the hard cut; none is synthesized.

- **Evolution.** `EvolutionField`, `EvolutionSettings`, `EvolutionPreset`,
  `EvolutionMetrics`, `DrainageField` as a product field, all evolution
  commands (`project_physical_evolution`,
  `project_physical_derived_evolution`), the evolution job result, the terrain
  age control, and every evolution JSON field (`beforeElevationsMm`,
  `routingElevationMm`, `fillDepthMm`, `accumulationM3PerYear`, outlet/edge
  and drainage/erosion metrics). The engine owns erosion and exposes no
  editable before/after surface.
- **Humidity.** `ClimateField.humidity_ppm`,
  `ClimateMetrics.mean_humidity_ppm`, Find Place `humidityPpmMin/Max` and the
  `humidity` soft key, the humidity raster overlay and legend, the Atlas
  `daena-atlas-humidity` style, Atlas studio humidity sample, and the humidity
  help text. The engine publishes no relative humidity; a value would be
  fabricated. Atlas's humidity-driven vegetation-resistance term in
  `crates/daena-atlas/src/erosion.rs` is replaced with engine precipitation
  and aridity inputs, and its golden relief fixtures are regenerated with a
  written reason.
- **Ocean currents.** `ClimateField.current_east_milli`,
  `current_north_milli`, `ClimateMetrics.mean_current_speed_milli`, the
  `currents` reserved layer, current arrows and rasters, Atlas current
  overlay/arrows and studio surface current. The engine has no ocean
  circulation model.
- **Ice thickness.** `HydrologyField.ice_thickness_mm`, its Atlas studio row
  and any thickness metric. Ice classes remain; the engine does not model
  thickness.
- **Empty climate fields.** `ClimateField.moisture_mm_per_year` and
  `ClimateMetrics.meanHumidityPpm`/`meanCurrentSpeedMilli` are removed
  because no engine field or consumer supports them. `maritime_factor_ppm`
  stays because the engine publishes `ClimateField.maritime`.
- **Basin fields.** `volume_to_spill_m3`, `parent_basin`, `children`,
  `destination`, `direct_precipitation_m3_per_year` and the `Merged` status
  leave the product `Basin`. Flood fill becomes a full-lake boolean; Atlas
  wet-basin checks use lake status/volume.
- **Hydrology water-balance metrics.** `totalWaterM3`, `oceanWaterM3`,
  `inlandWaterM3`, `landIceM3`, `balanceErrorM3`, `toleranceM3`,
  `fixedPointIterations` and `converged` leave `HydrologyField.metrics`;
  epoch conservation stays in `HistoricalMetrics`.
- **Retry and repair.** `retryIndex` in the generation input, job, generation
  JSON, source, identity manifest and `PhysicalField`; `retry_index` in
  product seeds; `CODE_GENERATOR_RETRY_EXHAUSTED` and
  `PhysicalErrorCode::GeneratorRetryExhausted`; retry-scoped
  `SeedDomain`/`derive_subsystem_seed` use in generator code.
  `HistoricalForcingParameters::default_for(seed, retry_index)` becomes
  seed-only (retry fixed to zero); a generation JSON that already carries
  explicit forcing keeps using it. One settings set produces one world.
- **Tectonic style and terrain age controls.** `TectonicStyle` in the
  generation input/JSON/UI and `EvolutionPreset` as above. The engine surface
  preset replaces both; `TectonicStyle::Any` is not a physical cause.
- **Old source and generator.** `DAENAPW1`, `TECTONIC_SOURCE_VERSION`,
  `TECTONIC_SOURCE_HEADER_BYTES`, the old tectonic/crust/plate/boundary/hotspot
  source payload, old `decode_source_v2`/`encode_source_v2`, old
  `TectonicWorld` as canonical source, old `generate_field`,
  `generate_world_with_evolution`, the old climate/hydrology/hazards/evolution
  generator, `PRODUCTION_DEFAULT_WIDTH/HEIGHT`,
  `PRODUCTION_MAX_WIDTH/HEIGHT`, `DEFAULT_WIDTH/HEIGHT`, `MAX_WIDTH/HEIGHT`
  as production authority, and the 384×192 assumptions. A `Grid` type may
  remain only as an equirect view descriptor.
- **Planetary generation fields.** `bond_albedo_ppm` leaves generation and the
  UI; `mean_density_kg_m3` stays only as display metadata for
  `surfaceGravityMilliG` if Claude keeps it. Neither is a cause in the source.
  This is an open question below.
- **Old product climate classes.** The ten-class biome constants and legend
  (`BIOME_OCEAN` … `BIOME_TROPICAL_FOREST`) are removed in favour of the
  engine's 26 `Biome` ids and names. Atlas's existing style groups are fed by
  an explicit grouping adapter, not by the old product classification.
- **Old ids and caches.** Equirect cell ids in claims, old epoch cache keys,
  old derived cache format, old Atlas golden fixtures, and old physical
  identity manifests are not readable. Deleting `.daena/` must rebuild from
  the new source.

#### Steps E2–E8

| Step | Content | Exit tests |
| --- | --- | --- |
| E2 | Engine additions: `generate_tectonics`, `SurfaceContext`, `prepare_surface`, `derive_after_surface`, `derive_world`, `derive_epoch`, canonical elevation, public hashes/version, cube inventory solver, cancellation callbacks, point sampling, equirect view/resamplers, shared storm tracer, lake spill/minimum cells, monthly storm weights, on-demand solstice month access, planet offset helper. Refactor `generate_inner` to call the shared downstream path. Regenerate the canonical-elevation baseline. | `cargo test -p daena-worldgen --locked --offline`; tests that generation and `derive_world` produce identical fields/hashes; canonical `f32`/`i32` round trip; every hash changes when its field changes; storm tracer equals the old inline trace cell-for-cell; cancellation ends within 100 ms; 1-thread/12-thread identity; point and equirect sampling seam tests. |
| E3 | Source `DAENAPW2`, product generation path, accept and reload. `daena-physical` defines source magic/version, manual codec, `PhysicalIdentityManifestV2`, `GENERATOR_VERSION = 2`, validation and old-version rejection. Generation calls `generate`, encodes v2, creates the product view and generation JSON. Progress mapping and cancellation. `project_physical_generate/preview/accept/status/cancel` use the engine job. | Source round trip is byte-exact and length-strict; decode+derive reproduces all hashes; corrupt elevation, tectonic hash, settings, trailing bytes and old magic are rejected with the right codes; accept writes the source asset and descriptor atomically; reload after deleting `.daena/` rebuilds and validates; old source is `physical.source.unsupported-version`; cancellation latency under 100 ms end to end; materialized events untouched. |
| E4 | Derived products and epochs. `daena_physical::engine::Products` adapters build product `ClimateField`, `HydrologyField`, `HazardField`, GeoJSON, contours and the equirect view; `derived_cache::StaticDerivedPhysics` stores the engine-backed products; `history.rs` epoch path solves the cube sea level from the land-ice and expansion changes relative to epoch zero, then calls `derive_epoch` once; cache version dir/key gains engine/source versions; remove evolution from job results and commands; bump `CLIMATE_DERIVATION_VERSION`, `HYDROLOGY_DERIVATION_VERSION`, `HISTORICAL_DERIVATION_VERSION`, `CACHE_FORMAT_VERSION`; hazard field versions may wait for E6 but the cache key includes them. | `cargo test -p daena-physical --locked --offline`; product field shapes and formulas; epoch 0 equals the present products; epoch sea level and `HistoricalMetrics` conservation; epoch at `n = 512` is at most 1.5 s (measured, estimate recorded); deleting `.daena/cache/physical-derived` rebuilds identical products; source-only rebuild after deleting `.daena/`; external checkpoint import validates; cache key changes with engine/source/epoch/view-size and planetary-token versions. |
| E5 | Tools. Find Place, routes, claims, landmass/region, previews, coastlines, contours and Atlas input move to canonical cells and the equirect view. Claims use canonical cube ids; Atlas `prepare_from_source` builds the view at the explicit view size and keeps its transform/styles/tile cache; remove humidity/current Atlas inputs and styles. `project_physical_find_place`, `suggest_routes`, `propose_region`, and Atlas Studio claim/inspect use the adapter. | Find Place criteria map one-to-one (no humidity), candidates use canonical ids and point coordinates; route paths, slope, crossings, lake/ice cells match engine data; claim ids resolve at the reference and at epochs, and old ids are unresolved; region geometry closes and is epoch-aware; GeoJSON feature/byte limits hold at 1024×512 and 2048×1024; Atlas golden tests regenerated with a reason each; Atlas tile seams/cache deletion unchanged. |
| E6 | Events and hazards. Build the product `HazardField` from engine fields with explicit unit functions; widen volcanic source ids and `MaterializedEvent.sampled_center_id` to `u64` and serialize them as decimal strings; use engine lake spill cells; replace the inline storm path with `trace_storm` and monthly genesis; bump `HAZARD_DERIVATION_VERSION = 2`, `VOLCANIC_SOURCE_DERIVATION_VERSION = 2`, earthquake/eruption materialization to 2, storm/flood materialization to 3, `EVENT_MATERIALIZATION_VERSION = 3`; keep already materialized events. | Unit tests for M ≥ 5 → M ≥ 4, area multiplication, nano/ppm conversions, u64 ids and JSON round trips beyond `2^53`, source ordering; event sampling deterministic and bounded; storm tracks match engine tracer; flood events use spill cells and fresh/overflowing lakes; expected lambda matches rate mass; materialized events keep authored provenance; version mismatches fail validation rather than mixing semantics. |
| E7 | Frontend. Remove humidity, current, evolution, terrain-age and tectonic-style views/controls; add world preset and resolution; 26-class biome legend; hazard layers and source metadata; show engine-derived metrics; update Find Place, routes, claims and epoch UI; Atlas studio surface samples lose humidity/current/thickness; Atlas humidity style is not offered. | `npm run check` and the Maps/Atlas frontend tests; Tauri command tests for new input/result shapes; rendered desktop checks for preset/resolution, layer toggles, biome legend, no removed overlays, claim/epoch labels, Atlas studio inspection and detach; no UI reads a removed field. |
| E8 | Clean-up and budgets. Delete uncalled old generator code, old source codec, old types and constants; move the cancellable chunked source hashing from `daena-physical` (`source_hashes_with_cancel`, an FNV replica locked to `World::generation_hashes` by a test) into the engine; measure full generation, memory, source size, epoch time, GeoJSON at 1024×512/2048×1024; set `GENERATION_TIME_BUDGET_MS`, `WORKING_MEMORY_BUDGET_BYTES`, `MAX_SOURCE_BYTES`, production resolution constants and GeoJSON limits from the measured envelopes; update ADR 0004 decision history and `MAPS.md` in the same commit; regenerate Atlas/engine golden fixtures with reasons. | Full `cargo test --workspace --locked --offline`; no reference to deleted generator/source symbols; measured budget table recorded; ADR/MAPS constants match code; clean checkpoint export, delete `.daena/`, rebuild, and compare physical identity/products; malformed source, external checkpoint edit and missing `.daena` recovery; packaged Tauri desktop lifecycle check for generate/accept/reload/export. |

#### E3 notes

Implemented in E3 on top of the E2 engine API.

- `daena-physical/src/engine.rs` is the only module that names
  `daena-worldgen` types. It maps product settings and `PlanetaryConfiguration`
  to `WorldSettings`, maps `Stage` to `ProgressPhase` with no independent
  product phases, and cancels through `Progress::cancelled` via the existing
  `ProgressSink`.
- `DAENAPW2` is encoded and decoded once, in the same module. Decode checks
  magic, format version, engine version, ranges, the exact record length,
  elevation hash, rebuilt tectonics and its hash, then the recomputed
  reference inventory. Old magic or a wrong format/engine version is
  `physical.source.unsupported-version`; every other mismatch is
  `physical.source.invalid`.
- Generation calls `generate`, encodes the source, builds the explicit
  1024×512 cell-centred equirect view and the generation JSON. Reload decodes,
  rebuilds tectonics with `generate_tectonics`, derives with `derive_world`,
  and compares climate, hydrology, biome and hazard hashes before returning
  products.
- `PhysicalIdentityManifestV2` and `GENERATOR_VERSION = 2` replace V1. The
  manifest covers resolution, engine version, the source SHA-256, elevation
  and tectonic hashes, the reference inventory and the historical forcing
  block. The product generation JSON now carries `resolution`, `preset`,
  `engineVersion`, `sourceVersion`, the resolved land fraction and the
  reference inventory; `width`, `height`, `retryIndex`, `evolutionPreset`
  and `tectonicStyle` are gone.
- Accept writes the source asset and descriptor through the existing core
  transaction and no longer pre-populates the derived cache. The product
  accept command rejects resolutions other than 512 and 1024.

Transitional state until E4–E7:

- **`ClimateField` (a).** The old `climate::derive_current_climate` runs on the
  engine-backed 1024×512 elevation view. It derives from elevation, the
  planetary block and the view grid, so it does not need engine climate state
  and does not fabricate one. E4 replaces it with the engine summary, wind,
  maritime and hazard inputs. Job climate and `project_physical_derived_climate`
  use this path; `project_physical_derived_geojson` reloads the engine world and
  emits the coastline preview from the view.
- Cancellation covers the product tail as well as the engine. The engine gains
  `World::equirect_elevation_with_cancel`, which is exactly the elevation
  subset of `equirect_view` and checks between row batches; the product view no
  longer builds unused monthly climate fields. Contour extraction, GeoJSON
  formatting and the old view-climate storm and land-distance passes check the
  sink. A release deadline sweep at `n = 512` and `n = 1024` keeps the worst
  lateness under the 100 ms budget.
- `target_land_fraction_ppm = 0` means "use the preset land fraction"; a
  non-zero value is an explicit override. The E7 UI must send 0 by default.
- **Current UI.** The generation input now requires `resolution` and `preset`
  and rejects unknown fields, so the unchanged frontend's generate request
  (width, height, retry index, evolution preset, tectonic style) fails until
  E7. When it is updated it must send `targetLandFractionPpm = 0`; today it
  sends 300,000, which would give every preset 30 % land.
- **No derived cache.** `project_physical_derived_geojson` and
  `project_physical_derived_climate` reload and re-derive the world on every
  call (about 2.7 s at `n = 512`). E4 adds the engine-backed derived cache.
- **`HydrologyField` (b).** Job hydrology and the derived hydrology command
  return a clear "not available until phase E4" error. No zero or placeholder
  field is published.
- **`EvolutionField` / `DrainageField` (b).** The evolution job result and the
  derived evolution command are not populated; the old erosion is never run on
  an engine world. E4 removes them from the command result and E8 deletes the
  old generator.
- **Hazard field (b).** E4/E6 build it from the engine hazard pass. E3 has no
  product hazard field and publishes none.
- **Events, epochs, Find Place, routes, regions and Atlas (b).** These consumers
  still name the retired `DAENAPW1` `TectonicWorld`; for a `DAENAPW2` source
  they fail with a clear unsupported-version error until E4–E6 move them to
  engine products. Atlas reads the new source in E5. Materialized events are
  never touched by generation or acceptance.

#### E4 notes

Implemented in E4 on top of E3 (`5eff345`). E4 adds engine-backed products,
the derived cache and epochs; the unmigrated E5–E7 consumers still use the old
product types.

- **Product types.** The engine-backed fields are
  `daena_physical::products::{ClimateField, HydrologyField, HazardField}` and
  their basin, river, hazard-source and metric records. The old
  `climate::ClimateField`, `hydrology::HydrologyField` and
  `hazards::HazardField` stay for Find Place, routes, claims, events, Atlas and
  the old generator tests until E5/E6 move those consumers; E8 deletes them.
  Nothing is renamed later.
- **Climate.** `products::ClimateField` is built from the explicit 1024×512
  cell-centred view: annual and solstice temperature and precipitation,
  runoff and runoff volume, maritime factor, annual and solstice wind,
  divergence and wind bands, aridity, the engine biome id `0..=25`, storm
  presentation indices and the kept metrics. The engine spin-up year count is
  a new `climateSpinupYears` metric; the old transport slot is not reused.
  Moisture, humidity and ocean currents are absent, not zero-filled.
- **Hydrology.** `products::HydrologyField` is built from the engine lakes,
  rivers and receivers: levels, slope, hillshade, bathymetry, receiver-root
  watershed ids, engine lake ids, lake/ice/shelf/island masks, engine basins
  (canonical minimum and spill cells), rivers (canonical source and mouth,
  computed Strahler order, Ocean/Lake/Confluence end), canonical cell-centre
  river coordinates, geometry and view counts. Ice thickness, volume-to-spill,
  the basin tree, direct precipitation and the water-balance fixed-point
  metrics are absent. Strahler order is computed on the river network only
  (river cells, terminal ocean/lake cell excluded) and each segment takes the
  maximum order along its own cells; the n=512 network reaches order 5.
- **Wind and ITCZ.** Divergence uses the spherical
  `(1/(R cosφ))(∂u/∂λ + ∂(v cosφ)/∂φ)` form with a pole floor and is stored
  in 1e-9 s⁻¹ as `wind_divergence_*_nano_per_s`; a 1e-6 scale rounded typical
  surface values (1e-6 to 1e-5 s⁻¹) to single digits. The ITCZ metric
  is the mean of the engine's `ClimateField.itcz_lat_deg`, not the Hadley edge.
- **Hazards.** `products::HazardField` maps engine `u8` potentials to ppm,
  converts `M ≥ 5` to `M ≥ 4` with `10^0.9` in
  `products::earthquake_rate_nano`, multiplies by m², sums eruption rates per
  cell and keeps `u64` source ids. Raster cells use the view cell area; point
  and event consumers use the canonical cube cell area. JSON serializes a
  source id as a decimal string. Source cells map to the view row at their own
  latitude; events and materialization stay E6.
- **Evolution.** It is gone from the job result. The registered
  `project_physical_derived_evolution` command remains and returns
  `physical evolution products were removed in phase E4`.
- **Geometry.** `latitude_micro` and `longitude_micro` now use cell centres.
  The legacy tectonic diagnostic calls them with one-past-the-edge indices, so
  those two out-of-range indices keep the edge value `180`/`90`.
- **Cache.** `derived_cache::StaticDerivedPhysics` stores the view, climate,
  hydrology, hazards, GeoJSON and the generation hashes. The cache format is
  `CACHE_FORMAT_VERSION = 3`; the directory string is
  `e<engine>-s<source>-c<climate>-h<hydrology>-z<hazard>-v<volcanic-source>-view<width>x<height>-<planet-token>`.
  Every fill checks the decoded source's generation hashes before publishing.
  Derived and reload commands fill on a miss and read the cache on a hit.
- **Versions.** `CLIMATE_DERIVATION_VERSION = 3`,
  `HYDROLOGY_DERIVATION_VERSION = 2`, `HISTORICAL_DERIVATION_VERSION = 3`,
  `CACHE_FORMAT_VERSION = 3`. Hazard derivation versions stay at `1` in E4
  but are already in the cache key; E6 bumps them to `2`.
- **Epochs.** `history::derive_historical_products` takes `I_ref` from the
  source, computes `ΔI_ice = I_eq(t) − I_eq(0)` and
  `ΔE = E(t) − E(0)`, solves the canonical cube sea level once with
  one `ocean_volume_curve` and `sea_level_for_curve`, calls `derive_epoch` once
  with the temperature offset relative to epoch zero, and adapts that world
  with the same product builders. Epoch zero reuses the present products
  byte for byte and performs no solve or climate run. `land_ice_m3` is `i64`:
  positive means more land ice than at epoch zero. The conservation error is
  `|V(S) − I_epoch|`.
- **Forcing seed.** `HistoricalForcingParameters::default_for_seed(u64)` mixes
  the full 64-bit seed through the new `derive_subsystem_seed64`; generation
  uses it. The old `default_for(u32, retry)` remains for unmigrated consumers.
- **Epoch cache key.** `physical_identity | source format | engine | history,
  hazard, volcanic-source, climate and hydrology versions | view size |
  planetary token | normalized epoch | every forcing field`.
- **Measured timings** (release, 12-thread M4 Pro; wall time):

  | Step | n=512 | n=1024 |
  | --- | ---: | ---: |
  | Generation through products | 2.13 s | 3.82 s |
  | Reload, cold derived cache | 1.38 s | 2.49 s |
  | Derived command, warm cache | 0.17 s | 0.20 s |
  | One epoch | 1.09 s | 2.03 s |

  The warm command time is a `DAENAPW2` decode/validate (150-176 ms) plus the
  cache read (22-23 ms); re-deriving the world is 1.23-2.32 s. The epoch target
  is 1.5 s at n=512; measured 1.09-1.15 s. The 1024 row is recorded but not an
  E4 target. The generated river network reaches Strahler order 5 at n=512 and
  n=1024.
- **Cancellation.** The E3 release deadline sweep now includes the product
  adapters and chunked source hashing: worst lateness was 51 ms at n=512 and
  51 ms at n=1024, both under the 100 ms budget. A new epoch sweep measured
  30 ms worst lateness. The adapters check between view row batches, inside the
  canonical root/island/river-order loops, in hazard mass loops, between
  polygon batches, and inside each source-hash chunk; the engine hash spans are
  replicated as chunked FNV with a lock test against the engine values.

Doubts:

- The view's solstice temperature and precipitation values call
  `climate_month_at_point` for every view cell, twice. That is the largest
  adapter cost (about 0.34 s), although the epoch is now about 1.1 s.
  A future engine-side cancellable equirect view would remove the duplicate
  resampling logic.
- The hazard `earthquake_rate_mass_nano` metric uses canonical cube cell areas,
  while the view rate arrays use view pixel areas, as required. A consumer that
  sums the view array will get the view mass, not the canonical mass.
- River `coordinate_count` is the engine cell count, as the field mapping says;
  `river_coordinates` has one cell-centre point per cell, so the two always
  match.

Decided 2026-11-11:

1. Yes: apply the sea-level threshold, round to millimetres, then flood.
2. Bond albedo leaves generation and the UI.
3. Mean density stays as display metadata for surface gravity only. It is
   kept in the map descriptor, not in the source.
4. Humidity is removed.
5. Ice thickness is removed.
6. Storm events keep a notable-storm product policy. `λ = f · storms_per_yr ·
   years`, where `f` is one product constant. Calibrate `f` so that Earthlike
   seed 7 keeps the current product's expected storm events per year at the
   same interval, and record the value. The physical census stays in
   `Hazards.storms`.
7. The 26 engine classes map onto the existing Atlas style groups in one
   adapter table.
8. The view is 1024×512 at both resolutions, until E4 measures 2048×1024 at
   `n = 1024` inside the budgets.
9. Epoch `PhysicalField` keeps the reference elevations and the epoch
   `sea_level_mm`.
10. `targetLandFractionPpm` stays as an advanced and test override.
11. The source codec accepts any power-of-two `n`. The product accepts 512
    and 1024. Tests use `n = 64`.
12. Old claim ids are left unresolved, with no migration.
13. Epoch land ice: neither option as proposed. The engine has no ice
    volume, so the solve uses the `history.rs` land-ice change relative to
    epoch zero, plus the expansion change, in one hypsometric solve, with no
    lake-volume iteration. Epoch zero equals the present world. See
    "Epochs".

#### E5a notes

Implemented in E5a on top of E4 (`72cfa8b`). E5a moves the physical tools to
canonical cube cells; Atlas and the frontend still consume the old product
types until E5b and E7.

- **Tool surface.** `daena_physical::engine::ToolSurface` is built from one
  `worldgen::World` and the world is dropped before the surface returns. It
  holds, per canonical cell: reference elevation `i32` mm (the epoch sea level
  is one scalar), packed ocean/river/ice/fresh-lake/wet-lake flags, slope `u32` ppm, engine lake id `u32`, biome
  `u8`, annual temperature `i32` centi-°C, precipitation and PET `u32` mm,
  earthquake and volcanic hazard `u8`, storm genesis presentation index
  `u32`, landmass component id `u32` (lowest cell, `u32::MAX` on ocean), the
  priority-flood receiver `u32`, `SurfaceLake` records (id, minimum and spill
  cells, fresh/overflowing/has-water, cell count) and `SurfaceRiver` cell
  paths. Slope uses the true `CubeSphere::neighbor_distance` on the four cube
  axes with one-sided differences at the poles. River is
  `discharge_m3s >= river_min_m3s && lake_id == NONE`. Aridity is computed as
  `1 - P/PET` on land rather than stored.
  Size: 63,122,468 bytes at `n = 512` (40.13 bytes/cell) and 251,989,524
  bytes at `n = 1024` (40.05 bytes/cell), measured in release.
- **Cache.** `physical_commands` keeps a two-entry in-process LRU of
  `Arc<ToolSurface>`. The key is the existing `historical_cache_key`, so it
  covers physical identity, source format, engine, derivation versions, view
  size, the planetary token, the normalized epoch and every forcing field.
  There is no disk cache. `project_physical_clear_epoch_cache` clears both
  this cache and the historical JSON cache.
- **Epochs.** `history::solve_epoch` is the shared forcing, land-ice,
  expansion, ocean-curve and sea-level solve. Both
  `derive_historical_products` and the tool surface call it; the surface path
  then makes one `derive_epoch` and drops the world. Epoch zero re-derives
  through `derive_world` and verifies every source hash; an epoch world is not
  hash-checked because its climate, hydrology, biome and hazard values are
  expected to change.
- **Find Place.** `find_place_canonical` maps every query field one to one:
  the removed humidity fields are gone from `FindPlaceQuery`,
  `FindPlaceMeans`, the soft keys, the fit score and the explanations.
  Altitude is reference elevation minus the epoch sea level; aridity is
  `1 - P/PET`; hazard ppm is the `u8` potential scaled by `1e6/255`. Coast and
  freshwater distances are multi-source Dijkstra over the eight cube
  neighbours with true cell-centre step lengths. Freshwater sources are
  river cells and cells of fresh lakes that hold water; saline lakes and dry
  salt flats are not freshwater. An absent source leaves
  `u32::MAX`, which is a defined non-match. Candidates carry canonical
  `cells`, canonical `means.islandId` and canonical cell-centre coordinates.
  Clustering uses the eight cube neighbours; compactness uses a latitude and
  wrapped-longitude bounding box at the canonical resolution.
- **Routes.** `suggest_routes_canonical` runs Dijkstra on the eight cube
  neighbours and never enters an ocean cell. Length, climb, edge slope and
  maximum slope use true neighbour distances and the surface slope in µm/m.
  River crossings are non-river to river transitions; lake cells are fresh or
  saline lakes holding water (a dry salt flat costs as land), and ice cells
  come from the surface. The 26 engine biomes map to the existing route-cost
  families in `surface_biome_ppm`: sea ice/ice sheet/glacier 2.5, alpine 1.8,
  tundra 1.4, salt flat/cold desert/hot desert 1.35, tropical seasonal
  forest/tropical rainforest/mangrove 1.45, default 1.0. Authored geometry is
  projected to canonical cells and marched with a slerp between consecutive
  cell centres, so the reuse strategy still sees a connected corridor.
- **Claims.** Canonical ids are `lake:c{minimum_cell}`,
  `river:c{first_cell}`, `flood:c{minimum_cell}` and
  `landmass:c{lowest_component_cell}`. `is_valid_id` accepts only the
  `c` form, so `lake:1234`-style ids are rejected by anchor validation and
  unresolved in `store_map_locations`. A lake resolves only while it holds
  water at that surface epoch; a merged lake resolves to the surviving
  component only if its own minimum cell is that component's lowest cell.
  Landmass resolution requires the id cell to be the component's lowest cell,
  so a joined landmass resolves under the lower id and the absorbed id stops
  resolving.
- **Regions.** `propose_region_canonical` selects canonical landmass or
  receiver-root cells, rasterizes them onto the requested view through each
  view pixel's nearest canonical cell, and polygonizes with
  `contours::polygons_from_mask_with_cancel`. Rings are closed before return.
  `region_id` is the canonical component id or receiver root; no view index
  reaches the result. At an epoch the mask uses that epoch's ocean, lakes and
  receivers.
- **Hard cases covered by tests.** Antimeridian routes (160° to −160°),
  ocean endpoints, unreachable islands, coast distance on a world with no
  ocean, freshwater distance from lakes and rivers, dry lakes, merged
  landmasses, epoch ocean movement, the same-cell route error, and empty
  landmass/watershed clicks.
- **GeoJSON limits.** The ignored release test builds E4's view products at
  `1024×512` and `2048×1024` for `n = 1024`:
  - `1024×512`: 290 features, 136,726 bytes, 1.06 s; 3,382 coastline
    segments, 1,322 lake records, 8,793 watershed polygons, 436 land
    polygons, 242 ocean polygons, 39 island components, 33,372 shelf cells,
    5,252 bathymetry contours, 3,201 rivers.
  - `2048×1024`: 427 features, 264,838 bytes, 2.88 s; 7,722 coastline
    segments, 17,769 watershed polygons, 1,069 land polygons, 334 ocean
    polygons, 74 island components, 133,379 shelf cells, 12,680 bathymetry
    contours.
  Both are far inside `MAX_GEOJSON_FEATURES` and
  `MAX_DERIVED_GEOJSON_BYTES`; the view size is unchanged.
- **Measured timings** (release, 12-thread M4 Pro; wall time, one source,
  seed 831429, Earthlike, 29 % land):

  | Step | n=512 | n=1024 |
  | --- | ---: | ---: |
  | Cold present surface (derive + surface) | 0.68 s | 1.81 s |
  | Cold epoch surface (solve + derive + surface) | 0.59-0.64 s | 1.50-1.52 s |
  | Warm Find Place with coast distance | 0.12-0.13 s | 0.56 s |
  | Warm route (3 suggestions) | 0.24-0.25 s | 1.08-1.10 s |
  | Warm landmass region on a 1024×512 view | 0.03 s | 0.03 s |

  The cold rows are the surface cache fill after source decode and validation.
  The epoch target is 1.5 s at `n = 512`; measured 0.59-0.64 s. Surface builds
  are faster than the E4 derive because they do not build the view, climate,
  hydrology, hazard or GeoJSON products.

Doubts:

- The E4 product view slope in `products::slope_ppm` divided millimetre
  differences by metres and multiplied by `1e6`, 1000× the µm/m ppm
  convention. Fixed in the E5a review: the view slope is now µm/m like the
  tool surface, and `HYDROLOGY_DERIVATION_VERSION` is 3 so old caches miss.
- The two-entry surface cache can hold 120 MiB at `n = 512` or 480 MiB at
  `n = 1024` for a present plus an epoch surface. E8 must set
  `WORKING_MEMORY_BUDGET_BYTES` with this in mind, or reduce the surface
  width (for example store temperature and precipitation as `i16`/`u16`).
- Storm genesis is stored per canonical cell as a presentation index
  normalized by the canonical maximum. The view index uses the view maximum,
  so the same point can differ by a few ppm between the two products.
- Warm routes rerun Dijkstra for up to four strategies; the `n = 1024`
  measurement is 1.08 s with three. No E5 target was set, but a future cache
  or a single bidirectional search would reduce it.
- `hydro_claim::claim_at_canonical` measures the river pick radius in raw
  micro-degrees, like the old view code: it ignores the antimeridian wrap and
  `cos(latitude)`. No command calls it yet; E5b must fix it when Atlas picks
  move to the surface.
- The old `find_place`, `suggest_routes`, `propose_region`, `claim_at` and
  old id formatters remain for Atlas and E8; `is_valid_id` already rejects
  their ids. Atlas claim anchors are unresolved until E5b moves them to the
  canonical surface.

The questions as raised:

1. **Canonicalization versus ocean flood order.** The design applies the
   sea-level threshold, rounds elevation, then floods. The current
   `sealevel::solve` floods inside the same function. Recommendation: adopt
   the design order; otherwise source-only reproduction needs a stored ocean
   mask or a pre-round elevation copy. Confirm this is the intended reading
   of decision 4.
2. **Bond albedo.** Recommendation: remove it from generation and the UI.
   The engine already models surface albedo; a second planetary albedo would
   double count. If it must stay, add `PlanetSettings::bond_albedo` with
   default `0.0` and recalibrate Earthlike as a separate change.
3. **Mean density.** Recommendation: keep it only as display metadata for
   surface gravity, not in the source or generation. If the product must not
   show a density that does not affect the world, remove the control too.
4. **Humidity.** Recommendation: remove it. If Claude wants an honest humidity
   view, add a published column-relative-humidity field to the engine climate
   as a separate output contract; do not reconstruct it from temperature and
   rain in the product.
5. **Ice thickness.** Recommendation: remove it; the engine's ice cover is a
   class. Add engine thickness later only if the ice model gains a thickness
   state.
6. **Storm event rate.** Recommendation: keep the product's notable-storm
   policy as an explicit product constant applied to the engine genesis
   census, so event volume stays useful; the physical census remains
   `Hazards.storms`. Alternatively sample the full census and let
   `maxEvents` cap. Confirm which semantics Maps wants.
7. **Atlas biome style.** Recommendation: keep the existing ten Atlas style
   groups and map the 26 engine classes to them in the adapter. A 26-class
   Atlas style rewrite is presentation work outside E1.
8. **Raster view size.** Recommendation: 1024×512 at both product
   resolutions initially, with 2048×1024 at `n = 1024` only if E4 measures
   memory and GeoJSON inside budget. The view size is explicit and never
   canonical.
9. **Epoch `PhysicalField` convention.** Recommendation: keep reference
   elevations in `elevations_mm` and put the epoch level in `sea_level_mm`,
   as today. This avoids rewriting every consumer and keeps source identity
   separate from epoch state.
10. **Land-fraction override.** Recommendation: keep
    `targetLandFractionPpm` as an advanced/test override; the UI defaults to
    the engine preset. There is no second sea-level mode in phase E.
11. **Source fixture resolutions.** Recommendation: let the source codec
    accept any engine power-of-two `n` for tests and CLI; let product
    acceptance allow only 512/1024. This keeps E3 fixtures at `n = 64`
    cheap.
12. **Old claim ids.** Recommendation: do not migrate or reinterpret them.
    They become unresolved; the UI offers repair. This follows the hard cut
    and ADR 0004.
13. **Epoch land-ice volume source.** This is the one place the code and a
    literal reading of decision 9 differ: `history.rs` has the logistic
    `land_ice_volume`, but the current sea-level solve withholds the
    climate-derived land-ice volume from `hydrology.rs`, while the logistic
    is only `land_ice_equilibrium_m3`. Recommendation: keep the
    climate-derived volume in the solve (epoch zero stays equal to present)
    and keep the history logistic as the equilibrium metric. If Claude wants
    the logistic volume in the solve, epoch zero no longer equals present and
    the epoch-zero shortcut must be redefined; that is a behavior change, not
    a detail.

#### E5b notes

Implemented on `world-engine-v2` on top of E5a (`cb49d2d`). Atlas now reads
`DAENAPW2` through `daena_physical::engine` and no longer names an old
generator or product type.

- **One entry.** `engine::DecodedSource::derive_atlas_inputs` makes one derive
  per call: `derive_world` with every source hash checked for the present, or
  `derive_epoch` with the solved sea level and temperature offset. It returns
  the view, product climate/hydrology/hazards, a product `TectonicView` and a
  canonical `ToolSurface`, then drops the world. `build_world_products` and
  Atlas share one inner product-field builder; a pinned FNV digest test keeps
  `build_world_products` output byte-identical.
- **Planet label.** The source carries `PlanetSettings`, not a product
  `PlanetaryConfiguration`. Atlas reconstructs the display configuration from
  those fields (`preset = Custom`, Earth-like bond albedo and density because
  the source does not carry them) instead of assuming `earth_like()`.
- **View.** Atlas uses `ATLAS_VIEW_WIDTH = 384`, `ATLAS_VIEW_HEIGHT = 192`
  (the old production grid), cell-centred with row 0 at −90°. Standard (×4)
  and Detailed (×8) pass the 96 MB refine budget; Print (×16) fails at 384×192
  and is not offered, as before. The cell-centre orientation test maps an
  engine interior land cell to a land view cell at the same lon/lat and checks
  its row; latitude is not flipped.
- **Epochs.** `history::solve_epoch` supplies the sea level and temperature
  offset. The present derive is still joined with the epoch derive when
  `offset_years != 0`. View elevations stay in the reference datum: the cold
  and present views are equal bit for bit (0 mm maximum difference, so the f32
  round trip did not move a value) and only `hydrology.sea_level_mm` differs.
- **Cancellation.** `AtlasProgress` is adapted to `daena_physical::ProgressSink`
  (the trait now requires `Send + Sync`), and source decode and the epoch solve
  run through the same adapter. `ToolSurface` construction takes a cancellation
  callback and checks in the slope, flags, storm, landmass and river passes.
  The ignored release sweep at n=512 reports **48.6 ms worst lateness** against
  the 100 ms budget. A cancelled prepare writes no cache entry.
- **Mixed indexing.** `products::HydrologyField.basin_by_cell` stores
  `Basin.id`, not an index; `refine::is_wet_basin` now looks the record up by
  id. River `mouth_cell`/`source_cell` are canonical ids; `refine` converts the
  handful of mouth and deposition checks through `river_coordinates` before
  indexing view arrays. Tests cover ocean river mouths reaching view sea
  through their paths (745 of 868 at the golden world, against 458 of 868
  when the canonical id is misused as a view index) and a wet basin's view
  minimum cell being a lake cell (at n=64, where the canonical grid is below
  the view count). In review, a tautological river-path test was replaced by
  the mouth test, and the progress adapter now reports decode as
  `validating` instead of `deriving-epoch`.
- **Biomes.** One adapter maps the 26 engine ids to the ten existing Atlas
  groups. The Biome style paints each land pixel from the canonical cell under
  it (`ToolSurface::biome_class`), so `classify_biome_cell` and humidity are no
  longer read. The `!ice` Ice→Tundra downgrade stays.
- **Surface samples.** `AtlasSurfaceSample` drops humidity, currents and ice
  thickness. Divergence fields are renamed to
  `wind_divergence_*_nano_per_s` (1e-9 s⁻¹). No Atlas threshold or scale
  consumed divergence, so there was nothing to rescale; the studio only
  displayed it. Biome and storm explanations are written in Atlas from the
  sampled seasonal temperature, precipitation, aridity, elevation, shear and
  storm indices, with no humidity.
- **Claims, floods, regions.** Studio inspect uses
  `hydro_claim::claim_at_canonical`, `landmass_claim::claim_at_canonical` and
  `overflowing_at_canonical`; region proposals use
  `region::propose_region_canonical` at 1024×512. Flood labels use
  `overflowing_claims_canonical`. `ToolSurface` carries an
  `overflowing_lake(cell)` flag for O(1) render shading and indexes lakes by id
  and minimum cell with a linear fallback for synthetic tests. The river pick
  now wraps longitude across ±180° and scales east–west degrees by
  `cos(latitude)`, with the radius still in micro-degrees of latitude; tests
  cover the antimeridian and 70°N.
- **Removed inputs.** The humidity style, current rasters/arrows and the
  `currents` layer are gone from the registry and request validation; a request
  naming either fails with `atlas.request.invalid`. Erosion vegetation uses
  `1_000_000 − aridity_ppm` as moisture. Ice thickness is gone; glacial work
  uses ice presence with the same cold curve (a removal, not a new model).
- **Versions.** `ATLAS_DETAIL_ALGORITHM_VERSION` 2→3,
  `ATLAS_DERIVED_DRAINAGE_VERSION` 2→3, `ATLAS_RENDERER_VERSION` 1→2. The core
  Atlas cache keys were already covered: the physical identity is
  `PhysicalIdentityManifestV2` (engine version, source format, source SHA-256,
  elevation and tectonic hashes, reference inventory), and the residual,
  drainage and artifact keys add the Atlas algorithm, drainage and renderer
  versions plus the source SHA-256.
- **Goldens and fixtures.** `golden_world()` is generated by
  `engine::generate` at n=128 and decoded back to a source; the studio test
  world is the same shape. Regenerated tests and reasons:
  - every old `GeneratedWorld`/`HistoricalWorld` fixture: the old generator is
    replaced and the view is engine-backed at 384×192;
  - `ATLAS_DETAIL_ALGORITHM_VERSION` assertions: 2→3;
  - `ATLAS_DERIVED_DRAINAGE_VERSION`: 2→3;
  - bundled style count: 17→16 (humidity style removed);
  - amplification/refine residual byte bounds: the golden view grew from
    64×32 to 384×192, so Standard residuals are now 4.7 MB; the bounds were
    raised to 8 MB;
  - the refine peak-RSS guard: the engine golden plus the 384×192 view reaches
    about 1.3 GB process peak in the test process; the guard is now 2 GiB;
  - the macro-erosion mean guard: fluvial and thermal deltas are bounded
    separately per scale, and the engine fixture reaches the combined bound;
    the guard is now three combined steps;
  - the studio detail-chunk cell budget: a 16× chunk of a 384×192 view needs
    more than 256² cells, so the budget is 512²;
  - orometry basin assertions: a coastline system feature can sit over a lake
    cell in the engine fixture, so the no-basin check applies only to non-lake
    ocean cells.
- **Atlas test suite.** Before (base `cb49d2d`, 102 tests): 6.5 s. After
  (111 tests including the new E5b cases): about 161 s single-threaded, 80 s
  with the default test threads. The growth is the n=128 engine golden and the
  384×192 view, not per-test work.

Measured timings (release, 12-thread M4 Pro; wall time, seed 831429, Earthlike,
29 % land; `atlas-map` not used):

| Step | n=512 | n=1024 |
| --- | ---: | ---: |
| Generation through products (excluded from the table below) | 2.16 s | 3.88 s |
| Cold prepare, present | 2.59 s | 3.87 s |
| Cold prepare, epoch (both derives) | 2.97 s | 4.82 s |
| Standard 1024×512 render, cold cache | 2.85 s | 4.26 s |
| Standard 1024×512 render, warm cache | 0.019 s | 0.052 s |
| Detailed 1024×512 render, cold | 2.75 s | 4.10 s |
| Biome style 1024×512 render, cold | 2.77 s | 4.14 s |
| Studio pick (`sample_surface`) | 0.7 µs | 0.7 µs |
| Studio landmass region at 1024×512 | 9.4 ms | 11.5 ms |
| Prepared scene bytes (surface reported separately) | 59.9 MB + 63.1 MB surface | 59.9 MB + 252.0 MB surface |

`TectonicView` has **9,038 boundary segments at n=512** (the same count at
n=1024 for this seed); the overlay draws them as two-point segments, and no
decimation was needed. The Biome style render is within measurement noise of
the relief render; the canonical-cell lookup added no measurable render time.

Doubts:

- The Atlas debug test suite is much slower (6.5 s → ~160 s) because the
  golden world is now a real n=128 engine world at 384×192. A smaller golden
  would speed it up but would not exercise the production view.
- Epoch prepare builds two full `ToolSurface`s (epoch and year 0), about 63 MB
  each at n=512 and 252 MB each at n=1024. The year-0 surface is dropped after
  the structure controls are built, but peak memory is higher than E5a.
- Ice thickness removal is represented by ice presence at full glacial
  strength in `glacial_work_ppm`; the old thickness-scaled term cannot be
  reconstructed from the engine.
- Rendered desktop behavior, the Tauri studio tile path and the frontend
  removed-field cleanup are E7 work; this phase verifies the Rust library and
  command adapters through tests.

#### E6 notes

Implemented on `world-engine-v2` on top of E5b (`fde26ac`). Event
materialization now samples the engine world; no event path reads a
`DAENAPW1` world or the retired product fields.

- **One entry.** `engine::DecodedSource::sample_events(epoch, request,
  progress)` derives one world (`derive_world`, every source hash checked, for
  the present; `derive_epoch` for a non-zero solve), calls
  `events::sample_world_events(&world, request)` and drops the world before it
  returns. Earthquakes and eruptions always read the present world. Storms and
  floods use the supplied epoch solve, and a solve whose normalized epoch is
  zero falls back to the hash-checked present derive. The command solves the
  midpoint epoch with `history::solve_epoch` and the generation forcing, as the
  tool surface does.
- **Earthquake.** The per-cell rate is
  `earthquake_rate_nano(hazards.quake_rate_km2[c], cell_area_m2(c))`, the
  canonical M ≥ 4 rate. `Λ = mass / 1e9 · years`; the cell is drawn from a
  prefix sum with `partition_point`. `hazard_ppm` comes from `quake_hazard`,
  `annual_rate_nano` is the cell rate, `rate_per_million_years_ppm` uses the
  product helper, and `sampled_center_id` is absent. The rate mass equals the
  E4 `earthquake_rate_mass_nano` metric because that metric also sums canonical
  cube areas; both were checked against an independent sum.
- **Eruption.** Sources are sorted by id and weighted by
  `round(eruptions_per_yr · 1e9)`. The event cell is `source.cell`, the
  sampled center is the engine `u64` source id, hazard comes from
  `volcanic_hazard`, and magnitude from the source rate. A world with no
  sources returns zero events; the old `sample_cell` cell-0 fallback is gone.
- **Storm.** The genesis weight is
  `genesis_km2 · coarse area km² · 1e9`; the month is drawn from
  `genesis_monthly_km2` (all-zero weights are an error, not month 0); the track
  is `worldgen::trace_storm`. Each coarse step is mapped to the canonical cell
  that contains the coarse cell centre, and consecutive duplicates are
  dropped. `hazard_ppm` is the maximum carried intensity in ppm and
  `sampled_center_id` is the canonical genesis cell. The notable policy is
  `Λ = census · STORM_NOTABLE_FRACTION · years`; the constant is `3.24e-7`.
  Calibration: the retired 384×192 generator at seed 831429 reports
  `mean_ocean_storm_suitability_ppm = 55_708`, so its old notable rate was
  `55_708 / 1e6 · 0.00035 = 1.94978e-5` per year. The engine Earthlike seed
  831429 census at `n = 512` is `60.219666` storms per year, so
  `1.94978e-5 / 60.219666 = 3.23778e-7` reproduces the old rate; `3.24e-7` is
  0.07 % away. The `n = 1024` census is `59.784407`.
- **Flood.** Candidates are every fresh engine lake with `outflow_m3 > 0`,
  sorted by `minimum_cell` and truncated to `maxEvents`. `cell` is
  `spill_cell`, the center is `minimum_cell`, the track is
  `[minimum_cell, spill_cell]` (or one cell when equal), fill is `1_000_000`
  because the lake is at its spill level, `year_offset` is the interval
  midpoint, and `expected_lambda` is the candidate count before truncation.
  The materialization key uses `products::HYDROLOGY_DERIVATION_VERSION` (3), so
  old flood keys never match; existing entities stay as authored history.
- **Ids and JSON.** `MaterializedEvent.sampled_center_id` is `Option<u64>` and
  serializes as a decimal string, or `null` when absent. Deserialization
  accepts the string and rejects a bare JSON number. Cells and track cells stay
  `u32`, with debug assertions against the canonical cell count. The
  TypeScript type is `sampledCenterId: string | null`; no consumer does
  arithmetic on it.
- **Provenance.** `sampledCenterId` is written as the decimal string. Two
  fields are new: `cellKind: "canonical-cube"` and `sourceResolution: n`.
  `generatorId` is `daena_physical::GENERATOR_ID`, `generatorVersion` is
  `WORLD_ENGINE_VERSION`, and `sourceRetryIndex` is gone because a v2 source has
  no retry index. Flood `featureId` is
  `hydro_claim::canonical_lake_id(minimum_cell)` and resolves with
  `hydro_claim::resolve_canonical` on the epoch `ToolSurface`; the fallback
  point is the spill-cell centre. Track points come from
  `DecodedSource::cell_center_microdegrees`, not a view grid.
- **Labels and versions.** The earthquake label stays
  `poisson-gutenberg-richter-v2`: the Poisson draw and Gutenberg–Richter
  magnitude model did not change. The eruption label is
  `engine-volcanic-source-v3` (a source is drawn, not a cell), the storm label
  is `engine-storm-tracer-v3` (census plus the shared tracer, not a
  climatology suitability draw), and the flood label is
  `engine-lake-spill-v3` (an engine spill cell, not a basin threshold).
  Materialization versions are earthquake 2, eruption 2, storm 3, flood 3, and
  `EVENT_MATERIALIZATION_VERSION = 3`. `HAZARD_DERIVATION_VERSION = 2` and
  `VOLCANIC_SOURCE_DERIVATION_VERSION = 2`; both are already in
  `derived_cache::version_dir` (`z` and `v`) and in `historical_cache_key`
  (`hazards-v`, `volcanic-v`), so old caches miss and refill. This was checked
  by reading both key builders, not by a new test.
- **Legacy.** `ValidatedPhysicalSource.legacy_world` and
  `ValidatedPhysicalSource::legacy_world` had no readers after the command
  moved, so both are deleted. The command no longer has the old static-derived
  event path. The remaining `TectonicWorld` names in `src-tauri` are test-only
  helpers of the retired generator.
- **Tests.** Fourteen events unit tests (two restored in review: the Poisson
  ensemble mean against the eruption rate mass and the Gutenberg–Richter
  M ≥ 5 share) cover determinism and byte-identical
  JSON, bounds, each kind's ids and fields, earthquake and eruption rate mass,
  the M ≥ 5 → M ≥ 4 conversion through the product helper, tracer parity, the
  coarse/canonical mapping at `n = 128` (where the climate grid is 64), zero
  mass worlds, the all-zero month error, u64 JSON beyond `2^53`, and a
  tampered source rejected on a normalized-zero epoch. One command test
  accepts an `n = 64` engine map, materializes floods and earthquakes, checks
  provenance, `cellKind`, engine generator fields and the canonical flood
  anchor resolving on the epoch surface, reruns floods for no new entities, then
  deletes `.daena/` and reopens with the events intact.
- **Measured timings** (release, 12-thread M4 Pro, seed 831429, Earthlike,
  interval −100000..100000, `max_events = 128`, median of three, sample only;
  source generation excluded):

  | Kind | Epoch | n=512 | n=1024 |
  | --- | --- | ---: | ---: |
  | Earthquake | present | 0.68 s | 1.75 s |
  | Storm | −8000 | 0.49 s | 1.29 s |
  | Flood | −8000 | 0.50 s | 1.28 s |

  The storm and flood rows pass an explicit −8000 solve, as the brief asks;
  the command derives the epoch from the request midpoint. The first run of
  each measured 1.04 s (earthquake, `n = 512`) and 1.78 s (`n = 1024`); later
  runs settled at the table values. No row is near 5 s.

Doubts:

- Earthquake `expected_lambda` over the full interval is about `3.0e9` events
  (`≈ 15,000` M ≥ 4 events per year × 200,001 years). The Poisson draw clamps
  the count and `maxEvents` truncates to 128, but the reported mean stays the
  unbounded rate mass. This is the existing `expected_lambda` contract.
- The generation JSON now records `hazardDerivationVersion = 2`. The first
  draft made `validate_source` reject an E5b map that records `1`, which would
  have locked every accepted engine map out after any hazard change. Fixed in
  review: the recorded value is the version at accept time, hazards are always
  re-derived at the current version (the derived cache and epoch keys carry
  it), so `1..=current` validates and only `0` or a future version is
  rejected. The physical identity does not include it.
- The storm census is resolution-dependent (`60.219666` at `n = 512`,
  `59.784407` at `n = 1024`), while the notable fraction is one constant
  calibrated at 512. The product rate therefore moves by about 0.7 % between
  the two accepted resolutions.

#### E7 notes

Implemented on `world-engine-v2` on top of E6 (`c0e6bfb`). The frontend now
speaks the engine contract; no UI path reads a removed product field.

- **Contract test.** `src-tauri/src/tests.rs`
  `physical_frontend_contract_fixture_matches_the_rust_json` generates a small
  `n = 64` engine world and writes the key/kind shape of the generation JSON,
  the climate, hydrology and hazard builders, an `AtlasSurfaceSample` and a
  materialized event to `src/lib/project/physical-contract.fixture.json`.
  Regeneration is explicit: `DAENA_UPDATE_PHYSICAL_CONTRACT=1 cargo test
  --manifest-path src-tauri/Cargo.toml --lib
  physical_frontend_contract_fixture_matches_the_rust_json`. The typed mirror
  `src/lib/project/physical-contract.ts` is checked by `svelte-check` against
  the TS interfaces; `scripts/maps-physical-contract.test.mjs` compares both
  shapes. A renamed Rust field, a removed TS field or an extra TS field fails
  one side or the other. The fixture intentionally injects one basin, river and
  volcanic source when the small world has none, so element shapes are never
  empty.
- **Generation form.** Preset (`earthlike`, `frozen`, `hothouse`, `desert`,
  `archipelago`, `water-world`, `pangea`, `shattered`, `custom`) and resolution
  (512 or 1024) replace terrain age, continent layout and retry index. Named
  presets send `targetLandFractionPpm = 0`; only `custom` sends the land
  fraction control. The wording says the generated world is the accepted
  world and gives the measured times (about 2 s at 512, about 4 s at 1024).
- **Removed paths.** The evolution client calls and both Rust stub commands are
  gone. Humidity and currents disappear from the editor, raster painter,
  native-vector editor, Atlas style registry and studio samples. Stored
  physical layer lists are filtered on read (`dropRetiredPhysicalLayers`), so
  an old `currents` or `humidity` layer does not reach a toggle.
- **Hazards.** `project_physical_hazards(jobId)` and
  `project_physical_derived_hazards(mapEntityId)` mirror the climate/hydrology
  pair. `engine_hazard_products` now also writes `longitudeMicro` and
  `latitudeMicro` for each volcanic source, derived from the canonical cell, so
  the frontend places markers by coordinate and never treats `cell` as a view
  index. Earthquake and volcanic ppm rasters and the source markers share the
  same 1024×512 product view as climate and hydrology.
- **Biome legend and readout.** The editor parses the Rust `#rrggbb` fill once
  per legend and renders all 26 classes. The click readout is the class name
  plus sampled annual temperature, precipitation and aridity; no per-class
  reason text is invented. Atlas keeps its ten-group biome style and its label
  now says "Biome groups".
- **Canonical indices.** A Node test scans the map frontend for the canonical
  hydrology fields (`minimumCell`, `spillCell`, `sourceCell`, `mouthCell`) used
  as array indices and for a volcanic canonical cell used as a view index. No
  such use exists; new ones fail `check:maps:physical`.
- **Region labels.** `ToolSurface` caches landmass and watershed size ranks in a
  `OnceLock`, computed from the surface once per epoch. `propose_region_canonical`
  labels the largest landmass "Landmass 1" and the largest watershed
  "Watershed 1", while `region_id` stays the stable canonical cell or receiver
  root. Tests call each region twice and check label stability and that rank 1
  has the larger cell count.
- **Events.** The materialize message now shows the event model label and the
  epoch or interval. `sampledCenterId` remains a string in the TS type and no
  frontend parser reads it; old entities with numeric provenance values still
  render through the generic entity view and their stored document.
- **Check scripts.** `maps-physical.test.mjs` drives the engine CLI with
  `--resolution`/`--preset`, checks `DAENAPW2`, the engine GeoJSON and the exact
  v2 record size. `maps-atlas.test.mjs` builds an engine source and drops the
  removed humidity style. The Atlas CLI renders its production 2048×1024 view,
  so the script asserts that size instead of the retired 128×64 harness size.
  Measured on this machine (seed 831429, Earthlike): n=64 source 98,652 bytes
  (348-byte header plus 4·6·64² elevation bytes) and 49,197 GeoJSON bytes;
  n=512 source 6,291,804 bytes and 100,249 GeoJSON bytes (2.20–2.46 s across
  the three runs).

Doubts:

- `check:maps:atlas` runs the full `daena-atlas` suite. One amplification test
  has a 5 s wall-clock budget and flaked once under parallel load; it passed
  standalone and on a second full run.
- Rendered desktop behaviour (Tauri webview, native file dialogs, actual
  generation clicks) is not exercised here. The done message lists the manual
  checklist.
- The Atlas studio sample's `climate` string remains "biome (group)"; the
  studio still title-cases it as the Biome row. It is a display label, not a
  contract field.

Review fixes:

- Engine biome names reached the UI as snake_case keys ("tropical_rainforest")
  in the legend, the Find Place explanations and the Atlas inspector.
  `products::biome_legend` and `products::biome_name` now return display names
  ("tropical rainforest"); `Biome::name` stays the harness key.
- The editor's click readout indexed the south-origin product view from the
  north, so a northern click read the mirrored southern cell (this predates E7,
  but E7 added hazard values to the same readout). It now uses
  `physicalViewIndex` from `native-vector/coordinates.ts`, and
  `maps-physical-canonical-index.test.mjs` checks its row order.
- The Atlas divergence label used a ±20 ×10⁻⁶ s⁻¹ neutral band. Typical surface
  values are 1–10 ×10⁻⁶ s⁻¹, so almost every cell read "Neutral". The band is
  now ±1 ×10⁻⁶ s⁻¹.
- Still open for E8: the climate legend's `reason` field is the name again and
  no consumer reads it.

#### E8 notes

Implemented on `world-engine-v2` on top of E7 (`96e3afe`). E8 deletes the
retired generator, moves the cancellable source hashing into the engine, bounds
the tool-surface cache by bytes and sets the budget constants from measured
envelopes. It changes no physics: generated worlds are byte-identical to E7.

- **Deleted.**
  - The `daena-physical` modules `tectonics`, `evolution`, `climate`,
    `hydrology`, `hazards` and `resolution`.
  - The `climate-profile` bin.
  - The view-grid `find_place`, `suggest_routes`, `propose_region` and
    `claim_at` paths and their id formatters.
  - `GenerationSettings`, `generate_world*`, `GeneratedWorld` and
    `LegacyDerivedPhysics`.
  - The old product JSON builders in `physical_commands.rs` and the
    `TectonicWorld` test helpers.
  - `PRODUCTION_{DEFAULT,MAX}_{WIDTH,HEIGHT}`, `DEFAULT_WIDTH/HEIGHT` and
    `MAX_WIDTH/HEIGHT`.

  `crates/daena-physical/src` went from 37,326 to 14,173 lines; the branch
  diff against E7 is 877 insertions and 24,418 deletions.
- **Kept or moved.**
  - The derivation versions now live in `products.rs`; their values are
    unchanged: climate 3, hydrology 3, hazard 2, volcanic source 2.
  - The thermal-expansion clamp moved into `history.rs`, byte-identical to the
    deleted `hydrology::thermal_expansion_delta_m3`.
  - `epoch-profile` was ported to the engine (`--resolution`, `--preset`,
    `--offset-years`).
  - `engine::PRODUCT_RESOLUTIONS` (`[512, 1024]`) is the production resolution
    constant.
  - Cache keys, the source magic and version checks are unchanged.
  - A `DAENAPW1` source still fails with `physical.source.unsupported-version`
    (tests in `engine.rs`, `lib.rs`, `daena-core` `maps/physical.rs` and Atlas).
- **Hashing.**
  - `worldgen::World::generation_hashes_with_cancel(elevation_mm, cancelled)`
    is the one implementation. It polls `cancelled` every 65,536 values and
    once per lake, river and volcanic source.
  - `generation_hashes` calls it with a never-cancel closure, and the per-field
    `hash_bits` of climate, hydrology, biome and hazards delegate to the same
    functions.
  - The hazard stage still computes `stats.hazard_hash` in two parts,
    overlapped with the extremes pass. `tests/hazards.rs` checks that it
    equals `Hazards::hash_bits`.
  - `generation_hash_values_are_pinned` pins an n=32 world's six hashes to
    literal values.
  - `daena_physical::engine::source_hashes_with_cancel` is now a thin adapter
    that maps `WorldError::Cancelled` to `PhysicalError::Cancelled`.
- **Tool-surface memory.**
  - `TOOL_SURFACE_CACHE_BYTES = 640 MiB` replaces the two-entry cap.
    `ToolSurface::heap_bytes` measures 63.2 MB at n=512 and 252.1 MB at
    n=1024, so a present surface and one epoch surface fit at either
    resolution. A surface larger than the budget is not cached.
  - Atlas builds the year-zero structure pass from `AtlasControlInputs`,
    without a `ToolSurface`.
  - No surface field was narrowed: none was proven lossless for every engine
    value.
  - Asset import and export now use their own
    `MAX_ASSET_TRANSFER_BYTES = 128 MiB` in `transfer.rs`. They are no longer
    tied to `MAX_SOURCE_BYTES`.
- **Identity gate.**
  - Compared against a `git archive` build of `96e3afe`.
  - Byte-identical: the `physical-map` CLI source and derived GeoJSON (seeds
    831429 and 4 at n=64 and n=512, Earthlike; seed 4 Pangea at n=64), and the
    generation JSON, hydrology, hazards and source bytes from the command
    builders (seed 831429 at n=512 and n=1024; seed 4 Earthlike and Pangea at
    n=64).
  - Find Place, routes and regions on the n=64 surfaces, and Find Place at
    −21,000 years, are byte-identical too.
  - The climate JSON differs only by the removed legend `reason` field.
  - The artifacts come from the ignored `e8_identity_artifacts` test in
    `src-tauri/src/tests.rs` (`DAENA_E8_ARTIFACTS=<dir>`).
- **Recovery.** Covered by the existing command tests:
  - `accepted_engine_source_rebuilds_and_rejects_an_external_checkpoint_edit`;
  - `engine_cache_rebuilds_after_the_derived_directory_is_deleted`;
  - `physical_event_materialization_uses_engine_events_and_survives_rebuild`,
    which deletes `.daena/` and reopens with the events intact.

  The physical identity, source hashes and products are compared after the
  rebuild.

**Budget table** (release, 12-thread M4 Pro, seed 831429, Earthlike; RSS from
`/usr/bin/time -l`; "live" is the peak memory footprint with
`MallocLargeCache=0`, see "Performance results" for why RSS includes the malloc
cache):

| Measurement | n=512 | n=1024 | Constant | Headroom |
| --- | ---: | ---: | --- | --- |
| Generation, engine plus product adapters (CLI `generationMs`) | 2.39–2.47 s | 4.16–4.23 s (ignored test, 3 runs) | `GENERATION_TIME_BUDGET_MS = 8_000` | 1.9× |
| Generation peak RSS / live | 532 MB / 328 MB | 1.33 GB / 0.95 GB | `WORKING_MEMORY_BUDGET_BYTES = 1.5 GiB` | 1.2× RSS, 1.7× live |
| Source bytes (`DAENAPW2`, 348 + 24·n²) | 6,291,804 | 25,166,172 | `MAX_SOURCE_BYTES = 32 MiB` | 1.33× |
| Derived GeoJSON bytes, worst of the sweep below | 236,372 | 242,827 | `MAX_DERIVED_GEOJSON_BYTES = 4 MiB` | 17× |
| Top-level GeoJSON features / region rings, worst | 342 / 730 | 427 / 932 | `MAX_GEOJSON_FEATURES = 4_096` | 4.4× |
| Coastline segments on the 1024×512 view, worst | 8,334 | 8,820 | `MAX_COASTLINE_SEGMENTS = 65_536` | 7.4× |
| `static.bin` derived cache, worst | 97.0 MB | 97.6 MB | `MAX_CACHE_BYTES = 196 MiB` | 2.1× |
| `ToolSurface::heap_bytes` | 63.2 MB | 252.1 MB | `TOOL_SURFACE_CACHE_BYTES = 640 MiB` | present plus one epoch |
| Epoch derive at −8,000 years (`epoch-profile`, mean of 3) | 1.18 s | 2.24 s | – | – |
| Atlas epoch prepare at −8,000 years (`e8_prepare_peak`, 1024×512 request) | 2.84–3.00 s, 1.16 GB RSS / 0.59 GB live | 4.62–4.80 s, 2.12 GB RSS / 1.64 GB live | – | over the working budget, see doubts |
| Event sampling | see the E6 table | see the E6 table | – | – |
| Hash cancellation, worst lateness over a 0–90 % deadline sweep | – | 14.8 ms (hash 48 ms) | `CANCELLATION_LATENCY_BUDGET_MS = 100` | 6.8× |

The "worst" rows come from a 24-world sweep: Earthlike, Frozen, Hothouse,
Desert, Archipelago, Water world, Pangea and Shattered, at seeds 831429 and 4
for n=512 and seed 831429 for n=1024, each at the present and at −21,000
years. The ignored tests
`e8_generation_meets_the_time_budget_at_1024` and
`e8_hash_cancellation_stays_under_the_deadline_budget` assert the time and
cancellation budgets in release. The memory budget is checked with
`/usr/bin/time -l` as above, not in-process.

Review fixes:

- **The feature cap broke generation.** Workhorse lowered
  `MAX_GEOJSON_FEATURES` to 4,096 from one seed's 290 features, but the same
  constant also bounded coastline segments. Seed 4 at n=512 (Earthlike) then
  failed with "engine coastline exceeds the feature budget", and half the
  sweep has more than 4,096 segments. Coastlines now have
  `MAX_COASTLINE_SEGMENTS`. `coastline_segments_have_their_own_limit_above_the_feature_cap`
  generates an n=128 world with more segments than the feature cap.
- **Two hash copies in worldgen.** The plain path called the new chunked code,
  but the per-field `hash_bits` still had their own copies, used by the
  harness reports and the hazard stats. They now delegate to the single
  implementation. The old "cancellable matches plain" test could no longer
  fail; the pinned-value test is the lock.
- **The elevation was rebuilt on every hash call.** Each call rebuilt
  `source_elevation_mm()` (25 MB at n=1024) although every caller already held
  it. The elevation is now passed in.
- **The cancellation sweep was flaky.** It cancelled at 100 % of the hash time,
  which races the hash finishing; it failed once the baseline and the
  measured path differed. The sweep now stops at 90 %, and the baseline is the
  same cancellable path.

Doubts:

- **Atlas epoch prepare is over the working budget.** At n=1024 it peaks at
  1.64 GB live (2.12 GB RSS) against the 1.5 GiB working budget, which is set
  from generation. Building the year-zero pass without a `ToolSurface` did not
  move that peak (E7 measured 1.64 GB live too). The peak is the epoch world
  derivation plus the Atlas scene, so the surface overlap was never the cause.
  Reducing it needs Atlas to drop the world before building its scene. That is
  follow-up work, not a budget change.
- **Windows malloc retention was not measured**; all figures are macOS.
- **The source limit has little headroom.** `MAX_SOURCE_BYTES` leaves 1.33× over
  an n=1024 source. A future n=2048 setting (100.7 MB) would need an explicit
  contract change.

## Known realism gaps, deferred to after E8

These were measured against Earth data at `n = 512` (Earthlike seeds 1–7) and
are not fixed in realism batch 1. The numbers and sources are in
`.agents/realism-check.md`, which is not tracked.

**Islands.** Large islands should come from tectonics: island arcs on
subduction zones and continental fragments rifted off a margin. Two attempts
looked artificial: first as random blobs in open ocean, then as islands strung
along the plate boundaries in straight lines. The root cause is that plate
boundaries are smooth, straight-edged Voronoi edges, so anything tied to them
inherits that shape. Revisit islands after boundaries become curved and
irregular, and perhaps after adding plate history, so arcs and fragments can
follow realistic arcs, ridges and transforms. Oceanic arcs and hotspots still
produce only small islands, and island counts stay below Earth's 18 islands
above 100,000 km² and 57 above 10,000 km².

**Climate.** Four gaps remain:

- **Cold land winters at 30–40°.** The engine's coldest month is −7.8 to
  +0.6 °C (mean −3.2) against Earth's +2.3, and 52–89 % of that land is below
  0 °C against Earth's 33 %. Summers are 1–5 K too warm (24.5–28.2 °C
  against 23.4), so the seasonal range is 27–33 K against 21 K. At 20–30° the
  coldest month is +3.2 to +9.0 °C against +13.7. The cause is that land
  couples to the ocean only through zonal mixing and heat capacity;
  temperature is not advected, so westerly ocean air never reaches continents
  in winter.
- **Too little hot desert and rainforest.** `BWh` is 1.3–4.4 % of land
  against Earth's 15.2 %, and `Af` is 0.3–3.6 % against 5.7 %; all of `A` is
  4.5–23 % against 21.8 %. The arid area is Earth-like or larger, but the
  winter bias classifies it as cold desert (`BWk` 4.4–22 % against 3.7 %).
- **Dry high latitudes.** Land at 60–70° gets 132–287 mm/yr against Earth's
  423. The storm-track eddy flux fades to zero at the pole, and cold air
  holds little vapour; part of this follows from the winter bias.
- **No east–west coast asymmetry.** The east-to-west coastal rain ratio at
  15–35° is 0.84–2.60 (median 1.22) against Earth's 3.07, and arid shares in
  the west/middle/east thirds of continents are about 49/38/42 % against
  Earth's 78/58/41 %. Wind, subsidence and evaporation depend only on
  latitude, with no per-basin subtropical highs and no cool eastern-boundary
  currents.

These need the phase-B climate work: an implicit upwind-ocean relaxation for
maritime winters, then recalibration of the dry belts, the eddy profile and
the subtropical highs.

**What remains after this batch.** Land above 1 km is still below Earth's
22 % on some Earthlike seeds, and rift lakes are mostly round (`L²/A` below
the 8–20 target) because erosion rounds the segmented basins. Lakes still
cover 2.3–5.9 % of land with 6–17 lakes over 100,000 km² (Earth: 1.8 % and
one): most of the remaining giants are arid closed basins on flat continental
interiors. The outlet rule calls a basin arid from its summed runoff minus
evaporation; routing's own lake-balance closure test would be the consistent
criterion, and erosion has no sediment infill to fill flat basins.
On some seeds thin, one-cell sea channels remain along continental rift
traces (seed 5 at `n = 512`): rift-segment axes or carved outlets that end
below sea level and flood from the ocean.

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
