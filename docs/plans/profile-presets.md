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
  customized?: boolean;
  allocation?: ProfileAllocation;
  components: ProfileComponent[];
};
```

`entityTypes` holds qualified runtime ids (`daena.lore:person`, or an overlay
id such as `daena.lore:knightly-order`). Omit the field for a universal
preset. Do not store bare ids (`person`) and do not list every builtin type
to mean "all". `genre` is `D&D`, `Fantasy`, `Sci-Fi`, or a user label. Omit
it for ungenred presets. `hidden` applies to bundled presets; user presets
are deleted instead of hidden. `customized` is set only when an author edits
a bundled preset, so later seed updates do not overwrite that copy. Omit it
on shipped seeds and user presets.

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
hidden entity, and do not store a nil UUID or an empty owner string.
`profile` and `profile-change` stay entity-owned. Do not weaken their owner
checks, and do not make `ownerEntityId` optional on those collections.

### Persistence

Today every module record requires an entity owner, in four places that Phase
1 must extend together:

- `RecordOwnerScope` is `package` or `effective-schema` (`daena-plugin-api`,
  `schemas/plugin-manifest-v1.json`, plugin SDK). Add `project`. A project
  collection omits `ownerEntityTypes` and must not set `uniquePerOwner`.
  Regenerate the manifest schema and RPC payloads. Do not hand-edit generated
  files.
- `module_records.owner_entity_id` is `TEXT NOT NULL REFERENCES entities(id)`.
  Project rows store SQL `NULL`. No new table. `CREATE TABLE IF NOT EXISTS`
  will not change an existing database.
- Checkpoint records in `plugins/<plugin-id>.json` require `ownerEntityId` as
  a UUID that exists in the entity set (`plugin.record.owner`, and
  `record owner entity is missing`). Make the field optional. Omit it for
  project rows. A present value must still be a UUID in the entity set.
- Broker `record.create`, `record.list`, `record.update`, and `record.delete`
  require `ownerEntityId` and load that entity. `ModuleContext.records` and
  `packages/module-api` take a required owner UUID. Keep those signatures.
  Add project-scoped methods that omit the owner argument. The shared RPC
  field may be absent only after the broker checks collection scope: entity
  collections still reject a missing owner; project collections reject a
  present owner.

Accept an absent `ownerEntityId` on restore only when the module manifest
declares that collection `ownerScope: project`. If the manifest is
unavailable, reject the row. Do not hardcode `profile-preset` in core.

Do not bump `PROJECT_FORMAT_VERSION` or the exporter contract. Existing
checkpoints keep `ownerEntityId` on entity-owned rows and must still open.
Bump `RUNTIME_SCHEMA_VERSION`. An older runtime is reset-required and rebuilds
from the portable checkpoint. Do not `ALTER` an open database in place, and
do not add a compatibility migration. Ship the optional-field reader in the
same change as the schema bump, and do not change how entity-owned records
are exported.

Entity delete already uses `DELETE FROM module_records WHERE owner_entity_id=?1`.
That statement must not gain a branch that matches `NULL`. `module_record_search`
is derived. Rebuild it. A project row has no owner token that can be parsed
as an entity id. Listing project records must not require an owner match.

Portrait assets (`role: profile`) are not these records.

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

Record ids are UUIDs, unique in `plugins/daena.lore.json`, not only inside
the collection. Do not use `"custom"`, `"dnd"`, `"fantasy"`, or `"scifi"` as
record ids. Those strings stay legacy `presetOrigin` values and map to the
four Phase 1 ids below. Phase 2 keeps those ids and inserts the rest. Do not
insert the Phase 2 ids early, and do not mint a second id for a renamed
character preset.

| Preset | Record ID | Phase |
|---|---|---|
| Custom | `ef2bce76-ca39-59f5-ab9d-28a0b665eb15` | 1 |
| D&D, later D&D Character | `9771b70b-04c4-506e-949b-b26ea5b61235` | 1 |
| Fantasy, later Fantasy Character | `1b2c999f-65b5-57b6-9d02-6ce2c40cc977` | 1 |
| Sci-Fi, later Sci-Fi Character | `d9ff8d59-1cd3-52ae-8adb-5700fbff6ba1` | 1 |
| D&D Faction | `e4baa36f-9591-5eb3-aa6c-96728fe77e61` | 2 |
| D&D Location | `f7f1d532-b04f-5ef6-b440-eb4e21ab6283` | 2 |
| Fantasy Faction | `b324c23c-aba7-5734-9a21-84894ffa93c5` | 2 |
| Fantasy Kingdom | `c1b642b3-f218-53e7-a8c5-db41c327ec51` | 2 |
| Fantasy Place | `a8d73270-c9a6-5e5a-8ed7-3bd22704b725` | 2 |
| Fantasy Artifact | `e9757343-bf50-5326-ae6f-915cd6894ab6` | 2 |
| Sci-Fi Faction | `25dae462-323c-5e93-8322-d7d42d2a9733` | 2 |
| Starship | `6f753e3f-342c-583d-88dc-9cdc55cb42ef` | 2 |
| Colony / Station | `3aa17fe2-dd28-50ba-b54a-2e7da9c4a65d` | 2 |
| Culture | `6fefd437-d376-5772-a5c4-7ebf00da0a69` | 2 |
| Concept | `26bf13cb-1970-5bfc-a728-69cabc9b47f0` | 2 |

---

## Delivery phases

Only one phase may be implemented at a time. A phase is not complete
because types compile or unit tests pass; every stated exit gate must have
evidence.

### Phase 1 — Data-driven preset infrastructure

Deliver:

- Project owner scope, as specified under Persistence. No sentinel entity,
  nil UUID, or empty owner. No portable-format bump. Bump
  `RUNTIME_SCHEMA_VERSION` and rebuild; do not alter an open database.
- `profile-preset` collection declared in the Lore manifest with
  `ownerScope: project` and capability grants.
- `ProfilePresetDocument` type definition in TypeScript.
- Preset CRUD through new project-scoped record methods. Do not widen
  `ModuleContext.records` so entity-owned calls can omit an owner.
- Bundled preset seeding: on project creation, seed the existing 4 presets
  (Custom, D&D, Fantasy, Sci-Fi — person-oriented, matching current
  hardcoded definitions) as `profile-preset` records with `builtin: true`.
  Use the four Phase 1 record IDs. Do not insert the Phase 2 ids. Omit
  `entityTypes` and `genre` in this phase so every entity still sees the
  same four choices.
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
from project-scoped records with no owner entity in the graph and no
`ownerEntityId` in `plugins/daena.lore.json`. Creating a Profile from
D&D/Fantasy/Sci-Fi/Custom produces the same result as before. Legacy
`presetOrigin` values survive migration and resolve to the Phase 1 record
IDs. Deleting an entity does not delete presets. A profile or profile-change
row with a missing owner still fails restore. An older runtime schema version
is reset-required; a clean checkpoint rebuilds entity-owned profile rows
unchanged. All existing profile and profile-change tests pass.

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
- Portable project files: preset records serialize through
  `plugins/daena.lore.json` with `ownerEntityId` omitted. Entity-owned
  records in that file still carry `ownerEntityId`.
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
- a seeded preset round-trips with `ownerEntityId` omitted, including after
  deleting `.daena/` and reopening the checkpoint;
- an entity-owned record with a missing owner still fails restore;
- deleting all `profile-preset` records and reopening does not crash
  (empty Profile, `presetOrigin` unset);
- preset picker filters by qualified entity type and does not treat bare ids as matches;
- user presets appear in the picker after creation.
