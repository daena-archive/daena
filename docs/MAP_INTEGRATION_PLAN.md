# Daena Maps Provider Integration Plan

## Status and purpose

This document turns the product direction in
[`MAP_OVERVIEW.md`](./MAP_OVERVIEW.md) into a phased implementation plan. It is
subordinate to the project-wide contracts in [`ARCHITECTURE.md`](./ARCHITECTURE.md),
[`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md), and
[`STORAGE.md`](./STORAGE.md).

The plan integrates Azgaar's Fantasy Map Generator (FMG) as Daena's first map
provider and adds [Watabou's Procgen Arcana](https://watabou.github.io/) as a
planned family of detail-map generators. FMG remains authoritative for
large-scale geographic creation, editing, and rendering. Watabou providers may
generate cities, villages, caves/glades, dungeons, dwellings, and regional
realms when their individual persistence and licensing constraints have passed
the provider spike. Daena owns stable map identity, entity links, semantic
overlays, search projections, navigation, and temporal/story context across
all providers.

The first release is successful when an author can create or open an FMG map
entirely offline, save it as canonical project content, link an FMG feature or
arbitrary location to an existing Daena entity, and navigate in both directions
without duplicating that entity.

## Current foundations and gaps

Daena already provides most of the required host foundations:

- stable entities, documents, namespaced fields, relationships, and assets;
- SQLite-authoritative runtime state with deterministic portable checkpoints;
- revision-aware mutations and explicit checkpoint/recovery barriers;
- broker-authorized plugin RPC, events, and services;
- sandboxed native child webviews for plugin UI; and
- first-party modules using the public plugin contract.

The map integration must extend those foundations rather than create a second
storage or identity system. The main gaps are:

1. a provider-neutral map and location-reference schema;
2. a bounded binary asset read/replace channel for sandboxed plugins;
3. stable adapters around FMG and each approved Watabou generator's internal
   model and UI lifecycle;
4. contextual commands/services for opening and focusing a map view;
5. derived spatial, layer, and temporal projections in the runtime database; and
6. packaged desktop tests for the real child-webview boundary.

## Architectural decisions

### 1. Maps are shared entities

Every map is a normal Daena entity with the qualified type `daena.maps:map`. It
has the same stable UUID, document, fields, relationships, assets, revision
behavior, and lifecycle as any other entity. There is no parallel `maps`
identity table and no provider-owned database.

The map's provider source is a native asset attached to that entity. For FMG,
the source is an opaque `.map` file stored below `assets/maps/` and registered
through the core asset model. An optional SVG or PNG preview is a separate
asset. Daena does not rewrite or normalize provider source bytes outside the
provider adapter.

This preserves both sides of the independence requirement:

- the `.map` file remains usable by FMG outside Daena; and
- entity knowledge and location references remain readable when FMG or the
  source asset is unavailable.

### 2. Semantic data stays outside the FMG source

Daena links, roles, layer membership, date ranges, and story metadata are stored
as canonical namespaced fields and relationships. They are not injected into FMG
notes, labels, or private save-file structures.

Only linked or annotated geography is represented semantically. Daena does not
eagerly duplicate every FMG burg, state, river, cell, or label into an entity.
An author may promote any provider feature or arbitrary location to a shared
entity when it becomes meaningful to the world model.

### 3. Provider adapters are replaceable

The Maps plugin owns a provider-neutral domain contract. The FMG adapter is one
implementation and is the only implementation required for the first release.
Provider-specific feature kinds and IDs may appear only inside an opaque
provider selector. Search, navigation, entity references, date ranges, and map
hierarchy use Daena-owned IDs and schemas.

Adding a future provider must not require rewriting existing entity or map
identities. A provider migration may translate selectors or retain them as
unresolved references, but it may not silently discard links.

### 4. FMG runs in an isolated plugin webview

FMG and its Daena wrapper run as a bundled, first-party sandboxed plugin in a
native child webview. They do not run in the main Svelte webview and receive no
raw Tauri, filesystem, shell, process, or unrestricted network access.

The package contains pinned, locally built FMG static assets and works with a
deny-by-default Content Security Policy. Runtime network access is not needed
for editing, saving, linking, or navigation. Host operations go through the same
broker, capability, session, project, revision, and request-ID checks used by
third-party plugins.

FMG is MIT-licensed, but every bundled release must retain its copyright and
license notice, record the exact upstream commit, and maintain a small patch
ledger. Upstream currently describes `.map` compatibility as a constraint while
it transitions from JavaScript toward TypeScript, so the Daena adapter must be
treated as a versioned anti-corruption layer rather than a stable upstream API.
See the [FMG repository](https://github.com/Azgaar/Fantasy-Map-Generator),
[upstream README](https://github.com/Azgaar/Fantasy-Map-Generator/blob/master/README.md),
and
[license](https://github.com/Azgaar/Fantasy-Map-Generator/blob/master/LICENSE).

### 5. Base geography and semantic overlays have separate ownership

FMG renders terrain, borders, rivers, labels, and its other native layers. Daena
renders semantic overlays such as linked entities, story locations, routes,
search results, and date-filtered annotations. The overlay renderer is inside
the sandboxed Maps webview so it can share FMG's viewport transform without
exposing FMG internals to the host shell.

The host sends only provider-neutral intents such as `open map`, `focus link`,
`show entity`, `set date`, and `apply result set`. The FMG adapter resolves
those intents into provider-specific selection and viewport operations.

### 6. Watabou generators provide linked detail maps

[Watabou's Procgen Arcana](https://watabou.github.io/) is a collection of
separate generators, not one interchangeable source format. Daena treats each
approved generator as its own provider adapter under a shared `watabou-*`
family. The initial candidates are:

- Medieval Fantasy City Generator for city and district maps;
- Village Generator for villages and small settlements;
- Cave/Glade Generator for caves, caverns, and natural clearings;
- One Page Dungeon Generator for dungeons; and
- Dwellings for buildings and floor plans.

Perilous Shores may later complement FMG for realm or regional maps, but it is
not required for the initial detail-map slice.

Watabou output fits Daena's normal map hierarchy: an FMG burg links to a shared
city entity, that entity may have a `daena.maps:detail-map` relationship to a
Watabou city map, and a building or dungeon entity may link to a still more
detailed map. Provider-specific IDs, seeds, tags, permalinks, JSON, or other
state stay inside the provider descriptor or opaque source asset; Daena entity
identity and semantic links remain provider-neutral.

No generator is bundled or framed from the public website by default. A
generator must first pass a source, license, offline-build, Content Security
Policy, export/import, deterministic-regeneration, and adapter-hook review. An
image or SVG export is a valid canonical map source when editable round-trip
state is unavailable, but the UI must label that map as generated/static rather
than promise lossless editing. Runtime dependence on `watabou.github.io`,
cross-origin framing, or unrestricted network access is not acceptable for the
offline desktop product.

## Canonical domain model

### Map descriptor

The `daena.maps` namespace on a `daena.maps:map` entity contains one `map` field
with a versioned descriptor:

```json
{
  "schemaVersion": 1,
  "provider": {
    "id": "azgaar-fmg",
    "adapterVersion": 1,
    "sourceFormat": "fmg-map"
  },
  "sourceAssetId": "018f89ec-25fc-7816-8b47-6f80905f2868",
  "previewAssetId": null,
  "defaultView": {
    "center": [0.5, 0.5],
    "zoom": 1
  }
}
```

Asset records remain authoritative for path, MIME type, size, content hash, and
revision. The descriptor stores asset IDs, never project paths.

### Location references

Any entity may contain a `daena.maps:locations` field. Each entry has its own
stable ID so it can be edited, referenced, indexed, and reconciled without
replacing unrelated entries:

```json
{
  "schemaVersion": 1,
  "locations": [
    {
      "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
      "mapEntityId": "018f89df-b93e-7ad0-a07f-08b1441d1550",
      "role": "birthplace",
      "label": "Old Harbor",
      "anchor": {
        "kind": "provider-feature",
        "provider": "azgaar-fmg",
        "featureKind": "burg",
        "featureId": "42",
        "fallbackPoint": [0.613, 0.428]
      },
      "validity": {
        "from": null,
        "to": null
      }
    }
  ]
}
```

The anchor union supports:

- `provider-feature`: an FMG object plus a normalized fallback point;
- `point`: one normalized `[x, y]` coordinate;
- `path`: an ordered normalized polyline; and
- `area`: one or more normalized polygon rings.

Coordinates are normalized against the provider's declared world extent, not
screen pixels. The adapter converts them to and from the current viewport.
Geometry uses explicit winding, closure, and precision rules in the JSON Schema
so logically identical data serializes deterministically.

Provider selectors are best-effort addresses, not Daena identities. If an FMG
edit removes or renumbers a selected feature, the link becomes `unresolved` and
retains its selector and fallback geometry. The user may rebind it; Daena must
never silently attach it to a different feature.

### Multiple maps and hierarchy

Every map remains independent. Hierarchy uses normal typed relationships:

- `daena.maps:detail-map`: a place entity points to a more detailed map;
- `daena.maps:overview-map`: the inverse semantic where useful; and
- `daena.maps:related-map`: a non-hierarchical association.

The world-map feature for a city therefore links to the city entity, and the
city entity may link to a city map. The child map does not become an FMG object
inside the parent source. The Maps plugin derives breadcrumbs and map-to-map
navigation from these relationships and rejects cycles in the detail-map
projection while preserving the underlying relationships for repair.

### Layers

Layer definitions live in a versioned `daena.maps:layers` field on the map
entity. A layer has a stable ID, name, order, default visibility, style, and a
provider-neutral selector. Selectors may reference:

- explicit location-reference IDs;
- entity types or entity IDs;
- relationship types;
- shared field predicates;
- story/timeline service results; or
- a custom Maps-owned annotation set.

Layer contents are derived on open and when relevant core events arrive.
Rendered features and caches are disposable. The canonical layer definition does
not copy entity names, documents, or relationship records.

### Temporal model

Every location reference and annotation may have an optional validity interval
using Daena's shared calendar/date contract. Missing bounds mean unbounded. Date
precision is preserved rather than defaulting absent month or day values.

The first temporal implementation filters semantic overlays only. Historical
changes to FMG-native borders or terrain use explicit map variants: multiple map
entities or source-asset versions connected by `daena.maps:variant-of`
relationships and validity intervals. Daena does not attempt to mutate FMG's
base geometry from timeline events.

## FMG adapter contract

The wrapper defines and contract-tests a small adapter interface around the
pinned FMG build:

```ts
interface MapProviderAdapter {
  capabilities(): Promise<ProviderCapabilities>;
  load(source: Uint8Array): Promise<LoadedMap>;
  serialize(): Promise<Uint8Array>;
  listFeatures(query?: FeatureQuery): Promise<ProviderFeature[]>;
  captureSelection(): Promise<MapAnchor | null>;
  resolveAnchor(anchor: MapAnchor): Promise<ResolvedAnchor>;
  focus(anchor: MapAnchor): Promise<void>;
  setSemanticOverlay(frame: OverlayFrame): Promise<void>;
  subscribe(listener: (event: ProviderEvent) => void): () => void;
  dispose(): Promise<void>;
}
```

The adapter reports `ready`, `dirty`, `selection-changed`, `source-changed`,
`viewport-changed`, and `fatal-error` events. It owns all access to FMG globals
or internal modules. No host or Maps-domain code may call those internals
directly.

The adapter must define feature identity behavior for every supported FMG
feature kind. The initial linkable set is deliberately limited to burgs, states,
provinces, rivers, markers, and arbitrary points. Unsupported feature kinds
still support arbitrary point/area links. Expanding the set requires fixtures
that prove save/reload and common editing operations do not silently retarget
existing selectors.

A no-edit close keeps the original source asset unchanged. After an actual edit,
FMG may produce different opaque bytes; Daena requires a valid load-save-reload
cycle and correct asset hashing, not byte-identical provider serialization.

## Broker and host contract changes

### Binary asset transfer

Large `.map` sources must not be base64-encoded into ordinary JSON RPC. Extend
the public asset API with session-bound, bounded binary transfers:

1. `asset.read.begin` returns a short-lived, one-use download handle for an
   authorized asset and revision.
2. The plugin fetches bytes from an application-controlled custom-protocol URL
   bound to its webview, session, project, asset, and size limit.
3. `asset.replace.begin` returns a short-lived upload handle after capability,
   namespace, expected-revision, MIME, and declared-size validation.
4. The plugin uploads binary chunks through the custom protocol. The host hashes
   bytes while streaming to transaction staging and enforces total size, time,
   and compression limits.
5. `asset.replace.commit` validates the completed hash and atomically replaces
   the asset record and bytes through the canonical journal. The request ID
   makes retries idempotent.
6. Cancellation, session revocation, project close, or timeout deletes only
   local staging data and never changes the canonical asset.

Add `asset.write:self` as a distinct capability. It permits replacing bytes of
an asset owned by the caller's namespace; it does not permit choosing a path,
reading other assets, or importing arbitrary files. Host-owned import/export
dialogs remain separate operations.

### Map navigation service

The Maps plugin provides a versioned `daena.maps/navigation@1` service:

- `openMap({ mapEntityId, linkId?, mode? })`;
- `focusEntity({ entityId, mapEntityId? })`;
- `setDate({ date })`;
- `showResults({ entityIds, mapEntityId? })`; and
- `listLocations({ entityId })`.

Calls carry stable IDs only. The service resolves links from canonical core
data, mounts or activates the Maps view through host navigation, and returns
typed `map-unavailable`, `link-unresolved`, `provider-unavailable`, and
`not-on-map` errors. Optional consumers such as Lore, Timeline, and Writing
Studio degrade to ordinary entity navigation when Maps is disabled.

The shell remains responsible for changing the active workspace. Plugin code
cannot create arbitrary native webviews or route around the lifecycle manager.

### Events and indexing

The Maps plugin subscribes to post-commit entity, field, relationship, asset,
and plugin-state events. Events are invalidation hints only; the plugin
re-queries authoritative records before updating an overlay.

The runtime database gains derived projections for map descriptors, location
references, provider selectors, normalized bounding boxes, layer membership,
validity intervals, resolution state, and reverse entity-to-map lookup. Every
projection records its canonical source path/hash and is rebuilt by the same
full and incremental scan paths.

Text search continues to use the core FTS API. Structured geographic search is
added as a typed query API over entity types, relationships, shared fields, date
ranges, and presence on a map. Results are ordinary entity IDs; the Maps plugin
highlights only the subset with resolvable locations on the active map.

## User experience

### Map workspace

The Maps workspace contains host-owned navigation chrome and one sandboxed map
surface. The map surface provides:

- map switcher and hierarchy breadcrumbs;
- FMG edit/view mode;
- save state, source-conflict state, and provider diagnostics;
- link-selection and arbitrary-anchor tools;
- semantic layer controls;
- date context when available; and
- an entity inspector that opens the shared Daena entity rather than a copied
  FMG note.

FMG's own embedded title/header is removed or hidden when it duplicates Daena
chrome. The child webview is explicitly closed or remounted on every workspace
navigation, including navigation to an already-active destination, following the
existing native plugin-webview lifecycle contract.

### Map to knowledge

Selecting a supported FMG feature or drawing an arbitrary anchor offers:

- open the already-linked entity;
- link an existing entity;
- create a new entity through a registered template and then link it;
- change the link role or validity; and
- unlink without deleting either the entity or FMG feature.

Creation uses the host's normal atomic entity/template operation. The link is a
separate revision-aware mutation unless the core gains a reviewed multi-record
transaction API; partial failure leaves the new entity visible and clearly
reports that linking must be retried.

### Knowledge to map

Entity views show a Maps contribution when `daena.maps:locations` is present. It
lists role, map name, validity, and resolution state. `Show on map` opens the
Maps workspace, chooses the requested map, and focuses the anchor. If an entity
has multiple links, the user chooses one or opens an overview that highlights
all resolvable links.

### Save, external edits, and conflicts

FMG dirty state is independent from Daena semantic-field dirty state. A map
source save uses the asset revision observed at load. If the source asset
changed externally or in another view, save fails with a typed conflict and
offers reload, export the draft as a recovery copy, or explicitly replace after
review. It never overwrites silently.

Valid external replacement of a closed map is picked up by the canonical file
watcher. If the map is open and clean, the user may reload. If it is dirty, the
current editor state is preserved until the conflict is resolved. Missing or
invalid source files do not hide the map entity or its semantic references.

## Delivery phases

### Phase 0: Upstream spike and contract lock

- Pin an FMG upstream commit and record its license, build inputs, static
  runtime dependencies, and patch ledger.
- Produce a disposable local wrapper proving offline build, child-webview boot,
  `.map` load, real edit, serialize, reload, feature selection, viewport focus,
  cleanup, and absence of network/Tauri access.
- Measure representative source sizes, load/save time, memory use, and any
  compressed-input expansion before setting host limits.
- Document stable selectors for the initial feature set and destructive FMG
  operations that invalidate them.
- Finalize JSON Schemas for map descriptors, anchors, location references,
  layers, service requests/results, and typed errors.
- Add an ADR for the provider boundary, binary broker transport, and FMG fork
  maintenance policy.

**Exit gate:** A packaged development build completes the full
load-edit-save-reload-focus path offline in the real Tauri child webview, and
all provider-specific access is contained behind the proposed adapter. If an
upstream hook is missing, the spike identifies the smallest maintained patch;
the phase does not pass with undocumented global access.

### Phase 1: Provider-neutral Maps domain

- Add the bundled Maps manifest, schemas, relationships, capabilities, service,
  events, and migration declarations to the canonical plugin contract.
- Implement map-entity creation using normal entity, field, relationship, and
  asset records.
- Validate anchor geometry, map/entity existence, provider compatibility,
  namespace ownership, and date intervals in Rust.
- Build full and incremental disposable-index projections and reverse lookup.
- Add SDK/test-host types and conformance fixtures before building UI.

**Exit gate:** Canonical fixtures with multiple maps and multiple locations per
entity round-trip byte-identically; malformed geometry and dangling live map
references fail with stable diagnostics; deleting `.daena/` reconstructs
equivalent map/link/search projections.

### Phase 2: Binary assets and atomic map-source persistence

- Implement session-bound streaming download/upload handles and
  `asset.write:self` authorization in Rust.
- Add asset replacement to the filesystem journal, revision conflicts,
  idempotent request IDs, hash validation, cancellation, limits, and recovery.
- Generate the SDK client and test-host behavior from the RPC schema.
- Add adversarial tests for forged handles, replay, cross-project/session use,
  wrong size/hash, timeout, oversized/expanding input, and interrupted commit.

**Exit gate:** A sandboxed test plugin can read and replace only its own asset;
crash injection at every staging/commit step recovers to exactly the old or new
source and never a mixed asset record/byte state.

### Phase 3: Bundled FMG editor

- Vendor the pinned FMG build and notices through a reproducible packaging task;
  do not fetch upstream resources at runtime.
- Implement the adapter, wrapper UI, restrictive CSP, lifecycle cleanup,
  bounds/resize behavior, diagnostics, and error boundary.
- Connect map creation/opening, binary load/save, dirty state, explicit save,
  recovery export, and external-change conflict handling.
- Preserve the original source asset on no-edit close.
- Add one small example map to the example project.

Status: implemented 2026-08-07 (see `docs/PHASE3_MAPS_PLAN.md` for the
completion notes and per-platform packaged checks that remain).

**Exit gate:** New, existing, externally replaced, missing, malformed, and
conflicting maps have deterministic behavior in packaged Tauri checks on each
supported desktop platform. Closing or switching workspaces leaves no native
plugin webview, session, or unsaved staging data behind.

### Phase 4: Bidirectional linking and navigation

- Implement feature/arbitrary-anchor capture, existing-entity linking,
  template-backed entity creation, role/validity editing, rebind, and unlink.
- Implement the navigation service and shell workspace handoff.
- Add Maps contributions and contextual commands to Lore, Timeline, and other
  entity views using public services and shared data only.
- Reconcile links after each source save and surface unresolved selectors.
- Add core events and overlay invalidation without treating events as durable
  state.

**Exit gate:** A city link survives entity rename, source save/reload, app
restart, module disable/re-enable, and disposable-index deletion. Both
map-to-entity and entity-to-map navigation work in the rendered native app;
removing or renumbering the FMG feature produces an unresolved link rather than
silent retargeting.

This is the first shippable map-integration slice.

### Phase 5: Multiple maps, semantic layers, and geographic search

- Add detail/overview/related-map relationships, switcher, breadcrumbs, cycle
  diagnostics, and cross-map focus.
- Implement toggleable political, culture, infrastructure, population, story,
  search-result, and custom layer definitions as derived overlays.
- Add the typed structured-query API and combine list results with map
  highlights.
- Add viewport-aware batching, geometry simplification, and cache invalidation
  without making caches canonical.
- Test large worlds and representative FMG maps against explicit frame-time,
  memory, and query-latency budgets set from Phase 0 measurements.

**Exit gate:** A world/continent/city map hierarchy navigates without duplicate
entities; toggling or rebuilding layers does not change canonical provider
source; structured queries return the same entity IDs before and after index
rebuild and highlight the correct active-map subset.

### Phase 5A: Watabou detail-map generators

- Inventory the city, village, cave/glade, dungeon, and dwelling generators
  independently; record source availability, license/redistribution terms,
  build inputs, runtime dependencies, supported exports, imports, seeds or
  permalinks, and stable selection/focus hooks.
- Prove the hierarchy flow from an FMG feature to a shared place entity and from
  that entity to a Watabou-generated detail map without duplicating identity.
- Select the first generator by author value and adapter feasibility. City and
  village are preferred, but neither is committed until editable persistence
  and redistribution are verified.
- Add a provider descriptor and adapter only for the selected generator. Reuse
  the existing map entity, asset, anchor, navigation, and lifecycle contracts;
  do not add generator-specific core tables or host APIs.
- Package approved code and notices locally with a deny-by-default CSP. If
  bundling is not permitted, support an explicit external-generator workflow
  that imports exported PNG/SVG/JSON without granting the plugin network or
  filesystem access.
- Add rendered desktop checks for generation, import/save/reopen, map hierarchy
  navigation, resize/focus behavior, offline operation, and truthful static vs
  editable capability labels.

**Exit gate:** At least one Watabou detail-map provider works offline in the
packaged Tauri app or through the reviewed external-export workflow; its
canonical artifact survives restart and disposable-index rebuild; hierarchy
navigation preserves shared entity identity; and the implementation complies
with the selected generator's verified license and persistence capabilities.

### Phase 6: Temporal and story integration

- Consume Timeline's optional date/service contract and expose a host-owned
  current-date context.
- Filter anchors and semantic layers by validity without inventing missing date
  precision.
- Implement explicit historical map variants and a deterministic selection rule
  for overlapping or absent validity intervals.
- Add story projections for character locations/journeys, event sites,
  organization territories, quests, and battle paths using shared entities,
  relationships, and services.
- Degrade gracefully when Timeline or a story provider is disabled.

**Exit gate:** Changing the date deterministically changes only eligible
semantic overlays or selects an explicit map variant; disabling Timeline keeps
maps editable; clone/rebuild and provider unavailability preserve all temporal
and story references.

### Phase 7: Hardening and maintainability

- Add an automated upstream-update job that builds, runs adapter fixtures, and
  reports patch conflicts without silently changing the pinned version.
- Fuzz map/link schemas, binary-handle parsing, adapter messages, and compressed
  source limits.
- Stress repeated open/close, project switching, large overlays, cancellation,
  session revocation, and provider crashes.
- Verify offline installers include all FMG assets and required notices.
- Publish provider-adapter documentation only after the FMG boundary has proven
  stable; do not expose FMG internals as the public adapter API.

**Exit gate:** The packaged release passes security, recovery, lifecycle,
offline, performance, accessibility, and license-notice checks on every
supported desktop target, and an upstream FMG update can be evaluated without
changing canonical Daena data.

## Verification matrix

### Canonical storage and recovery

- Create, rename, archive, restore, and delete map entities through core APIs.
- Round-trip descriptors, anchors, layers, relationships, and temporal ranges.
- Replace a source asset under crash injection and verify journal recovery.
- Delete `.daena/` and compare maps, links, reverse lookup, search, layers, and
  resolution diagnostics after rebuild.
- Exercise Git checkout, unmerged JSON, externally replaced `.map` files, and
  stale-revision conflicts.

### Provider adapter

- Keep versioned fixtures for new, old, large, and malformed FMG sources.
- Load/save/reload after representative terrain, burg, state, river, label, and
  marker edits.
- Verify selectors across safe edits and deterministic unresolved behavior
  across destructive edits.
- Verify normalized point/path/area coordinates across pan, zoom, resize, and
  display scale changes.
- Confirm no-edit close does not replace or rewrite source bytes.

### Security and isolation

- Deny raw Tauri, host DOM, local files, arbitrary custom-protocol paths,
  process/environment access, and undeclared network origins.
- Reject forged, expired, replayed, cross-webview, cross-session, and
  cross-project binary handles.
- Enforce capability, namespace, asset ownership, payload, rate, time, memory,
  and decompression limits in Rust.
- Revoke transfers and navigation/service calls on disable, upgrade, project
  close, crash, and quarantine.

### Rendered desktop behavior

- Exercise feature selection, link creation, inspector navigation, article
  `Show on map`, map switching, layer toggles, date changes, save/conflict UI,
  resizing, and keyboard/focus behavior in the actual Tauri application.
- Verify native child-webview bounds at supported window sizes and display
  scales.
- Verify every workspace transition closes or remounts the previous plugin
  webview, including an already-active destination.
- Check that duplicate FMG and Daena headers are not rendered.

### Repository checks

Run focused tests while implementing each phase, followed by the cached
repository checks:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings
deno task check
deno task build
deno task check:plugin-contract
```

Build/type checks do not replace adapter fixtures, storage recovery, or real
Tauri child-webview verification.

## Explicitly deferred

The following do not block the first shippable slice:

- providers other than FMG and the planned Watabou detail-map family;
- automatic conversion between providers;
- continuous live synchronization with a separately running FMG instance;
- automatic temporal mutation of FMG-native terrain or borders;
- distance/travel-time simulation and route planning;
- climate, population, or political simulation;
- 3D terrain;
- collaboration, cloud sync, or publishing;
- automatic creation of Daena entities for every FMG feature; and
- a public third-party map-provider SDK.

Hand-drawn/image maps can later implement the same point/path/area anchor and
semantic-layer contracts without pretending to expose FMG feature selectors.

## Immediate next work

Begin with Phase 0 only. Do not change the public plugin or storage contracts
until the spike artifacts, schemas, size/resource measurements, adapter patch
surface, and ADR have been reviewed. The implementation handoff should name the
exact pinned FMG commit and the smallest end-to-end fixture used to prove the
exit gate.
