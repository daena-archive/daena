# ADR 0013: Native Vector Maps source, coordinates, and host surface

- Status: Accepted
- Date: 2026-08-14

## Context

Daena already has a provider-neutral Maps domain: one `daena.maps:map` entity,
`maps:map` / `maps:layers` fields, content-addressed source assets, and a
trusted host surface that dispatches FMG (child webview) and Image Maps
(Konva in the main webview). Native Vector Maps must extend that model without
creating a second identity system, storage root, or plugin surface.

MapLibre and Terra Draw are capable render/edit libraries, but they are not
durable authorities. Coordinates in GeoJSON are longitude/latitude ordered,
while existing Maps anchors stay normalized to `[0, 1]`.

## Decision

1. **One GeoJSON source asset.** A Native Vector Map (`provider.id:
   daena-vector`) stores base geography and authored features in a single
   RFC 7946 FeatureCollection asset (`application/geo+json`). Layer metadata
   stays in `maps:layers`. MapLibre sources, style layers, and Terra Draw's
   feature store are disposable projections of that asset.

2. **Anchor conversion stays outside storage.** Canonical geometry keeps
   `[longitude, latitude]`. The existing Maps `[0, 1]` anchor contract is
   converted at the adapter boundary:

   ```text
   x = (longitude + 180) / 360
   y = (90 - latitude) / 180
   ```

   Adapter version 1 rejects persisted `defaultView` centers outside the Web
   Mercator latitude limit rather than clamping them.

3. **Trusted host surface.** `NativeVectorMapEditor` runs in the main Tauri
   webview beside `ImageMapEditor`. It is not a plugin child webview. MapLibre
   uses the CSP bundle, an explicit same-origin worker URL, `worker-src
   'self'`, and a local style with no tiles, glyphs, sprites, or telemetry.
   FMG remains isolated in its child webview.

## Consequences

Phase 0 may only add dependencies, a local fixture editor, CSP worker
plumbing, and this ADR. Public RPC, descriptor variants, and SQLite-backed
acceptance begin in Phase 1. Packaged desktop checks must still prove
open/close leaves no MapLibre instance, Terra Draw listener, object URL, or
worker owned by the closed editor.
