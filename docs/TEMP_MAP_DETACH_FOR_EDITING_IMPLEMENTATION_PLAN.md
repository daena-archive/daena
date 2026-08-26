# Detach for Editing — Temporary Implementation Plan

## Status and authority

This is a temporary, implementation-oriented plan for the **Detach for
editing** feature defined in [`MAPS.md`](./MAPS.md). It is deliberately specific
to the current checkout. Implement the choices below exactly; do not introduce
alternative storage models, renderer-owned state, a second save path, or direct
mutation of accepted physical-world data.

Delete this file after every acceptance criterion in this plan passes. Do not
link it from durable project documentation.

## Required outcome

An author viewing an accepted physical map can detach either:

- every generated feature in one visible physical vector layer; or
- the currently selected generated features from that layer.

Daena copies that displayed-epoch snapshot into one new, ordinary authored
vector layer on the same map. The copies receive new feature IDs, become fully
editable, persist in the map's existing authored GeoJSON asset, participate in
the normal command history, and render in Atlas as authored overlays. The
source physical layer is hidden by the same command so the snapshot is not
drawn twice.

The operation must never modify, replace, or revise the accepted `.pworld`
asset. Detached features do not follow later epoch changes or physical
re-derivation.

Imported GeoJSON needs no detachment flow. It is already canonical authored
geometry after import and must remain editable through the existing vector-map
workflow.

## Fixed implementation decisions

These decisions are closed. Do not revisit them during implementation.

1. **Detach runs against the already displayed derived collection.** Do not add
   a Rust or Tauri detach command and do not derive physical data a second time.
2. **Detach is an unsaved editor command.** Confirmation adds one command to the
   existing `CommandStack`. The author then uses the existing Save action,
   which calls `project.applyMapEdit` and `ProjectStore::apply_map_edit`.
3. **The accepted physical source remains a separate authority.** The command
   changes only the map's `maps:layers` value and authored `map.geojson` draft.
4. **Derived and authored collections remain separate in frontend state.** The
   command stack contains authored features only. A disposable derived
   collection is composed with it only for OpenLayers rendering.
5. **One detach action creates one new vector layer.** Never merge into an
   existing authored layer and never reuse a physical layer ID.
6. **The target layer inherits presentation from the source layer.** Copy the
   source layer's style, opacity, and blend mode. The new layer is visible and
   unlocked.
7. **The source physical layer becomes hidden in the same undo entry.** Physical
   layer visibility becomes ordinary persisted layer state rather than a
   session-only `Map` override.
8. **Selected features are preferred when the source layer has a proper
   subset selected.** Otherwise the dialog offers only the entire layer.
9. **Selection copies whole features.** Do not clip geometry to the selection
   box or viewport extent.
10. **Saving remains explicit.** Do not auto-save after detachment.
11. **Epoch changes and Atlas entry are disabled while the map document is
    dirty.** This prevents discarding an unsaved snapshot or rendering Atlas
    from stale canonical state.
12. **A complete physical-world-to-new-vector-map export is out of scope.** It
    is a separate future operation, not another choice in this dialog.

## Supported source layers

Use exactly the reserved physical layer IDs already enforced by
`crates/daena-core/src/maps/physical.rs`:

```ts
export const PHYSICAL_DERIVED_LAYER_IDS = [
  "base",
  "ocean",
  "land",
  "shelves",
  "bathymetric-contours",
  "tectonic-plates",
  "tectonic-boundaries",
  "bathymetry",
  "volcanic-centers",
  "lakes",
  "rivers",
  "watersheds",
  "islands",
  "ice",
] as const;
```

Define this frontend constant in the new physical detach module described
below. Do not include `earthquake-hazard` or `volcanic-hazard`: they are not in
the accepted physical layer registry or the core reserved-layer contract.

The operation is available only when all of the following are true:

- the open map provider is `daena-physical`;
- the source layer ID is in `PHYSICAL_DERIVED_LAYER_IDS`;
- the source layer is a locked vector layer;
- the displayed derived collection contains at least one feature in that
  layer;
- historical derivation and map loading are idle; and
- the prospective authored document passes the detach preflight budgets.

