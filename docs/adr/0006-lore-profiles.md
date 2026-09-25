# ADR 0006: Lore Profiles

- Status: Accepted
- Decided: 2026-09-07
- Updated: 2026-09-25

## Context

[`LORE_PROFILES.md`](../lore/LORE_PROFILES.md) specifies optional structured
attributes, skills, traits, resources, and related values on any Lore entity,
with Timeline as the author-facing history. The current product in that
document is section 27. Planned preset scope is section 28 and
[`plans/profile-presets.md`](../plans/profile-presets.md).

Current storage cannot host it as ordinary fields: `entity_fields` is a
current-value UPSERT, Timeline only projects shared dates, and Fields & Types
cannot absorb a D&D-sized component list. Asset `role: profile` already means
portrait media. Record collections require a closed `ownerEntityTypes` list of
package types, so overlay custom types cannot own a record today. Module
records are capped at 64 KiB.

## Decision

Profiles are Lore meaning on the shared entity graph, not a second database
and not an RPG engine.

1. **Storage.** One `module_records` collection `profile` on `daena.lore`.
   Instance schema, authored component values, and preset origin live on that
   record. Do not persist a fold/display cache. Do not explode components into
   `entity_fields`. Do not add a core Profile table. At most one profile per
   owner is a Lore write-path invariant; core uniqueness remains
   `(module_id, collection, id)`.

2. **Owners.** Any live entity whose **qualified** type (`daena.lore:person`,
   overlay `daena.lore:…`) is in Lore’s **effective** schema. Disabled overlay
   types accept no new profiles; existing records are retained (owner type is
   checked on `record.create` only). Do not add
   Kingdom, Ship, Creature, or similar builtins. Record-owner validation must
   use the merged schema, not only the packaged type list.

3. **Presets.** Copy-on-create into the instance. Bundled or user preset
   updates never rewrite instances. Suggestions are hints; any preset may be
   applied to any Lore entity, and a suggestion is not required to attach a
   Profile. Entity-type suggestions use qualified runtime type ids
   (`daena.lore:person`, overlay `daena.lore:…`). A preset that omits entity
   types is universal. Do not add Kingdom, Ship, Creature, or similar builtins
   to host a preset.

   Preset definitions are project-scoped `profile-preset` records on
   `daena.lore` once that collection ships, not entity-owned records.
   `module_records.owner_entity_id` is `NOT NULL REFERENCES entities(id)`, the
   broker requires a live owner, and checkpoint restore rejects a record whose
   owner entity is missing. A sentinel or hidden entity is not a preset owner:
   it would appear in the entity graph, and deleting it would delete the
   presets. Shipping `profile-preset` requires a project owner scope whose
   rows have no entity owner, including on checkpoint restore. `profile` and
   `profile-change` stay entity-owned.

4. **History.** Authored changelog with optional `eventId`. Fold
   `baseline + changes with date ≤ T` on read. Editing current state updates
   baseline until a changelog exists; once history ships, edits append a
   change and do not rewrite older rows. Timeline events are not a generic
   field mutation engine. Do not overload `involves`. Split `profile-change`
   **before** long history (P4); do not grow a single record past the 64 KiB
   cap.

5. **Derived values.** Persist formula and dependency ids only. Evaluate on
   read. Reject cycles. No formula engine in `daena-core`. Fold and formula
   logic is a pure TypeScript module imported by the trusted shell (Wiki and
   inspectors live there). Core validates coarse record JSON shape only.
   Invalid formulas fail closed and do not corrupt inputs.

6. **Author-facing names.** The structured capability is **Profile**. Portrait
   storage stays `role: profile`; author-facing media copy is **Portrait**.
   Do not use “Sheet” or “Stats” as the product name.

## Rejected alternatives

- Modeling each attribute or skill as a Lore field or overlay field.
- Treating Git history or editor revisions as in-world chronology.
- Pure event sourcing with no Profile record (Timeline cannot reconstruct
  state today and must not become a mutation log for arbitrary fields).
- A formula or fold engine in `daena-core`.
- New first-class entity types for kingdoms, ships, or creatures.
- A sentinel, nil, or hidden entity as the owner of preset records.
- Reusing asset “profile” or RPG “character sheet” as the only UI name.

## Consequences

- Lore must declare `record.read:self` / `record.write:self` and a `profile`
  collection with `uniquePerOwner`. Semantic validation stays in Lore
  TypeScript; broker schema checks remain coarse. One-per-owner is enforced on
  `record.create` inside the core write transaction when that flag is set, not
  by a SQLite unique key.
- Record-owner contracts need an explicit extension so overlay types in Lore
  namespaces can own a Profile. That change begins in the Rust plugin API and
  must compare qualified runtime type ids.
- Current product is Custom, D&D, Fantasy, and Sci-Fi profiles on any Lore
  type, including overlay types, with trusted-shell wiki and editor surfaces,
  copy-on-create presets, an optional point pool, read-time formulas, and
  `profile-change` history folded against Timeline. Search indexes, if added,
  are disposable projections over stable component ids. Those four presets
  remain hardcoded until the delivery plan replaces them with project-scoped
  records.
- Product behavior remains [`LORE_PROFILES.md`](../lore/LORE_PROFILES.md).
  Preset delivery is [`plans/profile-presets.md`](../plans/profile-presets.md).
