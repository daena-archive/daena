# ADR 0017: Native physical-world current climate and runoff

- Status: Implemented pure-Rust derivation slice
- Date: 2026-08-15
- Scope: Iteration 3 in `NATIVE_MAP_GENERATOR.md`

## Decision

Add a deterministic current-climate and runoff derivation after initial
sea-level solving and before later terrain evolution. The result is a
disposable `ClimateField`; it is not added to the locked `physical-world-v2`
header or payload. Regenerating or deleting it cannot change canonical source
bytes.

The derivation version is `1`. Its versioned Rust settings include global
temperature, latitude cooling, altitude lapse rate, maritime moderation,
ocean moisture, physical-distance moisture-decay scale, wind-band convergence,
orographic response, and a hydrology preset. `Arid`, `Balanced`, and `Wet` presets adjust source
moisture, transport persistence, runoff base, and runoff response as a group;
they do not select a final river count.

## Model

Temperature is continuous centi-degrees Celsius and combines global
temperature, nonlinear absolute-latitude cooling, elevation lapse rate, and a
geodesic maritime factor. Maritime distance is the nearest ocean-cell
great-circle distance and decays exponentially over the configured scale.

Moisture is transported in six broad latitude bands. Equatorial and polar
bands move westward; mid-latitude bands move eastward. Each fixed traversal
uses wrapped longitude neighbors, ocean-cell source moisture, distance decay,
and a bounded convergence contribution from the adjacent equator-facing row.
The transport is a contractive fixed-point pass with a fixed row/column order,
96 iterations maximum, and a 1 mm/year convergence tolerance. This explicitly
defines wrap behavior and convergence when wind paths meet.

At each cell, precipitation is removed from incoming moisture. It combines a
versioned base fraction with an uphill/orographic term based on physical
distance to the upwind cell and is capped at 65% per transport step. The
remaining moisture produces the rain-shadow signal. All public derived fields
are non-negative integer millimetres per year or bounded integer temperatures;
internal floating-point calculations are rejected when non-finite.

Runoff is precipitation multiplied by an effective, temperature-adjusted
coefficient on land. Runoff volume is `runoff_mm / 1000 * exact_cell_area` and
is rounded only at the derived-field boundary. Global precipitation and runoff
volumes, temperature extrema, wet/dry cells, and transport iteration count are
reported as metrics.

The trusted host exposes the products on demand for both a completed temporary
job and an accepted physical map reopened from canonical source bytes. The
response contains the continuous arrays and metrics but is not stored as an
authoritative asset; the accepted-map path re-derives it after cache deletion.

## Validation

The pure-Rust fixture suite proves:

- uniform precipitation/runoff matches the analytic `4*pi*R^2` spherical
  total;
- temperature decreases from equatorial to polar rows and maritime factors
  remain bounded;
- coastal moisture decays toward the interior and hydrology presets change
  precipitation/runoff as coherent parameter groups;
- a controlled ridge creates windward precipitation and a measurable leeward
  moisture shadow;
- wrapped transport is periodic at the antimeridian;
- derived fields are deterministic, finite, bounded, and runoff is land-only;
- the controlled 16 x 8 fixture has the locked climate fingerprint
  `f6e0cca4b4e0add4`;
- the integrated generation result contains climate metrics while re-encoding
  the exact unchanged canonical source; and
- the Tauri product boundary exposes the same derived arrays for temporary and
  reopened maps without adding climate bytes to the canonical source; and
- cancellation remains observed through the existing progress boundary.

Native climate raster rendering, terrain evolution, final hydrology, and
historical climate remain later iteration work. This ADR therefore does not
claim the final world-generation exit gate.
