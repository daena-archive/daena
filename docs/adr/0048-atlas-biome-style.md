# ADR 0048: Atlas biome style

- Status: Accepted
- Date: 2026-08-16
- Scope: add bundled style `daena-atlas-biome` so Atlas can show climate
  classes as a separate map. Relief, antique, and political styles stay
  elevation or political paints. Renderer version is unchanged (`11`).

## Context

The Physical Map has no stored biome raster. Atlas already derives a climate
class (ice, tundra, arid, grassland, forest) from ice, temperature, and
precipitation for terrain *control*. The relief style paints hypsometry, so
deserts and forests at the same height look the same. Studio needs a second
map for cover, not a second generator.

## Decisions

### 1. Bundled style

| Contract | Locked value |
| -------- | ------------ |
| Style id | `daena-atlas-biome` |
| Version | `1` |
| File | `docs/maps/atlas/styles/daena-atlas-biome.v1.json` |
| Paint | nearest-cell climate class on land; ocean/lake/ice layers unchanged |

Palette keys `biomeTundra`, `biomeArid`, `biomeGrassland`, `biomeForest`
are required on every bundled style. Relief/antique/political do not use
them for land fill. Ice class uses the existing `ice` swatch.

### 2. Capabilities

`bundled_style_ids()` is relief, antique, political, biome. Studio and the
export panel list the new id. Default Studio style remains relief.

### 3. Authority

Climate class is derived from the selected epoch’s physical climate and ice.
It is not a new canonical biome product and must not be written back to the
Physical Map.

## Consequences

Style JSON for older bundled files gains the four biome keys, so relief
PNG provenance `style_hash` changes even though land fill stays
hypsometric. A later true biome product from the Physical Map would
replace the derived class without changing this style id.
