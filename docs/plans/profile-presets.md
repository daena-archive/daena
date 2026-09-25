# Profile presets delivery plan

Product and architecture are in [`LORE_PROFILES.md`](../lore/LORE_PROFILES.md)
and [ADR 0006](../adr/0006-lore-profiles.md). This file is the delivery
sequence for data-driven, entity-oriented Profile presets. Do not treat a
phase as done from this file alone; verify the worktree.

---

## Problem

Shipped presets are hardcoded TypeScript constants in `profilePresets.ts`,
and `presetOrigin` is a compile-time literal union (`"custom" | "dnd" |
"fantasy" | "scifi"`). All three genre presets define person-oriented
components. Non-person entities must use Custom and build their Profile from
scratch. Authors cannot create, edit, or share presets.

## Goal

Make presets data-driven records that are entity-type scoped, grouped by
genre, and fully user-customizable, while preserving ADR 0006 constraints
(module records, copy-on-create, no core tables, formulas in TypeScript).

---

## Data model

### Preset record

Presets are stored as `module_records` entries in a `profile-preset`
collection on `daena.lore`.

```ts
type ProfilePresetDocument = {
  schemaVersion: number;
  name: string;
  description?: string;
  icon?: string;
  entityTypes?: string[];
  genre?: string;
  builtin?: boolean;
  hidden?: boolean;
  allocation?: ProfileAllocation;
  components: ProfileComponent[];
};
```

`entityTypes` holds qualified runtime ids (`daena.lore:person`, or an overlay
id such as `daena.lore:knightly-order`). Omit the field for a universal
preset. Do not store bare ids (`person`) and do not list every builtin type
to mean "all". `genre` is `D&D`, `Fantasy`, `Sci-Fi`, or a user label. Omit
it for ungenred presets. `hidden` applies to bundled presets; user presets
are deleted instead of hidden.

### Profile document change

`presetOrigin` on `ProfileDocument` changes from the hardcoded enum
`ProfilePresetOrigin` to `string | undefined`. After migration it stores only
the preset record ID. Legacy `"custom"`, `"dnd"`, `"fantasy"`, and `"scifi"`
stay accepted on read and map to the four stable bundled-preset record IDs.
Do not keep `"custom"` as a second identity for the Custom preset. If every
`profile-preset` record is missing, create an empty Profile and leave
`presetOrigin` unset.

### Storage

| Collection | Owner | Unique | Notes |
|---|---|---|---|
| `profile` | entity | per-owner | Existing — no change |
| `profile-change` | entity | no | Existing — no change |
| `profile-preset` | project | no | New — preset definitions |

`profile-preset` is not entity-owned. Do not create a sentinel, nil, or
hidden entity to satisfy the current owner check. `module_records.owner_entity_id`
is `NOT NULL REFERENCES entities(id)`, the broker rejects a missing owner, and
the portable checkpoint rejects a record whose owner is not in the entity set.
Deleting that entity would also delete the presets.

Phase 1 must add a project owner scope before the collection can be seeded.
Project-scoped rows have no entity owner. List, create, update, and delete
for that scope must not require `ownerEntityId`. Checkpoint restore must
accept those rows. Entity delete must not cascade them. `profile` and
`profile-change` stay entity-owned; do not weaken their owner checks.

---

## Bundled presets

The following bundled presets are seeded on project creation (or added via
migration on existing projects). Each preset declares `builtin: true`.

### D&D genre

| Preset | Entity Types | Section |
|---|---|---|
| D&D Character | `daena.lore:person` | §9.1 |
| D&D Faction | `daena.lore:faction` | §9.2 |
| D&D Location | `daena.lore:place` | §9.3 |

### Fantasy genre

| Preset | Entity Types | Section |
|---|---|---|
| Fantasy Character | `daena.lore:person` | §10.1 |
| Fantasy Faction | `daena.lore:faction` | §10.2 |
| Fantasy Kingdom | `daena.lore:faction`, `daena.lore:place` | §10.3 |
| Fantasy Place | `daena.lore:place` | §10.4 |
| Fantasy Artifact | `daena.lore:artifact` | §10.5 |

