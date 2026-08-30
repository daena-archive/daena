# Surface inventory (Slice 0 fixtures)

Native screenshots are not required for the Slice 0 exit gate. This inventory is
the review stand-in: each surface names the current chrome, primary actions, and
linked scenarios so later slices can be judged against stable references.

Surface IDs match `SURFACE_IDS` in `src/lib/ui-ux/fixtures.ts`.

## Shell and shared dialogs

### `dialog.create-template`

- **Location:** Shell modal from rail New / mobile FAB / home New entry.
- **Current chrome:** Template gallery → schema-driven form; titles `Choose a template` / `Create {template}`.
- **Primary actions:** Create, Cancel, Escape closes.
- **Scenarios:** `empty-project`, `custom-schema-live-data`.

### `dialog.edit-identity`

- **Location:** Shell modal (`Edit {name}`, kicker `EDIT ENTRY`).
- **Current chrome:** Name field + type select with type-change warnings.
- **Primary actions:** Save, Cancel.
- **Gap vs target:** Label is still “Edit entry”, not **Edit identity**.

### `dialog.archive-confirm`

- **Location:** Shell confirm for selected entity.
- **Current chrome:** `Archive {name}?` / confirm `Archive`.
- **Gap vs target:** No post-archive **View Archive** affordance.

### `project.archive`

- **Location:** Project Center → Archive (`ArchivedDocumentsPanel`).
- **Current chrome:** Restore · Delete permanently.
- **Scenarios:** post-archive follow-up from any workspace.

### `picker.async-entity` / `workspace.houses.tree.root-picker`

- **Current:** Shared `AsyncEntityPicker` (`src/lib/ui-ux/`) backs Relationship,
  Tree root, and Tree relative pickers with backend-paged search, exclusions,
  type scopes, and stale-request rejection.
- **Scenarios:** `large-project`.

## Workspaces

### `workspace.lore.library` / `workspace.lore.entity-editor`

- Collection + generic editor/inspector.
- New via global gallery; Archive in editor footer; no row overflow menu yet.
- Scenarios: `empty-project`, `custom-schema-live-data`, `large-project`.

### `workspace.timeline.events` / `workspace.timeline.calendars`

- Tabbed collection; New should default to active tab (Slice 1).
- Scenarios: `empty-project`.

### `workspace.writing.manuscripts` / `workspace.writing.reference`

- Tabbed Writing Studio; identity actions must flush autosave (later slices).
- Scenarios: `empty-project`.

### `workspace.language.collection` / `workspace.language.overview`

- Collection plus specialized Overview with its own `Archive language` copy.
- Gap: shell and Overview lifecycle language differ.
- Scenarios: `empty-project`.

### `workspace.maps.collection` / `workspace.maps.editor`

- Provider-specific create menu; identity/archive must respect map save/conflict state.
- Scenarios: `empty-project`.

### `workspace.houses.collection`

- Houses collection for `house` entities with member count + head/heir summary (loading state while summaries fetch).
- Row + detail **Open tree** routes into the same House Tree session; inspector shows House summary + Open tree.
- New house uses the shared focused create dialog.
- Scenarios: `empty-project`, `disconnected-house`, `multiple-memberships`.

## Tree

### `workspace.houses.tree.landing`

- People + Houses backend-paged panels with member summaries on Houses.
- Always-visible **New person** / **New house** via shared focused create (stays in Tree when created from Tree).
- Scenarios: `empty-project`, `large-project`.

### `workspace.houses.tree.open-person`

- Canvas + Person dock.
- Dock: Open in Lore, Edit identity, Make root, Add parent/child/partner, house labels with
  roles, Archive Person, visible connections.
- Keyboard: Tab to Person cards; arrows move selection + DOM focus; Enter opens dock;
  Shift+Enter makes root; Escape closes dock/popover with focus return.
- Scenarios: `multiple-memberships`, `malformed-edge`, `revision-conflict`.

### `workspace.houses.tree.open-house`

- House neighborhood via `loadHouseNeighborhood` with scope control:
  - Members only (default): members + intra-member kinship.
  - Members + immediate family: one-hop parents/partners/children outside the house,
    muted, capped by visible-people limit.
- House dock: members list/search/role filter, Add existing / Create person, edit/remove
  membership (conflict reload), Open full entry, Archive House.
- Empty-house copy invites in-place membership; disconnected components show “N family groups”.
- Role badges on leadership roles. Trees Back prefers shell history when entered from Houses.
- Legend includes scope vocabulary (member emphasis, muted outsider, role badge).
- Scenarios: `disconnected-house`, `multiple-memberships`.

### `workspace.houses.tree.relationship-dock`

- Metadata editor with Save / Delete and conflict reload.
- Titles: “A and B” for partnerships; “A → B” for parent links; author-language type label.
- Scenarios: `revision-conflict`, `malformed-edge`.

### Tree toolbar (open states)

Groups:

1. Navigation: Back, current Person selector (person trees)
2. View: scope (house trees), secondary label, Fit
3. Expansion: Reset branches
4. More: generation limits (person trees), person cap, minimap, reduced detail, warning details
5. Create: New person, New house

## Fields & Types

### `project.fields.plugin-list`

- Project Center → Fields & Types plugin cards (`SchemaSettingsPanel`).
- Cards show **active** Type/Field/Template counts, Default vs Customized, and
  validation Error badge when overlay status is error.
- Language/Maps appear as Managed by extension (no overlay editor).

### `project.fields.types` / `project.fields.fields` / `project.fields.templates`

- Two-pane workbench (`workbench-list` / `workbench-detail`) with search/status
  filters and Advanced disclosure.
- Focused panes: `SchemaTypesPane`, `SchemaFieldsPane`, `SchemaTemplatesPane`.
- Type detail shows origin, fields/templates/projections usage, entity count
  placeholder; custom type **name** edits never rewrite stable IDs.
- Type removal requires explicit exclusive-field disposition (remove / disable /
  reassign) and entity reassignment when live entities use the type; never
  broadens exclusive fields to all types.
- Builtin fields: one list row with status + Enable toggle (no duplicate chips).
- Field forms show inline property errors; Kind uses author terms (Yes/No).
- Templates: Type → included → required → defaults → description → opening note
  → shared Preview create form.
- Multi-namespace package schemas flattened; shell uses `primarySchemaNamespace`.
- Scenarios: `empty-project`, `custom-schema-live-data`, `revision-conflict`.
- Slice 6: trusted-core impact preview with live entity/field counts; Save shows
  `SchemaImpactReview` when acknowledgement is required; overlay load/save uses
  opaque `contentRevision` + request ID (editor remount key is separate);
  conflict offers Compare / Reload / Reapply; core rejects unresolved type
  removals even if the UI is bypassed.
## Scenario → surface matrix

| Scenario | Primary surfaces |
| --- | --- |
| `empty-project` | All workspace empties + three schema tabs + Tree landing |
| `large-project` | Lore library, Tree landing, async picker, root picker |
| `disconnected-house` | House Tree open + Houses collection |
| `multiple-memberships` | Person Tree + House Tree |
| `malformed-edge` | Person Tree + relationship dock |
| `custom-schema-live-data` | Schema tabs + Lore library |
| `revision-conflict` | Relationship dock + schema Types tab |
