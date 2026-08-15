# ADR 0036: Atlas Rendering iteration-4 derived drainage and cache

- Status: Accepted
- Date: 2026-08-15
- Scope: topology-affecting minor tributaries, atlas-only feature IDs,
  core-owned disposable disk cache

## Decision

Iteration 4 adds a second geographic product, **derived drainage**, that is
versioned independently from the visual elevation residual. Visual residual
stays on detail algorithm version `1` and must not create, remove, or reroute
rivers. Derived drainage uses:

| Contract                         | Locked value |
| -------------------------------- | ------------ |
| Derived drainage version         | `1`          |
| Seed domain                      | `minor-tributaries` |
| Renderer version                 | `5`          |
| Promotion to canonical geography | not supported |

Every derived tributary has a stable ID `atlas:tributary:v1:{sourceCell}`.
The source cell is a canonical-grid index. IDs and source coordinates do not
depend on output dimensions, style, format, tile order, thread count, or cache
presence. A later year may omit a tributary when ice, ocean, or lake covers the
source; it must not reuse that ID for a different cell.

Routing uses canonical elevation and the current epoch hydrology. A tributary
may only occupy cells that share the parent river's `watershed_id` and, when
the join cell has a real basin, the same `basin_by_cell` label. Downhill noise
cannot cross a watershed. Derived tributaries never invent lakes, islands, or
canonical `RiverSegment` identities. They are atlas-only display geometry.
Promotion into authored map layers is an explicit future mutation and is not
implemented.

Disk cache lives at the core-owned disposable path `.daena/cache/atlas/`
already named by ADR 0031. `daena-atlas` accepts an explicit cache directory
from the caller; it does not discover project roots. Entries are:

- `residual`: epoch-independent elevation residual;
- `drainage`: epoch-dependent derived tributaries;
- `artifact`: encoded PNG plus provenance for an identical normalized request.

Writes use a sibling `.part` file and rename, refuse symlinks, and store
`DAENAATL` headers with payload SHA-256. Truncated, wrong-version, or
checksum-mismatched files are misses: they cannot crash a render or alter
pixels. LRU eviction keeps the directory within 512 MiB, 64 entries, and
160 MiB per entry. Deleting the cache must not change source, preset, or
checkpoint bytes.

## Consequences

- Repeating a render can reuse residual, drainage, and encoded artifacts.
- Cache deletion rebuilds the same tributary IDs and the same pixels.
- Style packs and tributary promotion remain deferred.