## State model correction required before the UI work

The current accepted-physical-map editor combines derived physical features
with authored features inside `CommandStack`, then filters the physical
features back out at save time. That arrangement is not safe for detachment:
resetting the command stack on an epoch change can discard unsaved authored
work, and the current physical save path would remove the displayed derived
features after a successful save.

Correct it before adding the dialog.

### Frontend state

In
`src/lib/maps/native-vector/NativeVectorMapEditor.svelte`, replace the combined
document model with these states:

```ts
let derivedPhysical = $state<VectorFeatureCollection>({
  type: "FeatureCollection",
  features: [],
});

// `CommandStack.document.collection` is always authored-only.
// `draft` remains the collection sent to OpenLayers.
let draft = $state<VectorFeatureCollection>({
  type: "FeatureCollection",
  features: [],
});
```

Add one helper with this exact ordering:

```ts
function renderedCollection(authored: VectorFeatureCollection): VectorFeatureCollection {
  if (!physicalMap) return authored;
  return {
    type: "FeatureCollection",
    features: [...derivedPhysical.features, ...authored.features],
  };
}
```

Derived features must appear first and authored features second so authored
features remain the later presentation input where ordering is otherwise equal.

Apply these rules everywhere:

- `resetCommandStack` receives authored geometry only.
- `syncUiFromStack` assigns `draft = renderedCollection(snapshot.document.collection)`.
- `mountEditor` supplies `draft` to OpenLayers but passes all map layers.
- an OpenLayers `replace-collection` payload is still handled by
  `captureReplaceCollection`; its protected-layer merge strips physical
  features from the command document and keeps authored locked/hidden features
  from the current authored document.
- `save` serializes `commandStack.document.collection` directly. Remove
  `persistedCollection`.
- after save, keep `derivedPhysical` unchanged and assign
  `draft = renderedCollection(snapshot)`.
- recovery export continues to package the command document, so it contains
  authored geometry and layer state but no disposable physical features.

### Loading and epoch changes

During physical-map load:

1. parse the authored asset strictly into `authored`;
2. derive and parse the displayed physical GeoJSON leniently into
   `derivedPhysical`;
3. initialize `CommandStack` with `authored`, not the combined collection; and
4. render `renderedCollection(authored)`.

During `applyHistoricalProducts`:

1. require a clean command stack;
2. replace `derivedPhysical` with the parsed response;
3. leave `CommandStack.document` and its baseline unchanged;
4. update the physical raster;
5. set `draft = renderedCollection(commandStack.document.collection)`; and
6. call `editor.syncDocument(draft, layers, runtimeLayerRasters())`.

The epoch slider, exact-year input, and past/future controls must use
`disabled={busy || epochBusy || dirty}`. Show this hint while dirty:

> Save or undo authored changes before changing the physical epoch.

Do not silently save, reset, or carry a dirty authored document across an
epoch request.

## Pure detach module

Create `src/lib/maps/physical/detach.ts`. Keep transformation and preflight
logic out of the Svelte component.

Export these values and types:

```ts
export const PHYSICAL_DERIVED_LAYER_IDS = [/* exact list above */] as const;
export type PhysicalDerivedLayerId = (typeof PHYSICAL_DERIVED_LAYER_IDS)[number];
export type PhysicalDetachScope = "selected" | "layer";

export type PhysicalDetachPlan = {
  sourceLayerId: PhysicalDerivedLayerId;
  sourceLayerName: string;
  epochOffsetYears: number;
  scope: PhysicalDetachScope;
  sourceFeatureIds: string[];
  targetLayer: VectorLayerDefinition;
  copies: VectorFeature[];
};

export type PhysicalDetachError = {
  code: "physical.detach.invalid-layer" | "physical.detach.empty" | "vector.limit.exceeded";
  message: string;
};
```

Export these functions:

```ts
isPhysicalDerivedLayerId(id: string): id is PhysicalDerivedLayerId
physicalFeaturesForLayer(collection, layerId): VectorFeature[]
selectedPhysicalFeatures(collection, layerId, selectedIds): VectorFeature[]
physicalDetachLayerName(sourceLayerName, epochOffsetYears): string
buildPhysicalDetachPlan(input): PhysicalDetachPlan | PhysicalDetachError
```

