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

- Generic Houses collection for `house` entities.
- Gaps: no member count / leadership summary; no **Open tree** row or detail action; New is only the global gallery.
- Scenarios: `empty-project`, `disconnected-house`, `multiple-memberships`.

## Tree

### `workspace.houses.tree.landing`

- People + Houses searchable panels.
- Current create: **Create person** / **Create house** only in empty states; name-only prompts.
- Target: always-visible **New person** / **New house** using the shared dialog.
- Scenarios: `empty-project`, `large-project`.

### `workspace.houses.tree.open-person`

- Canvas + Person dock.
- Dock today: Open in Lore, Make root, Add parent/child/partner, visible connections.
- Gaps: no Edit identity, Archive, or House membership editor; houses shown as name list only.
- Scenarios: `multiple-memberships`, `malformed-edge`, `revision-conflict`.

### `workspace.houses.tree.open-house`

- House neighborhood via `loadHouseNeighborhood` (members + intra-member kinship only).
- Gaps: scope control unused; no House dock; empty-house copy pushes authors to Person neighborhoods; **New house** buried in View settings.
- Scenarios: `disconnected-house`, `multiple-memberships`.

### `workspace.houses.tree.relationship-dock`

- Metadata editor with Save / Delete and conflict reload.
- Gap: titles use directional arrows for undirected partnerships.
- Scenarios: `revision-conflict`, `malformed-edge`.

### Tree toolbar (open states)

Current groups are flat: Trees (back) · Fit · Reset · View settings (limits + New house) · duplicate Secondary field in subbar.

Target groups (Slice 4):

1. Navigation: Back, current Person/House selector
2. View: scope, secondary label, Fit
3. Expansion: Reset branches
4. More: generation limits, person cap, minimap, reduced detail
5. Create: New person, New house

## Fields & Types

### `project.fields.plugin-list`

- Project Center → Fields & Types plugin cards (`SchemaSettingsPanel`).
- Gap: weak customization / “Managed by extension” explanation for Language and Maps.

### `project.fields.types` / `project.fields.fields` / `project.fields.templates`

- Single large `ModuleSchemaPanel` with three tabs.
- Current: dense builtin enable chips + detail rows; package IDs visible early; errors mostly post-Save.
- Scenarios: `empty-project`, `custom-schema-live-data`, `revision-conflict`.

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
