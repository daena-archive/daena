# ADR 0006: Lore Profiles

- Status: Accepted
- Decided: 2026-09-07

## Context

[`LORE_PROFILES.md`](../LORE_PROFILES.md) specifies optional structured
attributes, skills, traits, resources, and related values on any Lore entity,
with Timeline as the author-facing history. Nothing in that spec is shipped.

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

3. **Presets.** Copy-on-create into the instance. Packaged preset updates never
   rewrite instances. Entity-type default templates are optional later, not
   required to attach a Profile.

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
- P1 is current-state Custom profiles on any Lore type, including overlay
  types, with trusted-shell wiki and editor surfaces. P2 copies D&D, Fantasy,
  Sci-Fi, or Custom into the instance and may store an optional point pool.
  Formulas and Timeline fold are later phases. Search indexes, if added, are
  disposable projections over stable component ids.
- Product intent remains [`LORE_PROFILES.md`](../LORE_PROFILES.md). Delivery
  order remains [`LORE_PROFILES_PLAN.md`](../LORE_PROFILES_PLAN.md).