`buildPhysicalDetachPlan` must accept an optional `newId: () => string` and use
`crypto.randomUUID` by default. Tests pass a deterministic ID generator; product
code does not.

### Scope resolution

When opening the dialog, calculate:

```ts
const all = physicalFeaturesForLayer(derivedPhysical, sourceLayer.id);
const selected = selectedPhysicalFeatures(derivedPhysical, sourceLayer.id, selectedFeatureIds);
```

If `selected.length > 0 && selected.length < all.length`, show both scope
choices and default to `selected`. Otherwise show only `Entire layer` and use
`layer` scope. A selection from another layer has no effect.

### Target layer

Build the target with these exact values:

```ts
{
  id: newId(),
  kind: "vector",
  name: physicalDetachLayerName(source.name, appliedEpochOffsetYears),
  order: nextLayerOrder(document.layers),
  defaultVisible: true,
  locked: false,
  opacity: source.opacity,
  blendMode: source.blendMode,
  selector: {},
  style: JSON.parse(JSON.stringify(source.style)),
}
```

Use these exact generated names:

- epoch `0`: `<source name> — detached at reference epoch`
- positive epoch: `<source name> — detached at +<N> years`
- negative epoch: `<source name> — detached at -<N> years`

Use the JSON round trip above rather than `structuredClone`; Svelte reactive
proxies can reach this boundary and the browser structured-clone algorithm
rejects them. The layer can be renamed normally after creation. Do not ask for
a name in the detach dialog.

### Feature copies and provenance

Deep-clone each source feature. Preserve its supported geometry, semantic type,
name, style override, and label override. Replace the feature ID and owning
layer ID. Merge these exact keys into `properties.daena.custom`:

```ts
{
  detachedFromProvider: "daena-physical",
  detachedFromLayerId: sourceLayer.id,
  detachedFromFeatureId: String(sourceFeature.id),
  detachedAtEpochYears: appliedEpochOffsetYears,
}
```

The new ID must be unrelated to the derived ID. Do not retain a derived feature
ID as authored identity. Do not write provenance into top-level GeoJSON
properties outside `properties.daena`.

Sort selected source features by their existing string ID before assigning new
IDs. Preserve that order in `copies`; canonical save may subsequently sort by
the new IDs.

### Preflight budgets

Reject the operation before dispatch when any prospective limit fails:

- `document.layers.length + 1 <= VECTOR_MAX_LAYERS`;
- authored feature count plus copy count is at most `VECTOR_MAX_FEATURES`;
- every copied feature contains at most `VECTOR_MAX_FEATURE_POSITIONS`
  positions;
- the prospective authored collection contains at most
  `VECTOR_MAX_POSITIONS` positions; and
- `collectionBytes(prospectiveCollection).byteLength <= VECTOR_MAX_BYTES`.

Use the constants exported from `packages/plugin-sdk/src/maps.ts`. Add a small
iterative position counter in `detach.ts`; do not flatten large coordinate
arrays for budget calculation.

Return `vector.limit.exceeded` with a message naming the failed limit. Do not
partially detach a layer to fit a budget. Core canonicalization remains the
final validation boundary for rings, coordinate ranges, property bytes, and
all other source constraints.

Use these exact messages, substituting the published numeric constant:

- layers: `Detaching would exceed the map layer limit of <N>.`
- authored features: `Detaching would exceed the authored feature limit of <N>.`
- one feature's positions: `A detached feature exceeds the per-feature position limit of <N>.`
- total positions: `Detaching would exceed the authored position limit of <N>.`
- encoded bytes: `Detaching would exceed the authored GeoJSON byte limit of <N>.`

## One atomic undo command

In `src/lib/maps/editor/commands.ts`:

1. add `"DetachPhysicalFeatures"` to `MapCommandKind`;
2. export `detachPhysicalFeaturesCommand`; and
3. implement its inverse explicitly.

Use this signature:

```ts
export function detachPhysicalFeaturesCommand(input: {
  sourceLayerId: string;
  sourceLayerName: string;
  sourceWasVisible: boolean;
  targetLayer: VectorLayerDefinition;
  copies: VectorFeature[];
}): MapCommand;
```

