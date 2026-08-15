# Atlas style schema (iteration 1)

Bundled styles are declarative JSON. They cannot contain JavaScript, shaders,
filesystem paths, remote URLs, CSS, HTML, or executable expressions.

Required keys: `id`, `version`, `title`, `fontId`, `paperGrainPpm`, palette
triples including biome classes (`biomeTundra`, `biomeArid`, `biomeGrassland`,
`biomeForest`), `background`, `defaultLayerIds`.

Bundled styles: `daena-atlas-relief`, `daena-atlas-biome`,
`daena-atlas-temperature`, `daena-atlas-precipitation`,
`daena-atlas-bathymetry`, `daena-atlas-hydrology`, `daena-atlas-antique`,
`daena-atlas-political`.
