# Daena Archive UI/UX

This document is the UI/UX record for entity lifecycle, Houses and Tree, and
Fields & Types. Product architecture and storage remain in
[`ARCHITECTURE.md`](./ARCHITECTURE.md) and [`STORAGE.md`](./STORAGE.md). Tree
data contracts remain in
[`HOUSES.md`](./HOUSES.md).

Author-facing labels in the app must match this vocabulary. Do not invent
parallel names for the same actions. Shared host chrome for these actions lives
in `src/lib/entity-lifecycle`.

## Principles

- An entity keeps one stable ID across modules. Rename and type change never
  replace that ID.
- Modules interpret shared entities; they do not copy them into private stores.
- Interactive lists and pickers use paged queries. Do not load the full project
  to open a collection or picker.
- Mutations use opaque revisions and stable request IDs. Conflicts offer reload,
  review draft, and retry — never silent overwrite.
- Schema customization is a project-owned overlay on immutable package defaults.
- Permanent deletion stays in Project Center → Archive. Everyday workspace menus
  only archive.

## Shared lifecycle

Every first-party workspace uses the same actions:

| Action             | Label              | Where it appears                                                           |
| ------------------ | ------------------ | -------------------------------------------------------------------------- |
| New                | New                | Workspace header, rail, mobile FAB                                         |
| Open               | Open               | Collection row overflow                                                    |
| Open in…           | Open in…           | Row and dock menus; moves to a specialized view without copying the entity |
| Edit identity      | Edit identity      | Row overflow, docks, editor; rename, and type change only when safe        |
| Archive            | Archive            | Row overflow, editor, specialized Overview; reversible                     |
| View Archive       | View Archive       | Post-archive follow-up; routes to Project Center → Archive                 |
| Restore            | Restore            | Project Center → Archive only                                              |
| Delete permanently | Delete permanently | Project Center → Archive only                                              |
| Open tree          | Open tree          | Houses collection and House detail                                         |
| Open in Lore       | Open in Lore       | Tree Person dock                                                           |
| Make root          | Make root          | Tree Person dock / Shift+Enter                                             |
| New person         | New person         | Tree landing                                                               |
| New house          | New house          | Houses header and Tree landing                                             |

### Confirmations

| Action             | Title                          | Message                                                                                          | Confirm            |
| ------------------ | ------------------------------ | ------------------------------------------------------------------------------------------------ | ------------------ |
| Archive            | `Archive {name}?`              | `{name} will be archived and hidden from the workspace.`                                         | Archive            |
| Restore            | `Restore "{name}"?`            | `Returns the entry to the workspace and search.`                                                 | Restore            |
| Delete permanently | `Delete "{name}" permanently?` | `Removes the entry, its content, and relationships. This cannot be undone.`                      | Delete permanently |
| Remove membership  | Names the person and house     | `Removes this person from the house. The person remains in Lore and is not archived or deleted.` | Remove from House  |

Shared chrome for every lifecycle action:

- the same icon family for New, Archive, Restore, and Delete permanently;
- pending label `Working…` on confirm buttons while the mutation runs;
- focus returns to the invoking control after close, or to the next collection
  row after archive.

### Mutation status

| State    | Author-facing signal                | Recovery                                               |
| -------- | ----------------------------------- | ------------------------------------------------------ |
| Idle     | No status chrome                    | —                                                      |
| Saving   | Saving…                             | Disable duplicate submits                              |
| Saved    | Saved                               | Optional timestamp                                     |
| Conflict | This record changed in another view | Reload current values · Review draft · Retry when safe |
| Failed   | The change could not be saved       | Retry · Dismiss                                        |

Optimistic-concurrency failures use the machine code `revision-conflict`.

### Contextual New

The global New action (rail / ⌘N) remains the full template gallery. Workspace
headers default as follows:

| Workspace      | Default New                                             |
| -------------- | ------------------------------------------------------- |
| Lore           | First enabled Lore template                             |
| Timeline       | Current Events or Calendars tab template                |
| Writing Studio | Current Manuscripts or Reference tab template           |
| Language       | Language template via the shared dialog                 |
| Houses         | House template                                          |
| Houses / Tree  | **New person** and **New house** on Tree landing        |
| Maps           | Provider-specific create menu (not a template mutation) |

After creation, route to the owning workspace, select the new entity, and keep
the previous location in shell history.

### Collection rows

Every interactive collection row overflow menu, in order:

1. Open
2. Edit identity
3. Archive
4. Module-specific extras (Houses: Open tree)

The overflow must not steal the row’s primary selection action. Its accessible
name includes the entity name (for example `Actions for Aria`). Permanent delete
is absent.

## Houses and Tree

Houses and Tree are two views of the same records, not two products.

**Houses** is the master-detail workspace: searchable Houses with member count
and head/heir summary; House document and author-facing fields; inspector with
members, leadership, relationships, assets, and backlinks. Primary actions are
New house and Open tree.

**Tree** is exploration and relationship editing. Landing searches the same
paged House and Person sources. Creation buttons stay visible even when the
collections are not empty. Opening a House from either view lands on the same
House Tree session. Back returns to the prior Houses location when entered from
a House.

