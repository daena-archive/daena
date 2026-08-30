# Temporary UI/UX plan: Houses, Tree, entities, and Fields & Types

Status: temporary implementation plan. **Slices 0–7 are done.** Durable decisions
are folded into [`ARCHITECTURE.md`](./ARCHITECTURE.md), [`STORAGE.md`](./STORAGE.md),
[`FAMILY_TREE_PLUGIN_IMPLEMENTATION.md`](./FAMILY_TREE_PLUGIN_IMPLEMENTATION.md),
and [`ui-ux-slice0/`](./ui-ux-slice0/README.md). Remove this file once reviewers no
longer need the slice narrative.

## 1. Purpose

Improve four connected areas without creating a second data model:

1. make Houses a useful entity-management workspace rather than only a thin
   collection;
2. make Tree easier to enter, understand, edit, and navigate;
3. make entity lifecycle actions consistent in every first-party module; and
4. make project schema management understandable and safe for authors.

The work must preserve the existing architecture:

- an entity keeps one stable ID across modules;
- SQLite remains runtime authority and portable files remain checkpoints;
- modules interpret shared entities instead of copying them;
- interactive lists use `EntityListQuery`/`EntityPage`;
- mutations use current opaque revisions and stable request IDs; and
- schema customization remains a project-owned overlay on immutable package
  defaults.

## 2. Evidence and current behavior

This plan is grounded in the current implementation, especially:

- `src/routes/+page.svelte`: shell collection, creation, entity identity,
  archive, editor, inspector, and module routing;
- `src/lib/family-tree/FamilyTreeSurface.svelte`: Tree orchestration;
- `src/lib/family-tree/FamilyTreeLanding.svelte`: duplicate People/Houses
  landing lists and their minimal creation paths;
- `src/lib/family-tree/FamilyTreeCanvas.svelte`: pan, zoom, selection, keyboard
  navigation, minimap, and graph controls;
- `src/lib/family-tree/FamilyMemberDialog.svelte`: create-or-link relative flow;
- `src/lib/family-tree/FamilyRelationshipPanel.svelte`: relationship metadata
  editing and deletion;
- `src/lib/ModuleSchemaPanel.svelte`: the current all-in-one overlay editor;
- `src/lib/SchemaSettingsPanel.svelte`: plugin selection and editor routing;
- `packages/modules/*/manifest.json`: the actual types, templates,
  capabilities, fields, and records exposed by each bundled module; and
- `docs/ARCHITECTURE.md` and `docs/STORAGE.md`: the shared entity and canonical
  storage constraints.

Useful existing foundations that should be reused:

- the shell already has a template-driven create dialog;
- collection reads already use backend filtering, sorting, counts, and
  pagination;
- entity rename/type change and archive operations are revision-aware;
- the Project Center already provides restore and permanent deletion;
- family relationships already support validated metadata and conflict
  handling;
- Tree already preserves viewport/session state and bounds graph expansion; and
- project schema overlays already round-trip through runtime storage and the
  portable plugin state.

## 3. Main problems

### 3.1 Entity lifecycle is technically shared but experientially fragmented

Creation has several unrelated presentations:

- the shell has a complete template gallery and schema-driven form;
- Tree has minimal name-only Person and House creation;
- Maps has a provider-specific menu;
- Language edits and archives the selected language inside its own Overview;
- the generic editor hides Archive in the bottom footer; and
- permanent deletion and restore are far away in Project Center → Archive.

Consequences:

- authors cannot predict where New, Rename, Change type, Archive, Restore, or
  Delete will appear;
- minimal Tree creation bypasses useful template fields and opening notes;
- the same Language entity has shell-level and module-level lifecycle behavior
  with different save and error patterns;
- collection rows expose selection only, so common actions require opening the
  entity first; and
- success, conflict, and recovery feedback differs by surface.

The shell also maintains a full `project.listEntities()` result for several
pickers while the visible collection correctly uses paged queries. This will
become a responsiveness and memory problem on large projects and works against
the `EntityListQuery` architecture boundary.

### 3.2 The Houses list and Tree feel like separate products

The Houses workspace exposes two navigation items, `Houses` and `Tree`, but the
connection between them is weak:

