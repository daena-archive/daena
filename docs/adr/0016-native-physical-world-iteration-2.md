# ADR 0016: Native physical-world tectonic scaffold

- Status: Implemented hard-cut source contract and diagnostic layer slice
- Date: 2026-08-15
- Scope: Initial pure-Rust slice of Iteration 2 in `NATIVE_MAP_GENERATOR.md`

## Decision

Replace the physical provider's v1 spike contract with a deterministic,
tectonic v2 source behind the physical generator boundary. Backward-compatible
reading and dual-format acceptance are explicitly out of scope.
The scaffold produces:

- approximately even spherical plate sites from a Fibonacci-style sequence;
- complete cell-to-plate ownership with the existing longitude-wrap and pole
  adjacency policy;
- per-plate synthetic rotation axes, angular speeds, and crust types;
- pairwise convergent, divergent, or transform boundary classifications based
  on relative normal motion with a fixed tie threshold;
- continental and oceanic crust fields, including submerged continental
  shelves as a distinct future derivation concern; and
- causal relief terms for crust baseline, boundary motion, restrained detail,
  hotspots, and subduction-arc centers.

The active provider tuple is now `daena-physical` / adapter `2` /
`physical-world-v2`, with generator version `5`. The v2 source includes the
signed terrain field, target/sea-level provenance, per-cell plate ownership and
crust, plate motion metadata, boundary classifications, and volcanic centers.
The decoder is strict about counts, widths, bounds, and exact total length.
There is no v1 reader or dual writer in the active path.

The derived diagnostic collection exposes coastline, tectonic plate cells,
bathymetry cells, boundary lines, and volcanic-center points without changing
the canonical source bytes. The physical-map surface presents the non-base
layers as locked, read-only visibility toggles; authored map editing remains
separate from these generated diagnostics.

The generator version is `5`: version `3` introduced the deterministic
neighboring-cell placement for subduction-arc centers, version `4` named and
separated the subsystem seed schedule, and version `5` adds craton-grown
continental crust plus causal rift-shoulder and spreading-ridge relief. The
source adapter and `physical-world-v2` codec remain unchanged.

The seed schedule is explicit in the pure-Rust boundary: plate sites,
continental cratons, rotation axes, relief detail, hotspots, climate, erosion,
hydrology, and hazards each have a named derived domain. The latter four are
reserved for their future derivation stages so adding their draws cannot alter
earlier tectonic output. Continental crust is grown from multiple spherical
craton seeds using geodesic distance and correlated low-frequency variation;
the per-cell crust array is authoritative, so continental shelves are
continental crust below sea level rather than a relabeled land mask.

The v2 boundary-classification threshold is the versioned constant
`25,000 nanoradians/year`. It remains outside the locked header and payload;
reversal-invariance tests prove that swapping a boundary's endpoints does not
change its physical classification.

Generation progress uses the locked eight-phase vocabulary from
`NATIVE_MAP_GENERATOR.md`. The current tectonic slice reports the phases that
have real work (`Building tectonic structure`, `Building terrain`, `Calculating
water`, `Preparing geography`, and `Validating world`); climate, erosion, and
river/lake phases remain reserved until those derivations are implemented.

The native job handle is scoped to the active project session, expires after 15
minutes, and is cancelled on project close, open, workspace replacement, app
shutdown, component teardown, or a newer generation. Stable `errorCode`
values are returned with failures and cancellation. The measured reference
budget is recorded in `docs/maps/physical-map-budgets.md`; native-rendered
fixtures remain an environment-dependent gate rather than an inference from
source tests.

The cross-target determinism decision is option (a): the supported target
matrix is locked to macOS arm64, Linux x64, and Windows x64, with exact source
and coastline hashes enforced by
`docs/maps/physical-map-golden-targets.md` and its CI workflow. The current
implementation still uses double-precision spherical calculations, so the
matrix gate must pass on every runner before those targets are declared
supported; approximate matches are never accepted.

## Source layout

The v2 header is 68 bytes, little-endian, and retains the `DAENAPW1` magic:

| Field | Width |
| --- | ---: |
| Magic, version, header length | 8 + 2 + 2 |
| Grid width, height, radius | 4 + 4 + 8 |
| Seed, retry index, target land fraction, sea level, sample count | 4 + 4 + 4 + 4 + 4 |
| Plate count, continental plate count, tectonic activity, island activity | 2 + 2 + 4 + 4 |
| Boundary count, volcanic-center count | 4 + 4 |

Payload order is fixed: signed elevation samples (`i32` millimetres), plate
ownership (`u16` per cell), crust kind (`u8` per cell), plate records (31
bytes), boundary records (21 bytes), and volcanic-center records (11 bytes).
The decoder rejects unsupported enum values, invalid references, count
overflow, trailing bytes, truncation, and sources over 16 MiB.

## Validation

The current slice is covered by pure-crate tests for:

- one valid plate and crust assignment per spherical cell;
- non-duplicated wrapped boundary topology and all three motion classes;
- positive and negative relief, continental/oceanic separation, and volcanic
  center creation; and
- same-input stability plus retry-index divergence.

The v2 source round-trip and `check:maps:physical` fixture cover durable plate,
boundary, crust, and volcanic-center bytes. The diagnostic GeoJSON test covers
layer presence, read-only layer identifiers, and bounded output. Native
rendered tectonic fixtures remain deferred; the focused surface contract check
covers local-only sources, locked diagnostic layers, editor disposal, and
active-job teardown but is not a rendered-app proof. The exact cross-target
golden gate is now defined by the matrix workflow; its first successful run
remains an exit-gate requirement. The metrics fixture covers plate
area distribution, boundary-class counts and coverage, continental crust,
exposed land, and submerged shelf area,
elevation percentiles, trench/ridge separation, volcanic-arc offset, and land
component bounds.