### Membership

Membership is a relationship (`family_member_of`) with roles from the Houses
schema (`member`, `head`, `consort`, `heir`, `founder`, and `custom`) plus notes.
The default role may be `member`; the author can change it. Removing membership
deletes only that relationship. The Person remains in Lore.

The House dock provides:

- editable House name and Open full entry;
- member search and role filters;
- Add existing Person;
- Create Person and add;
- Edit membership;
- Remove from House;
- Open Person; and
- Archive House.

Archive House does not archive People. Role badges mark leadership roles.

### Tree scope

| Scope                           | Label                      | Behavior                                                                         |
| ------------------------------- | -------------------------- | -------------------------------------------------------------------------------- |
| `members-only`                  | Members only               | Default. House members and kinship edges whose both endpoints are members        |
| `members-plus-immediate-family` | Members + immediate family | Plus one-hop parents, partners, and children outside the House, muted and capped |

Legend:

- full emphasis = house member;
- muted = relative outside the house;
- role badge = head, heir, founder, and so on.

Disconnected components state `{N} family groups` instead of looking like a
layout error. Fit all groups by default.

### Toolbar and docks

Open-tree toolbar groups:

1. Navigation: Back, current Person selector (person trees)
2. View: scope (house trees), secondary label, Fit
3. Expansion: Reset branches
4. More: generation limits (person trees), person cap, minimap, reduced detail, warning details

Domain actions do not live in View settings. The secondary-label control appears
once.

The Person dock contains Open in Lore, Edit identity, Make root, Add parent /
child / partner, House memberships with role, visible connections, and Archive
Person in a destructive section.

The relationship dock uses “A and B” for partnerships and “A → B” only for
parent links, shows the type in author language, and restores focus to the
originating edge or node on close.

### Keyboard

| Input           | Behavior                                                         |
| --------------- | ---------------------------------------------------------------- |
| Tab             | Enters the canvas at the selected or root Person card            |
| Arrow keys      | Move visual selection and focus to the nearest Person            |
| Enter           | Opens the Person dock                                            |
| Shift+Enter     | Makes the Person the root                                        |
| R               | Opens or cycles a relationship of the focused person             |
| Escape          | Closes dock or popover and returns focus to the origin           |
| Tab within card | Branch controls remain separately tabbable on the focused person |

Hidden help is referenced with `aria-describedby`. Popovers implement Escape,
initial focus, and focus return. Icon-only controls have stable labels and meet
the repository minimum target size. Reduced motion disables layout and viewport
animation. Prefer ordinary grouped controls over application semantics unless
the latter clearly helps screen readers.

## Workspaces

### Lore

New is visible from Library and defaults to a Lore template. Rows expose Open,
Edit identity, and Archive. A Person opened from Tree returns through shell
history. Identity edits refresh Library, Wiki, Graph, Timeline labels, and Tree
labels without changing the entity ID.

### Timeline

New defaults to the current tab. Calendar creation and template creation use the
same lifecycle language. Archive is available from collection and detail.
Date, era, and participant relationships survive identity edits. The chronology
view updates after create, archive, restore, or type change. Type chips and
filters show schema names, not raw type IDs.

### Writing Studio

New defaults to Manuscript or Reference according to the active tab. Rename and
Archive remain available while document autosave is visible. Leaving for an
identity action flushes or explicitly resolves unsaved text. Returning from
Archive restores the entry to its correct tab.

### Language

New Language uses the shared template dialog. Collection row actions and the
specialized Overview use the same labels, confirmation text, and mutation
status. Identity edits refresh the shell collection and Language breadcrumb.
Archive clears specialized state and focuses the remaining collection. Lexicon
and Grammar record deletion is record management, not entity archive.

### Maps

Create map stays provider-specific. Existing map rows still expose Edit identity
and Archive when the map is not in a blocking save or conflict state. Identity
changes do not replace provider IDs, map entity IDs, assets, or projections.
Archive requires editor flush or recovery and removes the map from active
collections without silently deleting recovery copies.

### Houses

New House is visible in Houses and Tree at all times. Open tree is available
from each House row and House detail. Member count and leadership summary update
after membership mutations. House rename updates landing, cards, Tree title, and
Person membership badges.

## Fields & Types

Project Center lists each overlay-capable module with author-facing name; counts
of active Types, Fields, and Templates; Default or Customized; validation
status; and a Managed by extension explanation when overlays are unavailable.

Inside a module, a two-pane workbench shows a searchable Types / Fields /
Templates list with status filters on the left and the selected item on the
right, with one shared save/discard bar. Narrow layouts use list → detail with
Back.

### Author terms

Default labels: Name, Kind, Used by, Applies to, Choices, Required, Show on
Timeline.

Advanced disclosure only: stable IDs and keys, relationship type identifiers,
metadata storage keys, one-of internals, namespace and ownership, Timeline
role/group/layer, package/local qualification.

### Types

Each Type shows origin (Built in or Project custom), enabled state, icon and
color, existing entity count, applying fields, creating templates, and
projections that understand it.

