# ADR 0042: Atlas Studio iteration-5 release hardening

- Status: Accepted for the release-hardening slice
- Date: 2026-08-15
- Scope: accessibility and keyboard operation, typed user-facing
  diagnostics, cache-control copy, provenance and derived-feature
  explanations, locked Studio tile fixtures, and current-view export
  alignment. Packaged desktop-target, display-scale, GPU/webview,
  offline-packaging, and upgrade/restart exercise is deferred and is
  not part of this checkout's exit gate. Production detail algorithm
  `1`, derived drainage `1`, renderer `5`, and static export hashes
  remain unchanged.

## Context

Iterations 0–4 shipped the XYZ tile contract, host protocol, interactive
composition, and hidden experimental terrain/drainage spikes. Iteration 5
in `docs/ATLAS_STUDIO.md` hardens the released Studio. Experimental
detail algorithm `2` and refined drainage `2` are not obsolete: they
stay hidden research spikes until a later ADR enables them with
production-grid conservation evidence.

## Decisions

### 1. Accessibility and keyboard

The Studio viewport is a focusable `application` region. With the map
focused:

| Key | Action |
| --- | ------ |
| Arrow keys | Pan (Shift for a larger step) |
| `+` / `-` | Zoom in / out |
| `Home` or `0` | Jump to the reference view (`0°, 20°`, zoom `1`) |
| `Enter` | Inspect features at the map center |
| `Escape` | Clear inspection |

Form controls in the aside keep normal Tab order and do not receive
those map shortcuts. A skip link moves focus to the map. `prefers-reduced-motion`
disables pan/zoom animation (MapLibre `fadeDuration` stays `0`). Buttons
and fields use visible focus rings.

### 2. Diagnostics

Host errors continue to use `code: message` strings. The UI maps known
`atlas.studio.*` codes to a title and a safe corrective action. Codes
and messages must not include project paths, cache paths, SQL, or source
payloads.

| Code | Title | Action |
| ---- | ----- | ------ |
| `atlas.studio.request.invalid` | This Atlas Studio request is not valid. | Refresh Atlas and use a supported style, epoch, and layer set. |
| `atlas.studio.tile.invalid` | That map tile request is not valid. | Pan or zoom back into the supported range, then retry. |
| `atlas.studio.resource-limit` | Atlas Studio is busy or at a resource limit. | Wait for visible tiles; a full queue is retryable and is not a sticky error. |
| `atlas.studio.cancelled` | Atlas work was cancelled. | Refresh Atlas if the map is still open. |
| `atlas.studio.unsupported` | Atlas Studio is not available for this map. | Enable Maps and open an accepted physical map. |
| `atlas.studio.stale` | The project changed after this Atlas session. | Refresh Atlas to capture the current generation. |
| `atlas.studio.expired` | This Atlas session expired. | Refresh Atlas to open a new session. |
| `atlas.studio.tile.failed` | Atlas Studio failed to draw a tile. | Retry. If it continues, regenerate the disposable cache. |
| `atlas.studio.protocol.denied` | Atlas Studio refused that tile request. | Refresh Atlas. Do not paste file paths into the map. |

Transient tile `503` / queue-full / CORS fetch failures stay silent in
the viewport overlay.

### 3. Cache controls and provenance copy

**Regenerate cache** remains a dedicated operation with no caller-supplied
path. The UI explains that it deletes only validated files under the
core-owned `.daena/cache/atlas/` directory and leaves canonical maps,
presets, and checkpoints unchanged. The control asks for confirmation
before running.

The aside shows a short provenance note:

- the Physical Map is canonical;
- Atlas geography is derived, deterministic, and disposable;
- released products are detail algorithm `1`, derived drainage `1`,
  renderer `5`;
- atlas-only tributaries inspected from Studio are derived and cannot be
  edited or promoted to canonical geography.

### 4. Golden Studio tiles and current-view export

Relief-only golden tiles on the `64 x 32` physical source stay byte-locked:

| Tile | Device scale | Locked PNG SHA-256 |
| ---- | ------------ | ------------------ |
| `z=0 / x=0 / y=0` | `1` | `sha256:92723037557c172c686383b160f6eb0b307d8b1bc93dc766fe8f123815b4b8ab` (`GOLDEN_TILE_Z0_SHA256`) |
| `z=8 / x=120 / y=90` | `1` | `sha256:7d8ff654abb8516a2e5730aff02d17be847ac29c21488f6e80900d9ccc7d13ba` (`GOLDEN_TILE_Z8_SHA256`) |

**Export** builds a regional Web Mercator `AtlasRenderRequest` from the
visible geographic extent through `current_view_export_request`: width
`2048`, height from the lat/lon span clamped to `256..=2048`,
`unlockAspect: true`, algorithm `1`, the session style, epoch, and
layers (still omitting `frame`). Browser pixels and session tokens never
enter the request. World-space samples at a named longitude/latitude
match between the Studio tile path and that export request.

Existing static whole-world hashes in `docs/maps/atlas/budgets.md` remain
the visual contract for Atlas Rendering.

### 5. Experimental paths are retained, not removed

Detail algorithm `2` and derived drainage `2` stay in `daena-atlas` as
hidden spikes. They are not listed in capabilities, Studio, or the export
panel. Removing them would drop conservation evidence without covering a
released product. A later ADR may enable or delete them.

### 6. Deferred packaged-target exercise

Exercising supported desktop targets, display scales, GPU/webview
combinations, offline packaging, and app upgrade/restart is out of this
slice. Packaged Tauri inspection remains required before calling the
Studio complete on those hosts.

## Consequences

- Studio copy, diagnostics, and capability reporting describe the same
  released versions.
- Static export hashes and renderer version `5` stay unchanged.
- Experimental algorithm/drainage `2` remain test-only.
