# ADR 0038: Atlas Studio iteration-1 host slice

- Status: Accepted for the first usable Studio
- Date: 2026-08-15
- Scope: provider-neutral Studio capability, runtime session wrapper,
  bounded session registry, application-controlled PNG tile protocol,
  capability-gated OpenLayers viewport, cache regeneration, and Export
  reuse. Epoch switching, extra styles, overlays/labels, prefetch, and
  current-view regional export remain deferred to iteration 2 in
  `docs/ATLAS_STUDIO.md`.

## Context

ADR 0037 locked XYZ tile math, north-origin rows, and the protocol
shape without a host. Iteration 1 must serve those tiles from an
immutable captured generation through Tauri without exposing project or
cache paths, and without reinterpreting `AtlasRenderRequest` or
`atlasPresets` as a pan-and-zoom contract.

## Decisions

### 1. Capability and session wrapper

| Contract | Locked value |
| -------- | ------------ |
| Studio session schema | `1` (`AtlasStudioSessionRequestV1` in `daena-core`) |
| Studio scene subset | `1` (`AtlasStudioSceneRequestV1` in `daena-atlas`) |
| Studio tile schema | `1` (`AtlasStudioTileRequestV1`) |
| Iteration-1 style | `daena-atlas-relief` |
| Iteration-1 epoch | physical offset `0` (reference) |
| Iteration-1 layers | `ocean`, `relief`, `ice`, `lakes` |
| Iteration-1 projection | `web-mercator` |
| Capability flag | `supportsStudio` on `AtlasRenderCapabilities` |

`supportsStudio` follows the existing provider-neutral Atlas gate. Svelte
must not hardcode `daena-physical`. Disabling Maps unmounts the Studio
entry; canonical maps and presets remain.

`AtlasStudioSessionRequestV1` is runtime-only. It is never persisted in
project files, presets, provenance, or checkpoints. Session and request
IDs never enter geographic seeds or tile bytes.

### 2. Session registry

| Limit | Locked value |
| ----- | ------------ |
| Idle expiry | 15 minutes |
| Sessions per app | 4 |
| Sessions per project/map | 1 live session |
| Tile workers | 1 (global mutex) |
| Waiting tile queue | 24 |
| Visible prefetch ring | OpenLayers default only; no extra ring |
| OpenLayers image requests | Browser-managed under the native wait queue |

Per-tile CPU time is measured in `docs/maps/atlas/budgets.md`. A full
queue returns `503` / `atlas.studio.resource-limit`; that is retryable
and must not replace the viewport with a sticky error banner.

Opening a session captures a snapshot on a read connection, then
prepares the shared scene without holding SQLite. Tiles reuse that
prepared scene. A newer open for the same project/map cancels the
previous session. Project close, database replacement, and app exit
cancel every session.

### 3. Tile protocol

Scheme `atlas-studio`. GET-like reads only. Path:

```text
/{sessionToken}/{z}/{x}/{y}.png?scale=1|2
```

The URL contains neither project paths, cache paths, nor renderer
internals. Successful responses are `image/png` with
`X-Content-Type-Options: nosniff`, `Cache-Control: no-store`, and CORS
headers so the shell origin (`http://localhost:1420` in `tauri dev`,
the packaged webview origin in release) can `fetch` tiles. `OPTIONS`
preflight is answered with those CORS headers and an empty body.
Writes, traversal, non-PNG suffixes, guessed tokens, cross-project
tokens, and methods other than `GET`/`OPTIONS` are
`atlas.studio.protocol.denied`. Expired sessions use
`atlas.studio.expired`. CSP allows only this scheme (and the Windows
`atlas-studio.localhost` form) in `img-src` and `connect-src`.

The main webview may use the URL. Plugin webviews must not.

### 4. Cache regeneration

`project_atlas_studio_regenerate_cache` deletes only validated files
under the core-owned `.daena/cache/atlas/` directory of the open
project. Callers cannot supply a path. Symlinks are refused. Canonical
files are untouched. In-memory sessions keep their prepared scene until
Refresh or close.

### 5. UI slice

Atlas Studio is a full-size OpenLayers XYZ viewport beside the Physical
Map when `supportsStudio` is true. Iteration 1 shows relief, reference
epoch, physical relief layers, loading/progress/error, cancellation on
close, Refresh Atlas (new session), Regenerate cache, cursor
coordinates, and Export. Export opens the existing `AtlasRenderPanel`
and existing Atlas Rendering jobs. It never screenshots OpenLayers.

Halo remains `0`. Authored/semantic overlays, labels, and style/epoch
controls wait for iteration 2.

### 6. Failure codes

In addition to ADR 0037 codes:

| Code | Use |
| ---- | --- |
| `atlas.studio.unsupported` | capability or module gate |
| `atlas.studio.stale` | captured generation is behind the project |
| `atlas.studio.expired` | idle expiry or cancelled session token |
| `atlas.studio.tile.failed` | encode/render failure after a valid request |
| `atlas.studio.protocol.denied` | method, path, token, or origin refusal |

Diagnostics must not leak paths, SQL, or source payloads.

## Consequences

- An accepted physical map can pan, zoom, and wrap longitude in Studio
  without changing canonical content.
- Static export hashes and renderer version `5` stay unchanged.
- Packaged Tauri inspection remains required for the protocol and
  native lifecycle; browser-only tests cannot prove this slice.
