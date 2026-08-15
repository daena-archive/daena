# ADR 0025: Native physical-world Packet 2 tectonic cause chain

- Status: Implemented Packet 2 generator slice
- Date: 2026-08-15
- Scope: Packet 2 of the production corrective sequence in `NATIVE_MAP_GENERATOR.md`

## Decision

Replace the Iteration 2 nearest-site tectonic scaffold with a deterministic
cause chain in `crates/daena-physical-spike/src/tectonics.rs`. The
`physical-world-v2` layout is unchanged. The generator version is `8` because
continent layout style is drawn from the craton seed and changes final crust
and elevation; no new persisted records were added.

Plate ownership is a spherical multi-source cost field. Plate sites remain an
approximately even Fibonacci-style sample. Cell assignment uses geodesic edge
lengths plus a named-seed, low-frequency correlated cost perturbation applied
before ownership is finalized. Extracted boundaries are never jittered
afterward. Every cell has exactly one plate; plate adjacency is reciprocal;
longitude wrap and polar connectivity use the existing grid topology.

Boundary segments are unique orientation-independent cell pairs. Classification
evaluates both plates' Euler-pole velocities at the segment midpoint, projects
relative velocity into tangent-plane normal and tangent components, and uses
the versioned `25,000 nanoradians/year` threshold with a deterministic
normal-versus-tangent tie rule. Reversing a pair does not change the physical
class.

Continental crust grows from grouped cratons by priority expansion. A
seed-derived layout class chooses one mega-continent, two major continents, or
many smaller ones. Each class has its own attraction, repulsion, plate-crossing
cost, terrane budget, and per-group area shares. Cost terms remain geodesic
distance relative to craton radius, correlated lithology, plate-crossing,
same-group attraction, other-group repulsion, and occupied crust. Detached
terranes use `SeedDomain::DetachedTerranes` and the layout's bounded area
budget. Crust type is independent of sea level; plate crust metadata is a
majority label after growth.

Initial relief is the rounded sum of named signed fields: crust baseline,
collision uplift, trench subsidence, inland volcanic arc, oceanic arc,
ridge/age bathymetry, rift floor, rift shoulders, transform minor relief,
hotspot uplift, and restrained detail. Kernels use geodesic distance to the
appropriate boundary side. Hotspot chains follow the owning plate's Euler axis
backward with monotonically decaying intensity. Whether a center is an island
remains solely `elevation > sea level` after the water solve.

Temporary cost, group, and named-relief arrays are not written to the source.
Diagnostic GeoJSON remains derived.

## Validation

Packet 2 fixtures cover complete ownership, reciprocal adjacency, pair-reversal
invariance, seam/pole topology, non-Voronoi boundary morphology, craton-group
connectedness, terrane bounds, submerged shelves, cause-field accounting,
controlled cross-section signs, hotspot age/direction, and mega/dual/scattered
continent layout gates. The generator
version bump is the intended golden-fixture change for the v8 source and
derived hashes in `scripts/maps-physical.test.mjs`.