### Sci-Fi genre

| Preset | Entity Types | Section |
|---|---|---|
| Sci-Fi Character | `daena.lore:person` | §11.1 |
| Sci-Fi Faction | `daena.lore:faction` | §11.2 |
| Starship | `daena.lore:artifact` | §11.3 |
| Colony / Station | `daena.lore:place` | §11.4 |

### Ungenred

| Preset | Entity Types | Section |
|---|---|---|
| Custom | omitted (universal) | §12.1 |
| Culture | `daena.lore:culture` | §12.2 |
| Concept | `daena.lore:concept` | §12.3 |

Total: 15 bundled presets. Three genres, plus three ungenred presets. Only
Custom is universal. Trait example lists in the spec are not components.

---

## Delivery phases

Only one phase may be implemented at a time. A phase is not complete
because types compile or unit tests pass; every stated exit gate must have
evidence.

### Phase 1 — Data-driven preset infrastructure

Deliver:

- Project owner scope for module records, including broker methods, the
  `module_records` owner constraint, and checkpoint restore. No sentinel entity.
- `profile-preset` collection declared in the Lore manifest with
  `ownerScope: project` and capability grants.
- `ProfilePresetDocument` type definition in TypeScript.
- Preset CRUD: create, read, update, delete, list via `ModuleContext.records`.
- Bundled preset seeding: on project creation, seed the existing 4 presets
  (Custom, D&D, Fantasy, Sci-Fi — person-oriented, matching current
  hardcoded definitions) as `profile-preset` records with `builtin: true`.
  Assign stable record IDs and keep them. Omit `entityTypes` and `genre` in
  this phase so every entity still sees the same four choices.
- `presetOrigin` on `ProfileDocument` changed from enum to `string`.
  Validation accepts legacy `"custom" | "dnd" | "fantasy" | "scifi"` and
  preset record IDs. New Profiles store the record ID, including Custom.
- `PROFILE_PRESET_ORIGINS` constant and `ProfilePresetOrigin` type removed
  or widened.
- `profilePresets.ts` refactored: hardcoded `PROFILE_PRESETS` array replaced
  with a function that loads presets from records. The hardcoded data
  moves to a seed/migration file.
- Preset picker (`ProfileEditor.svelte`) reads from the record-backed
  preset list instead of the hardcoded array.
- Migration for existing projects: seed the 4 bundled presets as records.
  Existing profile instances with `presetOrigin: "dnd"` etc. are
  mapped to the corresponding record ID.
- Existing tests updated. New tests for preset CRUD, seeding, migration,
  and preset picker.

Do not add new entity-oriented presets or preset management in this phase.

**Exit gate:** existing Profile workflows work unchanged. Presets are loaded
from project-scoped records with no owner entity in the graph. Creating a
Profile from D&D/Fantasy/Sci-Fi/Custom produces the same result as before.
Legacy `presetOrigin` values survive migration. Deleting an entity does not
delete presets. All existing profile and profile-change tests pass.

### Phase 2 — Entity-oriented bundled presets

Deliver:

- All 15 bundled presets as defined in sections 9–12 of `LORE_PROFILES.md`,
  seeded as `profile-preset` records.
- Evolve the Phase 1 D&D, Fantasy, and Sci-Fi records in place into D&D
  Character, Fantasy Character, and Sci-Fi Character. Keep their record IDs
  so existing `presetOrigin` values still resolve. Do not add a second
  character preset beside the old one. Removing Population and Fleet Strength
  from the Sci-Fi Character definition does not rewrite existing Profile
  instances.