The forward `apply` must:

1. confirm the source layer exists, is a locked vector layer, and the target ID
   does not exist;
2. set only the source layer's `defaultVisible` to `false`;
3. append the target layer;
4. append deep-cloned copies to the authored collection; and
5. leave the descriptor unchanged.

The inverse `apply` must:

1. remove the target layer;
2. remove features whose IDs are in the command's copy-ID set;
3. restore the source layer's `defaultVisible` to `sourceWasVisible`; and
4. leave every unrelated layer and feature unchanged.

The command label is `Detach <source layer name> for editing`, using
`input.sourceLayerName`. Do not derive a user-visible name from the layer ID.

One confirmation must add exactly one history entry. One Undo must remove the
detached layer and copies and restore physical visibility. One Redo must restore
the same target and feature IDs; it must not generate new IDs.

## Persist physical visibility

Remove `physicalLayerVisibility` and `withPhysicalVisibility` from
`NativeVectorMapEditor.svelte`.

Use the persisted `defaultVisible` value parsed from `maps:layers` for every
map provider. `toggleVisible` must always dispatch
`setLayerVisibilityCommand`, including for locked physical layers. Visibility
changes therefore participate in history, dirty state, conflict handling,
checkpoint recovery, and Save.

Do not allow any other mutation of a physical layer. Existing guards for
locking, renaming, reordering, duplicating, styling, and deleting reserved
physical layers remain in force.

Initial accepted physical layers already use `defaultVisible: false`; do not
change those defaults.

## Use one OpenLayers adapter for an accepted physical map

`PhysicalWorldView.svelte` remains the read-only generation-preview component
used by `PhysicalMapEditor.svelte`. It must no longer be the accepted physical
map surface inside `NativeVectorMapEditor.svelte`.

For an accepted physical map:

1. render the same `.map-host` and `createMapAdapter` path used by vector maps;
2. supply `PHYSICAL_COORDINATE_SPACE`;
3. supply the physical hillshade canvas through `runtimeBackgrounds()`;
4. supply `renderedCollection(authored)` and all physical plus authored layers;
5. keep physical layers locked; and
6. allow authored layers to use normal select, modify, translate, draw,
   snapping, metadata, style, label, geometry-operation, and delete behavior.

Remove `physicalEditor` and all accepted-map `PhysicalWorldView` plumbing.
Keep `PhysicalWorldView` imports and use in `PhysicalMapEditor.svelte` only.

### Locked-feature and box selection

Static mode already permits click selection of visible locked features. Extend
the OpenLayers interaction contract so a physical editor can also use
Ctrl/Cmd-drag box selection without making locked geometry editable.

Add `allowLockedBoxSelection?: boolean` to the session accepted by
`createMapAdapter`, pass it into `createInteractionManager`, and set it to
`true` only for accepted physical maps.

In `interaction-manager.ts`:

- activate `DragBox` in `static` mode only when
  `allowLockedBoxSelection === true`;
- in `static` mode, add intersecting features only when `featureSelectable`
  accepts them;
- in `select` mode, retain the current `layerAcceptsEdits` filter; and
- never activate `Modify` or `Translate` for a locked layer.

The selection box chooses whole features whose extents intersect the box. It
does not clip geometry.

### Label visibility

Do not restore labels for generated physical features merely because the
accepted map now uses the editable adapter. Extend the `labelsVisible` option
through `MapAdapter.ts`, `layer-registry.ts`, and `style-factory.ts` to accept
either a boolean or a `(layerId: string) => boolean` predicate.

For accepted physical maps pass:

```ts
labelsVisible: (layerId) => !isPhysicalDerivedLayerId(layerId);
```

For vector maps and authored layers, labels remain enabled. For generation
preview, preserve the component's existing label behavior.

## Detach dialog and editor behavior

Create
`src/lib/maps/physical/DetachPhysicalLayerDialog.svelte`. Do not use the shared
yes/no dialog because this flow must present an explicit scope.

The dialog receives:

- source layer name;
- applied epoch offset;
- selected feature count;
- total source-layer feature count;
- initial scope;
- busy state;
- `onconfirm(scope)`; and
- `oncancel()`.