- Houses uses the generic collection/editor;
- Tree starts with another People/Houses inventory;
- a House opened in the generic editor has no direct “Open house tree” action;
- the Tree side panel cannot edit or archive the active House;
- Tree-created Houses use a name-only prompt or empty-state form;
- the Tree landing only exposes Create person/Create house when the
  corresponding collection is completely empty; and
- “New house” is hidden inside View settings after a tree has opened.

This duplication creates stale mental context even when both surfaces happen
to read the same entity records.

### 3.3 House membership is under-modeled in the UI

The manifest already defines membership roles (`member`, `head`, `consort`,
`heir`, `founder`, and `custom`) plus notes. The current Tree helper writes
`{ role: "member" }` unconditionally. Tree cards display House names but not
membership roles, and the Tree flow has no clear way to:

- add an existing Person to the active House;
- create and add a Person in one flow;
- change a member's role;
- remove a member without deleting the Person;
- inspect membership notes; or
- distinguish dynastic leadership from ordinary membership.

The current empty-House message tells the author to leave and add members from
a Person neighborhood. That is an implementation-shaped workaround, not a
House-management workflow.

### 3.4 House tree scope is ambiguous

`loadHouseNeighborhood` includes House members and only kinship edges whose two
endpoints are both members. This is safe and bounded, but the UI does not state
that rule. Unconnected members can look like a layout error, while relatives
outside the House silently disappear.

The canvas already has dormant `houseFilterId` and `memberHouseIds` inputs, but
`FamilyTreeSurface` does not provide them. House filtering is therefore partly
designed but not available to authors.

### 3.5 Tree controls have weak information hierarchy

The same secondary-field selector appears in both the subbar and View settings.
The settings popover mixes rendering limits with the domain action New house.
Fit and Reset are clear, but the distinction between:

- changing the root,
- changing the viewing scope,
- expanding one branch,
- resetting expansion state, and
- returning to the Tree landing

is not explained by the control grouping.

Selection opens a useful dock, but Person actions are split between Tree and
Lore. The dock cannot rename/archive the Person or manage House memberships.
Relationship titles use a directional arrow even for undirected partnerships.
Warnings are mostly count-based and hidden in settings, so skipped malformed
edges are difficult to investigate.

### 3.6 Tree keyboard and screen-reader behavior is incomplete

The canvas declares `role="application"`. Arrow keys update the selected Person
in state, but focus is not explicitly moved to the newly selected Person card.
This can leave keyboard focus and visual selection out of sync. Nodes expose
branch buttons, yet there is no documented or announced keyboard model for
moving between nodes, opening details, making a root, or returning to the
canvas.

The settings and root-picker popovers close on an outside pointer event but do
not implement a complete menu/dialog keyboard lifecycle such as Escape,
initial focus, and focus return.

### 3.7 Fields & Types is too dense and too implementation-facing

`ModuleSchemaPanel.svelte` is a very large component containing normalization,
draft state, validation, impact rules, Types UI, Fields UI, Templates UI,
Timeline integration, relationship metadata, removal dialogs, and the save
bar. The result exposes too many controls at once and is difficult to maintain
or test in vertical slices.

Specific UX issues:

- there is no search or filter for long type, field, or template lists;
- package IDs and storage terms compete with author-facing names;
- enable/disable chips and detailed rows duplicate the same built-in fields;
- advanced concepts such as relationship metadata keys, one-of variants,
  namespace behavior, and Timeline layers are shown too early;
- errors frequently appear only after Save as an unstructured message;
- there is no live-data impact summary before disabling or removing schema;
- type removal understands overlay dependents but not existing entities using
  the type;
- removing a type can make an exclusively scoped custom field apply to all
  types, which is correctly mentioned in a confirmation but is still a
  surprising and dangerous default; and
- only `packageManifest.schemas[0]` is used, so a future plugin with multiple
  schema namespaces is not represented correctly.

Schema writes also lack the same mutation discipline used elsewhere:
`setModuleSchemaOverlay` does not send an expected revision or request ID even
though the core already has an idempotent request-aware storage method.

### 3.8 Customization availability is inconsistent and unexplained

Lore, Timeline, Writing, and Houses declare `schema.overlay`. Language and Maps
do not.

