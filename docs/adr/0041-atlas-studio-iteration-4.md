# ADR 0041: Atlas Studio iteration-4 refined drainage and erosion spike

- Status: Accepted for the experimental drainage/erosion spike
- Date: 2026-08-15
- Scope: bounded Priority-Flood-style pit policy, watershed-constrained
  continuous flow, atlas-only tributary identities, and multi-scale
  erosion/deposition on the experimental detail-algorithm `2` lattice.
  Production derived drainage version `1`, renderer `5`, Studio sessions,
  static export, and capabilities remain unchanged.

## Context

Iteration 3 locked experimental detail algorithm `2` as a hidden
amplification and orometry spike. Iteration 4 in `docs/ATLAS_STUDIO.md`
adds refined drainage and erosion on that lattice. It must not replace
canonical rivers, watersheds, mouths, lakes, or basins, and must not
enable a new product in Studio. No new crate (`noise-rs`, `geo`,
`image`) is required. Research papers are architectural references only.

## Decisions

### 1. Experimental product remains hidden

| Contract | Locked value |
| -------- | ------------ |
| Released derived drainage | `1` (`atlas:tributary:v1:{sourceCell}`) |
| Experimental refined drainage | `2` |
| Seed-policy version | `1` |
| Renderer version | `5` |
| Production `AtlasRenderRequest` / Studio session | algorithm `1`, drainage `1` only |

The experimental refined drainage product stays hidden. `request.normalize`, Studio scene/session normalize, and
`AtlasRenderCapabilities` continue to report only drainage version `1`.
The spike is invoked from the pure `daena-atlas` API and tests. It is not listed in capabilities and is not selectable in Studio or the export
panel.

### 2. Seed isolation

Geographic PRF for the spike uses prefix `daena-atlas-detail-v2\0` and
named domains sampled through the existing splitmix64 lattice PRF:

- `refined-drainage` — sampled at lattice points while marking channels
  and valley bottoms (threshold jitter `0..=2`)
- `multi-scale-erosion` — sampled per cell and scale to modulate flux
  (`0..=2.5%`)

Output size, style, tile index, worker count, zoom, and device scale stay
out of geographic seeds. Canonical river IDs are never reused.

Experimental tributary IDs are `atlas:tributary:v2:{lattice-index}`.

### 3. Intentional basins vs artificial pits

Canonical lake cells and hydrologically wet basins (`Endorheic`,
`Active`, `Overflowing`, or positive lake level above sea) are
**protected**, except where mountain influence is positive so recorded
peaks can remain local maxima. Priority-Flood-style filling seeds the ocean
and those protected cells as open outlets. Unprotected local minima on the refined
lattice may be raised to the spill elevation, capped at `4_800` mm.
Protected cells are never raised. Polar lattice rows stay constant in
longitude after fill and erosion.

### 4. Watershed-constrained continuous flow

Flow uses an integer two-neighbor steepest-drop model (D-infinity
reference, not a copy of Tarboton or Barnes code). Each land cell may
split to at most two downhill 8-neighbors. A neighbor is eligible only
when it shares the canonical `watershed_id`, or when it is ocean and the
current cell is a coastal cell of that watershed (including the canonical
mouth). Flow must not enter another watershed's land or change a
canonical `mouth_cell`.

### 5. Atlas-only tributaries and erosion

Channel cells above a detail-level accumulation threshold yield at most
128 atlas-only tributaries. Valley bottoms are channel cells that sit
below their non-downstream neighbors, capped at 64, with IDs
`atlas:valley:v2:{lattice-index}`. Paths stay inside the parent watershed
and join a canonical river or ocean. They do not replace `RiverSegment`
identities.

Two bounded erosion/deposition **scales** follow fill: a 2-hop pass along
the flow graph, then a 1-hop pass. Flux is slope- and accumulation-limited,
reduced on mountain influence below `500_000` ppm, and skipped on
protected lakes, ocean, poles, and mountain influence above `500_000` ppm.
After each scale the mean change inside every canonical cell is removed
(conservation correction), coastal sign outside the envelope is restored,
and mountain peak cells are re-enforced as local maxima within `80` mm.

Experimental identities encode as a version-`2` payload for round-trip
tests. Production `DerivedDrainage` decode rejects that version. The spike
does not write `AtlasDiskCache` drainage entries.

### 6. Libraries, cache, and budgets

No new dependency is added. The spike does not write production
`KIND_DRAINAGE` cache entries. Tests may round-trip identity and pixel
payloads through a temporary directory, then delete that directory and
rebuild; the rebuilt product must match. Lattice work, including
tributary and valley extraction, is cancelled every 4096 cells. On the
golden `64 x 32` fixture, `standard` refinement must finish in under 5
seconds, keep each in-process lattice under 2 MiB, and keep process peak
RSS under 512 MiB.

## Consequences

- Version `1` static hashes, tributary IDs, and Studio tiles are unchanged.
- Enabling drainage `2` in capabilities requires a later ADR, conservation
  evidence on the production `384 x 192` grid, and an interactive budget
  measurement.