Use `role="dialog"`, `aria-modal="true"`, a labeled title, Escape-to-cancel,
backdrop cancellation, a trapped Tab cycle, and focus restoration to the button
that opened it. When both scopes are shown, initially focus the selected-scope
radio. When only whole-layer scope is shown, initially focus the confirm button.

Use this exact title:

> Detach <layer name> for editing?

Use this exact consequence text, substituting the count and formatted epoch:

> Daena will copy <N> generated features from <epoch> into a new authored layer
> and hide <layer name>. The copy will no longer follow epoch changes or
> physical re-derivation. The accepted physical world will not be changed.

Scope labels are:

- `Selected features (<N>)`
- `Entire layer (<N>)`

Buttons are `Cancel` and `Detach snapshot`.

### Entry point

Add a Scissors icon button to each supported physical layer row immediately
after its visibility button. Use:

- aria-label: `Detach <layer name> for editing`
- title: `Detach for editing`

Disable it during loading, derivation, saving, or when the derived layer has no
features. Clicking it opens the dialog; it does not mutate state.

### Confirmation behavior

On confirmation:

1. rebuild the plan against the current `derivedPhysical`, current selection,
   current command document, and `appliedEpochOffsetYears`;
2. if preflight fails, keep the dialog open and display its diagnostic;
3. dispatch exactly one `detachPhysicalFeaturesCommand`;
4. close the dialog;
5. switch `activeLayerId` to the new authored layer;
6. set the tool to `select`;
7. select the new copy IDs in OpenLayers after the document sync; and
8. announce:

   `Detached <N> features from <layer name> at <epoch>. Save to commit the snapshot.`

Do not save automatically and do not clear unrelated selections or history
until the command succeeds.

## Physical-map authoring controls

Remove the broad `{#if !physicalMap}` gates that currently hide all authoring
controls. Replace them with capability checks:

- Pan, Select, measurement, Undo, Redo, Add vector, and Save are available on
  accepted physical maps.
- Draw tools are enabled only when `activeLayer` is a visible, unlocked,
  non-reserved vector layer.
- The feature inspector, geometry operations, duplication, deletion, metadata,
  labels, custom properties, and entity linking appear only when the selected
  features exist in `CommandStack.document.collection`. A generated physical
  selection alone must not show editable controls.
- Raster-layer creation and background calibration remain unavailable for
  physical maps in this feature.
- Physical layer rename, reorder, style, lock, duplicate, and delete controls
  remain unavailable.

Change the physical-map subtitle to:

- clean: `Generated world map`
- dirty: `Unsaved authored changes · <units label>`

The Save button uses the existing dirty/saving/saved labels and remains the only
commit action.

## Atlas behavior

Do not add an Atlas-specific detach path. Atlas already consumes canonical
authored GeoJSON and map layers.

While `dirty` is true:

- disable `Atlas Studio` entry;
- disable static Atlas export; and
- show `Save authored changes before opening Atlas.`

After save, verify that Atlas receives the detached layer as an authored vector
overlay and honors the persisted hidden state of the source physical layer.
Changing the physical epoch must leave the detached Atlas geometry fixed.

## Core and persistence boundaries

No production Rust API change is required. The existing
`ProjectStore::apply_map_edit` remains the only mutation boundary and already:

- checks descriptor, layers, and source revisions;
- preserves canonical authored source asset identity;
- canonicalizes the uploaded GeoJSON;
- rejects authored features on reserved physical layer IDs;
- checks locked-layer feature immutability;
- commits map, layers, authored source, and link changes atomically; and
- participates in request replay, projection refresh, checkpointing, and
  recovery.

Do not call `replace_vector_source`, `create_vector_layer`, or
`update_map_layer` as separate mutations for detachment. Doing so would split
one user action across revisions and defeat atomic Save.

The detach save must keep these values byte-for-byte or revision-for-revision
unchanged:

- physical descriptor `sourceAssetId`;
- physical `.pworld` asset content hash;
- physical `.pworld` asset size;
- physical `.pworld` asset revision; and
- accepted generation settings and physical identity.