This difference should not be hidden:

- Language has a specialized workbench that currently reads its packaged
  fields directly. Enabling arbitrary overlays before that workbench consumes
  the merged manifest would produce inconsistent forms.
- Maps contains provider-owned technical fields and specialized creation. It
  should not expose those fields as normal author schema merely for visual
  consistency.
- Houses can support project fields and templates, but Tree semantics must
  remain explicitly limited to Person, House, parent, partner, and membership
  contracts. A custom type must not silently appear as a Tree node.

## 4. Target experience

### 4.1 A shared entity-management language

Every first-party workspace should present the same lifecycle vocabulary:

- **New**: a visible primary action scoped to the current workspace/view;
- **Edit identity**: rename and, only when safe, change type;
- **Archive**: reversible removal from active views;
- **View Archive**: a direct route after archiving;
- **Restore** and **Delete permanently**: Project Center → Archive; and
- **Open in…**: move to a specialized projection without duplicating the
  entity.

Use the same icon, labels, confirmation language, pending state, conflict
message, and focus-return behavior everywhere.

Do not put permanent deletion in everyday workspace row menus. Keep it in the
Archive because it removes content and relationships and is intentionally a
separate destructive step.

### 4.2 Shared host components, public module operations

Extract presentation components rather than a private data API:

- `EntityCreateDialog`: the existing template gallery and schema form;
- `EntityIdentityDialog`: name and guarded type change;
- `EntityRowActions`: Open, Edit identity, Archive;
- `EntityArchiveAction`: confirmation and status handling;
- `AsyncEntityPicker`: backend-paged search with exclusions and type scopes;
- `MutationStatus`: saving, saved, conflict, failed, retry; and
- `EntityEmptyState`: contextual creation and recovery actions.

These components accept data and callbacks. Bundled modules continue to mutate
through `ModuleContext`; trusted shell flows continue through typed Tauri/core
clients. Do not give modules the shell's private project client.

### 4.3 Contextual New behavior

The global New action remains the way to browse all enabled templates.
Additionally:

- Lore defaults to the first enabled Lore template;
- Timeline defaults to the current Events/Calendars tab;
- Writing defaults to the current Manuscripts/Reference tab;
- Language defaults to the Language template;
- Houses/Houses defaults to House;
- Houses/Tree offers New person and New house as explicit actions, using the
  shared dialog in a focused mode;
- Maps keeps its provider menu because map creation is not a normal template
  mutation.

After creation, route to the owning workspace, select the new entity, and
preserve the previous location in shell history.

### 4.4 Collection actions and scale

Add an overflow action button to each collection row. It must not steal the
row's primary selection action and must have an accessible label containing the
entity name.

Replace pickers that depend on the full in-memory `entities` array with
`AsyncEntityPicker` using:

- server-side text search;
- manifest-derived type scopes;
- excluded IDs;
- deterministic sort;
- bounded pages; and
- cancellation/request tokens.

Keep exact reads on `get_entity`. After mutations, patch the visible row when
safe and re-query the current page/counts. Do not load the full project merely
to refresh one revision.

### 4.5 Houses as a master-detail workspace

Keep `Houses` and `Tree` as two views, but make them complementary:

**Houses view**

- collection: searchable, sortable Houses with member count and head/heir
  summary;
- content: House document and author-facing fields;
- inspector: Members, leadership, relationships, assets, and backlinks;
- primary actions: New house and Open tree;
- row actions: Open, Open tree, Edit identity, Archive.

**Tree view**

- exploration and relationship editing, not a second House database;
- landing searches the same paged House and Person sources;
- creation buttons are always visible, not only in empty states;
- opening a House from either view lands on the same House Tree session;
- Back returns to the prior Houses location when entered from a House; and
- active House identity and membership actions stay available in a dedicated
  House dock.

### 4.6 House membership editor

Extend the Tree membership projection to retain:

- relationship ID and revision;
- Person ID/revision;
- House ID;
- role, custom label, and notes.

Add a House dock with:

- editable House name and Open full entry;
- member search and role filters;
- Add existing Person;
- Create Person and add;
- Edit membership;
- Remove from House;
- Open Person; and
- Archive House.

