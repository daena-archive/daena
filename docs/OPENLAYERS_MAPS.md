# OpenLayers Maps Architecture

## Status

Accepted and implemented as a hard cut. OpenLayers is Daena's only in-window
2D map renderer and geometry interaction engine.

There is no legacy renderer path, renderer-state migration, dual rendering,
or compatibility adapter. Incompatible renderer-specific state is invalid.

## Architectural rule

Daena owns map identity, geometry, layers, styles, links, history, and
persistence. OpenLayers renders and edits the current Daena state.

Only Daena-owned plain data crosses the persistence boundary:

- canonical GeoJSON geometry and stable Daena feature IDs;
- Maps field descriptors and normalized anchors;
- Daena layer definitions and styles;
- raster asset identity and metadata; and
- entity links and relations.

OpenLayers maps, views, layers, sources, features, styles, interactions, and
event state are runtime objects and are never serialized.

## Runtime boundaries

`src/lib/maps/native-vector/openlayers-runtime.ts` owns the OpenLayers map
lifecycle for native vector, image, and physical-world views. It constructs a
Daena world projection, vector and image sources, layers, hit detection, and
the editing interactions.

The runtime uses OpenLayers `Draw`, `Modify`, `Select`, `Snap`, and `Translate`
directly. Draw tracing and snapping operate only on the current eligible Daena
features. A completed gesture becomes one Daena-owned history state; pointer
movement is not persisted.

`src/lib/maps/native-vector/openlayers-style.ts` converts Daena layer and
feature styles into runtime OpenLayers styles. It contains no remote resources
and is not a storage contract.

`src/lib/maps/atlas/AtlasStudioView.svelte` uses an OpenLayers `XYZ` source for
Daena's local Atlas tile protocol. Tile URLs remain session-scoped and
fail-closed. Atlas export continues to use the Rust CPU renderer and never
captures the interactive viewport.

## Coordinates

Daena's normalized anchor contract remains `[0, 1]` and renderer-independent.
The native vector source remains canonical GeoJSON. The OpenLayers runtime uses
a local Daena world projection with extent `[-180, -90, 180, 90]`; these values
are fictional world coordinates and do not imply Earth geography.

Imported raster images are static OpenLayers image sources placed in the same
map coordinate space. Their original assets remain independent of renderer
caches. Physical-world rasters use a full plate-carree extent without
Web-Mercator row resampling.

Atlas tiles intentionally retain Web Mercator because the native Atlas tile
protocol is an XYZ contract. This is a property of that derived tile product,
not a global Maps constraint.

## Editing and history

The current editing surface supports:

- point, line, polygon, rectangle, and freehand creation;
- feature selection, modifier-drag box selection, vertex modification, and translation;
- tracing and vertex, edge, and intersection snapping;
- selection deletion, duplication, layer reassignment, and feature naming;
- layer duplication with fresh stable IDs for copied features;
- canvas-native labels generated from Daena feature names;
- Daena-owned undo and redo snapshots;
- layer creation, duplication, deletion, rename, ordering, visibility, locking, opacity, and styles;
- raster backgrounds and full-world physical views; and
- Daena entity linking and navigation.

Continuous interactions update OpenLayers runtime geometry. Daena state is
updated only when the interaction completes, and persistence continues through
the existing revision-aware project commands.

## Offline and security rules

- The map runtime must not request public tiles, sprites, glyphs, styles, or
  telemetry.
- Atlas accepts only its local session-scoped tile protocol.
- Raster inputs use validated Daena assets or generated in-memory canvases.
- OpenLayers receives no filesystem, database, shell, or ambient Tauri access.

## Cleanup invariant

The previous rendering and drawing dependencies, CSP worker bundle, renderer
types, runtime, style generator, and renderer-specific source checks are
removed. New Maps work must extend the OpenLayers and Daena-owned model
boundaries above instead of recreating a compatibility layer.
