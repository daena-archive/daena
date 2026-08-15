# ADR 0021: Native physical-world hazards and bounded natural history

- Status: Implemented hazard and bounded event-materialization slices
- Date: 2026-08-15
- Scope: Initial Integration 7 slice in `NATIVE_MAP_GENERATOR.md`

## Decision

Integration 7 begins with deterministic derived hazard fields. The accepted
physical source remains the only canonical physical asset; hazards are
recomputed from its validated plate boundaries and persistent volcanic
centers. The source bytes, terrain, plate assignments, boundary
classifications, and volcanic-center metadata are never modified. Per-center
origin and activity class remain authoritative in that canonical source;
derived hazard samples intentionally persist combined hazard/rate values
rather than duplicating center provenance on every sample.

The hazard derivation is versioned as `HAZARD_DERIVATION_VERSION = 2`. The
earthquake field combines seeded background rate, boundary kind, relative
boundary speed, and a geodesic exponential distance decay from a bounded set
of strongest boundary sources. The volcanic field combines seeded background
rate, divergent/convergent/transform boundary influence, persistent hotspot or
subduction-center intensity, and geodesic distance decay. The exposed values
are normalized relative/generated rates in parts per million over a million-
year model interval; they are not real-world predictions. The independent
background component is keyed by geography seed, not retry index; retries
change the accepted causal tectonic structure without applying a second
background shift.

Derived GeoJSON exposes bounded strongest samples in two locked layers:
`earthquake-hazard` and `volcanic-hazard`. Each feature carries the model
label, normalized hazard value, and normalized rate provenance. The native and
standalone physical editors label these layers as generated hazard and keep
them read-only. Hazard samples are disposable; deleting/rebuilding derived
output does not touch the source.

Historical epoch products reuse the immutable hazard field. The historical
cache key includes the hazard derivation version so a hazard-model change
recomputes the product. Reopened responses expose the version and model label.

## Natural-event materialization

Explicit materialization uses `EVENT_MATERIALIZATION_VERSION = 1` and a
request with a named `hazardSeed`, an inclusive interval bounded to +/-100,000
years, and `maxEvents` bounded to 128. The seed is independent of the
geography seed and is part of the durable provenance. Earthquakes use a
bounded Poisson occurrence draw over persistent rate cells and a capped
Gutenberg-Richter-style magnitude draw. Eruptions use the persistent volcanic
rate field with the separate `persistent-rate-v1` event model. Aftershock
sequences and predictive interpretation are explicitly excluded.

Sampling and source validation complete before persistence. Accepted results
are normal revisioned Daena entities with readable names, markdown documents,
the `daena.maps:physical-event-on-map` relationship, and a canonical
`maps.locations` point pin. Provenance stores the request ID, request bounds,
hazard and materialization versions, model label, source hash, generator
identity/retry, cell coordinates, rate, and `prediction: false`. A single
receipt-fingerprinted core batch mutation commits all entities together; the
core validates the shared `maps.physicalChronology` contract before commit.
Replaying a request returns the same entity identities, while reusing its ID
with different inputs returns a conflict. The native UI retains the request ID
for a retry after an uncertain response. Derived hazard output and opaque
physical source bytes never own the event records.

Timeline and Lore are optional consumers of this shared entity/field contract.
The Maps manifest declares the physical natural-event entity type and shares
`maps.locations` and `maps.physicalChronology`; the
module context exposes these through `fields.listShared`, whose host bridge
filters the result to fields explicitly declared `shared: true` by the owning
manifest. Timeline recognizes
the versioned physical chronology and shows the event in its relative-items
surface without converting the offset to a fabricated Gregorian date. Lore
continues to consume the normal entity and relationship graph. Disabling
either module cannot remove or invalidate the durable event or its map link;
Timeline still reads the shared chronology field when Maps navigation is absent.

## Validation

Pure Rust tests prove deterministic bounded fields, seam-safe point output,
qualitative ordering for boundary kinds/speeds and volcanic center classes,
deterministic bounded event sampling, and the generated hazard layer contract.
Core tests prove one-batch identity replay and request-input conflicts. Tauri
and client checks prove that reopened/generated GeoJSON includes the hazard
layers, the cache key includes the hazard derivation version, physical hazard
layers remain locked/read-only in both editor surfaces, the event command is
registered with the public client contract, and the shared Timeline chronology
adapter is present.