Only the authored GeoJSON asset, `maps:layers`, and the map descriptor's normal
view state may receive new revisions through `apply_map_edit`.

## Implementation order

Follow this order. Do not start Svelte integration until the pure model and
command tests pass.

### Slice 1 — Pure model and history

1. Add `src/lib/maps/physical/detach.ts`.
2. Add `DetachPhysicalFeatures` and its inverse to `commands.ts`.
3. Export required command symbols from `src/lib/maps/editor/index.ts`.
4. Add pure tests for scope, IDs, provenance, style copying, budgets, apply,
   undo, and redo.

Exit condition: pure tests prove one command creates and reverses the entire
document change without touching its input objects.

### Slice 2 — Separate runtime-derived state

1. Add `derivedPhysical` and `renderedCollection`.
2. Make `CommandStack` authored-only.
3. Remove `persistedCollection`.
4. Update load, save, recovery, command sync, and epoch application.
5. Disable epoch changes while dirty.

Exit condition: saving an authored physical overlay leaves the derived world
visible, and an epoch refresh cannot discard or mark-clean authored changes.

### Slice 3 — Unified accepted physical editor

1. Replace accepted-map `PhysicalWorldView` with the normal map host.
2. Pass the physical raster and coordinate space to `createMapAdapter`.
3. Add locked box selection.
4. Add per-layer label visibility.
5. Expose authored controls through capability checks.

Exit condition: generated features can be selected but not modified, while an
ordinary authored layer on the same physical map supports the complete existing
vector editing workflow.

### Slice 4 — Dialog and detach integration

1. Add `DetachPhysicalLayerDialog.svelte`.
2. Add the physical-layer row action.
3. Build and dispatch the plan on confirmation.
4. Focus the target layer and select the copies.
5. Add notices and all accessibility behavior.

Exit condition: selected-feature and whole-layer detachment each create one
undoable dirty change with the derived source hidden.

### Slice 5 — Persistence, Atlas, and recovery evidence

1. Add core atomicity and immutability tests.
2. Add checkpoint recovery coverage.
3. Verify Atlas overlay and epoch independence.
4. Run the native rendered checklist.

Exit condition: every automated and rendered acceptance check below passes.

## Automated test changes

### New frontend test

Create `scripts/maps-physical-detach.test.mjs` and add it to
`check:maps:physical` in `package.json`.

It must import the pure TypeScript helpers with Node's type stripping and cover:

1. the supported physical ID list exactly matches the list in this plan;
2. another layer's selected IDs are ignored;
3. a proper selected subset becomes the default selected scope;
4. no selection and a full-layer selection both resolve to whole-layer scope;
5. target layer values exactly match the fixed contract;
6. source objects are not mutated;
7. source IDs are replaced with deterministic injected IDs;
8. provenance contains exactly the four required keys in addition to any
   preserved custom values;
9. feature order is source-ID order before ID allocation;
10. each individual budget failure returns `vector.limit.exceeded`;
11. command apply hides the source and creates one layer plus all copies;
12. one inverse restores an exact deep-equal original document;
13. redo reuses the original target and copy IDs; and
14. imported/authored features are not treated as physical detach sources.

Update existing source-contract assertions in:

- `scripts/maps-native-vector-editor.test.mjs`;
- `scripts/maps-openlayers.test.mjs`; and
- `scripts/maps-physical-surface.test.mjs`.

Remove the assertion that an accepted physical map must render through
`PhysicalWorldView`. Keep the assertion that the generation preview uses it.
Add assertions for the unified adapter, authored-only command document,
persistent physical visibility, dirty epoch guard, detach dialog, and Atlas
dirty guard.

### Rust tests

In `crates/daena-core/src/project/tests.rs`, add:

```text
apply_map_edit_accepts_detached_physical_snapshot_without_mutating_physical_source
physical_detach_save_is_atomic_on_stale_revision
physical_detach_survives_clean_checkpoint_rebuild
```

The success fixture must:

1. accept a physical map;
2. record the `.pworld` asset bytes, hash, size, and revision;
3. submit `apply_map_edit` with one new unlocked non-reserved vector layer,
   copied canonical authored features, and the source physical layer hidden;