Membership removal deletes only the `family_member_of` relationship. Every
dialog must state that the Person remains in Lore.

Use the manifest's merged relationship metadata to render membership fields.
Do not hardcode a second role catalog in the component. The default role may be
`member`, but the author must be able to change it.

### 4.7 Explicit House Tree scopes

Provide a scope control with clear behavior:

1. **Members only** (default): current behavior; show all House members and only
   relationships between them.
2. **Members + immediate family**: add parents, partners, and children one hop
   outside the House, visually de-emphasized and bounded by the existing person
   cap.

Show a compact legend:

- full emphasis = House member;
- muted = relative outside the House;
- role badge = head, heir, founder, and so on.

If the House contains disconnected components, state “N family groups” instead
of making the layout appear broken. Fit all groups by default.

### 4.8 Tree toolbar and dock

Reorganize Tree controls:

- navigation group: Back, current Person/House selector;
- view group: scope, secondary label, Fit;
- expansion group: Reset branches;
- more menu: generation limits, visible-person cap, minimap toggle, reduced
  detail; and
- create group: New person, New house.

Remove the duplicate secondary-field control. Keep domain actions out of View
settings.

The Person dock should contain:

- Open in Lore;
- Edit identity;
- Make root;
- Add parent, child, or partner;
- House memberships with role;
- visible connections; and
- Archive Person behind an overflow/destructive section.

The relationship dock should:

- use “A and B” for undirected partnerships;
- use “A → B” only for directed parent links;
- show the relationship type in author language;
- render fields from the merged metadata schema;
- preserve the current conflict-reload behavior; and
- restore focus to the originating edge/node on close.

### 4.9 Tree accessibility contract

Implement and test one explicit keyboard model:

- Tab enters the canvas at the selected/root Person;
- arrow keys move visual selection and DOM focus to the nearest Person;
- Enter opens the Person dock;
- Shift+Enter makes the Person the root;
- Escape closes the dock/popover and returns focus;
- branch buttons remain separately tabbable; and
- a short hidden instruction is referenced with `aria-describedby`.

Re-evaluate `role="application"` after the focus model is implemented. Prefer
ordinary grouped buttons if the custom application semantics do not add
screen-reader value.

Popovers must implement Escape, focus return, and correct menu/dialog roles.
All icon-only controls need stable labels and at least the repository's minimum
interactive target size. Respect reduced motion for layout and viewport
animations.

## 5. Fields & Types redesign

### 5.1 Information architecture

Keep plugin selection in Project Center, but show each plugin with:

- author-facing name;
- number of active Types, Fields, and Templates;
- customization state: Default or Customized;
- validation/error status; and
- “Managed by extension” explanation when overlays are intentionally
  unavailable.

Inside a plugin, use a two-pane workbench:

- left: searchable Types / Fields / Templates list with status filters;
- right: selected item's summary and editor;
- bottom/sticky: one shared save/discard bar.

On narrow screens, use list → detail navigation with an explicit Back action.

### 5.2 Progressive disclosure

Default mode should use author terms:

- Name, Kind, Used by, Applies to, Choices, Required, Show on Timeline.

Move these behind an Advanced disclosure:

- stable IDs and keys;
- relationship type identifiers;
- metadata storage keys;
- one-of variant internals;
- namespace/ownership details;
- Timeline role/group/layer; and
- package/local qualification.

Stable identifiers remain visible before save when they matter, but they should
not dominate the main editing path.

### 5.3 Type editor

For each Type show:

- origin: Built in or Project custom;
- enabled state;
- icon and color;
- existing entity count;
- fields that apply;
- templates that create it; and
- projections that understand it.

Changing a built-in Type only changes enabled state and appearance. Changing a
custom Type name must not silently change its stable ID after entities use it.
Treat ID rename as a migration-level advanced operation or disallow it once
usage is nonzero.

When removing a custom Type:

- offer reassignment of existing entities to another compatible Type;
- offer reassignment/removal for dependent fields and templates;
- never default an exclusively scoped field to “all Types”;
- require an explicit destination, disable the field, or remove it; and
- preview exact counts before Save.

### 5.4 Field editor

Group field Kind choices:

