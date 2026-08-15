# Atlas rendering licenses

Iteration 0 of Atlas Rendering bundles no fonts, no third-party style packs,
and no remote assets. The spike renderer uses only:

- this repository's Apache-2.0 code in `crates/daena-atlas`;
- `daena-physical` (Apache-2.0, same repository);
- the `png` crate (`MIT OR Apache-2.0`) as a pinned CPU encoder;
- `sha2` (`MIT OR Apache-2.0`) for domain keys and source digests;
- `serde` / `serde_json` (`MIT OR Apache-2.0`) for compact provenance JSON.

No runtime URL is requested. Iteration 1 must record bundled font and style
file licenses before those files are added under `docs/maps/atlas/`.
