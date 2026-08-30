# Interaction specification (Slice 0)

This document freezes the author-facing language and keyboard contract for the
UI/UX plan. Implementation slices must not invent parallel labels.

Source constants: `src/lib/ui-ux/vocabulary.ts`.

## 1. Entity-action vocabulary

| Action ID | Label | Where it may appear | Notes |
| --- | --- | --- | --- |
| `new` | New | Workspace header / rail / mobile FAB | Global template gallery remains available |
| `open` | Open | Collection row overflow | Primary row click still selects |
| `openIn` | Open in… | Row / dock menus | Moves to a specialized projection; does not copy the entity |
| `editIdentity` | Edit identity | Row overflow, docks, editor | Renames; type change only when safe |
| `archive` | Archive | Row overflow, editor, specialized Overview | Reversible; never permanent delete |
| `viewArchive` | View Archive | Post-archive feedback | Routes to Project Center → Archive |
| `restore` | Restore | Project Center → Archive only | |
| `deletePermanently` | Delete permanently | Project Center → Archive only | Never in everyday workspace row menus |
| `openTree` | Open tree | Houses collection / House detail | Same House Tree session from either view |
| `openInLore` | Open in Lore | Tree Person dock | Preserves shell history for return |
| `makeRoot` | Make root | Tree Person dock / Shift+Enter | |
| `newPerson` | New person | Tree create group | Shared create dialog, Person-focused |
| `newHouse` | New house | Houses header and Tree create group | Shared create dialog, House-focused |

### Confirmation copy

| Action | Title | Message | Confirm |
| --- | --- | --- | --- |
| Archive | `Archive {name}?` | `{name} will be archived and hidden from the workspace.` | Archive |
| Restore | `Restore "{name}"?` | `Returns the entry to the workspace and search.` | Restore |
| Delete permanently | `Delete "{name}" permanently?` | `Removes the entry, its content, and relationships. This cannot be undone.` | Delete permanently |
| Remove membership | (dialog title names the person + house) | `Removes this person from the house. The person remains in Lore and is not archived or deleted.` | Remove from House |

Shared chrome for every lifecycle action:

- same icon family for New / Archive / Restore / Delete permanently;
- pending label `Working…` on confirm buttons while the mutation runs;
- focus returns to the invoking control after close, or to the next sensible collection row after archive;
- conflict and failure use the mutation-status vocabulary below.

## 2. Mutation-status vocabulary

| State | Author-facing signal | Required recovery |
| --- | --- | --- |
| `idle` | No status chrome | — |
| `saving` | Saving… | Disable duplicate submits |
| `saved` | Saved | Optional timestamp |
| `conflict` | This record changed in another view | Reload current values · Review draft · Retry when safe |
| `failed` | The change could not be saved | Retry · Dismiss |

Machine code for optimistic-concurrency failures: `revision-conflict`.

Relationship docks already approximate this (`Reload current values`, `Review draft`).
Shell document saves, Language Overview archive, Maps flush, and schema overlay
saves must converge on the same labels in later slices.

## 3. Contextual New

| Workspace / view | Default New behavior |
| --- | --- |
| Lore | First enabled Lore template |
| Timeline | Current Events / Calendars tab template |
| Writing Studio | Current Manuscripts / Reference tab template |
| Language | Language template via shared dialog |
| Houses / Houses | House template |
| Houses / Tree | Explicit **New person** and **New house** |
| Maps | Provider-specific create menu (not a normal template mutation) |
| Global New (rail / ⌘N) | Full template gallery |

After creation: route to the owning workspace, select the new entity, and preserve
the previous location in shell history.

## 4. Collection row actions

Every interactive collection row overflow menu:

1. Open
2. Edit identity
3. Archive
4. Module-specific extras (Houses: Open tree)

Rules:

- overflow must not steal the row's primary selection action;
- accessible name includes the entity name (for example `Actions for Aria`);
- permanent delete is absent.

## 5. Tree keyboard model

Target contract (implement in Slice 4; baseline gaps in `BASELINE_A11Y.md`):

| Input | Behavior |
| --- | --- |
| Tab | Enters the canvas at the selected or root Person card |
| Arrow keys | Move visual selection **and** DOM focus to the nearest Person |
| Enter | Opens the Person dock |
| Shift+Enter | Makes the Person the root |
| Escape | Closes dock/popover and returns focus to the origin |
| Tab within card | Branch controls remain separately tabbable |

Supporting requirements:

- hidden help text referenced with `aria-describedby` (`family-tree-keyboard-help`);
- popovers (View settings, root picker) implement Escape, initial focus, and focus return;
- icon-only controls have stable labels and meet the repository minimum target size;
- reduced motion disables layout/viewport animation durations;
- re-evaluate `role="application"` after the focus model lands; prefer ordinary
  grouped buttons if application semantics do not help screen readers.

Current code already maps Enter / Shift+Enter / arrows on the canvas container,
but does not move DOM focus onto the newly selected Person card.

## 6. Tree scope vocabulary

| Scope ID | Label | Behavior |
| --- | --- | --- |
| `members-only` | Members only | Default. House members + kinship edges whose both endpoints are members |
| `members-plus-immediate-family` | Members + immediate family | Plus one-hop parents/partners/children outside the House, muted, capped |

Legend copy:

- Full emphasis = house member
- Muted = relative outside the house
- Role badge = head, heir, founder, and so on
- Disconnected components: `{N} family groups`

## 7. Fields & Types author terms (default mode)

Default labels: Name, Kind, Used by, Applies to, Choices, Required, Show on Timeline.

Advanced disclosure only: stable IDs/keys, relationship type identifiers, metadata
storage keys, one-of internals, namespace/ownership, Timeline role/group/layer,
package/local qualification.

Tabs remain Types / Fields / Templates in the workbench shell; Slice 5 splits
pane bodies into focused components with search/status filters and Advanced
disclosure for contract identifiers.
