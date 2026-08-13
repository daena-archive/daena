# Native Vector Maps Phase 0 spike record

- Status: dependencies, offline fixture editor, CSP worker plumbing, and ADR
  recorded. Packaged WebGL/lifecycle acceptance remains a desktop check.
- MapLibre: `5.24.0` CSP bundle (`dist/maplibre-gl-csp.js`) plus
  `dist/maplibre-gl-csp-worker.js` loaded through `setWorkerUrl` and
  `worker-src 'self'`.
- Terra Draw: `1.32.3` with `terra-draw-maplibre-gl-adapter@1.4.1`.
- Generator contour library: `d3-contour@4.0.2` (installed; unused until
  Phase 2).
- License notices: [`native-vector-licenses.md`](./native-vector-licenses.md).
- Fixture: [`native-vector-fixtures/phase0-land.geojson`](./native-vector-fixtures/phase0-land.geojson).
- ADR: [`../adr/0013-native-vector-maps.md`](../adr/0013-native-vector-maps.md).

Install with Deno, not npm:

```sh
deno install --node-modules-dir=auto npm:maplibre-gl@5 npm:terra-draw npm:terra-draw-maplibre-gl-adapter npm:d3-contour
```

## Renderer spike

`NativeVectorMapEditor` is a trusted host-surface editor, selected from both
Create map menus as **New vector map**. It loads the local fixture only. It
does not call `maps.vector.*` RPC, create an entity, or write a source asset.

The offline style has a background plus two GeoJSON sources (`daena-base`,
`daena-authored`). `transformRequest` rejects `http(s)` URLs that are not the
page origin. Glyphs, sprites, and tiles are omitted.

Terra Draw starts after `style.load`, loads only the active vector layer, and
is stopped, unsubscribed, and disposed on layer switch and component teardown.
`map.remove()` runs on teardown. The runtime tracks live editor instances and
revokes any object URLs it created (the spike currently creates none).

If WebGL2 context creation fails, the editor shows
`vector.renderer.unavailable` and does not start MapLibre.

## Resource measurements

Adapter version 1 budgets from `NATIVE_MAP_INTEGRATION.md`:

| Limit | Value |
| --- | --- |
| Source asset | 16 MiB |
| Features | 20,000 |
| Total positions | 200,000 |
| Positions in one feature | 20,000 |
| Freehand raw cancel | 8,192 |
| Freehand simplified | 2,048 |
| Vector layers | 64 |

Observed frontend bundle sizes from the locked packages (not runtime memory):

| Artifact | Size |
| --- | --- |
| `maplibre-gl-csp.js` | 948.9 KiB |
| `maplibre-gl-csp-worker.js` | 447.1 KiB |
| `maplibre-gl.css` | 68.4 KiB |
| Phase 0 fixture | 5 features / 2 vector layers |

Parse, canonicalization, save, and peak-memory numbers at the 20k-feature /
200k-position budgets require a packaged Tauri pass. They are not claimed by
this Node check.

## Remaining exit-gate evidence

A packaged `deno task tauri dev` build must still prove, in the real host
webview: offline MapLibre + Terra Draw create/edit/delete/layer-switch; CSP
worker load; WebGL2 failure copy; and that repeated map open/close leaves no
MapLibre instance, Terra Draw listener, object URL, or worker owned by the
closed editor.
