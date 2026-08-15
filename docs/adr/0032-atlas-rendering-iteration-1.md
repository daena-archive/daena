# ADR 0032: Atlas Rendering iteration-1 host slice

- Status: Accepted; first usable export is in tree
- Date: 2026-08-15
- Scope: capabilities, snapshot capture, bundled styles, PNG jobs, in-app panel

## Decision

Iteration 1 keeps PNG as the only format. JPEG remains unapproved. Atlas
capability reporting lives in `daena-core::maps::atlas` and is dispatched from
the validated map descriptor; the Svelte panel appears only when
`supported` is true.

The host captures an immutable snapshot (source bytes, opaque physical
identity, stored historical forcing, content generation) on a read connection,
then renders on a blocking worker without holding SQLite. Preview and export
jobs write application-owned PNGs under the app cache `daena-atlas/` directory.
Save uses the host dialog and an atomic sibling `.png.partial` replace. One
export may run at a time; a newer preview for the same map supersedes the
previous preview.

Styles `daena-atlas-relief` and `daena-atlas-antique` are bundled JSON. Label
shaping and TTF files remain iteration 2; overlay numerals use the Apache-2.0
bitmap id `daena-atlas-bitmap-5x7` reserved in the style files. Antique paper
grain uses a style-only seed domain that includes output dimensions.

Renderer version is `2`. Geographic detail algorithm version remains `1`.

## Remaining exit-gate evidence

`npm run check:maps:atlas` covers crate tests, core capability dispatch, atomic
PNG overwrite/symlink refusal, bundled style validation, and release PNG repeat
hashes for the golden `64 x 32` source. Packaged desktop renders at reference,
negative, and positive offset years, restart determinism from a captured
project generation, and host save-dialog overwrite/locked-file confirmation
still require a Tauri app session before calling Iteration 1 complete.