- Basic: Text, Number, Yes/No, Date, Choice;
- Linking: Relationship;
- Advanced: One of.

The detail panel changes by Kind. Validate inline as the author types:

- required name/key;
- unique key;
- nonempty and unique Choice options;
- at least one valid One-of variant;
- Relationship target Types and cardinality;
- unique relationship metadata keys;
- date-sharing requirement for Timeline; and
- valid Type scopes.

For built-in Fields, replace the duplicate enable chips plus rows with one list
row containing status and an Enable toggle. Keep scope, relationship
attributes, and Timeline settings in its detail panel.

### 5.5 Template editor

Show the author the resulting create form, in order:

- Type;
- included Fields;
- which Fields are required;
- defaults;
- description; and
- opening document behavior.

Add a read-only “Preview create form” using the same field renderer as the real
create dialog. This prevents schema and creation UI from drifting.

### 5.6 Read-only impact preview

Add a trusted-core read operation:

`preview_module_schema_overlay(module_id, candidate_overlay)`

It should normalize and validate the overlay against the installed package and
return structured data:

- errors keyed to Type/Field/Template and property;
- warnings;
- entity counts by affected Type;
- stored field-value counts by affected Field;
- templates affected;
- relationship metadata affected;
- Timeline/projection compatibility notes; and
- whether the change is additive, hiding-only, or requires reassignment.

The preview must not mutate runtime rows or portable files. The final Set
operation remains the only mutation.

Add an opaque overlay revision to editor load/save and pass:

- `expectedRevision`; and
- a request ID retained for retry of the same candidate overlay.

On conflict, keep the draft and offer:

- Compare current vs draft;
- Reload current;
- Reapply draft onto current when normalization proves it safe.

### 5.7 Multiple schema namespaces

Remove the `schemas[0]` assumption. Build a normalized editor model from all
package schemas while preserving each Field and Type's owning namespace.

The default UI may group by plugin, but Advanced mode must show namespace
provenance. Save must emit one module overlay validated against the complete
manifest.

### 5.8 Module-specific customization policy

Apply explicit compatibility rules:

- **Lore:** full Types, Fields, Templates, relationship metadata, and Timeline
  options.
- **Timeline:** full Types, Fields, and Templates, with calendar/date invariants
  validated by preview.
- **Writing:** full Types, Fields, and Templates.
- **Houses:** House authoring fields/templates and allowed custom Types; Tree
  shows only contract-compatible Person/House semantics and labels unsupported
  custom Types as collection-only.
- **Language:** keep “Managed by extension” until the specialized workspace
  reads merged schema definitions and renders custom fields consistently.
- **Maps:** keep provider/internal schema managed by Maps. If author-defined map
  metadata is needed later, expose a separate author-facing schema namespace
  rather than the provider fields.

Do not grant `schema.overlay` merely to make every plugin card look the same.

## 6. Module-by-module entity acceptance criteria

### Lore

- New is visible from Library and defaults to a Lore template.
- Rows expose Open, Edit identity, and Archive.
- Person opened from Tree returns through shell history correctly.
- Rename/type changes refresh Library, Wiki, Graph, Timeline labels, and Tree
  labels without changing the entity ID.

### Timeline

- New defaults to the current tab.
- Calendar creation and normal template creation use one lifecycle language.
- Archive is available from collection and detail surfaces.
- Date/era/participant relationships survive identity edits.
- Projection selection updates after create, archive, restore, or type change.

### Writing Studio

- New defaults to Manuscript or Reference according to the active tab.
- Rename and Archive remain available while document autosave is visible.
- Leaving for an identity action flushes or explicitly resolves unsaved text.
- Returning from Archive restores the entry to its correct tab.

### Language

- New Language uses the shared template dialog.
- Collection row actions and the specialized Overview use the same labels,
  confirmation text, and mutation-status component.
- Identity edits refresh both the shell collection and Language breadcrumb.
- Archive clears specialized state and routes focus to the remaining
  collection.
- Record deletion inside Lexicon/Grammar remains record management, not entity
  archive; the UI must preserve that distinction.

### Maps

- Create map stays provider-specific.
- Existing map rows still expose Edit identity and Archive when the map is not
  in a blocking save/conflict state.
