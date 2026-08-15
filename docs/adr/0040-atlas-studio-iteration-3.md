# ADR 0040: Atlas Studio iteration-3 terrain synthesis spike

- Status: Accepted for the experimental detail-algorithm spike
- Date: 2026-08-15
- Scope: separately selectable experimental detail algorithm `2` in
  `daena-atlas`, world-space control samplers, hierarchical amplification,
  and bounded mountain topology on the golden physical fixture. Production
  capabilities, Studio sessions, static export, renderer version `5`, and
  detail algorithm `1` remain unchanged.

## Context

Iteration 2 completed interactive composition on algorithm `1`. Iteration 3
in `docs/ATLAS_STUDIO.md` must not replace that algorithm in place. A new
version is allowed only as an opt-in spike that keeps version `1` fixture
hashes reproducible. No new crate (`noise-rs`, `geo`, `image`) is required
for this spike; the existing SHA-256 / splitmix64 seed policy and integer
bilinear samplers are sufficient.

There is no canonical biome product on the accepted physical source. The
spike derives a climate class from temperature, precipitation, and ice for
control sampling only.

## Decisions

### 1. Experimental algorithm remains hidden

| Contract | Locked value |
| -------- | ------------ |
| Released detail algorithm | `1` |
| Experimental detail algorithm | `2` |
| Seed-policy version | `1` (same PRF construction) |
| Renderer version | `5` |
| Production `AtlasRenderRequest` / Studio session `algorithmVersion` | `1` only |

`request.normalize`, Studio scene/session normalize, and
`AtlasRenderCapabilities` continue to accept and report only version `1`.
Algorithm `2` is invoked from the pure `daena-atlas` spike API and tests.
It is not listed in capabilities and is not selectable in Studio or the
export panel.

### 2. Seed isolation

Geographic PRF for algorithm `2` uses prefix `daena-atlas-detail-v2\0` and
the same length-prefixed identity, algorithm version, variant, and named
domain bytes as ADR 0031. Algorithm `1` keeps prefix
`daena-atlas-detail-v1\0`. Named domains for version `2`:

- `hierarchical-relief` — multi-octave residual
- `mountain-orometry` — bounded peak/saddle/ridge/valley identities

Output size, style, tile index, worker count, zoom, device scale, and
historical year stay out of geographic seeds. Residual millimetres do not
depend on epoch sea level, climate, ice, or hydrology.

### 3. Control samplers

`ControlFields` samples accepted products at arbitrary microdegree
coordinates:

| Field | Interpolation | Notes |
| ----- | ------------- | ----- |
| elevation, crust influence, temperature, precipitation, runoff, ice thickness, water level, lake level, mountain influence, sea level | bilinear, ppm weights | longitude wraps; latitude clamps; polar rows do not wrap in `j` |
| climate class, watershed id, crust class | nearest cell | categorical |

Crust influence is `1_000_000` on continental cells and `0` on oceanic
cells. Climate class is a derived control (`ice`, `tundra`, `arid`,
`grassland`, `forest`), not a stored biome layer. Mountain influence is
derived from convergent boundaries, volcanic centers, and the upper
quartile of continental elevation, then dilated by one neighbor.

### 4. Hierarchical amplification

Version `2` builds a world-space lattice at the requested detail factor,
then adds three octaves (`0`, `1`, `2`) at factors `max(1, f/4)`,
`max(1, f/2)`, and `f`. Coarser octaves are bilinearly sampled onto the
finest lattice. Polar octave rows and the composed polar lattice use
lattice index `0` (no bilinear mix with the adjacent latitude). Mean
removal may then add a per-canonical-cell offset along those rows.
Amplitude is conditioned by canonical elevation magnitude
and crust influence only.
The mean residual in each canonical cell is removed. The coastal sign
clamp from algorithm `1` (`COASTAL_ENVELOPE_PPM = 350_000`) still applies
when forming refined elevation.

### 5. Bounded mountain topology

On the golden fixture, one window of `16 x 12` canonical cells is chosen
by maximum mountain-influence sum, ties broken by smallest `(row, col)`.
Inside that window, only cells with positive mountain influence participate.
A divide-tree on the version-`2` lattice records at
most 64 features: peaks (strict local maxima), saddles (merge cells),
ridges (peak–saddle links), valleys (local minima on land), and foothills
(near ridges, below peak elevation). Feature IDs are stable
`atlas:orometry:v2:{kind}:{lattice-index}`. The `mountain-orometry` domain
key is mixed into the locked topology fingerprint. Canonical river and
watershed IDs are not replaced. Named residual samples at `(0,0)` and
`(12.345678°, -8°)` are locked in the crate tests.

### 6. Libraries and research code

No new dependency is added. The spike does not copy research
implementations. Orometry / divide-tree and hyper-amplification papers
remain architectural references only.

### 7. Budgets

The spike stays experimental and hidden. Lattice work is cancelled every
4096 rows. On the golden `64 x 32` fixture, `standard` amplification must
finish in under 5 seconds and keep the residual lattice under 2 MiB. It
must complete in-process without allocating a full export framebuffer.

## Consequences

- Version `1` static hashes, drainage, and Studio tiles are unchanged.
- Version `2` geography is world-addressed and epoch/style invariant.
- Enabling the version in capabilities requires a later ADR, conservation
  evidence on the production `384 x 192` grid, and an interactive budget
  measurement.
