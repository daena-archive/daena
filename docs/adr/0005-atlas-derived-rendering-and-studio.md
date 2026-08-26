# ADR 0005: Atlas-derived rendering and Studio

- Status: Accepted
- Decided: 2026-08-15
- Consolidated: 2026-08-26

## Context

Atlas must turn an accepted map into detailed cartography for interactive
inspection and export without becoming a second map authority. Geographic
detail must remain stable across tiles, output sizes, worker schedules, cache
state, and export formats, while authored overlays and labels come from one
captured project generation.

## Decision

Atlas is a deterministic derived consumer of an immutable validated map
snapshot. It refines geography and composes presentation but does not mutate
the map or physical source. The physical map constrains continents, oceans,
mountain systems, watersheds, river mouths, lakes, ice, and other canonical
facts. Refined terrain, coastline detail, drainage, hillshade, labels, and
thematic paints are disposable Atlas products.

Geographic detail is generated in world space from canonical inputs and a
versioned algorithm contract. Tile coordinates, output dimensions, format,
device scale, traversal order, concurrency, and cache state cannot move a
geographic result. Algorithm replacement is a new explicit version; removed
experimental or production versions are not silently revived.

Atlas Studio uses local Web Mercator XYZ tiles with a north-origin row
convention and OpenLayers for interaction. A Studio session captures one
project generation, physical epoch, style, layer set, and detail contract.
Later project changes make the session stale; they do not mix generations
inside existing tiles. Studio state such as viewport, hover, selection,
panels, and tile cache is not project content.

Physical and Atlas-derived features remain read-only in Studio. Authored map
features use the shared Maps authoring model; Atlas does not define a second
editor or persistence path. Calendar years are converted through the map's
explicit calendar binding rather than host date APIs.

Static export captures the same immutable input and composition model as
Studio. It renders supported regional or whole-world outputs directly rather
than screenshotting the viewport. Output capabilities are advertised only
when the encoder exists and is validated. Rendering creates a temporary
application-owned artifact; saving to an external destination and registering
an export as a project asset are separate explicit actions.

Styles are declarative, versioned, bundled offline resources. Relief,
political, biome, temperature, precipitation, bathymetry, hydrology, and other
styles select presentation from the same captured data; they do not create
new canonical climate, biome, or hydrology products. Authored overlays,
semantic content, and labels are composed deterministically above derived
geography.

Atlas caches live in application-owned derived storage. Cache keys bind all
inputs that affect the result. Cache deletion, corruption, or absence may
affect performance but not output identity or canonical data. Local tile and
artifact delivery uses bounded opaque authority and never exposes filesystem
paths to plugins or web content.

## Rejected alternatives

- Persisting Atlas detail or Studio tiles as map authority.
- Deriving geography independently per tile or output resolution.
- Using viewport screenshots for static export.
- Allowing style selection to rewrite physical climate or terrain.
- Querying live project state during a render and mixing revisions in one
  artifact.

## Consequences

- Studio and export share deterministic geography and composition semantics.
- A stale session must be refreshed explicitly before it can include later
  authored content.
- Every export includes bounded provenance for the captured map generation,
  physical identity and epoch when applicable, style, layers, detail contract,
  and output settings without local paths or secrets.
- Tile seams, traversal order, cache removal, concurrency, overlay order,
  labels, epoch selection, formats, and native save behavior are required
  verification boundaries.

## Decision history

- 2026-08-15: deterministic static rendering, authored composition, regional
  and print output, derived drainage, and disposable caching were accepted.
- 2026-08-15: Studio adopted local XYZ tiles, captured sessions, OpenLayers
  interaction, explicit staleness, and shared static-export requests.
- 2026-08-15 to 2026-08-16: terrain, coastline, orometry, drainage, erosion,
  and renderer experiments were promoted through explicit versions; older
  versions ceased to be production contracts.
- 2026-08-16: biome and thematic styles were added as presentation over
  existing physical fields.
- 2026-08-26: rendering, Studio, and algorithm iteration records were
  consolidated here. [`MAPS.md`](../MAPS.md) remains the product authority.