- Identity changes do not replace provider IDs, map entity IDs, assets, or
  projections.
- Archive requires editor flush/recovery handling and removes the map from
  active collections without silently deleting runtime recovery copies.

### Houses

- New House is visible in Houses and Tree at all times.
- Open tree is available from each House row and House detail.
- Member count and leadership summary update after membership mutations.
- House rename updates landing, cards, Tree title, and Person membership badges.
- Archive House does not archive People; removing membership does not delete
  either endpoint.

## 7. Delivery slices

### Slice 0: interaction specification and fixtures

Status: **done.** Artifacts live under [`docs/ui-ux-slice0/`](./ui-ux-slice0/README.md)
with machine fixtures in `src/lib/ui-ux/` and lock test `npm run test:ui-ux-slice0`.

Before UI refactoring:

1. capture current screenshots or rendered component fixtures for each
   workspace, Tree landing/open states, and all three schema tabs
   → surface inventory in `docs/ui-ux-slice0/SURFACES.md` (screenshots optional);
2. define entity-action labels and state vocabulary
   → `docs/ui-ux-slice0/INTERACTION_SPEC.md` + `src/lib/ui-ux/vocabulary.ts`;
3. define the Tree keyboard model
   → same interaction spec / `TREE_KEYBOARD` constant;
4. add representative fixtures: empty project, large project, disconnected
   House, multiple memberships, malformed edge, custom schema with live data,
   and simulated revision conflict
   → `src/lib/ui-ux/fixtures.ts`; and
5. record baseline keyboard and screen-reader issues
   → `docs/ui-ux-slice0/BASELINE_A11Y.md`.

Exit gate: reviewers can evaluate later slices against stable scenarios rather
than visual memory.

### Slice 1: shared entity lifecycle

Status: **done** with follow-up hardening from review. Shared components live under
`src/lib/ui-ux/`; shell wiring is in `src/routes/+page.svelte`; Language Overview
uses the same archive confirm copy and mutation vocabulary.
Contract lock: `npm run test:entity-lifecycle`.

Explicitly deferred (not Slice 1 exit-gate blockers):

- **EntityCreateDialog** full extract — create still uses shell
  `openCreationMenu` / `openFocusedCreate`; Tree New person/house route through
  those focused paths. Full dialog extraction tracks with Slice 5 workbench splits.
- Language Overview inline name field remains the specialized workbench editor;
  shell **Edit identity** dialog covers collection/editor identity edits.
  Overview uses shared archive confirm + `MUTATION_STATUS` labels; full
  `MutationStatus.svelte` chrome lands with later workbench convergence.
- Full `listEntities()` warm cache on project open remains for hover/editor
  mentions until those surfaces move to exact reads (follow-up).

1. Extract shared identity, archive, status, row-action, and empty-state components.
2. Add contextual New to every normal workspace header (Houses: New house; Tree:
   New person + New house; ⌘/Ctrl+N = gallery).
3. Add collection row actions (Open → Edit identity → Archive → Open tree).
4. Route first-party archive/identity through the shared mutation controller while
   preserving ModuleContext authority.
5. Add View Archive follow-up feedback with focus return.
6. Keep permanent delete in Project Center.

Exit gate: Lore, Timeline, Writing, Language, and Houses pass the same
create/rename/archive/restore contract tests.

### Slice 2: paged pickers and refresh behavior

Status: **done**. Shared `AsyncEntityPicker` + `asyncEntityQuery` helpers live under
`src/lib/ui-ux/`. Relationship, Tree root, and Tree relative pickers search via
backend pages with request tokens. Shell collection refresh uses
`collectionRefreshEpoch` / `refreshAfterEntityMutation` instead of rematerializing
`project.listEntities()` on every identity/archive/create mutation.
Contract lock: `npm run test:async-entity-picker`.

Still uses a full `listEntities()` warm cache on project open / seed / external
import for hover cards, editor mention hydration, and home “recent” strips;
interactive pickers (including editor @-mentions), map save/link handlers, and
collection pages do not depend on that full list. `workspaceEntityCount` /
`recentlyUpdatedEntities` still scan the warm cache until those home surfaces
move to paged totals.

