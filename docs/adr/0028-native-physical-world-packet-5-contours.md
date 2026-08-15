# ADR 0028: Native physical-world Packet 5 topology-preserving contours

- Status: Implemented Packet 5 generator slice
- Date: 2026-08-15
- Scope: Packet 5 of the production corrective sequence in `NATIVE_MAP_GENERATOR.md`

## Decision

Replace Iteration 5 per-cell-edge presentation geometry with interpolated
contour extraction in `crates/daena-physical-spike/src/contours.rs`. Hydrology
still owns basin, river, and water-state products; it does not implement
marching squares. The `physical-world-v2` layout is unchanged. The generator
version is `11` and `HYDROLOGY_DERIVATION_VERSION` is `4` because assembled
coastline, land/ocean/island/lake/shelf polygons, and bathymetric isolines
change disposable GeoJSON. `CONTOUR_DERIVATION_VERSION` is `1`.

Dual-grid vertices are cell centers. Column `width` is a ghost copy of column
zero so longitude wraps as an ordinary quad. Crossings interpolate in integer
microdegrees from the two scalar samples on an edge, then quantize. Ambiguous
`0101`/`1010` cells use the asymptotic decider on the bilinear saddle; a zero
denominator keeps the disconnected pairing. Polar faces are marching triangles
whose pole sample is the rounded mean of the polar row.

Segments join by stable edge identity, unwrap longitude while tracing, then
emit GeoJSON in `[-180, 180]`. Rings that span more than 180° of unwrapped
longitude are split at the antimeridian. Hole ownership uses spherical
containment of a ring sample, not planar bounding boxes. Geodesic
Douglas-Peucker simplification protects saddle vertices and rejects a
simplification that would change vertex count sign or drop a ring below four
points. River mouths and lake outlets snap to an analytic contour crossing on
the final drainage cell when one exists.

## Validation

Packet 5 fixtures cover all marching-squares cases, exact-threshold vertices,
ambiguous saddles, antimeridian wrapping, polar triangles, nested
island/lake rings, strait preservation, river-mouth snap-or-fail, and
self-intersection rejection. Host hydrology products keep derivation version
`4` and round-trip derived GeoJSON coordinates into the legal globe range.
The generator version bump is the intended golden-fixture change for the v11
derived hashes in `scripts/maps-physical.test.mjs`. Canonical source bytes
change only if later packets alter terrain; Packet 5 is a disposable-geometry
change.
