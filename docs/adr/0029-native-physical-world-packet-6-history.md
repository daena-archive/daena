# ADR 0029: Native physical-world Packet 6 historical forcing

- Status: Implemented Packet 6 generator slice
- Date: 2026-08-15
- Scope: Packet 6 of the production corrective sequence in `NATIVE_MAP_GENERATOR.md`

## Decision

Replace ADR 0020's single bounded triangle wave in
`crates/daena-physical-spike/src/history.rs` with a versioned sum of three
independently persisted cosine components. Integer physical time evaluates:

```text
forcing(t) = sum(amplitude_i * cos(2π (t + phase_i) / period_i))
temperature(t) = reference + sensitivity * forcing(t)
```

Quantized integer cosine and exponential steps replace platform `cos`/`exp`.
Temperature at an epoch is reported relative to epoch zero so replay of the
accepted source is the reference climate.

`HISTORICAL_DERIVATION_VERSION` is `2`. The `physical-world-v2` layout, source
format, and generator version `11` are unchanged. Persisted generation
metadata now stores the three components, sensitivity, ice midpoint/width, and
thermal-expansion coefficient, so composite identity hashes change. Cell land
ice remains the hydrology inventory lock from current climate. The logistic
`land_ice_equilibrium_m3` value is a lagged diagnostic only. Thermal expansion
`delta_volume = alpha * remaining_ocean_mass * delta_temperature` is applied
inside the coupled water iteration, not by rewriting inventory before climate.

## Validation

Packet 6 fixtures cover integer cosine bounds, forcing continuity and first
derivative, ice lag hysteresis and timestep refinement, thermal-expansion
sign, water conservation, epoch-zero replay against the accepted field, and
cold/warm land-bridge, shelf, and lake differences. The host cache key includes
`history-v2` and every persisted forcing term. Canonical source and Packet 5
coastline hashes are unchanged.