Built-in Types only change enabled state and appearance. Changing a custom Type
name never rewrites its stable ID after entities use it.

Removing a custom Type requires:

- reassignment of existing entities to another compatible Type;
- explicit disposition of dependent fields and templates (remove, disable, or
  reassign — never broaden an exclusive field to all Types); and
- a live-data impact preview before Save.

### Fields

Kind groups: Basic (Text, Number, Yes/No, Date, Choice), Linking (Relationship),
Advanced (One of). The detail panel changes by Kind and validates inline.
Built-in fields use one list row with status and an Enable toggle.

### Templates

Order shown to the author: Type, included Fields, which are required, defaults,
description, opening document behavior. A read-only Preview create form uses the
same field renderer as the real create dialog.

### Impact and concurrency

Risky overlay saves show a structured preview: errors keyed to Type, Field, or
Template; warnings; entity and stored-field counts; templates and relationship
metadata affected; projection notes; and whether the change is additive,
hiding-only, or requires reassignment. The preview does not mutate data.

Load and save use an opaque overlay revision and a request ID retained for
retry. On conflict, keep the draft and offer Compare current vs draft, Reload
current, and Reapply draft onto current when safe. Core rejects unresolved type
removals even if the UI is bypassed. Disabled fields stay stored; values are
hidden, not purged.

The workbench represents every schema namespace on a package, not only the
first.

## Schema customization policy

Do not grant overlay capability only to make plugin cards look the same.

| Module   | Fields & Types       | Notes                                                                                                                        |
| -------- | -------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Lore     | Workbench            | Full Types, Fields, Templates, relationship metadata, Timeline options.                                                      |
| Timeline | Workbench            | Full Types, Fields, Templates; calendar and date invariants via preview.                                                     |
| Writing  | Workbench            | Full Types, Fields, Templates.                                                                                               |
| Houses   | Workbench            | House authoring fields, templates, and custom Types. Tree stays contract-limited.                                            |
| Language | Managed by extension | Specialized Overview still reads packaged fields. Overlay stays off until that workspace renders custom fields consistently. |
| Maps     | Managed by extension | Provider and internal fields stay extension-managed. Future author map metadata would be a separate namespace.               |

Tree hydrates only Person (`daena.lore:person`) and House (`daena.houses:house`).
In the Houses type editor, only the contract House type is Tree-compatible.
Every other Houses type — including custom types — is collection-only: it
appears in the Houses collection and generic editor, never as a Tree node. Open
tree remains available only for the contract House type.

Projection labels:

- builtin House → Houses collection, Tree
- all other Houses types → Houses collection only
- Lore → Library, Wiki, Graph (Person also lists Tree)
- Timeline → Timeline
- Writing → Writing Studio

## Surfaces

Stable surface IDs for review and fixtures:

| ID                                                                            | Role                                                  |
| ----------------------------------------------------------------------------- | ----------------------------------------------------- |
| `dialog.create-template`                                                      | Template gallery and schema-driven create form        |
| `dialog.edit-identity`                                                        | Name and guarded type change                          |
| `dialog.archive-confirm`                                                      | Archive confirmation                                  |
| `project.archive`                                                             | Restore and Delete permanently                        |
| `picker.async-entity`                                                         | Paged entity search with exclusions and type scopes   |
| `workspace.lore.library` / `workspace.lore.entity-editor`                     | Lore collection and editor                            |
| `workspace.timeline.events` / `workspace.timeline.calendars`                  | Tabbed Timeline collections                           |
| `workspace.writing.manuscripts` / `workspace.writing.reference`               | Tabbed Writing Studio                                 |
| `workspace.language.collection` / `workspace.language.overview`               | Language collection and specialized Overview          |
| `workspace.maps.collection` / `workspace.maps.editor`                         | Maps collection and editor                            |
| `workspace.houses.collection`                                                 | Houses collection with member summaries and Open tree |
| `workspace.houses.tree.landing`                                               | Tree People and Houses landing                        |
| `workspace.houses.tree.open-person`                                           | Person tree and Person dock                           |
| `workspace.houses.tree.open-house`                                            | House tree and House dock                             |
| `workspace.houses.tree.root-picker`                                           | Tree root picker                                      |
| `workspace.houses.tree.relationship-dock`                                     | Relationship metadata                                 |
| `project.fields.plugin-list`                                                  | Fields & Types module cards                           |
| `project.fields.types` / `project.fields.fields` / `project.fields.templates` | Schema workbench tabs                                 |

Representative scenarios: empty project, large project, disconnected House,
multiple memberships, malformed edge, custom schema with live data, and
revision conflict.

## Guardrails

- Do not add a Houses-only entity store, membership table, or schema format.
- Do not bypass core relationship validation from Tree.
- Do not use frontend-only checks as authority for type reassignment or schema
  compatibility.
- Do not silently purge values when a Field is hidden or disabled.
- Do not silently broaden a Field’s scope after removing a Type.
- Do not change stable entity IDs during rename or type change.
- Do not expose Maps provider fields as author schema.
- Do not restore full-project list-and-filter behavior for convenience.
- Do not combine permanent deletion with ordinary Archive.