- Keep the Phase 1 Custom record ID. It stays universal (`entityTypes` omitted).
- Each other preset declares qualified `entityTypes` from the product spec.
- Genre presets declare `genre`. Custom, Culture, and Concept omit it.
- Preset picker: **Suggested** is presets whose `entityTypes` include the
  current entity's qualified type, plus universal presets. **All Presets**
  is the rest, grouped by genre. No separate Custom action, and no fourth genre.
- Migration for existing projects: update the four Phase 1 records in place
  and insert the new presets. Do not insert duplicates.
- Tests: suggestions match qualified type for every builtin Lore type.
  Each builtin type has at least one suggested preset besides Custom.
  Overlay types are suggested only Custom, plus presets that list that
  overlay id. Character presets are not suggested for non-person entities.
  Fantasy Kingdom is suggested for both faction and place.

Do not add user preset management, "Save as Preset", or preset editing in
this phase.

**Exit gate:** when creating a Profile on a faction, Suggested includes the
faction presets and Custom, not the character presets. A place suggests
place presets, including Fantasy Kingdom as a hint. Applying each bundled
preset produces a valid Profile containing the spec's components and not the
example trait lists. Existing Profiles still resolve their Phase 1 origin
IDs. All existing tests pass.

### Phase 3 — User preset management

Deliver:

- "Save as Preset" action in the Profile editor: copies component structure,
  scales, units, formulas, allocation, and starting defaults. It does not
  copy the entity's current progressed values. The author supplies a name,
  optional description, qualified entity types, and optional genre.
- Preset management surface: browse, create blank, duplicate, edit, hide,
  delete, and reset (bundled only) presets.
- Entity-type scoping editor: multi-select which types a preset applies to.
- Genre editor: pick from existing genres or create new labels.
- Bundled presets: may be edited or hidden, but not deleted. Hidden presets
  stay stored and are omitted from the picker. "Reset to Default" restores
  the shipped definition and clears `hidden`.
- User presets: may be renamed, edited, duplicated, and deleted.
  Deletion requires confirmation and does not affect existing Profile
  instances.
- Tests: user preset CRUD, "Save as Preset" copies structure and not current
  values, bundled reset, hide omits a bundled preset from the picker without
  deleting it, deletion does not affect instances, preset picker shows user
  presets.

**Exit gate:** an author can create a Profile on a Faction, customize it,
save it as a preset named "Trade Guild", and use that preset to create
Profiles on other Factions. Bundled presets can be edited and reset.
Deleting a user preset does not corrupt existing Profiles. All existing
tests pass.

---

## Constraints

- Presets are `module_records` on `daena.lore`. No new core tables. The
  project owner scope is a contract extension, not a new table, and it does
  not relax owner checks on entity-owned collections.
- Copy-on-create: creating a Profile from a preset produces an
  independent instance. Preset updates never rewrite instances.
- 64 KiB record cap: a preset with ~50 components is well under 10 KiB.
- Overlay types: user-defined overlay types (e.g. `daena.lore:knightly-order`)
  may be referenced in a preset's `entityTypes`. The picker compares those
  qualified ids with the entity's qualified type. Bare ids do not match.
- Portable project files: preset records serialize through the checkpoint
  path once project-scoped rows are valid without an owner entity.
- Formulas stay in TypeScript and use the existing `{component-id}` syntax.
  Spec formulas are product language. Persist `avg(...)`, not sums, for the
  indexes in sections 10–11. Do not persist a Fuel or Population ratio.

---

## Verification

Use focused tests plus the relevant full checks. The normal command forms
are:

```text
cargo fmt -- --check
cargo test --workspace --locked --offline
cargo clippy --workspace --locked --offline --all-targets -- -D warnings
npm run check
npm run test -- --run scripts/lore-profiles.test.mjs
```

For preset phases also verify:

- existing Profiles survive the presetOrigin migration;
- creating a Profile from each bundled preset produces a valid document;
- deleting all `profile-preset` records and reopening does not crash
  (empty Profile, `presetOrigin` unset);
- preset picker filters by qualified entity type and does not treat bare ids as matches;
- user presets appear in the picker after creation.