1. Build `AsyncEntityPicker`.
2. Migrate relationship, Tree root, relative, and other large entity pickers.
3. Remove full-list filtering from interactive collection paths.
4. Re-query only affected pages/counts after mutation.
5. Preserve selection and scroll position.

Exit gate: a project with at least 10,000 entities does not load all entities
to open a picker or collection, and stale requests cannot replace newer
results.

### Slice 3: Houses management

Status: **done** (review follow-ups applied). House collection rows show member count + head/heir summaries via
`houseMemberSummaries`. Tree House sessions expose `FamilyHousePanel` (add/edit/remove
membership from merged metadata, Open full entry, Archive). Empty houses invite add/
create in-place. Role badges and “N family groups” messaging cover disconnected houses.
Tree New person/house continue to route through shared focused create and stay in the
Tree session when invoked from Tree. Membership mutations bump collection refresh.
Tree Back prefers shell history when entered from Houses. Contract lock:
`npm run test:houses-management`.

Explicitly deferred / later:

- Membership “Create person” tab still uses create-and-add (`createMinimalPerson`) so the
  new person is linked in one step; full template fields remain on header New person.
- Houses inspector shows leadership summary + Open tree; rich membership editing stays in
  the Tree House dock (Members relationship field remains in the generic inspector).

1. Add House member summaries to the collection.
2. Add Open tree routing and the House dock.
3. Implement membership add/edit/remove from merged metadata.
4. Replace minimal House/Person creation with focused shared creation.
5. Add role badges and disconnected-family-group messaging.

Exit gate: an author can create a House, create/add members, assign a head and
heir, edit membership, open the Tree, and remove a member without leaving the
Houses module or deleting a Person.

### Slice 4: Tree interaction and accessibility

Status: **done**. Toolbar regrouped into Navigation / View / Expansion / More / Create;
duplicate Secondary control removed. House trees expose Members only vs Members +
immediate family (capped, muted outsiders via `houseFilterId`). Canvas uses
`role="group"` with `aria-describedby` keyboard help; arrow selection moves DOM
focus onto Person cards; Escape closes dock/popovers with focus return. Person
dock adds Edit identity + Archive. Partnership titles use “A and B”; warning
details expand in More. Minimap / reduced-detail toggles and reduced-motion
fit durations land. Contract lock: `npm run test:tree-interaction`.

1. Reorganize toolbar and remove duplicate controls.
2. Implement scope controls and bounded Members + immediate family loading.
3. Align visual selection and DOM focus.
4. Complete dock/popover focus lifecycle.
5. Improve relationship titles and warning details.
6. Test at narrow desktop widths and with reduced motion.

Exit gate: all Tree actions are keyboard reachable; focus is never lost after
reroot, dock close, mutation, or relayout; caps and truncation remain enforced.

### Slice 5: schema workbench shell — **done**

1. Split normalization/domain logic into `src/lib/schema-workbench/model.ts`.
2. Split Types, Fields, and Templates into `SchemaTypesPane` / `SchemaFieldsPane` /
   `SchemaTemplatesPane`.
3. Added search, status filters, namespace provenance (Advanced), and progressive
   disclosure.
4. Shared `SchemaFieldInput` powers Template preview and entity creation.
5. Package model flattens all schema namespaces (no `schemas[0]` assumption).
6. Plugin cards show Type/Field/Template counts, Default/Customized, and
   Managed-by-extension entries for Language/Maps.

Exit gate: overlay normalize is idempotent (byte-equivalent on re-normalize), and
all current schema features remain editable. Contract lock:
`npm run test:schema-workbench`.

### Slice 6: safe schema preview and concurrency — **done**

1. Typed preview models in `crates/daena-plugin-api/src/schema_preview.rs` and
   TypeScript `SchemaOverlayPreviewResult`.
2. Trusted-core `preview_module_schema_overlay` with bounded SQL entity/field
   counts; unresolved type removals with live entities block Save.
3. Opaque overlay revisions on editor load (`contentRevision`); Save uses
   `expectedRevision` + idempotent request IDs minted at preview time
   (`ModuleSchemaOverlayMutationResult`). Editor remount uses a separate integer key.
