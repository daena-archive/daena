# Atlas rendering licenses

The atlas renderer uses only:

- this repository's Apache-2.0 code in `crates/daena-atlas` and `crates/daena-core`;
- `daena-physical` (Apache-2.0, same repository);
- bundled style JSON under `docs/maps/atlas/styles/` (Apache-2.0);
- the reserved bitmap font id `daena-atlas-bitmap-5x7` (Apache-2.0, original
  5×7 glyphs in the renderer; no third-party TTF in iteration 1);
- the `png` crate (`MIT OR Apache-2.0`);
- `sha2` (`MIT OR Apache-2.0`);
- `serde` / `serde_json` (`MIT OR Apache-2.0`).

Iteration 3–5 experimental spikes are now the released detail algorithm `2`
and drainage `2` (ADR 0043). No extra crates were added. `noise-rs`, `geo`,
and `image` remain unused.

No runtime URL is requested. Styles are rejected if they contain `http://`,
`https://`, `javascript:`, `file:`, `<script`, or `shader`. Labels use the
bundled `daena-atlas-bitmap-5x7` glyphs; hashed TTF files are not required
for the released Studio.
