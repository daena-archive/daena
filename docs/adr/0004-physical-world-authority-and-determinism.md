# ADR 0004: Physical-world authority and determinism

- Status: Accepted
- Decided: 2026-08-15
- Consolidated: 2026-08-26

## Context

A generated world exposes many visible products: terrain, tectonics, climate,
water, ice, coastlines, drainage, and hazards. Persisting those views as
independent truth would allow them to drift apart, make historical playback
ambiguous, and make safe editing indistinguishable from changing the simulated
world.

## Decision

An accepted physical map owns one strict, versioned physical source containing
the canonical planetary field and the causes and settings needed to validate
the world. The source is immutable after acceptance. Generation and bounded
repair happen in temporary state; cancellation or failure creates no map, and
acceptance atomically publishes one complete validated source.

Physical derivation follows a causal order: planetary geometry and seed,
tectonics and crust, terrain, climate and runoff, erosion and drainage, water
and ice, coastlines and hydrology, then hazards. A downstream visible class
does not replace its upstream cause. The production implementation is
deterministic for the same canonical inputs and declared contract versions.
Unsupported source versions are rejected rather than reinterpreted through a
hidden compatibility path.

Resolution, memory, source bytes, derived bytes, geometry, and execution are
bounded product contracts. Limits are justified by validated production
envelopes and may change only with measured evidence and an explicit contract
change. Presentation-level simplification may not alter canonical physical
identity.

Physical time is a signed offset from the world's reference epoch. Epoch
derivation may change climate, water, ice, coastlines, rivers, lakes, biomes,
and hazards while retaining stable world identity and epoch-independent
causes. An optional calendar binding performs explicit chronology conversion;
host date semantics and an assumed year zero are not authoritative.

Hazards are bounded rate or probability fields. They do not become authored
history automatically. A user may explicitly materialize a selected natural
event into normal Timeline and Lore records, after which it follows ordinary
project persistence and revision rules.

Derived physical geometry is read-only and disposable. Authored vector,
raster, semantic, and relationship-backed layers remain separate canonical
content above the physical base. Editing those layers does not rewrite the
source. Editing a physical shape requires an explicit snapshot into authored
GeoJSON; the detached copy no longer follows future derivation or epoch
changes.

## Rejected alternatives

- Treating generated GeoJSON, rasters, contours, or hazards as independent
  canonical stores.
- Mutating an accepted physical source through normal map editing.
- Keeping old source versions alive through implicit readers or reinterpretive
  migration.
- Selecting a hidden best world from many candidates instead of presenting the
  deterministic result and explicit repair outcomes.

## Consequences

- Rerolling produces temporary candidates; replacing an accepted world means
  explicitly accepting a new map identity.
- Derived epoch and presentation products may be deleted and rebuilt without
  changing canonical project data.
- Algorithm changes use explicit versions and do not silently reinterpret an
  accepted world.
- A clean portable checkpoint must preserve the accepted source, authored
  overlays, bindings, and materialized events, but not derived render products.

## Decision history

- 2026-08-15: feasibility work established deterministic generation and the
  immutable-source/derived-product split.
- 2026-08-15: the production source adopted a strict hard-cut format and added
  causal tectonic, climate, erosion, drainage, basin, contour, historical, and
  hazard derivations under one identity.
- 2026-08-16: production morphology and erosion refinements changed the
  generator contract without changing source ownership.
- 2026-08-26: phase and packet records were consolidated here; current product
  behavior is documented in [`MAPS.md`](../MAPS.md), and future work remains in
  [`PHYSICAL_WORLD_ROADMAP.md`](../PHYSICAL_WORLD_ROADMAP.md).
