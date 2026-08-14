# Native physical map iteration 0 spike record

- ADR: [`0014-native-physical-world-iteration-0`](../adr/0014-native-physical-world-iteration-0.md)
- Status: implementation started; packaged desktop evidence remains open
- Pure spike: [`daena-physical-spike`](../../crates/daena-physical-spike)
- Focused gate: `rtk npm run check:maps:physical`

## Existing path audit

| Concern                     | Existing authority/path                                            | Iteration-0 result                                                                   |
| --------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| Map identity and descriptor | `crates/daena-core/src/maps.rs`, `MapDescriptor`, `validate_field` | Reused; no physical union added                                                      |
| Runtime source assets       | `ProjectStore` asset registration and `maps` namespace             | Reused later; spike writes only temp output                                          |
| Layers, anchors, locations  | `maps:layers`, `maps:locations`, normalized anchors                | Reused later; physical output is not an authored layer                               |
| Revisions and request IDs   | core mutation methods and asset transfer receipts                  | Reused later; no new RPC or mutation                                                 |
| Checkpoint/rebuild          | `docs/STORAGE.md`, `flush_checkpoint`, asset export                | Reused later; no project mutation in the spike                                       |
| Binary transfer             | `asset.read.begin`, `asset.replace.begin/commit`                   | Existing bounded channel is the later integration boundary                           |
| Provider dispatch           | Maps host/provider adapter selection                               | Physical provider remains unregistered until iteration 1                             |
| Native rendering            | trusted `NativeVectorMapEditor`, local MapLibre style/CSP worker   | Derived GeoJSON is compatible as an offline local fixture; packaged run remains open |

The current six-candidate vector path is `CANDIDATE_COUNT = 6` and
`generateCandidates` in `src/lib/maps/native-vector/generator.ts`, presented by
`NativeVectorGenerator.svelte`, accepted through `project.acceptVectorMap`, and
committed by `maps.vector.create.begin/commit`. Its eventual replacement is
deferred until the physical vertical slice passes iteration 1; existing
`daena-vector` maps remain unchanged.

## Contract-drift evidence

The active TypeScript generator, Rust validator, SDK source/declaration, phase
2 and phase 3 fixtures, and the Tauri vector-create test now all declare
Native Vector generator version `1`. `scripts/maps-native-vector-provenance.test.mjs`
fails if the SDK returns to `2 | 3` or any active fixture/test drifts.

## Spike fixture evidence

The default fixture uses `64 × 32`, seed `831429`, retry index `0`, and target
land fraction `300000` ppm.

| Metric                  |                                                            Recorded value |
| ----------------------- | ------------------------------------------------------------------------: |
| Sea level               |                                                               `158477` mm |
| Land fraction           |                                                          `0.299654074534` |
| Canonical source        |                                                             `8,240` bytes |
| Source SHA-256          | `sha256:6ecf77ded12723d9cec4343c416c90e73cee5328a3e8a5333c0726ed10d2b1a7` |
| Derived GeoJSON         |                                                           `341,082` bytes |
| Derived GeoJSON SHA-256 | `sha256:caf92dcc92c07d0bdcec70865c0ea4da9b25d444189cb31985ceadef7246eb31` |

The release-mode maximum fixture (`128 × 64`) measured `32,816` source bytes,
`1,406,527` derived bytes, `6,812` coastline features, and `310.0 ms`
generation on `aarch64-apple-darwin` with rustc `1.97.1`. Its source hash is
`sha256:d2002207d4785ebf9fd86b82aabf3073cd0f1e32919055c9b5293cb6b37cd1a0`
and its derived hash is
`sha256:f0e994b3d0dfab8c234ea94871dc233b238c5707fe2377447806bc3fc4cb898b`.
The sandboxed `/usr/bin/time -l` probe could not report peak memory because
the host denied its `sysctl kern.clockrate` query; the record therefore keeps
memory as an explicit open measurement rather than claiming it passed.

Follow-up release-mode measurement on 2026-08-15 succeeded on macOS arm64:
the default workload measured 380 ms wall time, 30.6 ms generator time, and
6.17 MiB peak RSS; the maximum measured 690 ms wall time, 386.7 ms generator
time, and 19.06 MiB peak RSS. The maximum cancellation fixture observed the
cancel flag below the locked 100 ms budget. The current v2 hashes and the
complete benchmark record are in `docs/maps/physical-map-budgets.md`.

The Rust tests prove exact source round-trip, strict trailing/truncation
rejection, whole-sphere cell-area coverage, wrapped distance, explicit pole
adjacency, monotonic sea-level selection, and bounded seam-safe GeoJSON. The
Node gate verifies the recorded hashes and that the derived output contains no
network URL and only bounded `LineString` coordinates.

## Open exit-gate evidence

- Run the same golden input on every supported CI target available for this
  repository; compare canonical bytes, not only metrics.
- Load the generated local GeoJSON in the packaged Tauri host with network
  disabled and exercise MapLibre projection, antimeridian continuity, pole
  behavior, WebGL failure diagnostics, and repeated open/close teardown.
- Add the physical source to production descriptor/provider dispatch only in
  iteration 1, after this gate is closed.
