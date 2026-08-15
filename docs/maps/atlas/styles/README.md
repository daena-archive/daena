# Atlas style schema (iteration 1)

Bundled styles are declarative JSON. They cannot contain JavaScript, shaders,
filesystem paths, remote URLs, CSS, HTML, or executable expressions.

Required keys: `id`, `version`, `title`, `fontId`, `paperGrainPpm`, palette
triples, `background`, `defaultLayerIds`.

Unknown fields are rejected. `fontId` must be `daena-atlas-bitmap-5x7` until
iteration 2 introduces hashed TTF files and offline shaping.