4. assert the authored source and layers revisions changed;
5. assert every recorded `.pworld` value is unchanged;
6. assert detached features use only the new layer ID; and
7. reopen the project and assert the same state.

The stale-revision fixture must pass one stale expected revision and assert no
descriptor, layer, authored-source, physical-source, or checkpoint state
changed.

The recovery fixture must export a clean checkpoint after the successful save,
remove only the disposable `.daena/` runtime state using the existing test
helper, reopen from the checkpoint, and assert:

- target layer ID, order, style, visibility, and lock state;
- source physical layer hidden state;
- detached feature IDs, geometry, metadata, and provenance; and
- unchanged physical identity and accepted source hash.

Keep the existing rejection test for authored features placed directly on a
reserved physical layer. Add an assertion that detachment succeeds only because
the copies use the new non-reserved layer ID.

## Rendered desktop acceptance checklist

Run this against a development Tauri desktop window started with
`rtk npm run tauri dev`; browser-only automation is not sufficient for the
native asset and lifecycle boundaries.

1. Open an accepted physical map at reference epoch.
2. Show Rivers and Ctrl/Cmd-drag a box intersecting two or more river features.
3. Confirm the features highlight but vertices cannot move and Delete changes
   nothing.
4. Click `Detach Rivers for editing`.
5. Confirm the dialog defaults to `Selected features` and states the reference
   epoch consequence.
6. Confirm detachment. Verify one new layer appears, Rivers becomes hidden, the
   copies stay visible and selected, and the map becomes dirty.
7. Move a vertex and translate one detached feature. Verify generated physical
   geometry remains unchanged.
8. Undo once. Verify the new layer and copies disappear and Rivers returns to
   its previous visibility. Redo once and verify the same IDs return.
9. Save. Close and reopen the map. Verify the detached layer, edits,
   provenance, and hidden Rivers state survive.
10. Change the epoch. Verify generated hydrology changes as expected while the
    detached geometry does not move.
11. Open Atlas Studio. Verify the detached layer renders as an authored overlay
    and the hidden generated Rivers layer is not duplicated.
12. Return to the physical map, make an unsaved edit, and verify epoch controls,
    Atlas Studio, and static Atlas export are disabled with the prescribed
    hints.
13. Create another detach using `Entire layer`, save, reopen, and verify it
    independently.
14. Open an imported GeoJSON vector map and verify ordinary editing and saving
    still work without any detach control.

## Verification commands

Run all commands from the repository root.

```bash
rtk node --experimental-strip-types scripts/maps-physical-detach.test.mjs
rtk npm run check:maps:native-vector
rtk npm run check:maps:physical
rtk npm run check:maps:atlas
rtk npx svelte-check --tsconfig ./tsconfig.json --config ./svelte.config.js
rtk cargo test --manifest-path crates/daena-core/Cargo.toml --locked --offline physical_detach
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
rtk git diff --check
```

If a broad command encounters an unrelated known fixture or environment
blocker, still run every focused detach test and report the blocker separately.
Do not treat compilation or source-string assertions as rendered desktop
evidence.

## Definition of done

The feature is complete only when all of the following are true:

- selected-subset and entire-layer detachment both work;
- detachment is one exact undo/redo entry;
- target layer and features use new stable IDs;
- provenance records source provider, layer, feature, and applied epoch;
- the source derived layer is hidden and that state persists;
- generated features remain immutable at UI, command, and core boundaries;
- authored physical overlays use the complete existing vector editing tools;
- Save uses only `applyMapEdit` and conflict recovery retains the full draft;
- epoch changes cannot discard unsaved authored work;
- `.pworld` bytes, identity, hash, size, and revision remain unchanged;
- clean checkpoint recovery reconstructs the detached snapshot exactly;
- Atlas renders saved detached geometry as an authored overlay;
- imported GeoJSON behavior has no regression;
- focused frontend, core, Svelte, Atlas, and native rendered checks pass; and
- no temporary compatibility path, alternate persistence API, or
  renderer-owned durable state was introduced.

After these conditions pass, delete this temporary plan in the same cleanup
change that removes any implementation-only TODO references to it.
