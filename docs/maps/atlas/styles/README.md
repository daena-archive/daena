# Atlas style schema (iteration 1)

Bundled styles are declarative JSON. They cannot contain JavaScript, shaders,
filesystem paths, remote URLs, CSS, HTML, or executable expressions.

Required keys: `id`, `version`, `title`, `fontId`, `paperGrainPpm`, palette
triples, `background`, `defaultLayerIds`.

Unknown fields are rejected. `fontId` is `daena-atlas-bitmap-5x7` with a hashed
glyph table. A licensed TTF/shaper is not required for iteration 2 labels.