4. `SchemaImpactReview` shows item-level errors/warnings and live impact before
   risky saves; conflict UI offers Compare / Reload / Reapply.
5. Type removal reassignment is required; core rejects unresolved type removals
   even if the UI is bypassed. Tauri set also requires `acknowledgeImpact` when
   preview reports live-data impact.

Exit gate: no schema change with live-data impact can be saved without showing
the impact, and conflicting editors cannot silently overwrite each other.
Contract lock: `npm run test:schema-workbench` plus
`schema_overlay_preview_counts_and_revision_cas_are_idempotent`.

### Slice 7: module compatibility — **done**

1. Verified §5.8: Lore, Timeline, Writing, and Houses declare `schema.overlay`;
   Language and Maps do not.
2. Maps technical/provider schema remains unavailable (no overlay capability;
   managed card explains why).
3. Language overlay **not** enabled: Overview still reads packaged field
   definitions (`LANGUAGE_SCHEMA_OVERLAY_READY = false`). Managed reason documents
   the gate.
4. Tree-compatible vs collection-only Houses types documented and labeled in the
   type editor via `module-compatibility.ts` (`Houses collection` + `Tree` vs
   `Houses collection only`). See
   [`docs/ui-ux-slice0/MODULE_SCHEMA_COMPATIBILITY.md`](./ui-ux-slice0/MODULE_SCHEMA_COMPATIBILITY.md).

Exit gate: every enabled module either offers a consistent schema editor or a
clear reason that its structure is managed by the extension.
Contract lock: `npm run test:schema-workbench` (manifest overlay policy + Tree
compat helpers).

## 8. Verification requirements

### Automated

- component tests for shared create, identity, archive, and row actions;
- keyboard tests for collection rows, dialogs, Tree nodes, docks, and popovers;
- paged-query tests covering search races, exclusions, type scopes, and 10k+
  entities;
- Tree projection tests for Members only, immediate family, disconnected
  groups, caps, cycles, duplicate edges, and truncation;
- membership mutation tests for every role and custom metadata;
- schema normalization snapshots for single- and multi-namespace manifests;
- schema preview tests for disabling/removing used and unused Types/Fields;
- stale overlay revision and idempotent retry tests;
- portable checkpoint/reopen tests after entity and schema changes; and
- recovery after deleting `.daena/` from a clean checkpoint.

### Rendered/native boundary

Unit tests are not enough. Exercise the packaged or Tauri-rendered app for:

- pane resizing and narrow layouts;
- Tree pan/zoom/fit/reroot and focus after ELK relayout;
- minimap and dark theme contrast;
- dialog focus trapping and Escape behavior;
- long translated labels and long entity names;
- 200+ House/member lists and a 10k-entity picker source;
- autosave before rename/archive/type change;
- archive/restore routing; and
- schema Save, conflict, impact preview, and discard navigation guards.

Browser-only automation is not evidence for the native Tauri lifecycle.

## 9. Guardrails

- Do not add a Houses-only entity store, membership table, or schema format.
- Do not bypass core relationship validation from Tree.
- Do not use frontend-only checks as authority for type reassignment or schema
  compatibility.
- Do not silently purge values when a Field is hidden or disabled.
- Do not silently broaden a Field's scope after removing a Type.
- Do not change stable entity IDs during rename or type change.
- Do not expose Maps provider fields as author schema.
- Do not restore full-project list-and-filter behavior for convenience.
- Do not combine permanent deletion with ordinary Archive.
- Do not stage, commit, or push this temporary plan unless explicitly asked.

## 10. Definition of done

The work is complete when:

- entity creation and lifecycle language is predictable in every bundled
  module;
- Houses and Tree share navigation, identity, creation, and membership flows;
- Tree clearly communicates scope and is fully keyboard operable;
- large collections and pickers remain backend-paged;
- Fields & Types separates common author choices from advanced contract
  details;
- every risky schema change has a structured, live-data-aware preview;
- overlay saves are revision-aware and idempotent;
- multi-namespace manifests are represented correctly;
- storage round trips and clean rebuild remain valid; and
- this temporary document's durable decisions are folded into architecture /
  storage / Family Tree / ui-ux-slice0 docs (slice narrative may then be deleted).
