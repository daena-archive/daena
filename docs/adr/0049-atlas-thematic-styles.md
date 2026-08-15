# ADR 0049: Atlas thematic styles

- Status: Accepted
- Date: 2026-08-16
- Scope: add bundled styles for temperature, precipitation, bathymetry, and
  hydrology. Renderer version stays `11`. Elevation synthesis is unchanged.

## Context

ADR 0048 added a biome cover map. Studio still lacked the other standard
atlas plates that can be painted from accepted Physical Map fields without a
second generator: surface temperature, rainfall, ocean depth, and drainage.

## Decisions

### 1. Bundled styles

| Style id | Paint |
| -------- | ----- |
| `daena-atlas-temperature` | land ramp from nearest-cell temperature (`-35°C`–`40°C`) |
| `daena-atlas-precipitation` | land ramp from nearest-cell annual precipitation (`0`–`2800` mm) |
| `daena-atlas-bathymetry` | hypsometry with ocean-led palette and muted land |
| `daena-atlas-hydrology` | muted hypsometry; rivers, lakes, and watersheds on by default |

Files live under `docs/maps/atlas/styles/` as `*.v1.json`. Temperature and
rainfall reuse `landLow` / `landHigh` / `landPeak` as ramp stops. Bathymetry
and hydrology do not reinterpret climate class.

### 2. Capabilities

`bundled_style_ids()` is elevation, biome, temperature, precipitation,
bathymetry, hydrology, antique, political. Default Studio style remains
relief.

### 3. Authority

Temperature and precipitation are the selected epoch’s physical climate
fields. They are not new canonical rasters and must not be written back to
the Physical Map.

## Consequences

Thematic land fill is a style-id paint, same as biome. Relief PNG hashes are
unchanged. A later stored climate product can replace nearest-cell samples
without changing these style ids.
